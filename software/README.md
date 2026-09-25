# Software

Programs kept in this repository, apart from the theory files in the root.

## parsers_for_AI/

Two portable Windows tools for AI-assisted code work - single static
executables, zero dependencies:

| Tool | Version | Purpose |
|---|---|---|
| `navguard.exe` | 0.1.0 | Structural map: file tree, line/token counts, symbol index |
| `any_txt_parser.exe` | 1.0.2 | Rule-based extractor: YAML template -> Markdown log with blocks |

- `help_full_parser.md` - AI field guide: orient, drill, recon, extract
- `learn_full_parser.md` - examples library and YAML templates

Give an AI these files plus the executables and it can orient in any
project and extract data without other documentation.

## Standalone repositories

- [agent-data-circuit](https://github.com/UserPC-SPB/agent-data-circuit) - ADC,
  a publisher-side protocol that exposes site data to AI agents over plain
  HTTP + JSON (manifest, anchors, token budgets, cold entry).