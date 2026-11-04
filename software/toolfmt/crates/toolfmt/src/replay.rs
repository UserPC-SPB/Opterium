//! Replay of the fixture corpus against recorded baselines.

use crate::capture::{walk_fixtures, write_baseline};
use crate::config::Config;
use crate::meta::{clean_text, run_fixture};
use crate::standard::Store;
use std::path::Path;

pub struct Report {
    pub green: usize,
    pub failed: Vec<String>,
    pub unbaselined: usize,
    pub updated: usize,
}

pub fn replay(home: &Path, cfg: Option<&Config>, update: bool) -> Report {
    let store = Store::new(home.to_path_buf());
    let mut rep = Report {
        green: 0,
        failed: Vec::new(),
        unbaselined: 0,
        updated: 0,
    };
    for f in walk_fixtures(&store.fixtures_dir()) {
        let Some((out, raw)) = run_fixture(Some(&store), cfg, &f, None) else {
            continue;
        };
        let text = clean_text(&out, &raw);
        let rel = f
            .path
            .strip_prefix(home)
            .unwrap_or(&f.path)
            .display()
            .to_string();
        match f.baseline() {
            Some((bt, bu)) if bt == text && bu == out.used => rep.green += 1,
            Some((bt, bu)) => {
                if update {
                    if write_baseline(&f, &text, &out.used).is_ok() {
                        rep.updated += 1;
                    }
                    continue;
                }
                let mut why = format!("{rel}:");
                if bu != out.used {
                    why.push_str(&format!(" used {bu} -> {}", out.used));
                }
                if bt != text {
                    let (n, a, b) = first_diff(bt, &text);
                    why.push_str(&format!(" line {n}: {a:?} -> {b:?}"));
                }
                rep.failed.push(why);
            }
            None => {
                if update && write_baseline(&f, &text, &out.used).is_ok() {
                    rep.updated += 1;
                } else {
                    rep.unbaselined += 1;
                }
            }
        }
    }
    rep
}

fn first_diff(a: &str, b: &str) -> (usize, String, String) {
    let mut la = a.lines();
    let mut lb = b.lines();
    let mut n = 1;
    loop {
        match (la.next(), lb.next()) {
            (Some(x), Some(y)) if x == y => n += 1,
            (x, y) => {
                let cut = |s: Option<&str>| s.unwrap_or("<end>").chars().take(80).collect();
                return (n, cut(x), cut(y));
            }
        }
    }
}
