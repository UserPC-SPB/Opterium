"""Anchor issuance, wire grammar, and agent-local arithmetic
(SPEC.md 5.3, Annex B.1, CONFORMANCE.md F.1, F.14)."""

import pytest

from adc.anchors import (
    AnchorError,
    AnchorTable,
    NamespacedAnchor,
    normalize_origin,
    parse_wire_anchor,
    resolve_suffix,
    rollback_to,
    validate_namespace,
)
from adc.errors import AdcError

PATHS = [
    "/",
    "/electronics",
    "/electronics/audio",
    "/electronics/audio/headphones",
    "/electronics/audio/speakers",
]


# --------------------------------------------------------------------------
# Issuance (SPEC 5.3.1 idempotent issuance)
# --------------------------------------------------------------------------


def test_issuance_is_idempotent_across_builds():
    first = AnchorTable.from_paths("gen-1", PATHS)
    second = AnchorTable.from_paths("gen-1", list(reversed(PATHS)))
    assert first.snapshot() == second.snapshot()


def test_same_path_always_yields_same_anchor():
    table = AnchorTable.from_paths("gen-1", PATHS)
    other = AnchorTable.from_paths("gen-1", PATHS)
    path = "/electronics/audio/headphones"
    assert table.anchor_for(path) == table.anchor_for(path)
    assert table.anchor_for(path) == other.anchor_for(path)


def test_unknown_anchor_is_anchor_not_found():
    # SPEC 5.3.3: unknown anchor is 404 ANCHOR_NOT_FOUND.
    table = AnchorTable.from_paths("gen-1", PATHS)
    with pytest.raises(AdcError) as exc:
        table.path_for("[a99999]")
    assert exc.value.code == "ANCHOR_NOT_FOUND"
    assert exc.value.status == 404


def test_unknown_path_is_anchor_not_found_with_path_detail():
    table = AnchorTable.from_paths("gen-1", PATHS)
    with pytest.raises(AdcError) as exc:
        table.anchor_for("/no/such/path")
    assert exc.value.code == "ANCHOR_NOT_FOUND"
    assert exc.value.details.get("path") == "/no/such/path"


def test_table_is_bound_to_tree_generation():
    table = AnchorTable.from_paths("gen-1", PATHS)
    assert table.tree_generation == "gen-1"


def test_resolve_requires_exactly_one_input():
    table = AnchorTable.from_paths("gen-1", PATHS)
    with pytest.raises(AdcError) as both:
        table.resolve(anchor="[a1]", path="/")
    assert both.value.code == "ANCHOR_INVALID"
    with pytest.raises(AdcError) as neither:
        table.resolve(anchor=None, path=None)
    assert neither.value.code == "ANCHOR_INVALID"


# --------------------------------------------------------------------------
# Wire grammar (Annex B.1)
# --------------------------------------------------------------------------


def test_wire_plain_anchor_parses():
    parsed = parse_wire_anchor("[a10]")
    assert parsed.anchor_id == 10
    assert parsed.rollback is None
    assert parsed.suffix is None


def test_wire_rollback_parses():
    parsed = parse_wire_anchor("[a10]-2")
    assert parsed.anchor_id == 10
    assert parsed.rollback == 2


def test_wire_zero_rollback_rejected():
    # Annex B.1: rollback = "-" 1*DIGIT with k >= 1; SPEC 5.3.2: [aN]-0 invalid.
    with pytest.raises(AdcError) as exc:
        parse_wire_anchor("[a10]-0")
    assert exc.value.code == "ANCHOR_INVALID"


def test_wire_suffix_parses():
    parsed = parse_wire_anchor("[a10]/beta")
    assert parsed.suffix == "beta"
    parsed = parse_wire_anchor("[a10]/beta/gamma")
    assert parsed.suffix == "beta/gamma"


@pytest.mark.parametrize("bad", ["a10", "[a10", "[x]", "[a10]x", "[a10]/", ""])
def test_malformed_wire_anchor_rejected(bad):
    with pytest.raises(AdcError) as exc:
        parse_wire_anchor(bad)
    assert exc.value.code == "ANCHOR_INVALID"


def test_rollback_and_suffix_are_agent_local_not_wire():
    # SPEC 5.3.2: derived expressions never reach the server as-is; the
    # server resolves bare anchors only.
    table = AnchorTable.from_paths("gen-1", PATHS)
    with pytest.raises(AdcError) as exc:
        table.path_for("[a2]-1")
    assert exc.value.code == "ANCHOR_INVALID"


# --------------------------------------------------------------------------
# Namespace validation (SPEC 5.6, CONFORMANCE F.14)
# --------------------------------------------------------------------------


def test_validate_namespace_accepts_matching_origin():
    raw = validate_namespace(
        "https://shop.example.com#[a7]", "https://shop.example.com"
    )
    assert raw == "[a7]"


def test_validate_namespace_rejects_foreign_origin():
    # SPEC 5.6: a runtime MUST abort locally on a mismatched namespace.
    with pytest.raises(AnchorError) as exc:
        validate_namespace(
            "https://evil.example#[a7]", "https://shop.example.com"
        )
    assert exc.value.code == "NAMESPACE_MISMATCH"


def test_validate_namespace_passes_bare_anchor_through():
    assert validate_namespace("[a7]", "https://shop.example.com") == "[a7]"


def test_normalize_origin_elides_default_port():
    # Annex B.1: default ports elided per RFC 3986.
    assert normalize_origin("https://x.example:443") == "https://x.example"
    assert normalize_origin("http://x.example:80") == "http://x.example"


def test_normalize_origin_lowercases_host():
    assert normalize_origin("https://X.Example") == "https://x.example"


def test_normalize_origin_rejects_garbage():
    with pytest.raises(AnchorError) as exc:
        normalize_origin("not-an-origin")
    assert exc.value.code == "NAMESPACE_INVALID"


# --------------------------------------------------------------------------
# Suffix resolution (SPEC 5.3.2, CONFORMANCE F.1.4)
# --------------------------------------------------------------------------

SUFFIX_PATHS = ["/a", "/a/b", "/a/b/x", "/a/b/y", "/a/c"]


def test_suffix_resolves_unique_descendant():
    assert resolve_suffix("/a", "b/x", SUFFIX_PATHS) == "/a/b/x"


def test_suffix_multi_segment_resolves():
    assert resolve_suffix("/a", "c", SUFFIX_PATHS) == "/a/c"


def test_suffix_without_match_is_no_such_child():
    with pytest.raises(AnchorError) as exc:
        resolve_suffix("/a", "zzz", SUFFIX_PATHS)
    assert exc.value.code == "NO_SUCH_CHILD"


def test_ambiguous_suffix_is_ambiguous_child():
    # CONFORMANCE F.1.4: a suffix matching multiple anchors must fail.
    with pytest.raises(AnchorError) as exc:
        resolve_suffix("/a", "b", SUFFIX_PATHS)
    assert exc.value.code == "AMBIGUOUS_CHILD"


def test_empty_suffix_is_no_such_child():
    with pytest.raises(AnchorError) as exc:
        resolve_suffix("/a", "", SUFFIX_PATHS)
    assert exc.value.code == "NO_SUCH_CHILD"


# --------------------------------------------------------------------------
# Rollback (SPEC 5.3.2, Annex A.1, CONFORMANCE F.1)
# --------------------------------------------------------------------------

DEPTHS = {"[a30]": 3}


def test_rollback_zero_is_invalid():
    # CONFORMANCE F.1.3: rollback delta zero must be rejected.
    with pytest.raises(AnchorError) as exc:
        rollback_to("[a30]", 0, DEPTHS)
    assert exc.value.code == "ROLLBACK_INVALID"


def test_negative_rollback_is_invalid():
    with pytest.raises(AnchorError) as exc:
        rollback_to("[a30]", -1, DEPTHS)
    assert exc.value.code == "ROLLBACK_INVALID"


def test_rollback_beyond_depth_is_no_such_ancestor():
    # CONFORMANCE F.1.1: rolling back past the anchor depth must error.
    with pytest.raises(AnchorError) as exc:
        rollback_to("[a30]", 4, DEPTHS)
    assert exc.value.code == "NO_SUCH_ANCESTOR"


def test_rollback_to_depth_succeeds():
    # CONFORMANCE F.1.2: rolling back exactly to the root must succeed.
    assert rollback_to("[a30]", 3, DEPTHS) == "[a30]-3"


def test_rollback_partial_depth_succeeds():
    assert rollback_to("[a30]", 1, DEPTHS) == "[a30]-1"


def test_rollback_unknown_anchor_is_no_such_ancestor():
    with pytest.raises(AnchorError) as exc:
        rollback_to("[a777]", 1, DEPTHS)
    assert exc.value.code == "NO_SUCH_ANCESTOR"


# --------------------------------------------------------------------------
# Namespaced anchors (SPEC 5.6, Annex B.1 agent expression grammar)
# --------------------------------------------------------------------------


def test_namespaced_anchor_parse_and_wire_stripping():
    # CONFORMANCE F.14.2: the namespace never reaches the wire.
    parsed = NamespacedAnchor.parse("https://x.example#[a7]=alias")
    assert parsed.origin == "https://x.example"
    assert parsed.to_wire() == "[a7]"
    assert parsed.alias == "alias"
    assert parsed.to_expression() == "https://x.example#[a7]=alias"


def test_namespaced_anchor_without_alias():
    parsed = NamespacedAnchor.parse("https://x.example#[a7]")
    assert parsed.alias is None
    assert parsed.to_wire() == "[a7]"


def test_namespaced_anchor_rejects_bad_alias():
    with pytest.raises(AnchorError) as exc:
        NamespacedAnchor.parse("https://x.example#[a7]=bad!alias")
    assert exc.value.code == "ALIAS_INVALID"


def test_namespaced_anchor_rejects_missing_namespace():
    with pytest.raises(AnchorError) as exc:
        NamespacedAnchor.parse("[a7]")
    assert exc.value.code == "NAMESPACE_INVALID"
