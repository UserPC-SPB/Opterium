"""Aggregate grammar, computation, and k-anonymity privacy tests
(SPEC.md 6.9, 8.4, 8.5, Annex B.3, CONFORMANCE.md F.2, F.6)."""

import pytest

from adc.aggregate import (
    AggConfig,
    compute_aggregate,
    parse_group_by,
    parse_having,
    parse_metric,
    parse_metrics,
    parse_order_by,
)
from adc.errors import AdcError

HEADPHONES = "/electronics/audio/headphones"


# --------------------------------------------------------------------------
# Metric grammar (Annex B.3, CONFORMANCE F.6.1)
# --------------------------------------------------------------------------


def test_metric_count_without_field():
    metric = parse_metric("count()")
    assert metric.func == "count"
    assert metric.field_name is None
    assert metric.alias == "count_all"


def test_metric_sum_field_default_alias():
    metric = parse_metric("sum(price)")
    assert metric.func == "sum"
    assert metric.field_name == "price"
    assert metric.alias == "sum_price"


def test_metric_percentile_with_alias():
    metric = parse_metric("p95(price):p95_price")
    assert metric.func == "p95"
    assert metric.alias == "p95_price"


def test_metric_rejects_unknown_function():
    with pytest.raises(AdcError) as exc:
        parse_metric("nosuch()")
    assert exc.value.code == "INVALID_AGGREGATION"
    assert "nosuch" in exc.value.details["unsupported"]


def test_metric_rejects_missing_field_argument():
    # Annex B.3: field-name is optional only for count.
    with pytest.raises(AdcError) as exc:
        parse_metric("avg()")
    assert exc.value.code == "INVALID_AGGREGATION"


def test_metric_rejects_malformed_expression():
    for bad in ("count", "count(price", "1sum(price)", "sum(price):"):
        with pytest.raises(AdcError) as exc:
            parse_metric(bad)
        assert exc.value.code == "INVALID_AGGREGATION"


def test_parse_metrics_get_form():
    metrics = parse_metrics("count(),sum(price)")
    assert [m.alias for m in metrics] == ["count_all", "sum_price"]


def test_duplicate_metric_aliases_rejected():
    # CONFORMANCE F.6.1 requires aliases; duplicates would collide in values.
    with pytest.raises(AdcError) as exc:
        parse_metrics("sum(price):total,avg(price):total")
    assert exc.value.code == "INVALID_AGGREGATION"
    assert exc.value.details["duplicates"] == ["total"]


def test_metrics_required():
    with pytest.raises(AdcError) as exc:
        parse_metrics(None)
    assert exc.value.code == "INVALID_AGGREGATION"


def test_parse_group_by_accepts_comma_list():
    assert parse_group_by("brand,feature") == ["brand", "feature"]
    assert parse_group_by(None) == []


def test_parse_group_by_rejects_bad_field():
    with pytest.raises(AdcError) as exc:
        parse_group_by("brand space")
    assert exc.value.code == "INVALID_AGGREGATION_TARGET"


def test_parse_order_by_marks_descending():
    assert parse_order_by("-price") == [("price", True)]
    assert parse_order_by("brand") == [("brand", False)]


def test_parse_having_string_form():
    # SPEC 6.9.1: having is alias:op:value.
    assert parse_having("count:gt:5") == [("count", "gt", 5)]


# --------------------------------------------------------------------------
# Computation and privacy (SPEC 8.4 / 8.5, CONFORMANCE F.2)
# --------------------------------------------------------------------------


def _mixture_records():
    # CONFORMANCE F.2 vector: groups A=10, B=2 with k=5.
    rows = []
    rows.extend({"grp": "A", "price": 10.0} for _ in range(10))
    rows.extend({"grp": "B", "price": 20.0} for _ in range(2))
    return rows


def test_f2_large_group_is_emitted():
    config = AggConfig(
        metrics=[parse_metric("count()")], group_by=["grp"], min_group_size=5
    )
    result = compute_aggregate(_mixture_records(), config)
    keys = [g["key"]["grp"] for g in result["groups"]]
    assert keys == ["A"]
    assert result["groups"][0]["values"]["count_all"] == 10


def test_f2_small_group_is_suppressed():
    config = AggConfig(
        metrics=[parse_metric("count()")], group_by=["grp"], min_group_size=5
    )
    result = compute_aggregate(_mixture_records(), config)
    assert result["suppressed_groups"] == 1
    assert result["partial"] is True


def test_f2_totals_cover_visible_groups_only():
    # CONFORMANCE F.2.4 / SPEC 8.5: totals must not fold in suppressed data.
    config = AggConfig(
        metrics=[parse_metric("count()")], group_by=["grp"], min_group_size=5
    )
    result = compute_aggregate(_mixture_records(), config)
    assert result["totals"] == {"count_all": 10}
    assert result["totals_visible_only"] is True


def test_coverage_never_exceeds_one(site):
    node = site.resolve(path=HEADPHONES)
    records = site.records(node)
    config = AggConfig(
        metrics=[parse_metric("count()")], group_by=["brand"], min_group_size=5
    )
    result = compute_aggregate(records, config)
    assert 0.0 < result["coverage"] <= 1.0


def test_ungrouped_aggregate_returns_values_without_groups(site):
    # SPEC 6.9.4: ungrouped responses carry top-level values, never groups.
    node = site.resolve(path=HEADPHONES)
    records = site.records(node)
    config = AggConfig(metrics=[parse_metric("count()")])
    result = compute_aggregate(records, config)
    assert result["values"] == {"count_all": 48}
    assert "groups" not in result
    assert result["coverage"] == 1.0


def test_all_groups_suppressed_omits_totals(site):
    # SPEC 8.5: totals over zero visible groups must be omitted entirely,
    # otherwise differencing would recover suppressed data.
    node = site.resolve(path=HEADPHONES)
    records = site.records(node)
    config = AggConfig(
        metrics=[parse_metric("count()")],
        group_by=["brand"],
        min_group_size=40,
    )
    result = compute_aggregate(records, config)
    assert result["groups"] == []
    assert "totals" not in result


# --------------------------------------------------------------------------
# End-to-end privacy over the example collection
# --------------------------------------------------------------------------


def test_small_brand_groups_suppressed(client):
    response = client.getq(
        "/adc/aggregate", path=HEADPHONES, metrics="count()", group_by="brand"
    )
    assert response.status == 200
    body = response.json
    sizes = [g["values"]["count_all"] for g in body["groups"]]
    # SPEC 8.4.1: groups with 0 < count < k are suppressed (k = 5 default).
    assert all(size >= 5 for size in sizes)
    assert body["suppressed_groups"] >= 1
    assert body["partial"] is True
    brands = [g["key"]["brand"] for g in body["groups"]]
    assert "Apple" not in brands  # Apple has only 4 items in the example


def test_totals_flagged_visible_only_when_groups_suppressed(client):
    # SPEC 8.5: suppressed_groups > 0 implies totals_visible_only: true
    # (or totals omitted).
    response = client.getq(
        "/adc/aggregate", path=HEADPHONES, metrics="count()", group_by="brand"
    )
    body = response.json
    assert body["suppressed_groups"] > 0
    visible_sum = sum(g["values"]["count_all"] for g in body["groups"])
    assert body["totals"]["count_all"] == visible_sum
    assert body["totals_visible_only"] is True


def test_all_groups_suppressed_response_has_no_totals_key(client):
    # SPEC 8.5: with every group suppressed there are no visible groups, so
    # totals must be absent rather than zero.
    response = client.getq(
        "/adc/aggregate",
        path=HEADPHONES,
        metrics="count()",
        group_by="brand",
        min_group_size=40,
    )
    assert response.status == 200
    body = response.json
    assert body["groups"] == []
    assert "totals" not in body
    assert body["suppressed_groups"] >= 1
    assert body["partial"] is True


def test_grouped_aggregate_coverage_within_one(client):
    response = client.getq(
        "/adc/aggregate", path=HEADPHONES, metrics="count()", group_by="brand"
    )
    assert response.json["coverage"] <= 1.0


def test_ungrouped_aggregate_shape_over_http(client):
    # SPEC 6.9.4 / CONFORMANCE F.6.6.
    response = client.getq("/adc/aggregate", path=HEADPHONES, metrics="count()")
    assert response.status == 200
    body = response.json
    assert body["values"]["count_all"] == 48
    assert "groups" not in body


def test_min_group_size_below_floor_is_rejected(client):
    # SPEC 8.5: "The min_group_size request parameter is a publisher-enforced
    # floor: agents MAY request higher, servers MUST reject lower with 400
    # FILTER_INVALID and details.privacy_floor."
    response = client.getq(
        "/adc/aggregate",
        path=HEADPHONES,
        metrics="count()",
        group_by="brand",
        min_group_size=1,
    )
    assert response.status == 400
    assert response.json["type"].endswith("/FILTER_INVALID")
    assert response.json["details"]["privacy_floor"] == 5
