"""Path canonicalization (SPEC.md 5.1).

Paths are absolute with respect to the manifest ``root``, normalized per
RFC 3986 5.2. They MUST be case-sensitive, use ``/``, be NFC-normalized, and
MUST NOT contain ``..``, ``//``, a trailing slash (except root ``/``), or
percent-encoded separators. Violations raise ``PATH_INVALID``.
"""

from __future__ import annotations

import unicodedata

from .errors import AdcError

# Percent-encoded slash / backslash in any case.
_ENCODED_SEPARATORS = ("%2f", "%5c")


def canonicalize_path(raw: str) -> str:
    """Return the canonical form of ``raw`` or raise :class:`AdcError`."""
    if not isinstance(raw, str) or not raw:
        raise AdcError("PATH_INVALID", "path must be a non-empty string")
    if not raw.startswith("/"):
        raise AdcError("PATH_INVALID", f"path must be absolute: {raw!r}")

    lowered = raw.lower()
    for sep in _ENCODED_SEPARATORS:
        if sep in lowered:
            raise AdcError(
                "PATH_INVALID", f"percent-encoded path separator is forbidden: {raw!r}"
            )

    if raw != "/" and raw.endswith("/"):
        raise AdcError("PATH_INVALID", f"trailing slash is forbidden: {raw!r}")
    if "//" in raw:
        raise AdcError("PATH_INVALID", f"empty segment ('//') is forbidden: {raw!r}")

    segments = raw.split("/")[1:]
    for seg in segments:
        if seg in (".", ".."):
            raise AdcError("PATH_INVALID", f"dot segment is forbidden: {raw!r}")

    normalized = unicodedata.normalize("NFC", raw)
    return normalized


def path_depth(path: str) -> int:
    """Number of segments below the root; ``/`` is depth 0."""
    if path == "/":
        return 0
    return len(path.split("/")) - 1


def path_segments(path: str) -> list[str]:
    if path == "/":
        return []
    return path.split("/")[1:]


def join_path(parent: str, name: str) -> str:
    return "/" + name if parent == "/" else f"{parent}/{name}"


def is_prefix_of(prefix: str, path: str) -> bool:
    """True when ``path`` is at or below ``prefix`` on segment boundaries."""
    if prefix == "/":
        return True
    return path == prefix or path.startswith(prefix + "/")


def parent_path(path: str) -> str | None:
    if path == "/":
        return None
    segs = path_segments(path)
    return "/" + "/".join(segs[:-1]) if len(segs) > 1 else "/"