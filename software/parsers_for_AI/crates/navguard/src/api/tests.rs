use super::*;
use std::collections::BTreeSet;

/// A temporary project:
///   navguard.json   (excludes "ignored")
///   README.md       ("Intro. The parser reads files! TODO: docs")
///   src/lib.rs      (struct + impl)
///   src/util/strings.py
///   ignored/skip.rs
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src/util")).unwrap();
    fs::create_dir_all(root.join("ignored")).unwrap();
    fs::write(
        root.join("navguard.json"),
        r#"{"root": "somewhere/else", "exclude": [".git", "target", "ignored"]}"#,
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Intro. The parser reads files! TODO: docs\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub struct Parser {\n    depth: usize,\n}\n\nimpl Parser {\n    pub fn parse(&self) -> usize {\n        // FIXME: real parsing\n        self.depth\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("src/util/strings.py"),
        "class Text:\n    def upper(self):\n        return 1\n",
    )
    .unwrap();
    fs::write(root.join("ignored/skip.rs"), "fn skipped() {}\n").unwrap();
    dir
}

fn listing(root: &Path) -> BTreeSet<String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            out.insert(posix(&path.strip_prefix(root).unwrap().to_string_lossy()));
            if path.is_dir() {
                walk(&path, root, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn map_lists_files_totals_and_a_tree_from_the_scan() {
    let dir = project();
    let before = listing(dir.path());
    let m = map(dir.path(), &MapOptions::default()).unwrap();
    let paths: Vec<&str> = m.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "README.md",
            "navguard.json",
            "src/lib.rs",
            "src/util/strings.py"
        ]
    );
    assert_eq!(m.totals.files, 4);
    assert_eq!(m.totals.lines, m.files.iter().map(|f| f.lines).sum::<u64>());
    assert_eq!(
        m.totals.tokens,
        m.files.iter().map(|f| f.tokens).sum::<u64>()
    );
    assert!(!m.truncated);
    assert_eq!(
        m.tree,
        "├── src/\n│   ├── util/\n│   │   └── strings.py\n│   └── lib.rs\n├── README.md\n└── navguard.json\n"
    );
    assert!(!m.root.contains('\\'));
    assert!(m.files.iter().all(|f| f.sha.len() == 8));

    let capped = map(dir.path(), &MapOptions { max_files: Some(2) }).unwrap();
    assert!(capped.truncated);
    assert_eq!(capped.files.len(), 2);
    assert_eq!(capped.totals.files, 4);
    assert_eq!(capped.tree, "├── README.md\n└── navguard.json\n");
    assert_eq!(listing(dir.path()), before, "map must not write files");
}

#[test]
fn open_rejects_a_missing_root() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");
    let error = map(&missing, &MapOptions::default()).unwrap_err();
    assert!(error.contains("not a directory"), "{}", error);
}

#[test]
fn symbols_of_one_file_or_all_files() {
    let dir = project();
    let one = symbols(dir.path(), Some("src/lib.rs")).unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].path, "src/lib.rs");
    let table: Vec<(&str, &str, usize, usize, usize)> = one[0]
        .symbols
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.kind.as_str(),
                s.line_start,
                s.line_end,
                s.depth,
            )
        })
        .collect();
    assert_eq!(
        table,
        [
            ("Parser", "struct", 1, 3, 0),
            ("Parser", "impl", 5, 10, 0),
            ("parse", "method", 6, 9, 1),
        ]
    );
    assert_eq!(one[0].symbols[2].parent.as_deref(), Some("Parser"));
    assert_eq!(one[0].symbols[2].label, "pub fn parse(&self) -> usize {");

    // Absolute and backslash paths resolve to the same relative path.
    let absolute = dir.path().join("src").join("util").join("strings.py");
    let by_abs = symbols(dir.path(), Some(&absolute.to_string_lossy())).unwrap();
    assert_eq!(by_abs[0].path, "src/util/strings.py");
    assert_eq!(by_abs[0].symbols[1].kind, "method");
    // Excluded files can still be inspected explicitly.
    assert_eq!(
        symbols(dir.path(), Some("ignored/skip.rs")).unwrap()[0]
            .symbols
            .len(),
        1
    );

    let all = symbols(dir.path(), None).unwrap();
    let paths: Vec<&str> = all.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["src/lib.rs", "src/util/strings.py"]);
}

#[test]
fn path_arguments_must_exist_inside_the_root() {
    let dir = project();
    let outside = tempfile::NamedTempFile::new().unwrap();
    let error = symbols(dir.path(), Some(&outside.path().to_string_lossy())).unwrap_err();
    assert!(error.contains("outside the project root"), "{}", error);
    let error = read_lines(dir.path(), "../escape.rs", 1, None).unwrap_err();
    assert!(
        error.contains("not found") || error.contains("outside"),
        "{}",
        error
    );
    let error = read_lines(dir.path(), "src/missing.rs", 1, None).unwrap_err();
    assert!(
        error.contains("file not found: src/missing.rs"),
        "{}",
        error
    );
    let error = read_lines(dir.path(), "src", 1, None).unwrap_err();
    assert!(error.contains("not a file"), "{}", error);
}

#[test]
fn find_symbols_is_case_insensitive_and_does_not_write_by_default() {
    let dir = project();
    let before = listing(dir.path());
    let hits = find_symbols(dir.path(), "PARSE", &FindOptions::default()).unwrap();
    let found: Vec<(&str, &str, usize)> = hits
        .iter()
        .map(|h| (h.path.as_str(), h.symbol.name.as_str(), h.symbol.line_start))
        .collect();
    assert_eq!(
        found,
        [
            ("src/lib.rs", "Parser", 1),
            ("src/lib.rs", "Parser", 5),
            ("src/lib.rs", "parse", 6)
        ]
    );
    assert_eq!(
        listing(dir.path()),
        before,
        "in-memory search must not write"
    );

    let exact = FindOptions {
        case_sensitive: true,
        ..FindOptions::default()
    };
    assert!(find_symbols(dir.path(), "PARSE", &exact)
        .unwrap()
        .is_empty());
    // The file path matches too.
    let by_path = find_symbols(dir.path(), "util/", &FindOptions::default()).unwrap();
    assert_eq!(by_path.len(), 2);
    let capped = FindOptions {
        max_hits: Some(1),
        ..FindOptions::default()
    };
    assert_eq!(find_symbols(dir.path(), "", &capped).unwrap().len(), 1);
}

#[test]
fn find_symbols_exact_matches_the_whole_name_only() {
    let dir = project();
    fs::write(
        dir.path().join("src/shape.cpp"),
        "void Shape::parse() {\n}\n\nint parser_count() {\n    return 0;\n}\n",
    )
    .unwrap();
    let exact = FindOptions {
        exact: true,
        ..FindOptions::default()
    };
    let found = |pattern: &str, opts: &FindOptions| -> Vec<(String, String, usize)> {
        find_symbols(dir.path(), pattern, opts)
            .unwrap()
            .into_iter()
            .map(|h| (h.path, h.symbol.name, h.symbol.line_start))
            .collect()
    };
    // Substring search also finds Parser, parser_count and the C++ method.
    assert_eq!(found("parse", &FindOptions::default()).len(), 5);
    // Exact: the name, or the part after the last `::`; any case.
    assert_eq!(
        found("PARSE", &exact),
        [
            ("src/lib.rs".to_string(), "parse".to_string(), 6),
            ("src/shape.cpp".to_string(), "Shape::parse".to_string(), 1),
        ]
    );
    assert_eq!(
        found("Shape::parse", &exact),
        [("src/shape.cpp".to_string(), "Shape::parse".to_string(), 1)]
    );
    assert_eq!(found("Parser", &exact).len(), 2, "struct and impl");
    let exact_case = FindOptions {
        exact: true,
        case_sensitive: true,
        ..FindOptions::default()
    };
    assert!(found("PARSE", &exact_case).is_empty());
    assert_eq!(found("parse", &exact_case).len(), 2);
    // Neither a part of a name nor the path counts.
    assert!(found("pars", &exact).is_empty());
    assert!(found("src/util/strings.py", &exact).is_empty());
    assert!(found("", &exact).is_empty());
    // The cache gives the same answer.
    let cached = FindOptions {
        use_cache: true,
        ..exact.clone()
    };
    assert_eq!(found("PARSE", &cached), found("PARSE", &exact));
}

#[test]
fn find_symbols_with_cache_writes_and_reuses_the_cache() {
    let dir = project();
    let opts = FindOptions {
        use_cache: true,
        ..FindOptions::default()
    };
    let in_memory = find_symbols(dir.path(), "parse", &FindOptions::default()).unwrap();
    let cached = find_symbols(dir.path(), "parse", &opts).unwrap();
    assert_eq!(in_memory, cached);
    assert!(dir.path().join(CACHE_FILE).is_file());
    let again = find_symbols(dir.path(), "parse", &opts).unwrap();
    assert_eq!(again, cached);
}

#[test]
fn read_lines_ranges_and_errors() {
    let dir = project();
    let lines = read_lines(dir.path(), "src/lib.rs", 6, Some(7)).unwrap();
    assert_eq!(
        lines,
        [
            NumberedLine {
                line: 6,
                text: "    pub fn parse(&self) -> usize {".to_string()
            },
            NumberedLine {
                line: 7,
                text: "        // FIXME: real parsing".to_string()
            },
        ]
    );
    assert_eq!(
        read_lines(dir.path(), "src/lib.rs", 1, None).unwrap().len(),
        1
    );
    // The end is clamped to the last line.
    let tail = read_lines(dir.path(), "src/lib.rs", 9, Some(500)).unwrap();
    assert_eq!(tail.iter().map(|l| l.line).collect::<Vec<_>>(), [9, 10]);

    let zero = read_lines(dir.path(), "src/lib.rs", 0, None).unwrap_err();
    assert!(zero.contains("1-based"), "{}", zero);
    let past = read_lines(dir.path(), "src/lib.rs", 11, None).unwrap_err();
    assert_eq!(
        past,
        "start line 11 is out of range: src/lib.rs has 10 line(s)"
    );
    let reversed = read_lines(dir.path(), "src/lib.rs", 5, Some(4)).unwrap_err();
    assert_eq!(reversed, "end line 4 is before start line 5");
}

#[test]
fn search_text_reports_sentences_with_correct_lines() {
    let dir = project();
    fs::write(
        dir.path().join("src/notes.txt"),
        "first line\nsecond line has the needle inside. Next sentence.\nneedle at start\n",
    )
    .unwrap();
    let hits = search_text(dir.path(), "needle", None, None, SearchContext::Auto).unwrap();
    assert_eq!(
        hits,
        [
            TextHit {
                path: "src/notes.txt".to_string(),
                line: 2,
                text: "second line has the needle inside.".to_string(),
                truncated: false,
            },
            TextHit {
                path: "src/notes.txt".to_string(),
                line: 3,
                text: "needle at start".to_string(),
                truncated: false,
            },
        ]
    );
    let one = search_text(
        dir.path(),
        "parser",
        Some("README.md"),
        None,
        SearchContext::Auto,
    )
    .unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].text, "The parser reads files!");
    assert_eq!(
        search_text(dir.path(), "needle", None, Some(1), SearchContext::Auto)
            .unwrap()
            .len(),
        1
    );
    assert!(search_text(dir.path(), "", None, None, SearchContext::Line).is_err());

    let long = format!("start {} end", "x".repeat(400));
    fs::write(dir.path().join("long.txt"), &long).unwrap();
    let cut = search_text(
        dir.path(),
        "start",
        Some("long.txt"),
        None,
        SearchContext::Auto,
    )
    .unwrap();
    assert!(cut[0].truncated);
    assert_eq!(cut[0].text.chars().count(), SEARCH_SNIPPET_CHARS);
}

#[test]
fn search_text_shows_lines_in_code_and_sentences_in_prose() {
    let dir = project();
    fs::write(
        dir.path().join("src/conf.rs"),
        "fn f() {\n    let a = cfg.get(\"a\").or(cfg.get(\"b\"));\n}\n",
    )
    .unwrap();
    let hits = |path: &str, context| {
        search_text(dir.path(), "get", Some(path), None, context)
            .unwrap()
            .into_iter()
            .map(|h| (h.line, h.text))
            .collect::<Vec<_>>()
    };
    let line = vec![(2, "let a = cfg.get(\"a\").or(cfg.get(\"b\"));".to_string())];
    assert_eq!(hits("src/conf.rs", SearchContext::Auto), line);
    assert_eq!(hits("src/conf.rs", SearchContext::Line), line);
    assert_eq!(
        hits("src/conf.rs", SearchContext::Sentence),
        [
            (2, "get(\"a\").".to_string()),
            (2, "get(\"b\"));\n}".to_string())
        ]
    );
    assert_eq!(
        hits("README.md", SearchContext::Line).len(),
        0,
        "README.md has no 'get'"
    );
    assert!(is_prose_path("docs/Guide.MD") && is_prose_path("a\\notes.txt"));
    assert!(!is_prose_path("src/main.rs") && !is_prose_path("Makefile"));
    assert_eq!(SearchContext::parse("Lines"), Some(SearchContext::Line));
    assert_eq!(SearchContext::parse("word"), None);
    assert_eq!(
        line_hits("x = a.b\n  a.b.c  \nnone", "a.b"),
        [
            (1, "x = a.b".to_string(), false),
            (2, "a.b.c".to_string(), false)
        ]
    );
}

#[test]
fn todos_lists_markers_in_scanned_files() {
    let dir = project();
    let hits = todos(dir.path(), TodoMatch::Strict).unwrap();
    let found: Vec<(&str, usize, &str)> = hits
        .iter()
        .map(|h| (h.path.as_str(), h.line, h.text.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("README.md", 1, "Intro. The parser reads files! TODO: docs"),
            ("src/lib.rs", 7, "// FIXME: real parsing"),
        ]
    );
}

#[test]
fn todos_strict_skips_words_that_only_contain_a_marker() {
    let dir = project();
    fs::write(
        dir.path().join("notes.txt"),
        "List todos here.\nconst TODO_LIMIT: usize = 3;\nfn todo_hits() {}\nThe hackathon starts.\nUnfixmeable.\n",
    )
    .unwrap();
    let strict = todos(dir.path(), TodoMatch::Strict).unwrap();
    assert_eq!(strict.len(), 2, "{:?}", strict);
    assert!(strict.iter().all(|h| h.path != "notes.txt"));
    // The loose mode is the old substring match: every line counts.
    let loose = todos(dir.path(), TodoMatch::Loose).unwrap();
    let noisy: Vec<usize> = loose
        .iter()
        .filter(|h| h.path == "notes.txt")
        .map(|h| h.line)
        .collect();
    assert_eq!(noisy, [1, 2, 3, 4, 5]);
}

#[test]
fn todo_marker_rules() {
    let strict = |line: &str| has_todo_marker(line, TodoMatch::Strict);
    for line in [
        "// TODO",
        "# TODO fix the parser",
        "todo: lower case with a colon",
        "Fixme(bob): owner",
        "todo!()",
        "/** @todo document */",
        "x = 1  # HACK",
        "XXX remove before release",
        "— TODO —",
        "(TODO)",
        "TODO-list",
    ] {
        assert!(strict(line), "should count: {}", line);
    }
    for line in [
        "todos",
        "List TODOS here",
        "TODO_TEXT_CHARS",
        "fn todo_hits()",
        "a todo list",
        "hackathon",
        "Fixme later",
        "TODOы",
        "0xXXX",
        "",
    ] {
        assert!(!strict(line), "should not count: {}", line);
    }
    assert!(has_todo_marker("hackathon", TodoMatch::Loose));
    assert!(
        !has_todo_marker("XXX", TodoMatch::Loose),
        "loose = old markers only"
    );
    assert_eq!(TodoMatch::loose(true), TodoMatch::Loose);
    assert_eq!(TodoMatch::default(), TodoMatch::Strict);
    assert_eq!(
        serde_json::to_string(&TodoMatch::Loose).unwrap(),
        r#""loose""#
    );
}

#[test]
fn changes_are_read_only_unless_persisted() {
    let dir = project();
    let before = listing(dir.path());
    let cold = changes(dir.path(), false).unwrap();
    assert!(cold.cold && !cold.persisted);
    assert_eq!((cold.new, cold.changed, cold.deleted), (4, 0, 0));
    assert!(cold.changes.iter().all(|c| c.status == "new"));
    assert_eq!(listing(dir.path()), before, "persist=false must not write");

    let saved = changes(dir.path(), true).unwrap();
    assert!(saved.cold && saved.persisted && saved.persist_error.is_none());
    assert!(dir.path().join(CACHE_FILE).is_file());

    let clean = changes(dir.path(), false).unwrap();
    assert!(!clean.cold);
    assert!(clean.changes.is_empty());

    fs::write(dir.path().join("src/lib.rs"), "pub fn changed() {}\n").unwrap();
    fs::write(dir.path().join("src/new.rs"), "fn new() {}\n").unwrap();
    fs::remove_file(dir.path().join("README.md")).unwrap();
    fs::remove_file(dir.path().join("src/util/strings.py")).unwrap();
    let report = changes(dir.path(), false).unwrap();
    let found: Vec<(&str, &str)> = report
        .changes
        .iter()
        .map(|c| (c.path.as_str(), c.status.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("src/lib.rs", "changed"),
            ("src/new.rs", "new"),
            ("README.md", "deleted"),
            ("src/util/strings.py", "deleted"),
        ]
    );
    assert_eq!((report.new, report.changed, report.deleted), (1, 1, 2));
    // Not persisted: the same differences are reported again.
    assert_eq!(changes(dir.path(), true).unwrap().changes.len(), 4);
    assert!(changes(dir.path(), false).unwrap().changes.is_empty());
}

#[test]
fn state_reports_whether_the_navigators_match_the_files() {
    let dir = project();
    let root = dir.path();
    let before = listing(root);
    let s = state(root).unwrap();
    assert_eq!(listing(root), before, "state must not write");
    assert_eq!(s.navigators.status, "missing");
    assert_eq!((s.totals.files, s.snapshot.as_deref()), (4, None));
    assert_eq!(s.workflow.triangle, [0, 0, 1]);
    assert!(Path::new(&s.root).is_absolute(), "{}", s.root);
    assert!(chrono::DateTime::parse_from_rfc3339(&s.checked).is_ok());

    let project = Project::open(root).unwrap();
    navigator::write(root, &project.update_cache(false)).unwrap();
    let s = state(root).unwrap();
    assert_eq!(s.navigators.status, "fresh", "{:?}", s.navigators);
    assert!(s.navigators.paths.is_empty());
    let updated = s.navigators.updated.clone().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(&updated).is_ok());
    assert!(s.snapshot.is_some());

    fs::write(root.join("src/lib.rs"), "pub fn changed() {}\n").unwrap();
    fs::write(root.join("src/new.rs"), "fn new() {}\n").unwrap();
    fs::remove_file(root.join("README.md")).unwrap();
    let s = state(root).unwrap().navigators;
    assert_eq!(s.status, "stale");
    assert_eq!((s.new, s.changed, s.deleted), (1, 1, 1));
    assert_eq!(s.paths, ["README.md", "src/lib.rs", "src/new.rs"]);
    assert_eq!(s.updated.as_deref(), Some(updated.as_str()));

    fs::remove_file(root.join(navigator::DEEP2)).unwrap();
    assert_eq!(state(root).unwrap().navigators.status, "incomplete");
}

#[test]
fn json_field_names_are_stable() {
    let dir = project();
    // Sorted: serde_json may preserve insertion order when another
    // workspace member enables its `preserve_order` feature.
    let keys = |v: serde_json::Value| -> Vec<String> {
        let mut keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    };
    let m = serde_json::to_value(map(dir.path(), &MapOptions::default()).unwrap()).unwrap();
    assert_eq!(
        keys(m.clone()),
        ["files", "root", "totals", "tree", "truncated"]
    );
    assert_eq!(
        keys(m["files"][0].clone()),
        ["binary", "bytes", "lines", "path", "sha", "tokens"]
    );
    assert_eq!(keys(m["totals"].clone()), ["files", "lines", "tokens"]);

    let hit = &find_symbols(dir.path(), "parse", &FindOptions::default()).unwrap()[2];
    assert_eq!(
        keys(serde_json::to_value(hit).unwrap()),
        [
            "depth",
            "kind",
            "label",
            "line_end",
            "line_start",
            "name",
            "parent",
            "path",
            "tokens"
        ]
    );
    let file = &symbols(dir.path(), Some("src/lib.rs")).unwrap()[0];
    assert_eq!(
        keys(serde_json::to_value(file).unwrap()),
        ["path", "symbols"]
    );
    let line = &read_lines(dir.path(), "src/lib.rs", 1, None).unwrap()[0];
    assert_eq!(keys(serde_json::to_value(line).unwrap()), ["line", "text"]);
    let todo = &todos(dir.path(), TodoMatch::default()).unwrap()[0];
    assert_eq!(
        keys(serde_json::to_value(todo).unwrap()),
        ["line", "path", "text", "truncated"]
    );
    let report = changes(dir.path(), false).unwrap();
    assert_eq!(
        keys(serde_json::to_value(&report).unwrap()),
        [
            "changed",
            "changes",
            "cold",
            "deleted",
            "new",
            "persist_error",
            "persisted"
        ]
    );
    assert_eq!(
        keys(serde_json::to_value(&report.changes[0]).unwrap()),
        ["lines", "path", "sha", "status", "tokens"]
    );
    let opts: FindOptions = serde_json::from_str(r#"{"case_sensitive": true}"#).unwrap();
    assert!(opts.case_sensitive && !opts.use_cache && !opts.exact);
    let opts: FindOptions = serde_json::from_str(r#"{"exact": true}"#).unwrap();
    assert!(opts.exact && !opts.case_sensitive);
}

#[test]
fn line_selection_and_snippet_helpers() {
    assert_eq!(
        select_lines("a\nb\n", 3, None),
        Err(LineRangeError::StartOutOfRange { start: 3, len: 2 })
    );
    assert_eq!(
        select_lines("", 1, None),
        Err(LineRangeError::StartOutOfRange { start: 1, len: 0 })
    );
    assert!(sentence_hits("abc", "").is_empty());
    assert_eq!(
        sentence_hits("one. two x? three x", "x"),
        [
            (1, "two x?".to_string(), false),
            (1, "three x".to_string(), false)
        ]
    );
    assert_eq!(
        todo_hits("a\n  // todo: lower\nHACK\ntodos", TodoMatch::Strict),
        [
            (2, "// todo: lower".to_string(), false),
            (3, "HACK".to_string(), false)
        ]
    );
}
