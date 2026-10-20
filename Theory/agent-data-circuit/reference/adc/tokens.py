"""Tokenizer support for ADC token accounting.

SPEC.md 4.2 requires every ``tokens`` field to be computed with the tokenizer
declared in the manifest. The only tokenizer we can implement *exactly* without
third-party dependencies is ``utf8_bytes_div_4`` (``ceil(bytes / 4)``), so the
bundled example uses it and the validator can verify it byte-for-byte.

``cl100k_base`` and ``o200k_base`` are delegated to ``tiktoken`` when it is
installed. When it is not, we fall back to ``ceil(bytes / 4)`` and raise a
flag so callers can report that they are approximating rather than pretending
to be exact.
"""

from __future__ import annotations

import math
from typing import Callable

KNOWN_TOKENIZERS = ("cl100k_base", "o200k_base", "utf8_bytes_div_4")


def count_tokens(text: str, tokenizer: str) -> int:
    """Return the token count of ``text`` for the declared ``tokenizer``.

    Raises ``ValueError`` for a tokenizer string that is neither a known name
    nor an absolute URI.
    """
    data = text.encode("utf-8")
    if tokenizer == "utf8_bytes_div_4":
        return math.ceil(len(data) / 4)
    if tokenizer in ("cl100k_base", "o200k_base"):
        enc = _try_tiktoken(tokenizer)
        if enc is not None:
            return len(enc.encode(text))
        # Honest fallback: approximate and let the caller flag it.
        return math.ceil(len(data) / 4)
    if "://" in tokenizer:
        raise ValueError(
            f"custom tokenizer {tokenizer!r} cannot be evaluated by the "
            "reference implementation; use utf8_bytes_div_4 for the demo"
        )
    raise ValueError(f"unknown tokenizer: {tokenizer!r}")


def is_exact(tokenizer: str) -> bool:
    """True if :func:`count_tokens` computes this tokenizer exactly."""
    if tokenizer == "utf8_bytes_div_4":
        return True
    if tokenizer in ("cl100k_base", "o200k_base"):
        return _try_tiktoken(tokenizer) is not None
    return False


def _try_tiktoken(name: str):
    try:  # pragma: no cover - depends on optional dependency
        import tiktoken

        return tiktoken.get_encoding(name)
    except Exception:
        return None


def bytes_for(text: str) -> int:
    return len(text.encode("utf-8"))