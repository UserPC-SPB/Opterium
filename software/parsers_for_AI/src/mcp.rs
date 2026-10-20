//! A Model Context Protocol server over stdio (`any_txt_parser mcp`).
//!
//! Messages are newline-delimited JSON-RPC 2.0, one per line, as the MCP
//! stdio transport requires. stdout carries protocol messages only; any
//! diagnostics go to stderr. Requests are answered in order.

use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// Protocol revisions this server speaks, newest first. The newest one is
/// offered when a client asks for a revision not in the list.
pub const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// One tool the server exposes.
pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// JSON Schema of the `arguments` object.
    pub input_schema: Value,
    /// Runs the tool. `Ok` is the text returned to the model; `Err` is
    /// reported as a tool error (`isError: true`) the model can act on.
    pub handler: fn(&Value) -> Result<String, String>,
}

/// One resource the server exposes (read with `resources/read`).
pub struct Resource {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
    /// Produces the current content; `Err` is reported as a JSON-RPC error.
    pub reader: fn() -> Result<String, String>,
}

/// JSON-RPC error code of `resources/read` for an unknown URI (MCP).
pub const RESOURCE_NOT_FOUND: i64 = -32002;

pub struct Server {
    pub name: &'static str,
    pub version: &'static str,
    /// Sent in the initialize result: how a model should start using the
    /// server.
    pub instructions: &'static str,
    pub tools: Vec<Tool>,
    pub resources: Vec<Resource>,
}

impl Server {
    /// Serves until stdin closes. Returns the process exit code.
    pub fn serve(&self, input: impl BufRead, mut output: impl Write) -> i32 {
        for line in input.lines() {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    eprintln!("any_txt_parser mcp: cannot read stdin: {}", e);
                    return 1;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reply) = self.handle_line(&line) {
                let text = reply.to_string();
                if writeln!(output, "{}", text)
                    .and_then(|_| output.flush())
                    .is_err()
                {
                    // The client went away.
                    return 0;
                }
            }
        }
        0
    }

    /// Handles one incoming line; returns the reply, if the message needs one.
    pub fn handle_line(&self, line: &str) -> Option<Value> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {}", e))),
        };
        match message {
            Value::Array(batch) => {
                // Older revisions allowed batches; answer every request in it.
                let replies: Vec<Value> = batch.iter().filter_map(|m| self.handle(m)).collect();
                (!replies.is_empty()).then_some(Value::Array(replies))
            }
            message => self.handle(&message),
        }
    }

    fn handle(&self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // A response from the client (we never send requests) or garbage.
            return id.map(|id| error(id, -32600, "invalid request: missing method"));
        };
        // Notifications (no id) never get a reply.
        let id = id?;
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        Some(match method {
            "initialize" => success(id, self.initialize(&params)),
            "ping" => success(id, json!({})),
            "tools/list" => success(id, json!({ "tools": self.tool_list() })),
            "tools/call" => match self.call_tool(&params) {
                Ok(result) => success(id, result),
                Err(message) => error(id, -32602, &message),
            },
            "resources/list" => success(id, json!({ "resources": self.resource_list() })),
            "resources/read" => match self.read_resource(&params) {
                Ok(result) => success(id, result),
                Err((code, message)) => error(id, code, &message),
            },
            // Not offered in capabilities, answered so lenient clients that
            // probe them anyway do not see errors.
            "resources/templates/list" => success(id, json!({ "resourceTemplates": [] })),
            "prompts/list" => success(id, json!({ "prompts": [] })),
            other => error(id, -32601, &format!("method not found: {}", other)),
        })
    }

    fn initialize(&self, params: &Value) -> Value {
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        let version = requested
            .filter(|v| PROTOCOL_VERSIONS.contains(v))
            .unwrap_or(PROTOCOL_VERSIONS[0]);
        let mut capabilities = json!({ "tools": { "listChanged": false } });
        if !self.resources.is_empty() {
            capabilities["resources"] = json!({ "subscribe": false, "listChanged": false });
        }
        json!({
            "protocolVersion": version,
            "capabilities": capabilities,
            "serverInfo": { "name": self.name, "version": self.version },
            "instructions": self.instructions,
        })
    }

    fn resource_list(&self) -> Vec<Value> {
        self.resources
            .iter()
            .map(|r| {
                json!({
                    "uri": r.uri,
                    "name": r.name,
                    "title": r.title,
                    "description": r.description,
                    "mimeType": r.mime_type,
                })
            })
            .collect()
    }

    /// `Err` is (JSON-RPC error code, message).
    fn read_resource(&self, params: &Value) -> Result<Value, (i64, String)> {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or((-32602, "resources/read needs params.uri".to_string()))?;
        let resource = self
            .resources
            .iter()
            .find(|r| r.uri == uri)
            .ok_or_else(|| {
                let known: Vec<&str> = self.resources.iter().map(|r| r.uri).collect();
                (
                    RESOURCE_NOT_FOUND,
                    format!("resource not found: {} (known: {})", uri, known.join(", ")),
                )
            })?;
        // A panic while reading must not take the server down.
        let text = std::panic::catch_unwind(resource.reader)
            .unwrap_or_else(|_| Err(format!("internal error reading {} (a bug)", uri)))
            .map_err(|message| (-32603, message))?;
        Ok(json!({
            "contents": [{ "uri": resource.uri, "mimeType": resource.mime_type, "text": text }]
        }))
    }

    fn tool_list(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "title": tool.title,
                    "description": tool.description,
                    "inputSchema": tool.input_schema,
                })
            })
            .collect()
    }

    /// `Err` is a protocol error (unknown tool, malformed params); tool
    /// failures are successful calls with `isError: true`.
    fn call_tool(&self, params: &Value) -> Result<Value, String> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or("tools/call needs params.name")?;
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .ok_or_else(|| format!("unknown tool: {}", name))?;
        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => json!({}),
            Some(Value::Object(map)) => Value::Object(map.clone()),
            Some(_) => return Err("tools/call params.arguments must be an object".to_string()),
        };
        // A panic inside one tool must not take the server down.
        let outcome = std::panic::catch_unwind(|| (tool.handler)(&arguments))
            .unwrap_or_else(|_| Err(format!("internal error in tool '{}' (a bug)", name)));
        Ok(match outcome {
            Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
            Err(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
        })
    }
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Typed access to tool arguments with error messages a model can act on.
pub mod args {
    use serde_json::Value;

    pub fn string(args: &Value, name: &str) -> Result<String, String> {
        optional_string(args, name)?.ok_or_else(|| format!("missing required argument '{}'", name))
    }

    pub fn optional_string(args: &Value, name: &str) -> Result<Option<String>, String> {
        match args.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(other) => Err(format!(
                "argument '{}' must be a string, got {}",
                name, other
            )),
        }
    }

    pub fn optional_u64(args: &Value, name: &str) -> Result<Option<u64>, String> {
        match args.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(n)) => n
                .as_u64()
                .map(Some)
                .ok_or_else(|| format!("argument '{}' must be a non-negative integer", name)),
            Some(other) => Err(format!(
                "argument '{}' must be a non-negative integer, got {}",
                name, other
            )),
        }
    }

    pub fn optional_bool(args: &Value, name: &str) -> Result<Option<bool>, String> {
        match args.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(other) => Err(format!(
                "argument '{}' must be true or false, got {}",
                name, other
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo(args: &Value) -> Result<String, String> {
        match args::optional_string(args, "fail")? {
            Some(reason) => Err(reason),
            None => Ok(format!("echo {}", args::string(args, "text")?)),
        }
    }

    fn panics(_: &Value) -> Result<String, String> {
        panic!("boom")
    }

    fn server() -> Server {
        Server {
            name: "test",
            version: "0.0.1",
            instructions: "Call echo.",
            tools: vec![
                Tool {
                    name: "echo",
                    title: "Echo",
                    description: "Echoes text",
                    input_schema: json!({"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}),
                    handler: echo,
                },
                Tool {
                    name: "panics",
                    title: "Panics",
                    description: "Always panics",
                    input_schema: json!({"type": "object"}),
                    handler: panics,
                },
            ],
            resources: vec![
                Resource {
                    uri: "test://hello",
                    name: "hello",
                    title: "Hello",
                    description: "Greets",
                    mime_type: "text/plain",
                    reader: || Ok("hello".to_string()),
                },
                Resource {
                    uri: "test://broken",
                    name: "broken",
                    title: "Broken",
                    description: "Always fails",
                    mime_type: "text/plain",
                    reader: || Err("cannot read".to_string()),
                },
            ],
        }
    }

    #[test]
    fn resources_are_listed_and_read() {
        let s = server();
        let init = s
            .handle_line(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#)
            .unwrap();
        assert_eq!(
            init["result"]["capabilities"]["resources"],
            json!({"subscribe": false, "listChanged": false})
        );
        let list = s
            .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#)
            .unwrap();
        assert_eq!(list["result"]["resources"][0]["uri"], "test://hello");
        assert_eq!(list["result"]["resources"][0]["mimeType"], "text/plain");
        let read = s
            .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"test://hello"}}"#)
            .unwrap();
        assert_eq!(
            read["result"]["contents"],
            json!([{"uri": "test://hello", "mimeType": "text/plain", "text": "hello"}])
        );
        let missing = s
            .handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"test://nope"}}"#)
            .unwrap();
        assert_eq!(missing["error"]["code"], RESOURCE_NOT_FOUND);
        let broken = s
            .handle_line(r#"{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"test://broken"}}"#)
            .unwrap();
        assert_eq!(broken["error"]["message"], "cannot read");
        let no_uri = s
            .handle_line(r#"{"jsonrpc":"2.0","id":5,"method":"resources/read","params":{}}"#)
            .unwrap();
        assert_eq!(no_uri["error"]["code"], -32602);
    }

    fn roundtrip(lines: &[Value]) -> Vec<Value> {
        let input: String = lines.iter().map(|l| format!("{}\n", l)).collect();
        let mut out = Vec::new();
        assert_eq!(server().serve(input.as_bytes(), &mut out), 0);
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn full_session_answers_requests_in_order_and_skips_notifications() {
        let replies = roundtrip(&[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":"two","method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"text":"hi"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"ping"}),
        ]);
        assert_eq!(replies.len(), 4);
        assert_eq!(replies[0]["id"], 1);
        assert_eq!(replies[0]["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(replies[0]["result"]["serverInfo"]["name"], "test");
        assert_eq!(replies[0]["result"]["instructions"], "Call echo.");
        assert_eq!(replies[1]["id"], "two");
        assert_eq!(replies[1]["result"]["tools"][0]["name"], "echo");
        assert_eq!(
            replies[1]["result"]["tools"][0]["inputSchema"]["required"][0],
            "text"
        );
        assert_eq!(replies[2]["result"]["content"][0]["text"], "echo hi");
        assert_eq!(replies[2]["result"]["isError"], false);
        assert_eq!(replies[3]["result"], json!({}));
    }

    #[test]
    fn unknown_versions_get_the_newest_one() {
        let replies = roundtrip(&[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}),
        ]);
        assert_eq!(
            replies[0]["result"]["protocolVersion"],
            PROTOCOL_VERSIONS[0]
        );
    }

    #[test]
    fn errors_are_reported_without_stopping_the_server() {
        let s = server();
        let reply = s.handle_line("{not json").unwrap();
        assert_eq!(reply["error"]["code"], -32700);
        let reply = s
            .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#)
            .unwrap();
        assert_eq!(reply["error"]["code"], -32601);
        let reply = s
            .handle_line(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"missing"}}"#,
            )
            .unwrap();
        assert_eq!(reply["error"]["code"], -32602);
        // Tool failures and panics are results the model can read.
        let reply = s
            .handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"fail":"bad input"}}}"#)
            .unwrap();
        assert_eq!(reply["result"]["isError"], true);
        assert_eq!(reply["result"]["content"][0]["text"], "bad input");
        let reply = s
            .handle_line(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"echo","arguments":{}}}"#)
            .unwrap();
        assert_eq!(
            reply["result"]["content"][0]["text"],
            "missing required argument 'text'"
        );
        let reply = s
            .handle_line(
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"panics"}}"#,
            )
            .unwrap();
        assert_eq!(reply["result"]["isError"], true);
        assert!(
            s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/cancelled"}"#)
                .is_none()
        );
    }

    #[test]
    fn batches_get_one_reply_array() {
        let reply = server()
            .handle_line(r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#)
            .unwrap();
        assert_eq!(reply.as_array().unwrap().len(), 2);
    }
}
