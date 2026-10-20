"""MCP bridge tests (reference/mcp-tools/adc_mcp.py).

The bridge is a stdio JSON-RPC 2.0 server, so each test drives it as a
subprocess over stdin/stdout and parses the JSON lines it writes back.
Every ADC call goes through the live server started by the ``client``
fixture, making these end-to-end: subprocess -> HTTP -> reference server.
"""

import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
MCP_SCRIPT = REPO_ROOT / "reference" / "mcp-tools" / "adc_mcp.py"
PROTOCOL_VERSION = "2024-11-05"


def session(root, calls, timeout=30):
    """Run the bridge with initialize + initialized + the given tool calls."""
    messages = [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "pytest", "version": "0"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
    ]
    for index, (tool, arguments) in enumerate(calls, start=2):
        arguments = dict(arguments, root=root)
        messages.append(
            {
                "jsonrpc": "2.0",
                "id": index,
                "method": "tools/call",
                "params": {"name": tool, "arguments": arguments},
            }
        )

    payload = "".join(json.dumps(message) + "\n" for message in messages).encode("utf-8")
    completed = subprocess.run(
        [sys.executable, str(MCP_SCRIPT)],
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        cwd=str(REPO_ROOT),
    )
    assert completed.returncode == 0, completed.stderr.decode("utf-8", "replace")

    replies = {}
    for line in completed.stdout.decode("utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        message = json.loads(line)
        if "id" in message:
            replies[message["id"]] = message
    return replies


def text_of(reply):
    """Decode a tool call reply into its JSON payload or raw text."""
    result = reply["result"]
    return result["content"][0]["text"]


def test_initialize_matches_the_mcp_shape():
    completed = subprocess.run(
        [sys.executable, str(MCP_SCRIPT)],
        input=json.dumps(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "pytest", "version": "0"},
                },
            }
        ).encode("utf-8")
        + b"\n",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        cwd=str(REPO_ROOT),
    )
    reply = json.loads(completed.stdout.decode("utf-8").strip())
    result = reply["result"]
    assert result["protocolVersion"] == PROTOCOL_VERSION
    assert result["capabilities"] == {"tools": {}}
    assert result["serverInfo"]["name"] == "adc-mcp"
    # the initialize result carries only the three protocol-defined members
    assert set(result) == {"protocolVersion", "capabilities", "serverInfo"}


def test_tools_list_exposes_the_adc_methods():
    completed = subprocess.run(
        [sys.executable, str(MCP_SCRIPT)],
        input=(
            json.dumps(
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": {"name": "pytest", "version": "0"},
                    },
                }
            )
            + "\n"
            + json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
            + "\n"
        ).encode("utf-8"),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
        cwd=str(REPO_ROOT),
    )
    reply = json.loads(completed.stdout.decode("utf-8").strip().splitlines()[-1])
    names = [tool["name"] for tool in reply["result"]["tools"]]
    assert names == [
        "adc_orient",
        "adc_tree",
        "adc_passport",
        "adc_fetch",
        "adc_aggregate",
        # last: the cold entry is optional and never a conformance requirement
        "adc_cold_entry",
    ]
    for tool in reply["result"]["tools"]:
        assert "inputSchema" in tool
        props = tool["inputSchema"]["properties"]
        if tool["name"] == "adc_cold_entry":
            assert tool["inputSchema"]["required"] == ["origin"]
            assert "origin" in props
        else:
            assert "root" in props


def test_orient_and_fetch_reach_the_live_server(client, site):
    replies = session(
        client.base_url,
        [
            ("adc_orient", {}),
            ("adc_fetch", {"anchor": "[a30]", "limit": 2}),
        ],
    )

    orient = json.loads(text_of(replies[2]))
    assert orient["adc"] == "0.1"
    # SPEC P5: `root` is the manifest's declared origin, not whatever port
    # this test happens to bind - which is exactly why README warns about it.
    assert orient["root"] == site.root

    fetched = json.loads(text_of(replies[3]))
    assert len(fetched["items"]) == 2
    assert fetched["parent_anchor"] == "[a30]"


def test_aggregate_reaches_the_live_server(client):
    replies = session(
        client.base_url,
        [
            (
                "adc_aggregate",
                {"anchor": "[a30]", "metrics": "count():count", "group_by": "brand"},
            )
        ],
    )
    payload = json.loads(text_of(replies[2]))
    assert payload["anchor"] == "[a30]"
    assert payload["groups"]
    assert payload["partial"] is True


def test_cold_entry_reaches_the_live_server(client):
    # The cold entry takes `origin`, not `root`: it never consults the ADC
    # manifest (AI-ENTRY.md section 8).
    replies = session(
        client.base_url,
        [
            ("adc_cold_entry", {"origin": client.base_url}),
            ("adc_cold_entry", {"origin": client.base_url, "var": "h1"}),
            (
                "adc_cold_entry",
                {"origin": client.base_url, "var": "h1", "get": "price"},
            ),
        ],
    )

    tunnel = json.loads(text_of(replies[2]))
    assert tunnel["ai"] == "1.0"
    assert tunnel["vars"]["h1"] == "/electronics/audio/headphones"

    fragment = json.loads(text_of(replies[3]))
    assert fragment["get"] == "text"
    assert fragment["path"] == "/electronics/audio/headphones"

    # key_unavailable stays inside the tunnel format (AI-ENTRY.md section 7)
    denied = replies[4]
    assert "error" not in denied
    assert denied["result"]["isError"] is True
    payload = json.loads(text_of(denied))
    assert payload["error"] == "key_unavailable"
    assert payload["available"] == ["text", "meta"]


def test_server_errors_return_is_error_not_jsonrpc_error(client):
    # SPEC 6.12 Problem Details must reach the model as tool content with
    # isError set; a JSON-RPC-level error would hide the RFC 9457 body.
    replies = session(client.base_url, [("adc_tree", {"path": "/nope"})])
    reply = replies[2]
    assert "error" not in reply
    assert reply["result"]["isError"] is True
    payload = json.loads(text_of(reply))
    assert payload["status"] == 404
    assert payload["type"].endswith("/ANCHOR_NOT_FOUND")
