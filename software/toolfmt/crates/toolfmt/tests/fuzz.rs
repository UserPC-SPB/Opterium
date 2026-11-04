//! Randomized tests with a fixed seed (reproducible, no dependencies):
//! the JSON type round-trips, the parser never panics on garbage, and no
//! render or operation chain ever panics on any input (D2: never crash).

use toolfmt::json::J;
use toolfmt::pipeline::{Body, Data, Overrides, Request, run};
use toolfmt::standard::compile;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

const STRS: [&str; 12] = [
    "",
    "a",
    "x,y",
    "say \"hi\"",
    "two\nlines",
    " pad ",
    "true",
    "007",
    "имя",
    "😀",
    "\\",
    "ERROR boom",
];
const NUMS: [&str; 10] = [
    "0",
    "-1",
    "5.0",
    "1e16",
    "1E-7",
    "123456789012345678901234567890",
    "-0",
    "3.14",
    "1536",
    "170141183460469231731687303715884105728",
];
const KEYS: [&str; 8] = [
    "id",
    "path",
    "size",
    "next_cursor",
    "has_more",
    "user",
    "a.b",
    "items",
];

fn value(r: &mut Rng, depth: u32) -> J {
    let k = if depth == 0 { r.below(4) } else { r.below(6) };
    match k {
        0 => J::Null,
        1 => J::Bool(r.below(2) == 0),
        2 => J::Num(r.pick(&NUMS).to_string()),
        3 => J::Str(r.pick(&STRS).to_string()),
        4 => J::Arr((0..r.below(5)).map(|_| value(r, depth - 1)).collect()),
        _ => J::Obj(
            (0..r.below(5))
                .map(|_| (r.pick(&KEYS).to_string(), value(r, depth - 1)))
                .collect(),
        ),
    }
}

#[test]
fn json_round_trips() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..5_000 {
        let v = value(&mut r, 4);
        let text = v.to_json();
        assert_eq!(J::parse(&text).unwrap(), v, "{text}");
        assert_eq!(J::parse(&v.to_pretty()).unwrap(), v);
    }
}

#[test]
fn parser_never_panics_on_garbage() {
    let mut r = Rng(42);
    let alphabet: Vec<char> = "{}[]\":,.-+eE0123456789tfnaul\\u ".chars().collect();
    for _ in 0..20_000 {
        let len = r.below(40);
        let s: String = (0..len).map(|_| *r.pick(&alphabet)).collect();
        let _ = J::parse(&s);
        // Also cut valid documents at random points.
        let doc = value(&mut r, 3).to_json();
        let cut = r.below(doc.len() as u64 + 1) as usize;
        if doc.is_char_boundary(cut) {
            let _ = J::parse(&doc[..cut]);
        }
    }
}

const STANDARDS: [&str; 16] = [
    "render: passthrough",
    "render: json",
    "render: objects_table",
    "render: objects_table\nmode: markdown\ncolumns: [id, user.login, missing]\nrows: 2\nmax_cell_chars: 3\nformats: {size: bytes, \"*\": thousands}",
    "render: matrix_table\nmax_rows: 1\nmax_cols: 1",
    "render: matrix_table\nmode: markdown",
    "render: dict_table\nformats: {size: fixed:2}",
    "render: dict_table\nmode: markdown\nmax_rows: 1",
    "render: expression\nformat: \"{result} {a} {result.items[0]}\"",
    "render: log\nmax_items: 1",
    "render: num_stats\nsample: 1",
    "render: tree\nmax_items: 1",
    "render: image_meta",
    "render: template\nrules:\n  - id: x\n    select: {type: jsonpath, expression: \"$..id\"}\n  - id: l\n    select: {type: regex, expression: \"(?m)^.{0,5}\"}",
    "render: json\nroot: items\npre: [unique, {sort_desc: size}, {limit_items: 2}, {select: [id, size]}]",
    "render: objects_table\npost: [{drop_lines: \"^$\"}, {replace: {from: \"(a)\", to: \"$1$1\", regex: true}}, {join: \" | \"}, {mask: true}]",
];

#[test]
fn no_input_makes_any_render_panic() {
    let compiled: Vec<_> = STANDARDS
        .iter()
        .map(|s| compile(s, "fuzz", "t").unwrap_or_else(|e| panic!("{s}: {e}")))
        .collect();
    let mut r = Rng(7);
    let args = J::parse(r#"{"a":1}"#).unwrap();
    let mut formatted = 0;
    for i in 0..6_000 {
        let v = value(&mut r, 4);
        let data = match i % 4 {
            0 => v.to_json(),
            1 => v.to_pretty(),
            2 => (0..r.below(8))
                .map(|_| r.pick(&STRS).to_string())
                .collect::<Vec<_>>()
                .join("\n"),
            _ => format!("data:image/png;base64,{}", r.pick(&STRS)),
        };
        let std = &compiled[i % compiled.len()];
        for auto in [false, true] {
            let req = Request {
                server: "s",
                tool: "t",
                args: &args,
                data: Data::Text(&data),
                keys: None,
                ov: Overrides::default(),
                auto,
                family: None,
            };
            let out = run(None, &req, Some(std));
            match &out.body {
                Body::Formatted(_) => {
                    formatted += 1;
                    assert!(out.header.as_deref().is_some_and(|h| h.starts_with('[')));
                }
                Body::Fallback(why) => assert!(!why.is_empty()),
                Body::Untouched => {}
            }
            // The agent text always carries the raw data when nothing applied.
            if !matches!(out.body, Body::Formatted(_)) {
                assert!(out.agent_text(&data).ends_with(data.as_str()));
            }
        }
    }
    // Random data mostly does not fit a render (and falls back); enough of
    // it must be formatted for the test to exercise the renders.
    assert!(formatted > 1_500, "only {formatted} formatted");
}
