"""Path canonicalization tests (SPEC.md 5.1, 6.12)."""

import pytest

from adc.errors import AdcError
from adc.pathing import canonicalize_path, is_prefix_of, parent_path, path_depth


def _reject(raw):
    with pytest.raises(AdcError) as exc:
        canonicalize_path(raw)
    assert exc.value.code == "PATH_INVALID"
    return exc.value


def test_root_is_canonical():
    # SPEC 5.1: root "/" is the one path allowed to end with a slash.
    assert canonicalize_path("/") == "/"


def test_absolute_path_passes_through():
    assert canonicalize_path("/electronics/audio/headphones") == (
        "/electronics/audio/headphones"
    )


def test_dot_dot_segment_rejected():
    # SPEC 5.1: paths MUST NOT contain "..".
    _reject("/electronics/../secret")


def test_single_dot_segment_rejected():
    _reject("/electronics/./audio")


def test_double_slash_rejected():
    # SPEC 5.1: paths MUST NOT contain "//".
    _reject("/electronics//audio")


def test_trailing_slash_rejected():
    # SPEC 5.1: no trailing slash except the root.
    _reject("/electronics/audio/")


def test_percent_encoded_slash_rejected():
    # SPEC 5.1: percent-encoded separators are forbidden, any case.
    _reject("/electronics%2Faudio")
    _reject("/electronics%2faudio")


def test_percent_encoded_backslash_rejected():
    _reject("/electronics%5Caudio")


def test_non_absolute_path_rejected():
    # SPEC 5.1: paths are absolute with respect to the manifest root.
    _reject("electronics/audio")
    _reject("a/b")


def test_empty_path_rejected():
    _reject("")


def test_non_string_path_rejected():
    _reject(None)
    _reject(42)


def test_path_invalid_maps_to_400():
    # SPEC 6.12: PATH_INVALID is a 400, not retryable.
    error = _reject("/a/../b")
    assert error.status == 400
    assert error.retryable is False


def test_paths_are_case_sensitive():
    # SPEC 5.1: paths MUST be case-sensitive; no case folding happens.
    assert canonicalize_path("/Electronics/Audio") == "/Electronics/Audio"


def test_unicode_input_is_nfc_normalized():
    # SPEC 5.1: paths MUST be NFC-normalized. "e" + combining acute (U+0301)
    # becomes the precomposed character U+00E9. Written with escapes so this
    # file stays ASCII.
    assert canonicalize_path("/cafe\u0301") == "/caf\u00e9"


def test_already_nfc_input_unchanged():
    assert canonicalize_path("/caf\u00e9") == "/caf\u00e9"


def test_path_depth_counts_segments():
    assert path_depth("/") == 0
    assert path_depth("/a") == 1
    assert path_depth("/a/b/c") == 3


def test_parent_path_walks_up():
    assert parent_path("/") is None
    assert parent_path("/a") == "/"
    assert parent_path("/a/b") == "/a"
    assert parent_path("/a/b/c") == "/a/b"


def test_is_prefix_of_is_segment_aware():
    # SPEC 5.3.2: suffix matching happens on segment boundaries.
    assert is_prefix_of("/", "/anything") is True
    assert is_prefix_of("/a", "/a") is True
    assert is_prefix_of("/a", "/a/b") is True
    assert is_prefix_of("/a", "/ab") is False
    assert is_prefix_of("/a/b", "/a") is False
