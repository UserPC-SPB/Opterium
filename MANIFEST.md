# MANIFEST

Cost map for visiting AI agents. Before reading anything in this repo, consult this manifest to know where each file is, how big it is, what it costs in tokens, and whether you may read it fully (`safe`), only by structure/targeted windows (`structure-only`), or not at all (`skip`). Generated 2026-11-22 on branch `v3.1-clean`; machine-readable twin: `MANIFEST.json`.

## Zone summary

| zone | files | KB | ~tokens | safe | structure-only | skip | how to read |
|---|---|---|---|---|---|---|---|
| root | 3 | 23.2 | 5890 | 2 | 0 | 1 | read the 3 root docs; everything else lives in the zones below |
| software/toolfmt | 267 | 6771.5 | 346105 | 129 | 5 | 133 | Rust tool + its source/tests; read src/docs, skip binaries and fixtures |
| software/parsers_for_AI | 200 | 7955.5 | 262670 | 70 | 6 | 124 | parser tool + tests/docs; skip the .exe and fixture data |
| software/witness_prover | 3 | 6650.6 | 48955 | 1 | 1 | 1 | 3 files; CUBE_REFERENCE.md is structure-only, read by headings |
| Theory/agent-data-circuit | 159 | 1028.5 | 235017 | 144 | 2 | 13 | theory + agent circuit notes; large docs are structure-only |
| Theory/ontology_of_number | 12 | 324.0 | 79214 | 9 | 3 | 0 | number-ontology theory; large docs are structure-only |
| software | 1 | 4.9 | 1254 | 1 | 0 | 0 | top-level software/ file (README); read fully |

## Zone: root

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| .gitignore | - | 0.1 | - | skip (git config) |
| ADAPTIVE_METHOD.md | 273 | 16.7 | 4251 | safe |
| README.md | 76 | 6.5 | 1639 | safe |

## Zone: software/toolfmt

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| software/toolfmt/.cargo/config.toml | 7 | 0.5 | 118 | safe |
| software/toolfmt/.gitattributes | 14 | 0.7 | 169 | safe |
| software/toolfmt/.gitignore | 13 | 0.2 | 58 | safe |
| software/toolfmt/Cargo.lock | - | 38.8 | - | skip (lockfile) |
| software/toolfmt/Cargo.toml | 43 | 1.1 | 293 | safe |
| software/toolfmt/HELP.md | 239 | 19.9 | 5073 | safe |
| software/toolfmt/LICENSE | 21 | 1.1 | 271 | safe |
| software/toolfmt/README.md | 165 | 9.5 | 2429 | safe |
| software/toolfmt/SHA256SUMS.txt | 2 | 0.2 | 43 | safe |
| software/toolfmt/build_release.bat | 9 | 0.5 | 136 | safe |
| software/toolfmt/build_test.bat | 4 | 0.2 | 42 | safe |
| software/toolfmt/crates/navguard/Cargo.toml | 21 | 0.5 | 121 | safe |
| software/toolfmt/crates/navguard/HELP.md | 59 | 3.2 | 816 | safe |
| software/toolfmt/crates/navguard/src/api.rs | 1049 | 36.3 | 9298 | safe |
| software/toolfmt/crates/navguard/src/api/tests.rs | 707 | 23.8 | 6090 | safe |
| software/toolfmt/crates/navguard/src/build_rs.rs | 218 | 7.3 | 1871 | safe |
| software/toolfmt/crates/navguard/src/cache.rs | 885 | 28.4 | 7223 | safe |
| software/toolfmt/crates/navguard/src/cli.rs | 971 | 34.8 | 8891 | safe |
| software/toolfmt/crates/navguard/src/config.rs | 97 | 2.9 | 753 | safe |
| software/toolfmt/crates/navguard/src/lib.rs | 24 | 0.7 | 172 | safe |
| software/toolfmt/crates/navguard/src/navigator.rs | 576 | 21.1 | 5354 | safe |
| software/toolfmt/crates/navguard/src/parser.rs | 297 | 9.2 | 2358 | safe |
| software/toolfmt/crates/navguard/src/parser/brace.rs | 1027 | 32.5 | 8309 | safe |
| software/toolfmt/crates/navguard/src/parser/indent.rs | 116 | 3.8 | 978 | safe |
| software/toolfmt/crates/navguard/src/parser/lexer.rs | 531 | 17.0 | 4348 | safe |
| software/toolfmt/crates/navguard/src/parser/tests.rs | 688 | 18.6 | 4767 | safe |
| software/toolfmt/crates/navguard/src/paths.rs | 162 | 5.7 | 1466 | safe |
| software/toolfmt/crates/navguard/src/responder.rs | 223 | 6.6 | 1693 | safe |
| software/toolfmt/crates/navguard/src/scanner.rs | 230 | 8.0 | 2048 | safe |
| software/toolfmt/crates/navguard/src/step.rs | 351 | 11.1 | 2836 | safe |
| software/toolfmt/crates/navguard/src/text.rs | 131 | 4.2 | 1071 | safe |
| software/toolfmt/crates/navguard/src/triangle.rs | 498 | 16.2 | 4137 | safe |
| software/toolfmt/crates/navguard/src/validate.rs | 71 | 2.1 | 549 | safe |
| software/toolfmt/crates/navguard/src/watch.rs | 130 | 4.3 | 1100 | safe |
| software/toolfmt/crates/navguard/src/write.rs | 51 | 1.6 | 408 | safe |
| software/toolfmt/crates/toolfmt/Cargo.toml | 22 | 0.5 | 124 | safe |
| software/toolfmt/crates/toolfmt/README.md | 153 | 6.8 | 1729 | safe |
| software/toolfmt/crates/toolfmt/REPORT_2026-11-04.md | 259 | 21.0 | 5354 | safe |
| software/toolfmt/crates/toolfmt/REPORT_2026-11-20.md | 111 | 11.0 | 2799 | safe |
| software/toolfmt/crates/toolfmt/SPEC.md | 711 | 34.4 | 8765 | safe |
| software/toolfmt/crates/toolfmt/examples/home/.gitignore | 2 | 0.0 | 7 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.add/01.json | 1 | 0.2 | 64 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.add/02.json | 1 | 0.4 | 100 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.delete_all/01.json | 1 | 0.3 | 68 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.list_files/01.json | 1 | 2.6 | 677 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.list_files/02.json | 1 | 2.6 | 668 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.list_files/90-empty.json | 1 | 0.2 | 59 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.list_files/91-malformed.json | 1 | 0.2 | 50 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.list_files/92-error-text.json | 1 | 0.2 | 52 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.log_tail/01.json | 1 | 1.9 | 474 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.matrix/01.json | 1 | 0.4 | 99 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.numbers/01.json | 1 | 0.3 | 89 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.paths/01.json | 1 | 0.4 | 114 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.pow/01.json | 1 | 0.3 | 85 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.search/01.json | 1 | 1.0 | 248 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.search/90-keys.json | 1 | 0.3 | 75 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/demo.snapshot/01.json | 1 | 0.4 | 95 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/01-no-standard.json | 1 | 0.2 | 48 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/02-auto-list.json | 1 | 0.4 | 110 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/03-auto-pretty-object.json | 1 | 0.4 | 103 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/04-override.json | 1 | 0.3 | 80 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/05-bad-inline-template.json | 1 | 0.2 | 40 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/06-scalar.json | 1 | 0.1 | 32 | safe |
| software/toolfmt/crates/toolfmt/examples/home/fixtures/edge.misc/07-family.json | 1 | 0.3 | 65 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.add.yml | 4 | 0.1 | 30 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.list_files.yml | 7 | 0.2 | 43 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.log_tail.yml | 3 | 0.0 | 9 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.matrix.yml | 4 | 0.1 | 14 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.numbers.yml | 3 | 0.0 | 10 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.paths.yml | 2 | 0.0 | 6 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.pow.yml | 3 | 0.1 | 19 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.search.yml | 11 | 0.3 | 89 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/demo.snapshot.yml | 2 | 0.0 | 8 | safe |
| software/toolfmt/crates/toolfmt/examples/home/standards/family/listing.yml | 4 | 0.1 | 29 | safe |
| software/toolfmt/crates/toolfmt/examples/home/upstreams.json | 9 | 0.1 | 37 | safe |
| software/toolfmt/crates/toolfmt/src/capture.rs | 455 | 14.3 | 3672 | safe |
| software/toolfmt/crates/toolfmt/src/catalog.rs | 564 | 18.0 | 4592 | safe |
| software/toolfmt/crates/toolfmt/src/config.rs | 298 | 10.5 | 2699 | safe |
| software/toolfmt/crates/toolfmt/src/demo.rs | 508 | 23.5 | 6020 | safe |
| software/toolfmt/crates/toolfmt/src/detect.rs | 184 | 5.4 | 1384 | safe |
| software/toolfmt/crates/toolfmt/src/gateway.rs | 2036 | 74.0 | 18940 | structure-only |
| software/toolfmt/crates/toolfmt/src/http.rs | 124 | 4.3 | 1096 | safe |
| software/toolfmt/crates/toolfmt/src/image.rs | 172 | 5.4 | 1376 | safe |
| software/toolfmt/crates/toolfmt/src/json.rs | 749 | 22.1 | 5667 | safe |
| software/toolfmt/crates/toolfmt/src/lib.rs | 28 | 0.7 | 181 | safe |
| software/toolfmt/crates/toolfmt/src/main.rs | 335 | 11.0 | 2814 | safe |
| software/toolfmt/crates/toolfmt/src/meta.rs | 171 | 7.8 | 2000 | safe |
| software/toolfmt/crates/toolfmt/src/ops.rs | 326 | 10.8 | 2756 | safe |
| software/toolfmt/crates/toolfmt/src/pipeline.rs | 706 | 21.6 | 5528 | safe |
| software/toolfmt/crates/toolfmt/src/render.rs | 820 | 25.8 | 6604 | safe |
| software/toolfmt/crates/toolfmt/src/replay.rs | 79 | 2.4 | 605 | safe |
| software/toolfmt/crates/toolfmt/src/rpc.rs | 99 | 2.8 | 716 | safe |
| software/toolfmt/crates/toolfmt/src/standard.rs | 767 | 23.1 | 5905 | safe |
| software/toolfmt/crates/toolfmt/src/tpl.rs | 141 | 4.9 | 1245 | safe |
| software/toolfmt/crates/toolfmt/tests/common/mod.rs | - | 6.2 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/corpus.rs | - | 1.2 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/failures.rs | - | 5.5 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/formatter_tools.rs | - | 1.3 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/fuzz.rs | - | 5.5 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/gateway.rs | - | 13.5 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/http.rs | - | 4.6 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/proxy.rs | - | 10.0 | - | skip (fixture) |
| software/toolfmt/crates/toolfmt/tests/renders.rs | - | 11.8 | - | skip (fixture) |
| software/toolfmt/docs/help/extract.md | 116 | 6.1 | 1570 | safe |
| software/toolfmt/docs/help/mcp.md | 93 | 4.6 | 1164 | safe |
| software/toolfmt/docs/help/navigation.md | 189 | 9.0 | 2298 | safe |
| software/toolfmt/docs/help/overview.md | 100 | 5.8 | 1477 | safe |
| software/toolfmt/docs/help/recon.md | 54 | 2.0 | 518 | safe |
| software/toolfmt/docs/help/rules.md | 83 | 3.7 | 942 | safe |
| software/toolfmt/examples/errors.yml | 18 | 0.3 | 73 | safe |
| software/toolfmt/examples/example.log | 4 | 0.1 | 26 | safe |
| software/toolfmt/rust-toolchain.toml | 7 | 0.3 | 74 | safe |
| software/toolfmt/scripts/release.bat | 13 | 0.7 | 183 | safe |
| software/toolfmt/scripts/release.ps1 | 699 | 31.5 | 8062 | safe |
| software/toolfmt/src/adapters/html.rs | 862 | 30.8 | 7891 | safe |
| software/toolfmt/src/adapters/json.rs | 530 | 17.9 | 4582 | safe |
| software/toolfmt/src/adapters/markdown.rs | 384 | 12.5 | 3189 | safe |
| software/toolfmt/src/adapters/mod.rs | 322 | 11.3 | 2884 | safe |
| software/toolfmt/src/adapters/path_selector.rs | 218 | 7.9 | 2012 | safe |
| software/toolfmt/src/adapters/text.rs | 222 | 7.2 | 1840 | safe |
| software/toolfmt/src/adapters/toml.rs | 84 | 2.4 | 618 | safe |
| software/toolfmt/src/adapters/xml.rs | 186 | 6.5 | 1676 | safe |
| software/toolfmt/src/adapters/xml_dtd.rs | 1136 | 43.3 | 11085 | structure-only |
| software/toolfmt/src/adapters/yaml.rs | 567 | 19.5 | 4983 | safe |
| software/toolfmt/src/api.rs | 2010 | 73.5 | 18813 | structure-only |
| software/toolfmt/src/collect.rs | 655 | 21.5 | 5503 | safe |
| software/toolfmt/src/commands.rs | 316 | 11.4 | 2931 | safe |
| software/toolfmt/src/confine.rs | 119 | 4.2 | 1076 | safe |
| software/toolfmt/src/discovery.rs | 671 | 23.4 | 5992 | safe |
| software/toolfmt/src/help.rs | 88 | 2.8 | 720 | safe |
| software/toolfmt/src/jsonpath.rs | 1230 | 42.9 | 10992 | structure-only |
| software/toolfmt/src/lib.rs | 21 | 0.4 | 90 | safe |
| software/toolfmt/src/limits.rs | 115 | 3.0 | 768 | safe |
| software/toolfmt/src/main.rs | 189 | 5.6 | 1432 | safe |
| software/toolfmt/src/materialize.rs | 525 | 18.5 | 4725 | safe |
| software/toolfmt/src/mcp.rs | 463 | 17.8 | 4551 | safe |
| software/toolfmt/src/navigation.rs | 352 | 10.1 | 2590 | safe |
| software/toolfmt/src/publish.rs | 149 | 4.5 | 1161 | safe |
| software/toolfmt/src/render.rs | 792 | 28.5 | 7288 | safe |
| software/toolfmt/src/run.rs | 769 | 26.6 | 6798 | safe |
| software/toolfmt/src/template.rs | 1713 | 54.5 | 13956 | structure-only |
| software/toolfmt/src/tools.rs | 636 | 27.1 | 6946 | safe |
| software/toolfmt/src/transform.rs | 449 | 14.2 | 3624 | safe |
| software/toolfmt/src/xpath.rs | 863 | 28.7 | 7346 | safe |
| software/toolfmt/tests/api.rs | - | 58.3 | - | skip (fixture) |
| software/toolfmt/tests/fuzz.rs | - | 111.7 | - | skip (fixture) |
| software/toolfmt/tests/fuzz_regressions.rs | - | 8.2 | - | skip (fixture) |
| software/toolfmt/tests/golden.rs | - | 18.7 | - | skip (fixture) |
| software/toolfmt/tests/known_limits.rs | - | 20.7 | - | skip (fixture) |
| software/toolfmt/tests/navigator_state.rs | - | 10.6 | - | skip (fixture) |
| software/toolfmt/tests/one_exe.rs | - | 18.2 | - | skip (fixture) |
| software/toolfmt/tests/regressions.rs | - | 17.5 | - | skip (fixture) |
| software/toolfmt/tests/root_bounds.rs | - | 12.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots.rs | - | 24.6 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/absolute_paths/args.txt | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/absolute_paths/expected.md | - | 3.5 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/absolute_paths/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/absolute_paths/sources/nested/orders.json | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/absolute_paths/sources/notes.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/absolute_paths/template.yml | - | 0.6 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/all_files_fail/expected.md | - | 3.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/all_files_fail/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/all_files_fail/sources/latin1.json | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/all_files_fail/sources/mismatched.xml | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/all_files_fail/sources/truncated.json | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/all_files_fail/template.yml | - | 0.4 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/anchor_text/expected.md | - | 5.3 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/anchor_text/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/anchor_text/sources/comments.json | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/anchor_text/sources/forged.txt | - | 0.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/anchor_text/template.yml | - | 0.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/cli_format_override/args.txt | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/cli_format_override/expected.md | - | 3.5 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/cli_format_override/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/cli_format_override/sources/alert.yaml | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/cli_format_override/sources/event.json | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/cli_format_override/template.yml | - | 0.4 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/dedup_sort/expected.md | - | 5.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/dedup_sort/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/dedup_sort/sources/a_server.log | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/dedup_sort/sources/b_worker.log | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/dedup_sort/template.yml | - | 0.6 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/args.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/expected.md | - | 3.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/sources/a_plain.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/sources/b_bom.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/sources/c_skipped.md | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/hash_glob_input/template.yml | - | 0.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/html_extract_modes/expected.md | - | 8.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/html_extract_modes/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/html_extract_modes/sources/product.html | - | 1.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/html_extract_modes/template.yml | - | 0.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/json_jsonpath_filters/expected.md | - | 6.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/json_jsonpath_filters/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/json_jsonpath_filters/sources/orders.json | - | 1.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/json_jsonpath_filters/template.yml | - | 0.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/markdown_nodes/expected.md | - | 9.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/markdown_nodes/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/markdown_nodes/sources/README.md | - | 0.7 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/markdown_nodes/template.yml | - | 0.8 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/expected.md | - | 7.8 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/Cargo.toml | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/build.log | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/config/app.yaml | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/config/service.conf | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/docs/guide.md | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/docs/index.html | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/manifest | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/modules.xml | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/sources/package.json | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/mixed_when_format/template.yml | - | 0.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/no_matches_not_applicable/expected.md | - | 5.5 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/no_matches_not_applicable/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/no_matches_not_applicable/sources/notes.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/no_matches_not_applicable/sources/users.json | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/no_matches_not_applicable/template.yml | - | 0.5 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/partial_unparsable/expected.md | - | 3.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/partial_unparsable/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/partial_unparsable/sources/a_users.json | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/partial_unparsable/sources/b_broken.json | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/partial_unparsable/sources/c_users.json | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/partial_unparsable/template.yml | - | 0.3 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_hard_line/args.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_hard_line/expected.md | - | 7.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_hard_line/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_hard_line/sources/one_line.txt | - | 2.6 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_hard_line/template.yml | - | 1.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_unicode_items/args.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_unicode_items/expected.md | - | 7.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_unicode_items/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_unicode_items/sources/release_notes.txt | - | 1.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/split_unicode_items/template.yml | - | 0.5 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/text_logs_context/expected.md | - | 10.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/text_logs_context/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/text_logs_context/sources/app.log | - | 0.4 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/text_logs_context/sources/classic_mac.log | - | 0.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/text_logs_context/sources/windows.log | - | 0.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/text_logs_context/template.yml | - | 0.7 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/toml_tables/expected.md | - | 6.7 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/toml_tables/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/toml_tables/sources/service.toml | - | 0.4 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/toml_tables/template.yml | - | 0.9 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/truncation_limits/expected.md | - | 6.1 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/truncation_limits/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/truncation_limits/sources/tickets.json | - | 1.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/truncation_limits/template.yml | - | 0.7 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/xml_namespaces_dtd/expected.md | - | 7.8 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/xml_namespaces_dtd/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/xml_namespaces_dtd/sources/catalog.xml | - | 0.8 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/xml_namespaces_dtd/template.yml | - | 1.2 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/yaml_multidoc_merge/expected.md | - | 5.5 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/yaml_multidoc_merge/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/yaml_multidoc_merge/sources/manifest.yaml | - | 0.7 | - | skip (fixture) |
| software/toolfmt/tests/snapshots/yaml_multidoc_merge/template.yml | - | 0.6 | - | skip (fixture) |
| software/toolfmt/tests/stabilization.rs | - | 14.8 | - | skip (fixture) |
| software/toolfmt/tests/support/mod.rs | - | 15.3 | - | skip (fixture) |
| software/toolfmt/tests/tier2_effective.rs | - | 21.1 | - | skip (fixture) |
| software/toolfmt/tests/tier2_html.rs | - | 7.9 | - | skip (fixture) |
| software/toolfmt/tests/tier2_jsonpath.rs | - | 10.9 | - | skip (fixture) |
| software/toolfmt/tests/tier2_release.rs | - | 31.8 | - | skip (fixture) |
| software/toolfmt/tests/tier2_status.rs | - | 26.9 | - | skip (fixture) |
| software/toolfmt/tests/tier2_unescape.rs | - | 2.7 | - | skip (fixture) |
| software/toolfmt/tests/tier2_xml.rs | - | 16.7 | - | skip (fixture) |
| software/toolfmt/tests/tier2_yaml.rs | - | 9.6 | - | skip (fixture) |
| software/toolfmt/toolfmt.exe | - | 4721.0 | - | skip (binary) |

## Zone: software/parsers_for_AI

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| software/parsers_for_AI/.cargo/config.toml | 7 | 0.5 | 118 | safe |
| software/parsers_for_AI/.gitattributes | 14 | 0.7 | 169 | safe |
| software/parsers_for_AI/.gitignore | 13 | 0.2 | 58 | safe |
| software/parsers_for_AI/CHANGELOG.md | 575 | 42.6 | 10827 | structure-only |
| software/parsers_for_AI/Cargo.lock | - | 38.7 | - | skip (lockfile) |
| software/parsers_for_AI/Cargo.toml | 43 | 1.1 | 288 | safe |
| software/parsers_for_AI/FULL_GUIDE.md | 626 | 31.2 | 7934 | structure-only (large reference doc (named anchor)) |
| software/parsers_for_AI/HELP.md | 236 | 19.8 | 5046 | safe |
| software/parsers_for_AI/LICENSE | 21 | 1.1 | 271 | safe |
| software/parsers_for_AI/README.md | 143 | 8.3 | 2113 | safe |
| software/parsers_for_AI/any_txt_parser.exe | - | 6292.0 | - | skip (binary) |
| software/parsers_for_AI/build_release.bat | 9 | 0.5 | 136 | safe |
| software/parsers_for_AI/build_test.bat | 4 | 0.2 | 42 | safe |
| software/parsers_for_AI/crates/navguard/Cargo.toml | 21 | 0.5 | 121 | safe |
| software/parsers_for_AI/crates/navguard/HELP.md | 59 | 3.2 | 816 | safe |
| software/parsers_for_AI/crates/navguard/src/api.rs | 1026 | 35.3 | 9024 | safe |
| software/parsers_for_AI/crates/navguard/src/api/tests.rs | 671 | 22.0 | 5610 | safe |
| software/parsers_for_AI/crates/navguard/src/build_rs.rs | 218 | 7.3 | 1871 | safe |
| software/parsers_for_AI/crates/navguard/src/cache.rs | 885 | 28.2 | 7228 | safe |
| software/parsers_for_AI/crates/navguard/src/cli.rs | 971 | 34.8 | 8891 | safe |
| software/parsers_for_AI/crates/navguard/src/config.rs | 97 | 2.9 | 753 | safe |
| software/parsers_for_AI/crates/navguard/src/lib.rs | 24 | 0.7 | 172 | safe |
| software/parsers_for_AI/crates/navguard/src/navigator.rs | 576 | 21.1 | 5354 | safe |
| software/parsers_for_AI/crates/navguard/src/parser.rs | 297 | 9.2 | 2358 | safe |
| software/parsers_for_AI/crates/navguard/src/parser/brace.rs | 1027 | 32.5 | 8309 | safe |
| software/parsers_for_AI/crates/navguard/src/parser/indent.rs | 116 | 3.8 | 978 | safe |
| software/parsers_for_AI/crates/navguard/src/parser/lexer.rs | 531 | 17.0 | 4348 | safe |
| software/parsers_for_AI/crates/navguard/src/parser/tests.rs | 688 | 18.6 | 4767 | safe |
| software/parsers_for_AI/crates/navguard/src/paths.rs | 162 | 5.7 | 1466 | safe |
| software/parsers_for_AI/crates/navguard/src/responder.rs | 223 | 6.6 | 1693 | safe |
| software/parsers_for_AI/crates/navguard/src/scanner.rs | 230 | 8.0 | 2048 | safe |
| software/parsers_for_AI/crates/navguard/src/step.rs | 351 | 11.1 | 2836 | safe |
| software/parsers_for_AI/crates/navguard/src/text.rs | 131 | 4.2 | 1071 | safe |
| software/parsers_for_AI/crates/navguard/src/triangle.rs | 498 | 16.2 | 4137 | safe |
| software/parsers_for_AI/crates/navguard/src/validate.rs | 71 | 2.1 | 549 | safe |
| software/parsers_for_AI/crates/navguard/src/watch.rs | 130 | 4.3 | 1100 | safe |
| software/parsers_for_AI/crates/navguard/src/write.rs | 51 | 1.6 | 408 | safe |
| software/parsers_for_AI/docs/help/extract.md | 116 | 6.1 | 1570 | safe |
| software/parsers_for_AI/docs/help/mcp.md | 93 | 4.6 | 1164 | safe |
| software/parsers_for_AI/docs/help/navigation.md | 174 | 8.4 | 2132 | safe |
| software/parsers_for_AI/docs/help/overview.md | 100 | 5.8 | 1477 | safe |
| software/parsers_for_AI/docs/help/recon.md | 54 | 2.0 | 518 | safe |
| software/parsers_for_AI/docs/help/rules.md | 83 | 3.7 | 942 | safe |
| software/parsers_for_AI/examples/errors.yml | 18 | 0.3 | 73 | safe |
| software/parsers_for_AI/examples/example.log | 4 | 0.1 | 26 | safe |
| software/parsers_for_AI/rust-toolchain.toml | 7 | 0.3 | 74 | safe |
| software/parsers_for_AI/scripts/release.bat | 13 | 0.7 | 183 | safe |
| software/parsers_for_AI/scripts/release.ps1 | 699 | 31.5 | 8062 | safe |
| software/parsers_for_AI/src/adapters/html.rs | 862 | 30.8 | 7891 | safe |
| software/parsers_for_AI/src/adapters/json.rs | 80 | 2.2 | 553 | safe |
| software/parsers_for_AI/src/adapters/markdown.rs | 384 | 12.5 | 3189 | safe |
| software/parsers_for_AI/src/adapters/mod.rs | 319 | 11.1 | 2845 | safe |
| software/parsers_for_AI/src/adapters/path_selector.rs | 218 | 7.9 | 2012 | safe |
| software/parsers_for_AI/src/adapters/text.rs | 222 | 7.2 | 1840 | safe |
| software/parsers_for_AI/src/adapters/toml.rs | 84 | 2.4 | 618 | safe |
| software/parsers_for_AI/src/adapters/xml.rs | 186 | 6.5 | 1676 | safe |
| software/parsers_for_AI/src/adapters/xml_dtd.rs | 1136 | 43.3 | 11085 | structure-only |
| software/parsers_for_AI/src/adapters/yaml.rs | 566 | 19.4 | 4968 | safe |
| software/parsers_for_AI/src/api.rs | 2007 | 73.4 | 18777 | structure-only |
| software/parsers_for_AI/src/collect.rs | 655 | 21.5 | 5503 | safe |
| software/parsers_for_AI/src/commands.rs | 316 | 11.4 | 2931 | safe |
| software/parsers_for_AI/src/confine.rs | 119 | 4.2 | 1076 | safe |
| software/parsers_for_AI/src/discovery.rs | 671 | 23.4 | 5992 | safe |
| software/parsers_for_AI/src/help.rs | 88 | 2.8 | 720 | safe |
| software/parsers_for_AI/src/jsonpath.rs | 1230 | 42.9 | 10992 | structure-only |
| software/parsers_for_AI/src/lib.rs | 21 | 0.4 | 90 | safe |
| software/parsers_for_AI/src/limits.rs | 115 | 3.0 | 768 | safe |
| software/parsers_for_AI/src/main.rs | 189 | 5.6 | 1432 | safe |
| software/parsers_for_AI/src/materialize.rs | 525 | 18.5 | 4725 | safe |
| software/parsers_for_AI/src/mcp.rs | 463 | 17.8 | 4551 | safe |
| software/parsers_for_AI/src/navigation.rs | 322 | 9.2 | 2347 | safe |
| software/parsers_for_AI/src/publish.rs | 149 | 4.5 | 1161 | safe |
| software/parsers_for_AI/src/render.rs | 792 | 28.5 | 7288 | safe |
| software/parsers_for_AI/src/run.rs | 769 | 26.6 | 6798 | safe |
| software/parsers_for_AI/src/template.rs | 1713 | 54.5 | 13956 | structure-only |
| software/parsers_for_AI/src/tools.rs | 628 | 26.5 | 6785 | safe |
| software/parsers_for_AI/src/transform.rs | 449 | 14.2 | 3624 | safe |
| software/parsers_for_AI/src/xpath.rs | 863 | 28.7 | 7346 | safe |
| software/parsers_for_AI/tests/api.rs | - | 58.3 | - | skip (fixture) |
| software/parsers_for_AI/tests/fuzz.rs | - | 111.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/fuzz_regressions.rs | - | 8.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/golden.rs | - | 18.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/known_limits.rs | - | 20.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/navigator_state.rs | - | 10.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/one_exe.rs | - | 18.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/regressions.rs | - | 17.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/root_bounds.rs | - | 12.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots.rs | - | 24.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/absolute_paths/args.txt | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/absolute_paths/expected.md | - | 3.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/absolute_paths/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/absolute_paths/sources/nested/orders.json | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/absolute_paths/sources/notes.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/absolute_paths/template.yml | - | 0.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/all_files_fail/expected.md | - | 3.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/all_files_fail/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/all_files_fail/sources/latin1.json | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/all_files_fail/sources/mismatched.xml | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/all_files_fail/sources/truncated.json | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/all_files_fail/template.yml | - | 0.4 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/anchor_text/expected.md | - | 5.3 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/anchor_text/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/anchor_text/sources/comments.json | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/anchor_text/sources/forged.txt | - | 0.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/anchor_text/template.yml | - | 0.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/cli_format_override/args.txt | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/cli_format_override/expected.md | - | 3.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/cli_format_override/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/cli_format_override/sources/alert.yaml | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/cli_format_override/sources/event.json | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/cli_format_override/template.yml | - | 0.4 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/dedup_sort/expected.md | - | 5.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/dedup_sort/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/dedup_sort/sources/a_server.log | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/dedup_sort/sources/b_worker.log | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/dedup_sort/template.yml | - | 0.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/args.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/expected.md | - | 3.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/sources/a_plain.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/sources/b_bom.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/sources/c_skipped.md | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/hash_glob_input/template.yml | - | 0.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/html_extract_modes/expected.md | - | 8.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/html_extract_modes/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/html_extract_modes/sources/product.html | - | 1.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/html_extract_modes/template.yml | - | 0.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/json_jsonpath_filters/expected.md | - | 6.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/json_jsonpath_filters/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/json_jsonpath_filters/sources/orders.json | - | 1.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/json_jsonpath_filters/template.yml | - | 0.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/markdown_nodes/expected.md | - | 9.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/markdown_nodes/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/markdown_nodes/sources/README.md | - | 0.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/markdown_nodes/template.yml | - | 0.8 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/expected.md | - | 7.8 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/Cargo.toml | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/build.log | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/config/app.yaml | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/config/service.conf | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/docs/guide.md | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/docs/index.html | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/manifest | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/modules.xml | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/sources/package.json | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/mixed_when_format/template.yml | - | 0.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/no_matches_not_applicable/expected.md | - | 5.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/no_matches_not_applicable/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/no_matches_not_applicable/sources/notes.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/no_matches_not_applicable/sources/users.json | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/no_matches_not_applicable/template.yml | - | 0.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/partial_unparsable/expected.md | - | 3.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/partial_unparsable/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/partial_unparsable/sources/a_users.json | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/partial_unparsable/sources/b_broken.json | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/partial_unparsable/sources/c_users.json | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/partial_unparsable/template.yml | - | 0.3 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_hard_line/args.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_hard_line/expected.md | - | 7.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_hard_line/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_hard_line/sources/one_line.txt | - | 2.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_hard_line/template.yml | - | 1.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_unicode_items/args.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_unicode_items/expected.md | - | 7.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_unicode_items/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_unicode_items/sources/release_notes.txt | - | 1.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/split_unicode_items/template.yml | - | 0.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/text_logs_context/expected.md | - | 10.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/text_logs_context/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/text_logs_context/sources/app.log | - | 0.4 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/text_logs_context/sources/classic_mac.log | - | 0.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/text_logs_context/sources/windows.log | - | 0.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/text_logs_context/template.yml | - | 0.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/toml_tables/expected.md | - | 6.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/toml_tables/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/toml_tables/sources/service.toml | - | 0.4 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/toml_tables/template.yml | - | 0.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/truncation_limits/expected.md | - | 6.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/truncation_limits/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/truncation_limits/sources/tickets.json | - | 1.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/truncation_limits/template.yml | - | 0.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/xml_namespaces_dtd/expected.md | - | 7.8 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/xml_namespaces_dtd/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/xml_namespaces_dtd/sources/catalog.xml | - | 0.8 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/xml_namespaces_dtd/template.yml | - | 1.2 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/yaml_multidoc_merge/expected.md | - | 5.5 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/yaml_multidoc_merge/expected_exit.txt | - | 0.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/yaml_multidoc_merge/sources/manifest.yaml | - | 0.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/snapshots/yaml_multidoc_merge/template.yml | - | 0.6 | - | skip (fixture) |
| software/parsers_for_AI/tests/stabilization.rs | - | 14.8 | - | skip (fixture) |
| software/parsers_for_AI/tests/support/mod.rs | - | 15.3 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_effective.rs | - | 21.1 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_html.rs | - | 7.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_jsonpath.rs | - | 10.0 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_release.rs | - | 31.8 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_status.rs | - | 26.9 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_unescape.rs | - | 2.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_xml.rs | - | 16.7 | - | skip (fixture) |
| software/parsers_for_AI/tests/tier2_yaml.rs | - | 9.6 | - | skip (fixture) |

## Zone: software/witness_prover

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| software/witness_prover/CUBE_REFERENCE.md | 3151 | 178.2 | 45280 | structure-only |
| software/witness_prover/README.md | 125 | 14.4 | 3675 | safe |
| software/witness_prover/cube.exe | - | 6458.0 | - | skip (binary) |

## Zone: Theory/agent-data-circuit

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| Theory/agent-data-circuit/.github/workflows/ci.yml | 53 | 1.6 | 405 | safe |
| Theory/agent-data-circuit/.gitignore | 32 | 0.4 | 91 | safe |
| Theory/agent-data-circuit/AI-ENTRY.md | 254 | 10.4 | 2665 | safe |
| Theory/agent-data-circuit/CASE-STUDY.md | 102 | 5.0 | 1279 | safe |
| Theory/agent-data-circuit/CHANGELOG.md | 31 | 1.1 | 290 | safe |
| Theory/agent-data-circuit/CODE_OF_CONDUCT.md | 80 | 5.1 | 1307 | safe |
| Theory/agent-data-circuit/CONFORMANCE.md | 153 | 10.0 | 2553 | safe |
| Theory/agent-data-circuit/CONTRIBUTING.md | 71 | 3.5 | 890 | safe |
| Theory/agent-data-circuit/ECONOMICS.md | 222 | 9.2 | 2357 | safe |
| Theory/agent-data-circuit/LICENSE-CODE | 202 | 11.3 | 2890 | safe |
| Theory/agent-data-circuit/LICENSE-DOCS | 396 | 18.6 | 4763 | safe |
| Theory/agent-data-circuit/NON-GOALS.md | 27 | 4.0 | 1031 | safe |
| Theory/agent-data-circuit/RATIONALE.md | 89 | 11.3 | 2899 | safe |
| Theory/agent-data-circuit/README.md | 288 | 13.2 | 3340 | safe |
| Theory/agent-data-circuit/SECURITY.md | 94 | 9.3 | 2380 | safe |
| Theory/agent-data-circuit/SPEC.md | 1182 | 50.3 | 12805 | structure-only |
| Theory/agent-data-circuit/agent-runtime/AGENT-RUNTIME.md | 149 | 7.9 | 2030 | safe |
| Theory/agent-data-circuit/examples/ecommerce/README.md | 47 | 2.5 | 639 | safe |
| Theory/agent-data-circuit/examples/ecommerce/ai.json | 12 | 0.3 | 80 | safe |
| Theory/agent-data-circuit/examples/ecommerce/generate.py | 777 | 28.2 | 7231 | safe |
| Theory/agent-data-circuit/examples/ecommerce/manifest.json | 101 | 2.1 | 533 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a1.json | 149 | 3.0 | 763 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a10.json | 194 | 4.1 | 1061 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a11.json | 195 | 4.2 | 1066 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a12.json | 194 | 4.1 | 1059 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a13.json | 194 | 4.1 | 1057 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a14.json | 195 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a15.json | 195 | 4.1 | 1061 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a16.json | 194 | 4.1 | 1054 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a17.json | 196 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a18.json | 195 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a19.json | 195 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a2.json | 142 | 2.8 | 729 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a20.json | 193 | 4.1 | 1054 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a21.json | 194 | 4.1 | 1059 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a22.json | 193 | 4.1 | 1058 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a23.json | 193 | 4.1 | 1062 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a24.json | 194 | 4.1 | 1058 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a25.json | 193 | 4.1 | 1057 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a26.json | 193 | 4.1 | 1061 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a27.json | 193 | 4.1 | 1052 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a28.json | 142 | 2.8 | 729 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a29.json | 149 | 3.0 | 776 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a3.json | 496 | 12.1 | 3099 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a30.json | 916 | 23.2 | 5950 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a31.json | 196 | 4.1 | 1055 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a32.json | 196 | 4.1 | 1056 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a33.json | 199 | 4.2 | 1073 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a34.json | 197 | 4.1 | 1062 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a35.json | 198 | 4.2 | 1069 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a36.json | 193 | 4.1 | 1048 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a37.json | 192 | 4.1 | 1044 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a38.json | 201 | 4.2 | 1086 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a39.json | 196 | 4.1 | 1054 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a4.json | 194 | 4.1 | 1055 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a40.json | 197 | 4.2 | 1064 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a41.json | 199 | 4.2 | 1075 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a42.json | 199 | 4.2 | 1072 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a43.json | 193 | 4.1 | 1048 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a44.json | 192 | 4.1 | 1042 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a45.json | 193 | 4.1 | 1050 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a46.json | 192 | 4.1 | 1042 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a47.json | 197 | 4.2 | 1067 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a48.json | 199 | 4.2 | 1071 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a49.json | 198 | 4.2 | 1074 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a5.json | 195 | 4.1 | 1062 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a50.json | 200 | 4.2 | 1085 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a51.json | 199 | 4.2 | 1070 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a52.json | 198 | 4.2 | 1064 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a53.json | 201 | 4.2 | 1082 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a54.json | 199 | 4.2 | 1068 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a55.json | 199 | 4.2 | 1073 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a56.json | 201 | 4.2 | 1082 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a57.json | 200 | 4.2 | 1075 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a58.json | 198 | 4.2 | 1066 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a59.json | 199 | 4.2 | 1072 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a6.json | 193 | 4.1 | 1052 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a60.json | 196 | 4.1 | 1054 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a61.json | 199 | 4.2 | 1072 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a62.json | 200 | 4.2 | 1078 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a63.json | 198 | 4.2 | 1069 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a64.json | 192 | 4.1 | 1039 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a65.json | 192 | 4.1 | 1040 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a66.json | 194 | 4.1 | 1050 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a67.json | 199 | 4.2 | 1073 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a68.json | 198 | 4.2 | 1076 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a69.json | 200 | 4.2 | 1076 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a7.json | 195 | 4.2 | 1063 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a70.json | 198 | 4.1 | 1062 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a71.json | 198 | 4.2 | 1066 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a72.json | 199 | 4.2 | 1070 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a73.json | 198 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a74.json | 198 | 4.2 | 1066 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a75.json | 197 | 4.2 | 1066 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a76.json | 198 | 4.2 | 1067 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a77.json | 197 | 4.1 | 1063 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a78.json | 199 | 4.2 | 1072 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a79.json | 376 | 9.0 | 2315 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a8.json | 193 | 4.1 | 1057 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a80.json | 196 | 4.1 | 1061 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a81.json | 196 | 4.1 | 1060 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a82.json | 198 | 4.2 | 1074 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a83.json | 197 | 4.2 | 1066 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a84.json | 197 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a85.json | 196 | 4.1 | 1060 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a86.json | 196 | 4.1 | 1058 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a87.json | 195 | 4.1 | 1048 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a88.json | 196 | 4.1 | 1054 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a89.json | 195 | 4.1 | 1054 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a9.json | 194 | 4.1 | 1062 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a90.json | 196 | 4.1 | 1059 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a91.json | 197 | 4.1 | 1059 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a92.json | 195 | 4.1 | 1052 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a93.json | 197 | 4.1 | 1061 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a94.json | 197 | 4.2 | 1065 | safe |
| Theory/agent-data-circuit/examples/ecommerce/nodes/a95.json | 196 | 4.1 | 1061 | safe |
| Theory/agent-data-circuit/examples/ecommerce/tree.json | 772 | 22.4 | 5743 | safe |
| Theory/agent-data-circuit/pytest.ini | 6 | 0.1 | 29 | safe |
| Theory/agent-data-circuit/reference/adc/__init__.py | 14 | 0.5 | 122 | safe |
| Theory/agent-data-circuit/reference/adc/aggregate.py | 511 | 17.5 | 4486 | safe |
| Theory/agent-data-circuit/reference/adc/anchors.py | 304 | 10.7 | 2727 | safe |
| Theory/agent-data-circuit/reference/adc/budget.py | 162 | 5.8 | 1497 | safe |
| Theory/agent-data-circuit/reference/adc/errors.py | 103 | 4.3 | 1098 | safe |
| Theory/agent-data-circuit/reference/adc/filters.py | 488 | 16.3 | 4162 | safe |
| Theory/agent-data-circuit/reference/adc/pathing.py | 75 | 2.4 | 605 | safe |
| Theory/agent-data-circuit/reference/adc/server.py | 1426 | 55.2 | 14120 | structure-only |
| Theory/agent-data-circuit/reference/adc/site.py | 596 | 23.3 | 5972 | safe |
| Theory/agent-data-circuit/reference/adc/tokens.py | 64 | 2.2 | 567 | safe |
| Theory/agent-data-circuit/reference/mcp-tools/README.md | 46 | 2.2 | 558 | safe |
| Theory/agent-data-circuit/reference/mcp-tools/adc_mcp.py | 891 | 31.4 | 8033 | safe |
| Theory/agent-data-circuit/reference/validator/adc.py | 128 | 4.3 | 1095 | safe |
| Theory/agent-data-circuit/rfcs/0001-stateless-anchors.md | 164 | 10.5 | 2689 | safe |
| Theory/agent-data-circuit/rfcs/0002-token-accounting.md | 153 | 10.0 | 2571 | safe |
| Theory/agent-data-circuit/rfcs/0003-k-anonymous-aggregates.md | 160 | 10.8 | 2762 | safe |
| Theory/agent-data-circuit/rfcs/0004-degradation-contract.md | 160 | 11.3 | 2900 | safe |
| Theory/agent-data-circuit/schemas/v0.1/aggregate-request.schema.json | 168 | 5.1 | 1302 | safe |
| Theory/agent-data-circuit/schemas/v0.1/aggregate-response.schema.json | 91 | 3.0 | 770 | safe |
| Theory/agent-data-circuit/schemas/v0.1/block.schema.json | 74 | 2.4 | 608 | safe |
| Theory/agent-data-circuit/schemas/v0.1/fetch-request.schema.json | 132 | 4.1 | 1040 | safe |
| Theory/agent-data-circuit/schemas/v0.1/fetch-response.schema.json | 84 | 2.7 | 698 | safe |
| Theory/agent-data-circuit/schemas/v0.1/index.json | 13 | 1.0 | 254 | safe |
| Theory/agent-data-circuit/schemas/v0.1/manifest.schema.json | 132 | 4.5 | 1159 | safe |
| Theory/agent-data-circuit/schemas/v0.1/passport.schema.json | 140 | 4.5 | 1150 | safe |
| Theory/agent-data-circuit/schemas/v0.1/problem.schema.json | 42 | 3.0 | 765 | safe |
| Theory/agent-data-circuit/schemas/v0.1/tree.schema.json | 38 | 1.3 | 338 | safe |
| Theory/agent-data-circuit/tests/conftest.py | - | 6.8 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_aggregate.py | - | 8.9 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_ai_entry.py | - | 7.7 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_anchors.py | - | 9.0 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_budget.py | - | 5.0 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_degradation.py | - | 6.1 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_filters.py | - | 10.6 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_mcp_bridge.py | - | 7.6 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_pathing.py | - | 3.1 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_server_endpoints.py | - | 26.5 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_site_validation.py | - | 6.3 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_validation_order.py | - | 8.0 | - | skip (fixture) |
| Theory/agent-data-circuit/tests/test_validator_cli.py | - | 4.6 | - | skip (fixture) |
| Theory/agent-data-circuit/tools/measure_case_study.py | 575 | 19.1 | 4878 | safe |

## Zone: Theory/ontology_of_number

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| Theory/ontology_of_number/APPENDIX_MATH.md | 30 | 1.4 | 348 | safe |
| Theory/ontology_of_number/AnchorFile_SuperTable_v1.0.json | 98 | 10.3 | 2630 | safe |
| Theory/ontology_of_number/Borisov_K_theory.md | 1782 | 98.4 | 23068 | structure-only |
| Theory/ontology_of_number/CRITIQUE.md | 42 | 3.7 | 921 | safe |
| Theory/ontology_of_number/New_Peano.md | 317 | 25.3 | 6454 | safe |
| Theory/ontology_of_number/README.md | 75 | 4.0 | 1003 | safe |
| Theory/ontology_of_number/helpdoc.py | 1419 | 60.1 | 14589 | structure-only |
| Theory/ontology_of_number/independent_verify.py | 483 | 22.7 | 5771 | safe |
| Theory/ontology_of_number/nav_index.md | 345 | 17.9 | 4113 | safe |
| Theory/ontology_of_number/opterium_v1.0_spec.txt | 908 | 51.8 | 13092 | structure-only |
| Theory/ontology_of_number/outside_AI_opinion.md | 104 | 15.4 | 3945 | safe |
| Theory/ontology_of_number/readme_supplement.md | 269 | 13.1 | 3280 | safe |

## Zone: software

| path | L | KB | ~tok | mode |
|---|---|---|---|---|
| software/README.md | 79 | 4.9 | 1254 | safe |

## Reading policy

- `safe` — small/medium text (< 1500 lines AND < 40 KB): fine to read fully.
- `structure-only` — large reference docs (>= 1500 lines or >= 40 KB): read the heading map or targeted windows only, never the whole file.
- `skip` — do not read: binaries (.exe, .bin, .so, images, assets), `Cargo.lock`, .git* config, and test fixture content under `tests/` / `tests/snapshots/`.

