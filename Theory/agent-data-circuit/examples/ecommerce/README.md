# Example circuit: examples/ecommerce

Synthetic English headphones/speakers/laptops catalog implementing ADC v0.1
(core + dynamic + commerce tiers). Brands echo real products; every value is
generated, nothing is scraped.

## Layout

    manifest.json                 served at /.well-known/adc.json
    tree.json                     navigation tree, no content
    nodes/a1.json ... a95.json    passport + slice payloads (95 nodes)
    generate.py                   deterministic generator for all of the above

95 nodes = 7 collections (`/`, `/computers`, `/computers/laptops`, `/electronics`,
`/electronics/audio`, `/electronics/audio/headphones`, `/electronics/audio/speakers`)
+ 88 entities (48 headphones, 16 speakers, 24 laptops).

## Invariants

* `tree_generation` is the fixed string `2026-10-01T12:00:00Z`; regeneration
  never changes it.
* Anchor `[aN]` is the 1-based rank of the canonical path in the sorted list of
  all node paths (`/` sorts first, so the root is `[a1]`); the same rank names
  the file (`[a10]` -> `nodes/a10.json`). `tree.json` is sorted by path.
* Tokenizer `utf8_bytes_div_4`: `tokens = ceil(bytes / 4)` over the exact
  UTF-8 bytes of `json.dumps(payload, sort_keys=True, separators=(",", ":"))`;
  slice `generation` is `V1:` + first 8 hex chars of its SHA-256.
* Every entity also appears as an item in its parent collection's `summary`
  slice with identical `fields`; the server fills in item `anchor` and
  `generation`, so payloads carry only `path` + `fields`.
* `reviews.histogram` is `[5, 4, 3, 2, 1]`-star buckets summing to `count`, and
  `summary.rating` equals `reviews.rating` (both in 3.5-5.0).
* `specs` are category-specific: headphones expose `anc`, `battery_life`,
  `weight_g`, `codec`; speakers expose `driver_mm`, `power`, `connectivity`,
  `weight_g`; laptops expose `cpu`, `ram_gb`, `storage`, `display`, `weight_g`.
* `logistics` is the only dynamic slice (0.0005 USD per call, stale fallback
  after 900 s); `summary` is the default slice on every passport. Collections
  declare `count` = length of their `summary.items`; entities do not.

## Regenerate

    python examples/ecommerce/generate.py           # rewrite manifest/tree/nodes
    python examples/ecommerce/generate.py --check   # CI guard: exit 1 on drift

The generator is stdlib-only and seeds every value from SHA-256 of the node
path (no RNG, no system clock); it re-verifies anchors, counts, item
resolution, slice hashes and ASCII on each run. Two runs are byte-identical.
