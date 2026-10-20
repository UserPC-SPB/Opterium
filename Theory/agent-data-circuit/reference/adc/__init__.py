"""Agent Data Circuit (ADC) reference implementation.

This package is intentionally stdlib-only so the reference server and the
validator run anywhere Python 3.11+ is available. It implements the protocol
described in SPEC.md at the draft 0.1 level:

    manifest -> tree -> passport -> fetch (+ estimate, aggregate)

Higher tiers (search, grep, verify) are stubbed where they cannot be
implemented without a publisher backend.
"""

__version__ = "0.1.0"
ADC_VERSION = "0.1"