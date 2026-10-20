"""Filter parsing and evaluation tests (SPEC.md 6.4.1, 6.4.5, 8.2, Annex B.2)."""

import pytest

from adc.errors import AdcError
from adc.filters import (
    FilterSpec,
    coerce_value,
    conditions_from_query,
    conditions_match,
    filter_specs,
    parse_post_filter,
    post_filter_match,
)

BRAND = FilterSpec(name="brand", type="string", ops=("eq", "in", "prefix"))
PRICE = FilterSpec(
    name="price", type="number", ops=("eq", "lt", "lte", "gt", "gte")
)
FEATURE = FilterSpec(
    name="feature",
    type="string",
    ops=("eq", "in", "contains", "contains_all", "contains_any"),
)
STOCK = FilterSpec(name="stock", type="number", ops=("eq", "exists"))
SPECS = {"brand": BRAND, "price": PRICE, "feature": FEATURE, "stock": STOCK}


# --------------------------------------------------------------------------
# GET grammar (SPEC 6.4.1)
# --------------------------------------------------------------------------


def test_get_bare_filter_is_implicit_eq():
    conditions = conditions_from_query({"filter[brand]": ["sony"]}, SPECS)
    assert len(conditions) == 1
    assert conditions[0].op == "eq"
    assert conditions[0].value == "sony"


def test_get_range_conditions_are_separate_and_anded():
    conditions = conditions_from_query(
        {"filter[price][gte]": ["50"], "filter[price][lte]": ["150"]}, SPECS
    )
    assert [c.op for c in conditions] == ["gte", "lte"]
    assert conditions_match(conditions, {"price": 100}) is True
    assert conditions_match(conditions, {"price": 200}) is False
    assert conditions_match(conditions, {"price": 10}) is False


def test_repeated_in_parameters_form_a_list():
    # SPEC 6.4.1: in/nin use repeated parameters, never comma lists.
    conditions = conditions_from_query(
        {"filter[brand][in]": ["Sony", "Apple"]}, SPECS
    )
    assert conditions[0].op == "in"
    assert conditions[0].value == ["Sony", "Apple"]
    assert conditions_match(conditions, {"brand": "Sony"}) is True


def test_comma_separated_in_value_is_not_split():
    # SPEC 6.4.1: comma-separated lists are NOT used; "Sony,Apple" is one
    # literal value that matches no brand.
    conditions = conditions_from_query(
        {"filter[brand][in]": ["Sony,Apple"]}, SPECS
    )
    assert conditions[0].value == ["Sony,Apple"]
    assert conditions_match(conditions, {"brand": "Sony"}) is False
    assert conditions_match(conditions, {"brand": "Apple"}) is False


def test_unknown_filter_field_reports_unknown_fields():
    # SPEC 6.4.1 / CONFORMANCE F.5.4.
    with pytest.raises(AdcError) as exc:
        conditions_from_query({"filter[color]": ["red"]}, SPECS)
    assert exc.value.code == "FILTER_INVALID"
    assert exc.value.status == 400
    assert exc.value.details["unknown_fields"] == ["color"]
    assert "allowed" in exc.value.details


def test_operator_not_declared_for_field_reports_unsupported_ops():
    # SPEC 6.4.1 / CONFORMANCE F.5.5: brand does not declare lt.
    with pytest.raises(AdcError) as exc:
        conditions_from_query({"filter[brand][lt]": ["1"]}, SPECS)
    assert exc.value.code == "FILTER_INVALID"
    assert exc.value.details["unsupported_ops"] == ["lt"]


def test_operator_outside_vocabulary_reports_unsupported_ops():
    with pytest.raises(AdcError) as exc:
        conditions_from_query({"filter[price][wat]": ["1"]}, SPECS)
    assert exc.value.code == "FILTER_INVALID"
    assert exc.value.details["unsupported_ops"] == ["wat"]


def test_exists_takes_true_or_false():
    # SPEC 6.4.1: exists takes true/false.
    conditions = conditions_from_query(
        {"filter[stock][exists]": ["true"]}, SPECS
    )
    assert conditions[0].value is True
    conditions = conditions_from_query(
        {"filter[stock][exists]": ["FALSE"]}, SPECS
    )
    assert conditions[0].value is False


def test_exists_rejects_non_boolean():
    with pytest.raises(AdcError) as exc:
        conditions_from_query({"filter[stock][exists]": ["maybe"]}, SPECS)
    assert exc.value.code == "FILTER_INVALID"


def test_condition_count_limit_is_enforced():
    # SPEC 6.4.1 / 8.2: condition count MUST NOT exceed
    # limits.max_filter_conditions.
    params = {
        "filter[price][eq]": ["1"],
        "filter[price][lt]": ["2"],
        "filter[price][lte]": ["3"],
    }
    with pytest.raises(AdcError) as exc:
        conditions_from_query(params, SPECS, max_conditions=2)
    assert exc.value.code == "FILTER_INVALID"
    assert exc.value.details["limit"] == 2


def test_scalar_operator_rejects_repeated_values():
    with pytest.raises(AdcError) as exc:
        conditions_from_query({"filter[price][eq]": ["1", "2"]}, SPECS)
    assert exc.value.code == "FILTER_INVALID"


# --------------------------------------------------------------------------
# Type coercion (SPEC 6.4.1)
# --------------------------------------------------------------------------


def test_number_coercion_accepts_numeric_strings():
    assert coerce_value(PRICE, "12.5") == 12.5
    assert coerce_value(PRICE, 42) == 42.0


def test_number_coercion_rejects_garbage():
    with pytest.raises(AdcError) as exc:
        coerce_value(PRICE, "abc")
    assert exc.value.code == "FILTER_INVALID"


def test_number_coercion_rejects_boolean():
    with pytest.raises(AdcError) as exc:
        coerce_value(PRICE, True)
    assert exc.value.code == "FILTER_INVALID"


def test_boolean_coercion_accepts_strings():
    bool_spec = FilterSpec(name="flag", type="boolean", ops=("eq",))
    assert coerce_value(bool_spec, "true") is True
    assert coerce_value(bool_spec, "False") is False
    assert coerce_value(bool_spec, False) is False


def test_boolean_coercion_rejects_garbage():
    bool_spec = FilterSpec(name="flag", type="boolean", ops=("eq",))
    with pytest.raises(AdcError) as exc:
        coerce_value(bool_spec, "maybe")
    assert exc.value.code == "FILTER_INVALID"


# --------------------------------------------------------------------------
# Passport filter descriptors (SPEC 6.3)
# --------------------------------------------------------------------------


def test_plain_string_descriptor_means_eq_only():
    # SPEC 6.3: a v0.1 plain string array means "eq only".
    specs = filter_specs(["brand"])
    assert specs["brand"].ops == ("eq",)


def test_duplicate_filter_descriptor_rejected():
    with pytest.raises(AdcError) as exc:
        filter_specs(
            [
                {"name": "brand", "type": "string", "ops": ["eq"]},
                {"name": "brand", "type": "string", "ops": ["eq"]},
            ]
        )
    assert exc.value.code == "FILTER_INVALID"


def test_descriptor_with_unknown_type_rejected():
    with pytest.raises(AdcError) as exc:
        filter_specs([{"name": "x", "type": "colour", "ops": ["eq"]}])
    assert exc.value.code == "FILTER_INVALID"


# --------------------------------------------------------------------------
# POST boolean tree (SPEC 6.4.5)
# --------------------------------------------------------------------------


def test_post_and_evaluates_both_sides():
    tree = parse_post_filter(
        {"and": [{"brand": {"eq": "Sony"}}, {"price": {"lte": 200}}]}, SPECS
    )
    assert post_filter_match(tree, {"brand": "Sony", "price": 100}) is True
    assert post_filter_match(tree, {"brand": "Sony", "price": 300}) is False
    assert post_filter_match(tree, {"brand": "Bose", "price": 100}) is False


def test_post_or_evaluates_either_side():
    tree = parse_post_filter(
        {"or": [{"brand": {"eq": "Sony"}}, {"brand": {"eq": "Bose"}}]}, SPECS
    )
    assert post_filter_match(tree, {"brand": "Sony"}) is True
    assert post_filter_match(tree, {"brand": "Bose"}) is True
    assert post_filter_match(tree, {"brand": "Apple"}) is False


def test_post_not_negates_child():
    tree = parse_post_filter({"not": {"brand": {"eq": "Sony"}}}, SPECS)
    assert post_filter_match(tree, {"brand": "Sony"}) is False
    assert post_filter_match(tree, {"brand": "Bose"}) is True


def test_post_implicit_eq_from_scalar_condition():
    tree = parse_post_filter({"brand": "Sony"}, SPECS)
    assert tree.kind == "cond"
    assert tree.op == "eq"
    assert post_filter_match(tree, {"brand": "Sony"}) is True


def test_post_contains_all_on_list_field():
    tree = parse_post_filter(
        {"feature": {"contains_all": ["anc", "wireless"]}}, SPECS
    )
    assert post_filter_match(tree, {"feature": ["anc", "wireless", "mic"]}) is True
    assert post_filter_match(tree, {"feature": ["anc"]}) is False


def test_post_none_filter_matches_everything():
    assert post_filter_match(None, {"anything": 1}) is True


def test_post_unknown_field_rejected():
    with pytest.raises(AdcError) as exc:
        parse_post_filter({"colour": {"eq": "red"}}, SPECS)
    assert exc.value.code == "FILTER_INVALID"
    assert exc.value.details["unknown_fields"] == ["colour"]


def test_post_undeclared_op_rejected():
    with pytest.raises(AdcError) as exc:
        parse_post_filter({"brand": {"lt": "x"}}, SPECS)
    assert exc.value.code == "FILTER_INVALID"
    assert exc.value.details["unsupported_ops"] == ["lt"]


def test_post_exists_requires_boolean_operand():
    with pytest.raises(AdcError) as exc:
        parse_post_filter({"stock": {"exists": "true"}}, SPECS)
    assert exc.value.code == "FILTER_INVALID"


def test_post_list_op_requires_array_operand():
    with pytest.raises(AdcError) as exc:
        parse_post_filter({"brand": {"in": "Sony"}}, SPECS)
    assert exc.value.code == "FILTER_INVALID"


def test_post_node_must_hold_exactly_one_key():
    with pytest.raises(AdcError) as exc:
        parse_post_filter({"brand": {"eq": "x"}, "price": {"eq": 1}}, SPECS)
    assert exc.value.code == "FILTER_INVALID"


def test_post_depth_over_max_filter_depth_rejected():
    # SPEC 8.2: "Publishers MUST enforce limits.max_filter_depth (default 4),
    # limits.max_filter_conditions (default 32)".
    node = {"brand": {"eq": "x"}}
    for _ in range(4):
        node = {"and": [node]}  # depth 5 > max_filter_depth 4
    with pytest.raises(AdcError) as exc:
        parse_post_filter(node, SPECS, max_depth=4, max_conditions=32)
    assert exc.value.code == "FILTER_INVALID"


def test_post_more_than_max_conditions_rejected():
    # SPEC 8.2: "Publishers MUST enforce limits.max_filter_depth (default 4),
    # limits.max_filter_conditions (default 32)".
    conds = [{"brand": {"eq": "x%d" % i}} for i in range(33)]
    with pytest.raises(AdcError) as exc:
        parse_post_filter({"and": conds}, SPECS, max_depth=4, max_conditions=32)
    assert exc.value.code == "FILTER_INVALID"
