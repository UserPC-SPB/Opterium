# Agent Data Circuit (ADC) Protocol

## [0.1-draft] - 2026-09-25

### Added

- Manifest definition at `/.well-known/adc.json`
- Navigation tree structure
- Per-node passports
- Sliceable blocks
- `GET` fetch operation with collection/entity polymorphism
- `GET` filter grammar
- Estimate endpoint
- Per-slice hydration map
- Explicit degradation modes: `stale_fallback`, `partial_omit`, `fail_closed`
- Billing invariants for degraded responses
- `GET` and `POST` aggregate operations with k-anonymity
- Validation order specification
- RFC 9457 error model integration
- Anchor namespacing with URI fragment syntax
- Tokenizer declaration
- Budget state machine with concurrency rules
- JSON Schemas for all protocol objects
- Python reference server implementation
- Validator CLI tool
- MCP tool bridge
- pytest test suite

### Notes

This is an unreviewed draft with no production adopters. The interface may change significantly before any 1.0 release. Earlier internal drafts used version "0.3" but were never implementation-backed. This repository restarts numbering at 0.1 to avoid implying stability that does not exist.