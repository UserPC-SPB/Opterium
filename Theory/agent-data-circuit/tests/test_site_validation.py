"""Structural site validation tests (SPEC.md 5.3.1, 5.4, 6.3, Annex E)."""

import json
import shutil

import pytest

from adc.errors import AdcError
from adc.site import Site, canonical_bytes, check_site, payload_stats
from adc.tokens import count_tokens, is_exact


def _copy_site(source, destination):
    shutil.copytree(source, destination)
    return destination


def _load(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def _dump(path, payload):
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, ensure_ascii=False, indent=2)


def _assert_problem(problems, *needles):
    assert problems, "expected at least one validation problem"
    joined = "\n".join(problems)
    for needle in needles:
        assert needle in joined, "missing %r in problems:\n%s" % (needle, joined)


def test_example_site_is_conforming(example_site_dir):
    assert check_site(example_site_dir) == []


def test_bad_anchor_in_tree_is_detected(example_site_dir, tmp_path):
    # SPEC 5.3.1: issuance is a pure function; a declared anchor that does
    # not match issuance breaks idempotency.
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    tree = _load(site_dir / "tree.json")
    for node in tree["nodes"]:
        if node["path"] == "/electronics/audio/headphones":
            node["anchor"] = "[a999]"
    _dump(site_dir / "tree.json", tree)
    problems = check_site(site_dir)
    _assert_problem(problems, "ANCHOR_INVALID", "issuance")


def test_wrong_slice_tokens_are_detected(example_site_dir, tmp_path):
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    node_path = site_dir / "nodes" / "a30.json"
    payload = _load(node_path)
    for descriptor in payload["passport"]["slices"]:
        if descriptor["name"] == "summary":
            descriptor["tokens"] = 1
    _dump(node_path, payload)
    problems = check_site(site_dir)
    _assert_problem(problems, "declared tokens 1 != computed")


def test_two_default_slices_are_detected(example_site_dir, tmp_path):
    # SPEC 6.3: exactly one slice MUST carry "default": true.
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    node_path = site_dir / "nodes" / "a31.json"
    payload = _load(node_path)
    for descriptor in payload["passport"]["slices"]:
        if descriptor["name"] == "specs":
            descriptor["default"] = True
    _dump(node_path, payload)
    problems = check_site(site_dir)
    _assert_problem(problems, "expected exactly one default slice, found 2")


def test_collection_count_mismatch_is_detected(example_site_dir, tmp_path):
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    tree = _load(site_dir / "tree.json")
    for node in tree["nodes"]:
        if node["path"] == "/electronics/audio/headphones":
            node["count"] = 47
    _dump(site_dir / "tree.json", tree)
    problems = check_site(site_dir)
    _assert_problem(problems, "count 47 != 48 summary items")


def test_item_path_must_be_direct_child(example_site_dir, tmp_path):
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    node_path = site_dir / "nodes" / "a30.json"
    payload = _load(node_path)
    payload["slices"]["summary"]["items"][0]["path"] = (
        "/computers/laptops/dell-xps-13"
    )
    _dump(node_path, payload)
    problems = check_site(site_dir)
    _assert_problem(problems, "is not a direct child")


def test_missing_node_file_is_detected(example_site_dir, tmp_path):
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    (site_dir / "nodes" / "a31.json").unlink()
    problems = check_site(site_dir)
    _assert_problem(problems, "missing site file", "a31.json")


def test_duplicate_node_path_is_detected(example_site_dir, tmp_path):
    # SPEC 5.4: a path identifies exactly one node in the tree.
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    tree = _load(site_dir / "tree.json")
    duplicate = None
    for node in tree["nodes"]:
        if node["path"] == "/electronics/audio/headphones":
            duplicate = dict(node)
            break
    assert duplicate is not None
    tree["nodes"].append(duplicate)
    _dump(site_dir / "tree.json", tree)
    problems = check_site(site_dir)
    _assert_problem(problems, "duplicate node path")


def test_tree_generation_mismatch_is_detected(example_site_dir, tmp_path):
    site_dir = _copy_site(example_site_dir, tmp_path / "site")
    tree = _load(site_dir / "tree.json")
    tree["tree_generation"] = "1999-01-01T00:00:00Z"
    _dump(site_dir / "tree.json", tree)
    problems = check_site(site_dir)
    _assert_problem(problems, "GENERATION_STALE")


def test_site_load_raises_for_missing_directory(tmp_path):
    with pytest.raises(AdcError) as exc:
        Site.load(tmp_path / "does-not-exist")
    assert exc.value.code == "NOT_IMPLEMENTED"


# --------------------------------------------------------------------------
# Tokenizer and canonical JSON helpers (SPEC 4.2, F.12)
# --------------------------------------------------------------------------


def test_example_manifest_tokenizer_is_exact():
    # The example declares utf8_bytes_div_4, which the reference tokenizer
    # evaluates exactly (SPEC 4.2).
    assert is_exact("utf8_bytes_div_4") is True


def test_utf8_bytes_div_4_counts_ceil_bytes_over_four():
    # SPEC 4.2: tokens = ceil(bytes / 4). "hello world!" is 12 bytes.
    assert count_tokens("hello world!", "utf8_bytes_div_4") == 3
    assert count_tokens("abcd", "utf8_bytes_div_4") == 1
    assert count_tokens("abcde", "utf8_bytes_div_4") == 2


def test_unknown_tokenizer_rejected():
    with pytest.raises(ValueError):
        count_tokens("x", "not-a-tokenizer")


def test_canonical_bytes_sorts_keys_and_omits_whitespace():
    # RFC 8785-flavoured canonical JSON used for generation hashes.
    assert canonical_bytes({"b": 1, "a": 2}) == b'{"a":2,"b":1}'


def test_payload_stats_generation_is_stable():
    payload = {"items": [1, 2, 3]}
    first = payload_stats(payload, "utf8_bytes_div_4")
    second = payload_stats(payload, "utf8_bytes_div_4")
    assert first == second
    tokens, byte_len, generation = first
    assert generation.startswith("V1:")
    assert tokens == -(-byte_len // 4)  # ceil division
