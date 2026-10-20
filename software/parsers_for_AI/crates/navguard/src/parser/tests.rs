use super::*;

/// (name, kind, line_start, line_end, depth) of every symbol.
fn table(src: &str, ext: &str) -> Vec<(String, &'static str, usize, usize, usize)> {
    parse_source(src, ext)
        .into_iter()
        .map(|a| (a.name, a.kind, a.line_start, a.line_end, a.depth))
        .collect()
}

fn expect(src: &str, ext: &str, expected: &[(&str, &str, usize, usize, usize)]) {
    let got = table(src, ext);
    let got_ref: Vec<(&str, &str, usize, usize, usize)> = got
        .iter()
        .map(|(n, k, s, e, d)| (n.as_str(), *k, *s, *e, *d))
        .collect();
    assert_eq!(got_ref, expected, "symbols of .{} source", ext);
}

#[test]
fn test_python_class_and_methods() {
    let code = "class Foo:\n    def __init__(self):\n        pass\n\n    def bar(self, x):\n        if x > 0:\n            return x\n        return 0\n\nclass Baz:\n    def qux(self):\n        print(\"hello\")\n";
    expect(
        code,
        "py",
        &[
            ("Foo", "class", 1, 8, 0),
            ("__init__", "method", 2, 3, 1),
            ("bar", "method", 5, 8, 1),
            ("Baz", "class", 10, 12, 0),
            ("qux", "method", 11, 12, 1),
        ],
    );
    let anchors = parse_source(code, "py");
    assert_eq!(anchors[0].label, "class Foo:");
    assert_eq!(anchors[1].parent.as_deref(), Some("Foo"));
    assert_eq!(anchors[0].parent, None);
}

#[test]
fn python_nesting_docstrings_continuations_and_comments() {
    let code = r#"import os


@decorator
class Foo(Base):
    """Doc with a fake
def not_a_function():
    """

    def __init__(self,
                 x,
):
        self.x = x

    # comment between methods
    async def run(self):
        def inner():
            return 1
        # trailing comment in run

    @property
    def value(self): return self.x


def top(a, b=(1,
2)):
    data = [
1, 2,
    ]
    return data
# column-0 comment

class Empty: pass
"#;
    expect(
        code,
        "py",
        &[
            ("Foo", "class", 5, 22, 0),
            ("__init__", "method", 10, 13, 1),
            ("run", "method", 16, 19, 1),
            ("inner", "fn", 17, 18, 2),
            ("value", "method", 22, 22, 1),
            ("top", "fn", 25, 30, 0),
            ("Empty", "class", 33, 33, 0),
        ],
    );
}

#[test]
fn test_rust_fns_and_structs() {
    let code = "pub struct Point {\n    x: f64,\n    y: f64,\n}\n\nimpl Point {\n    pub fn new(x: f64, y: f64) -> Self {\n        Self { x, y }\n    }\n\n    fn distance(&self, other: &Point) -> f64 {\n        let dx = self.x - other.x;\n        let dy = self.y - other.y;\n        (dx * dx + dy * dy).sqrt()\n    }\n}\n";
    expect(
        code,
        "rs",
        &[
            ("Point", "struct", 1, 4, 0),
            ("Point", "impl", 6, 16, 0),
            ("new", "method", 7, 9, 1),
            ("distance", "method", 11, 15, 1),
        ],
    );
    let anchors = parse_source(code, "rs");
    assert_eq!(anchors[0].label, "pub struct Point {");
    assert_eq!(anchors[2].label, "pub fn new(x: f64, y: f64) -> Self {");
}

#[test]
fn rust_signatures_visibility_nesting_and_masking() {
    let code = r#"use std::fmt;

/// A point.
#[derive(Debug)]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub(crate) const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub(super) async unsafe fn weird<'a>(
        &'a self,
        other: &Point,
    ) -> f64
    where
        Self: Sized,
    {
        let s = "}"; // }
        let c = '{';
        0.0
    }
}

impl<T: Into<f64>> From<(T, T)> for Point {
    fn from(p: (T, T)) -> Self {
        Point::new(p.0.into(), p.1.into())
    }
}

pub(in crate::geo) trait Shape: fmt::Debug {
    fn area(&self) -> f64;
    fn name(&self) -> &str {
        "shape"
    }
}

pub enum Kind { A, B }
pub type Pair = (Point, Point);
pub struct Unit;
mod tests;
extern "C" fn callback(x: i32) -> i32 { x }
macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}
pub mod inner {
    pub fn helper() {
        fn nested() {}
    }
}
return_value();
"#;
    expect(
        code,
        "rs",
        &[
            ("Point", "struct", 5, 8, 0),
            ("Point", "impl", 10, 26, 0),
            ("new", "method", 11, 13, 1),
            ("weird", "method", 15, 25, 1),
            ("From<(T, T)> for Point", "impl", 28, 32, 0),
            ("from", "method", 29, 31, 1),
            ("Shape", "trait", 34, 39, 0),
            ("area", "method", 35, 35, 1),
            ("name", "method", 36, 38, 1),
            ("Kind", "enum", 41, 41, 0),
            ("Pair", "type", 42, 42, 0),
            ("Unit", "struct", 43, 43, 0),
            ("tests", "mod", 44, 44, 0),
            ("callback", "fn", 45, 45, 0),
            ("square", "macro", 46, 50, 0),
            ("inner", "mod", 51, 55, 0),
            ("helper", "fn", 52, 54, 1),
            ("nested", "fn", 53, 53, 2),
        ],
    );
}

#[test]
fn rust_block_comments_and_raw_strings_hide_fake_items() {
    let code = "/*\nfn fake() {\n*/\nfn real() {\n    let s = r#\"\n}\nfn also_fake() {\n\"#;\n}\n";
    expect(code, "rs", &[("real", "fn", 4, 9, 0)]);
}

#[test]
fn test_javascript_class_and_function() {
    let code = "class Calculator {\n    constructor() {\n        this.value = 0;\n    }\n\n    add(x) {\n        this.value += x;\n    }\n}\n\nfunction helper() {\n    return 42;\n}\n";
    expect(
        code,
        "js",
        &[
            ("Calculator", "class", 1, 9, 0),
            ("constructor", "method", 2, 4, 1),
            ("add", "method", 6, 8, 1),
            ("helper", "fn", 11, 13, 0),
        ],
    );
}

#[test]
fn javascript_functions_arrows_methods_and_regexes() {
    let code = r#"import x from "y";

export default function main() {
  const s = "}";
  return s;
}

export async function load(url) {
  return fetch(url);
}

class Calculator extends Base {
  #total = 0;
  constructor() {
    super();
  }

  static async add(x) {
    if (x) {
      this.#total += x;
    }
  }

  get value() {
    return this.#total;
  }

  handle = (e) => {
    console.log(e);
  };
}

const double = (n) =>
  n * 2;

export const fetchAll = async ({
  page,
  size,
}) => {
  return [];
};

const legacy = function () {
  return /}/.test("x");
};

const total = (a + b) * 2;
function* gen() {
  yield 1;
}
"#;
    expect(
        code,
        "js",
        &[
            ("main", "fn", 3, 6, 0),
            ("load", "fn", 8, 10, 0),
            ("Calculator", "class", 12, 31, 0),
            ("constructor", "method", 14, 16, 1),
            ("add", "method", 18, 22, 1),
            ("value", "method", 24, 26, 1),
            ("handle", "method", 28, 30, 1),
            ("double", "fn", 33, 34, 0),
            ("fetchAll", "fn", 36, 41, 0),
            ("legacy", "fn", 43, 45, 0),
            ("gen", "fn", 48, 50, 0),
        ],
    );
}

#[test]
fn javascript_arrow_components_contain_nested_functions() {
    let code = "export const App = () => {\n  function helper() {\n    return 1;\n  }\n  const inner = (x) => x + 1;\n  return helper();\n};\n";
    expect(
        code,
        "tsx",
        &[
            ("App", "fn", 1, 7, 0),
            ("helper", "fn", 2, 4, 1),
            ("inner", "fn", 5, 5, 1),
        ],
    );
}

#[test]
fn javascript_file_level_function_assignments() {
    let code = "onmessage = function (ev) {\n    postMessage(ev);\n};\nexports.handler = async (event) => {\n  return 1;\n};\nFoo.prototype.bar = function () {};\ncount = count + 1;\nfunction outer() {\n  inner = function () {};\n}\n";
    expect(
        code,
        "js",
        &[
            ("onmessage", "fn", 1, 3, 0),
            ("exports.handler", "fn", 4, 6, 0),
            ("Foo.prototype.bar", "fn", 7, 7, 0),
            ("outer", "fn", 9, 11, 0),
        ],
    );
}

#[test]
fn typescript_types_interfaces_enums_and_modules() {
    let code = r#"export interface Shape {
  area(): number;
}

export type Id = string | number;
type Handler<T> = (event: T) => void;
export type Union =
  | { kind: "a" }
  | { kind: "b" };

export enum Color { Red, Green }

export abstract class Base<T> implements Shape {
  private readonly items: T[] = [];
  abstract area(): number;
  protected log<U>(msg: U): void {
    console.log(`${msg} {`);
  }
}

declare module "pkg" {
  export function helper(): void;
}

export default class {
  run() {}
}
"#;
    expect(
        code,
        "ts",
        &[
            ("Shape", "interface", 1, 3, 0),
            ("Id", "type", 5, 5, 0),
            ("Handler", "type", 6, 6, 0),
            ("Union", "type", 7, 9, 0),
            ("Color", "enum", 11, 11, 0),
            ("Base", "class", 13, 19, 0),
            ("area", "method", 15, 15, 1),
            ("log", "method", 16, 18, 1),
            ("pkg", "namespace", 21, 23, 0),
            ("helper", "fn", 22, 22, 1),
            ("default", "class", 25, 27, 0),
            ("run", "method", 26, 26, 1),
        ],
    );
}

#[test]
fn test_go_functions() {
    let code = "package main\n\ntype Point struct {\n    X, Y float64\n}\n\nfunc (p Point) Distance() float64 {\n    return math.Sqrt(p.X*p.X + p.Y*p.Y)\n}\n\nfunc main() {\n    p := Point{3, 4}\n    println(p.Distance())\n}\n";
    expect(
        code,
        "go",
        &[
            ("Point", "struct", 3, 5, 0),
            ("Distance", "method", 7, 9, 0),
            ("main", "fn", 11, 14, 0),
        ],
    );
    let anchors = parse_source(code, "go");
    assert!(anchors[1].label.contains("func (p Point) Distance"));
}

#[test]
fn go_types_generics_and_raw_strings() {
    let code = "package main\n\nimport \"fmt\"\n\ntype Point struct {\n\tX, Y float64\n}\n\ntype Celsius float64\n\ntype Shape interface {\n\tArea() float64\n}\n\ntype Stack[T any] struct {\n\titems []T\n}\n\nfunc (p *Point) Distance(q Point) float64 {\n\ts := \"}\"\n\treturn 0\n}\n\nfunc Map[T, U any](xs []T, f func(T) U) []U {\n\treturn nil\n}\n\nfunc main() {\n\tinc := func(x int) int {\n\t\treturn x + 1\n\t}\n\tfmt.Println(inc(1), `raw }`)\n}\n";
    expect(
        code,
        "go",
        &[
            ("Point", "struct", 5, 7, 0),
            ("Celsius", "type", 9, 9, 0),
            ("Shape", "interface", 11, 13, 0),
            ("Stack", "struct", 15, 17, 0),
            ("Distance", "method", 19, 22, 0),
            ("Map", "fn", 24, 26, 0),
            ("main", "fn", 28, 33, 0),
        ],
    );
}

#[test]
fn test_cpp_class() {
    let code = "class Matrix {\npublic:\n    Matrix(int rows, int cols);\n    void transpose();\nprivate:\n    int* data_;\n};\n";
    expect(
        code,
        "cpp",
        &[
            ("Matrix", "class", 1, 7, 0),
            ("Matrix", "method", 3, 3, 1),
            ("transpose", "method", 4, 4, 1),
        ],
    );
}

#[test]
fn cpp_namespaces_methods_operators_and_no_statement_false_positives() {
    let code = r#"#include <vector>
#define BEGIN {

namespace geo {

class Matrix : public Base {
public:
    Matrix(int rows, int cols);
    ~Matrix();
    void transpose() const;
    int at(int r, int c) const {
        return data_[r * cols_ + c];
    }
    bool operator==(const Matrix& o) const;
private:
    int* data_;
};

struct Point { int x, y; };
struct Forward;

Matrix::Matrix(int rows, int cols)
    : rows_(rows), cols_(cols) {
    const char* s = "}";
}

static int
helper(int x)
{
    if (x) { return 1; }
    return x;
}

}  // namespace geo

int main(int argc, char** argv) {
    geo::Matrix m(2, 2);
    return helper(argc);
}
int prototype(int);
enum class Color : int { Red, Green };
"#;
    expect(
        code,
        "cpp",
        &[
            ("geo", "namespace", 4, 34, 0),
            ("Matrix", "class", 6, 17, 1),
            ("Matrix", "method", 8, 8, 2),
            ("~Matrix", "method", 9, 9, 2),
            ("transpose", "method", 10, 10, 2),
            ("at", "method", 11, 13, 2),
            ("operator==", "method", 14, 14, 2),
            ("Point", "struct", 19, 19, 1),
            ("Matrix::Matrix", "fn", 22, 25, 1),
            ("helper", "fn", 28, 32, 1),
            ("main", "fn", 36, 39, 0),
            ("Color", "enum", 41, 41, 0),
        ],
    );
}

#[test]
fn c_statements_are_not_symbols() {
    let code = "struct point p = {1, 2};\nstruct stat st;\nstatic struct node *make(void) {\n    return x;\n}\nint f(void) {\n    foo(bar);\n    return x;\n    if (a) { b(); }\n}\n";
    expect(code, "c", &[("make", "fn", 3, 5, 0), ("f", "fn", 6, 10, 0)]);
}

#[test]
fn java_classes_methods_and_records() {
    let code = r#"package demo;

import java.util.List;

@Entity
public final class Service<T> extends Base implements Runnable {
    private static final String NAME = "x{";
    private final List<T> items;

    public Service(List<T> items) {
        this.items = items;
    }

    @Override
    public void run() {
        for (T item : items) {
            process(item);
        }
    }

    public static <U> List<U> of(U... values) {
        return List.of(values);
    }

    abstract int size();

    interface Listener {
        void onEvent(String name);
    }

    enum State { ON, OFF }

    record Pair(int a, int b) {}
}
"#;
    expect(
        code,
        "java",
        &[
            ("Service", "class", 6, 34, 0),
            ("Service", "method", 10, 12, 1),
            ("run", "method", 15, 19, 1),
            ("of", "method", 21, 23, 1),
            ("size", "method", 25, 25, 1),
            ("Listener", "interface", 27, 29, 1),
            ("onEvent", "method", 28, 28, 2),
            ("State", "enum", 31, 31, 1),
            ("Pair", "record", 33, 33, 1),
        ],
    );
}

#[test]
fn csharp_members_and_verbatim_strings() {
    let code = "using System;\nnamespace App.Core;\n\n[Serializable]\npublic partial class Repo<T> : IRepo<T> where T : class\n{\n    private readonly string _path = @\"C:\\{data}\";\n    public Repo(string path) => _path = path;\n    public T Get<TKey>(TKey id) where TKey : notnull\n    {\n        return default;\n    }\n    void IDisposable.Dispose() { }\n    public int Count { get; set; }\n}\npublic record Person(string Name, int Age);\n";
    expect(
        code,
        "cs",
        &[
            ("App.Core", "namespace", 2, 2, 0),
            ("Repo", "class", 5, 15, 0),
            ("Repo", "method", 8, 8, 1),
            ("Get", "method", 9, 12, 1),
            ("IDisposable.Dispose", "method", 13, 13, 1),
            ("Person", "record", 16, 16, 0),
        ],
    );
}

#[test]
fn kotlin_functions_classes_and_objects() {
    let code = "package demo\n\ndata class User(val name: String, val age: Int = 0)\n\nsealed class Shape {\n    abstract fun area(): Double\n    object Unit : Shape() {\n        override fun area() = 0.0\n    }\n}\n\nenum class Color { RED, GREEN }\n\nfun String.shout(): String {\n    val s = \"}\"\n    return uppercase()\n}\n\nsuspend fun <T> load(\n    id: T,\n): T = id\n\ntypealias Name = String\ninterface Repo\n";
    expect(
        code,
        "kt",
        &[
            ("User", "class", 3, 3, 0),
            ("Shape", "class", 5, 10, 0),
            ("area", "method", 6, 6, 1),
            ("Unit", "object", 7, 9, 1),
            ("area", "method", 8, 8, 2),
            ("Color", "enum", 12, 12, 0),
            ("shout", "fn", 14, 17, 0),
            ("load", "fn", 19, 21, 0),
            ("Name", "type", 23, 23, 0),
            ("Repo", "interface", 24, 24, 0),
        ],
    );
}

#[test]
fn test_empty_file() {
    assert!(parse_source("", "py").is_empty());
}

#[test]
fn test_no_anchors() {
    assert!(parse_source("x = 1\ny = 2\nz = 3", "py").is_empty());
    assert!(parse_source("return x;\nfoo(bar);\nx = y;\n", "cpp").is_empty());
}

#[test]
fn test_protocol_anchors() {
    let code = "[s1-1]\ndef hello():\n    pass\n[s1-1end]\n\n[s2-1]\nclass Foo:\n    def bar(self):\n        return 42\n[s2-1end]\n";
    let anchors = parse_source(code, "md");
    let protocol_anchors: Vec<_> = anchors
        .iter()
        .filter(|a| a.anchor_type == AnchorType::Protocol)
        .collect();
    assert_eq!(protocol_anchors.len(), 2);
    assert!(protocol_anchors[0].label.contains("s1-1"));
    assert!(protocol_anchors[1].label.contains("s2-1"));
    assert_eq!(
        (protocol_anchors[0].line_start, protocol_anchors[0].line_end),
        (1, 4)
    );
    // Opening tag and body (28 bytes), not the closing tag.
    assert_eq!(protocol_anchors[0].tokens, 10);
}

#[test]
fn test_mixed_anchors() {
    let code = "[s1-1]\ndef hello():\n    pass\n[s1-1end]\n\nclass Bar:\n    def method(self):\n        return 1\n";
    expect(
        code,
        "md",
        &[
            ("[s1-1]", "anchor", 1, 4, 0),
            ("hello", "fn", 2, 3, 1),
            ("Bar", "class", 6, 8, 0),
            ("method", "method", 7, 8, 1),
        ],
    );
}

#[test]
fn generic_mode_nests_anchors_and_ignores_prose() {
    let code = "# Title\n[s1-1]\ndef hello():\n    pass\n[s1-1end]\n\nclass Bar:\n    def method(self):\n        return 1\n\n```rust\nfn main() {\n    println!(\"{}\", 1);\n}\n```\ntype of thing is prose\n[s2]\n[s2-1]\ninner\n[s2-1end]\n[s2end]\n";
    expect(
        code,
        "md",
        &[
            ("[s1-1]", "anchor", 2, 5, 0),
            ("hello", "fn", 3, 4, 1),
            ("Bar", "class", 7, 9, 0),
            ("method", "method", 8, 9, 1),
            ("main", "fn", 12, 14, 0),
            ("[s2]", "anchor", 17, 21, 0),
            ("[s2-1]", "anchor", 18, 20, 1),
        ],
    );
    let anchors = parse_source(code, "md");
    assert_eq!(anchors[1].parent.as_deref(), Some("[s1-1]"));
}

#[test]
fn unclosed_bodies_fall_back_to_indentation() {
    let code = "Example:\n\n    fn broken() {\n        body();\n\nfn next() {}\n";
    expect(
        code,
        "md",
        &[("broken", "fn", 3, 4, 0), ("next", "fn", 6, 6, 0)],
    );
}

#[test]
fn code_symbols_are_clamped_to_their_protocol_anchor() {
    let code = "[s1]\nfn open() {\n[s1end]\nfn after() {}\n";
    expect(
        code,
        "rs",
        &[
            ("[s1]", "anchor", 1, 3, 0),
            ("open", "fn", 2, 2, 1),
            ("after", "fn", 4, 4, 0),
        ],
    );
}

#[test]
fn unmatched_protocol_tags_do_not_swallow_following_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unmatched.rs");
    fs::write(&path, "[s1end]\n[s2]\nfn retained() {}\n").unwrap();
    let anchors = parse_file(&path.to_string_lossy());
    assert_eq!(anchors.len(), 1);
    assert!(anchors[0].label.contains("retained"));
    assert_eq!(anchors[0].anchor_type, AnchorType::Code);
}

#[test]
fn parse_file_uses_the_extension_and_strips_a_bom() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Mod.PY");
    fs::write(&path, "\u{feff}def first():\n    return 1\n").unwrap();
    let anchors = parse_file(&path.to_string_lossy());
    assert_eq!(anchors.len(), 1);
    assert_eq!(
        (anchors[0].name.as_str(), anchors[0].line_end),
        ("first", 2)
    );
    assert!(parse_file(&dir.path().join("missing.rs").to_string_lossy()).is_empty());
}
