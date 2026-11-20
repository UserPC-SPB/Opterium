//! Every render, mode and operation on small exact cases, through the public
//! pipeline (what the agent would see).

use toolfmt::json::J;
use toolfmt::pipeline::{Body, Data, Overrides, Request, run};
use toolfmt::standard::compile;

/// Agent text for `data` under the standard `yaml`.
fn fmt_args(yaml: &str, data: &str, args: &str) -> String {
    let std = compile(yaml, "test", "t").unwrap_or_else(|e| panic!("{e}"));
    let args = J::parse(args).unwrap();
    let req = Request {
        server: "s",
        tool: "t",
        args: &args,
        data: Data::Text(data),
        keys: None,
        ov: Overrides::default(),
        auto: false,
        family: None,
    };
    run(None, &req, Some(&std)).agent_text(data)
}

fn fmt(yaml: &str, data: &str) -> String {
    fmt_args(yaml, data, "{}")
}

#[test]
fn toon_quotes_what_would_be_misread() {
    let data = r#"[{"a":"x,y","b":"say \"hi\"","c":"two\nlines","d":" pad","e":"","f":"true","g":"12","h":12,"i":null,"j":{"k":[1,2]}},{"a":"plain"}]"#;
    let out = fmt("render: objects_table", data);
    assert_eq!(
        out,
        "[objects_table]\n[2]{a,b,c,d,e,f,g,h,i,j}:\n  \"x,y\",\"say \\\"hi\\\"\",\"two\\nlines\",\" pad\",\"\",\"true\",\"12\",12,null,\"{\\\"k\\\":[1,2]}\"\n  plain,,,,,,,,,"
    );
}

#[test]
fn objects_table_columns_caps_and_formats() {
    let data = r#"[{"id":1,"user":{"login":"ann","id":7},"size":1536,"n":1234567},{"id":2,"user":{"login":"bob"},"size":10,"n":5},{"id":3,"user":{"login":"cy"},"size":0,"n":0}]"#;
    let out = fmt(
        "render: objects_table\ncolumns: [id, user.login, size, n]\nformats: {size: bytes, n: thousands}\nrows: 2\n",
        data,
    );
    assert_eq!(
        out,
        "[objects_table:lossy]\n[2]{id,user.login,size,n}:\n  1,ann,1.5 KB,\"1,234,567\"\n  2,bob,10 B,5\n… 1 more rows"
    );
    let out = fmt(
        "render: objects_table\nmax_cell_chars: 3\n",
        r#"[{"s":"abcdef"}]"#,
    );
    assert_eq!(
        out,
        "[objects_table:lossy]\n[1]{s}:\n  abc…\n… 1 cells cut to 3 chars"
    );
}

#[test]
fn objects_table_markdown_escapes() {
    let out = fmt(
        "render: objects_table\nmode: markdown\n",
        r#"[{"a":"x|y","b":"l1\nl2","c":""}]"#,
    );
    assert_eq!(
        out,
        "[objects_table]\n| a | b | c |\n|---|---|---|\n| x\\|y | l1<br>l2 | \"\" |"
    );
}

#[test]
fn envelopes_keep_their_fields() {
    let data = r#"{"data":{"items":[{"id":1}],"total":9},"meta":{"next_cursor":"c2"}}"#;
    let out = fmt("render: objects_table\nroot: data.items\n", data);
    assert_eq!(
        out,
        "[objects_table]\nmeta: {\"next_cursor\":\"c2\"}\ndata.total: 9\nitems[1]{id}:\n  1"
    );
    // Without root the only array is picked; siblings kept.
    let out = fmt(
        "render: objects_table",
        r#"{"rows":[{"a":1}],"has_more":true}"#,
    );
    assert_eq!(out, "[objects_table]\nhas_more: true\nrows[1]{a}:\n  1");
}

#[test]
fn matrix_plain_and_markdown() {
    let m = "[[1,2,3],[4,5,6],[7,8,9]]";
    assert_eq!(
        fmt("render: matrix_table\nmax_rows: 2\nmax_cols: 2\n", m),
        "[matrix_table:lossy]\n3x3\n1 2\n4 5\n… 1 more rows\n… 1 more columns"
    );
    assert_eq!(
        fmt("render: matrix_table\nmode: markdown\n", "[[1,2],[3]]"),
        "[matrix_table]\n| | 0 | 1 |\n|---|---|---|\n| 0 | 1 | 2 |\n| 1 | 3 |"
    );
    assert_eq!(
        fmt("render: matrix_table", r#"[["a b",1],["",2]]"#),
        "[matrix_table]\n2x2\n\"a b\" 1\n\"\" 2"
    );
}

#[test]
fn dict_lines_and_markdown() {
    let d = r#"{"name":"x","multi":"a\nb","a:b":1,"nested":{"k":[1]},"n":1500}"#;
    assert_eq!(
        fmt("render: dict_table\nformats: {n: thousands}\n", d),
        "[dict_table]\nname: x\nmulti: \"a\\nb\"\n\"a:b\": 1\nnested: {\"k\":[1]}\nn: 1,500"
    );
    assert_eq!(
        fmt("render: dict_table\nmode: markdown\nmax_rows: 1\n", d),
        "[dict_table:lossy]\n| key | value |\n|---|---|\n| name | x |\n… 4 more keys"
    );
}

#[test]
fn expression_refs_formats_and_failures() {
    let y =
        "render: expression\nformat: \"{{{a}}} x {b:thousands} = {result.v} ({result.w[1]})\"\n";
    assert_eq!(
        fmt_args(y, r#"{"v":"ok","w":[1,2]}"#, r#"{"a":"A","b":1234567}"#),
        "[expression]\n{A} x 1,234,567 = ok (2)"
    );
    // A missing value is a fallback, never an invented output (D2).
    let out = fmt_args("render: expression\nformat: \"{nope}\"\n", "5", "{}");
    assert_eq!(
        out,
        "[fallback:raw] expression: placeholder {nope} has no value\n5"
    );
    assert!(
        compile(
            "render: expression\nformat: \"{a.__class__()}\"\n",
            "t",
            "t"
        )
        .is_err()
    );
}

#[test]
fn log_caps_and_counts() {
    let mut lines = Vec::new();
    for i in 0..5 {
        lines.push(format!("2026-01-01 10:00:0{i} ERROR e{i}"));
    }
    lines.push("2026-01-01 10:00:09 ERROR e0".into());
    lines.push("[warn] w".into());
    lines.push("INFO fine".into());
    let out = fmt("render: log\nmax_items: 2\n", &lines.join("\n"));
    assert_eq!(
        out,
        "[log:lossy]\n8 lines: ERROR 6, WARN 1, INFO 1\nerrors (5 unique):\n  ERROR e0 ×2\n  ERROR e1\nwarnings (1 unique):\n  [warn] w\n… 3 more unique errors"
    );
    assert!(fmt("render: log", "no levels\nat all\nhere").starts_with("[fallback:raw] log:"));
}

#[test]
fn num_stats_exact_and_approximate() {
    assert_eq!(
        fmt("render: num_stats", "[1, 2]"),
        "[num_stats]\ncount 2 | min 1 | max 2 | sum 3 | mean 3/2 (≈1.5)\nvalues: 1, 2"
    );
    assert_eq!(
        fmt("render: num_stats", "[0.5, 1.5, 2]"),
        "[num_stats]\ncount 3 | min 0.5 | max 2 | sum 4 | mean ≈1.333333\nvalues: 0.5, 1.5, 2"
    );
    // Beyond i128 the sum is approximate and says so.
    let big = "[170141183460469231731687303715884105727, 170141183460469231731687303715884105727]";
    let out = fmt("render: num_stats", big);
    assert!(
        out.contains("sum ≈3.402823669209385e38 | mean ≈1.7014118346046923e38"),
        "{out}"
    );
    assert!(
        out.contains("max 170141183460469231731687303715884105727"),
        "{out}"
    );
}

#[test]
fn tree_and_json() {
    let out = fmt(
        "render: tree\nmax_items: 1\n",
        r#"["/srv/a/x.rs","/srv/a/y.rs","/srv/b/z.rs"]"#,
    );
    assert_eq!(
        out,
        "[tree:lossy]\n/srv/ (3 paths)\na/: x.rs, y.rs\n… 1 more directories"
    );
    let out = fmt(
        "render: json\nroot: items\npre: [{select: [id]}, {sort_desc: id}]\n",
        r#"{"items":[{"id":1,"x":0},{"id":2,"x":0}],"next":"n2"}"#,
    );
    assert_eq!(
        out,
        "[json:lossy]\n{\"next\":\"n2\",\"items\":[{\"id\":2},{\"id\":1}]}"
    );
}

#[test]
fn post_operations() {
    let text = "alpha 1\nbeta 2\ngamma 3\npassword=secret123";
    assert_eq!(
        fmt(
            "post: [{replace: {from: '(\\w+) (\\d)', to: '$2:$1', regex: true}}, {keep_lines: ':'}, {join: ' | '}]",
            text
        ),
        "[passthrough:lossy]\n1:alpha | 2:beta | 3:gamma\n… 1 lines dropped (keep_lines)"
    );
    assert_eq!(
        fmt(
            "post: [{mask: [password]}, {truncate_lines: 1}]",
            "password=secret123\nnext"
        ),
        "[passthrough:lossy]\npassword=***\n… 1 more lines"
    );
    // A literal replace does not expand $1.
    assert_eq!(
        fmt("post: [{replace: {from: a, to: '$1'}}]", "a"),
        "[passthrough]\n$1"
    );
}

#[test]
fn pre_operations_on_objects() {
    let data = r#"[{"k":2},{"k":1},{"k":2},{"z":0}]"#;
    let out = fmt("render: json\npre: [unique, {sort: k}]\n", data);
    // Items without the key sort first (null < number).
    assert_eq!(
        out,
        "[json:lossy]\n[{\"z\":0},{\"k\":1},{\"k\":2}]\n… 1 duplicate items removed"
    );
}

#[test]
fn cursors_survive_every_render() {
    let data = r#"{"items":[{"t":"a"}],"pageInfo":{"endCursor":"E1","hasNextPage":true},"has_more":false}"#;
    let out = fmt(
        "render: template\nrules:\n  - id: t\n    select: {type: jsonpath, expression: \"$.items[*].t\"}\n",
        data,
    );
    assert_eq!(
        out,
        "[template:lossy]\nt: a\ncursor: pageInfo.endCursor=E1\ncursor: pageInfo.hasNextPage=true"
    );
}

#[test]
fn declared_loss_and_fallbacks() {
    assert_eq!(
        fmt("render: json\nloss: lossy\n", "[1]"),
        "[json:lossy]\n[1]"
    );
    assert_eq!(
        fmt("render: json\nloss: interpretive\n", "[1]"),
        "[json:interpretive]\n[1]"
    );
    assert!(fmt("render: objects_table", "not json").starts_with("[fallback:raw] objects_table:"));
    // With auto mode a failed standard falls to the builtin layout.
    let std = compile("render: num_stats", "t", "t").unwrap();
    // Pretty JSON large enough that the builtin table is shorter than it.
    let rows: Vec<String> = (0..20)
        .map(|i| format!("  {{\n    \"a\": {i}\n  }}"))
        .collect();
    let data = format!("[\n{}\n]", rows.join(",\n"));
    let data = data.as_str();
    let req = Request {
        server: "s",
        tool: "t",
        args: &J::obj(),
        data: Data::Text(data),
        keys: None,
        ov: Overrides::default(),
        auto: true,
        family: None,
    };
    let out = run(None, &req, Some(&std));
    assert_eq!(out.used, "builtin:json_list->objects_table");
    assert!(matches!(out.body, Body::Formatted(_)));
}

/// Agent text for `data` in built-in auto mode (no standard).
fn auto(data: &str) -> String {
    let req = Request {
        server: "s",
        tool: "t",
        args: &J::obj(),
        data: Data::Text(data),
        keys: None,
        ov: Overrides::default(),
        auto: true,
        family: None,
    };
    run(None, &req, None).agent_text(data)
}

fn records(value: &str) -> String {
    let rows: Vec<String> = (0..8)
        .map(|i| format!(r#"{{"record_number":{i},"payload_value":{value}}}"#))
        .collect();
    format!("[{}]", rows.join(","))
}

/// Two different inputs must never look the same to the agent.
fn distinct(a: &str, b: &str) {
    let (x, y) = (auto(a), auto(b));
    assert_ne!(x, y, "identical agent text for\n{a}\n{b}");
}

#[test]
fn auto_keeps_nested_array_apart_from_its_string() {
    distinct(&records("[1,2]"), &records(r#""[1,2]""#));
    // The nested value makes the table ambiguous: the agent gets the JSON.
    let data = records("[1,2]");
    assert_eq!(auto(&data), data);
}

#[test]
fn auto_keeps_nested_object_apart_from_its_string() {
    distinct(&records(r#"{"a":1}"#), &records(r#""{\"a\":1}""#));
}

#[test]
fn auto_keeps_envelope_null_apart_from_its_string() {
    let rows = records("42");
    let a = format!(r#"{{"total":null,"items":{rows}}}"#);
    let b = format!(r#"{{"total":"null","items":{rows}}}"#);
    distinct(&a, &b);
    assert!(auto(&b).contains("total: \"null\"\n"), "{}", auto(&b));
}

#[test]
fn auto_never_drops_a_repeated_key() {
    let payload = |n: u32| {
        let row = format!(r#"{{"record_number":1,"record_number":{n},"description":"sample"}}"#);
        format!("[{}]", vec![row; 8].join(","))
    };
    distinct(&payload(11), &payload(12));
    // A standard asking for a table cannot show both values either.
    let out = fmt("render: objects_table", &payload(11));
    assert!(out.starts_with("[fallback:raw] objects_table: an item repeats a key"), "{out}");
}

#[test]
fn cursor_named_in_content_is_still_given() {
    // The content mentions both cursors; only the field says which is next.
    for token in ["p1", "p2"] {
        let data = format!(r#"{{"note":"next_cursor supports p1 and p2","next_cursor":"{token}"}}"#);
        let out = fmt("render: expression\nformat: \"{result.note}\"", &data);
        assert_eq!(
            out,
            format!("[expression]\nnext_cursor supports p1 and p2\ncursor: next_cursor={token}")
        );
    }
    // A cursor the format prints itself is not repeated.
    let data = r#"{"note":"x","next_cursor":"p2"}"#;
    let out = fmt("render: expression\nformat: \"{result.note} next={result.next_cursor}\"", data);
    assert_eq!(out, "[expression]\nx next=p2");
}
