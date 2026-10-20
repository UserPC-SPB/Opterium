# ADC MCP bridge

`adc_mcp.py` is a stdio JSON-RPC 2.0 server (MCP protocol 2024-11-05) exposing
Agent Data Circuit operations as MCP tools, plus the optional `/AI/` cold
entry. It is a thin HTTP client: each tool builds the query string or JSON
body it needs and returns the parsed JSON as the tool result.

## Run

    python reference/mcp-tools/adc_mcp.py

Reads newline-delimited JSON-RPC from stdin, writes one JSON object per line to stdout, and logs diagnostics to stderr. Standard library only, Python 3.11+.

## Tools

| Tool | Endpoint | Base URL argument |
|---|---|---|
| `adc_orient` | GET /adc/orient | `root` |
| `adc_tree` | GET /adc/tree | `root` |
| `adc_passport` | GET /adc/passport | `root` |
| `adc_fetch` | GET or POST /adc/fetch | `root` |
| `adc_aggregate` | GET or POST /adc/aggregate | `root` |
| `adc_cold_entry` | GET /AI/ and /AI[variable] | `origin` |

`adc_passport`, `adc_fetch` and `adc_aggregate` additionally require one of
`anchor` / `path`, `adc_aggregate` requires `metrics`, and `adc_cold_entry`
takes `var` and `get` (AI-ENTRY.md). `root` is the base URL of the ADC
server, for example `http://127.0.0.1:8080`; `origin` is the same idea for
the cold entry, which never consults the manifest. ADC endpoint paths come
from `GET <root>/.well-known/adc.json` when the manifest is served,
otherwise the SPEC 4.2 defaults are used. Flat filters use the GET grammar
`filter[field][op]=value`; `and` / `or` / `not` trees fall back to the POST
profile (SPEC 6.4.5, 6.9.3) with an `Idempotency-Key`.

## Errors

Non-2xx bodies are parsed as RFC 9457 Problem Details and returned with
`isError: true`, surfacing `type`, `title`, `status`, `detail`, `retryable`.
Bodies that are not Problem Details — the tunnel errors of AI-ENTRY.md, for
instance — keep their own fields alongside those keys. Connection failures
and local argument validation use the same shape. Unknown tools return
`-32602`, unknown methods `-32601`, bad lines `-32700`.

## Smoke test

    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' | python reference/mcp-tools/adc_mcp.py
