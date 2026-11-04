//! The golden corpus in examples/home: every standard is valid and every
//! fixture still renders exactly as its reviewed baseline.

use std::path::PathBuf;
use toolfmt::config::Config;

fn home() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("home")
}

#[test]
fn corpus_replays_green() {
    let cfg = Config::load(&home().join("upstreams.json")).unwrap();
    let rep = toolfmt::replay::replay(&home(), Some(&cfg), false);
    assert!(rep.failed.is_empty(), "{:#?}", rep.failed);
    assert_eq!(rep.unbaselined, 0);
    assert!(rep.green >= 23, "only {} fixtures", rep.green);
}

#[test]
fn corpus_standards_are_valid() {
    let dir = home().join("standards");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .chain(std::fs::read_dir(dir.join("family")).unwrap())
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .collect();
    files.sort();
    assert!(files.len() >= 10);
    for p in files {
        let text = std::fs::read_to_string(&p).unwrap();
        toolfmt::standard::compile(&text, &p.display().to_string(), "t")
            .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    }
}
