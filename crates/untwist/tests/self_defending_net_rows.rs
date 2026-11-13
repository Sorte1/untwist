use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, SOURCE_READER_UNGUARDED, SOURCE_READER_UNGUARDED_MODULE};
use untwist_core::guard_shape::{source_readers, SourceReaders};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const ROWS: &str = include_str!("data/self_defending_net_rows.txt");

struct Row {
    guarded: bool,
    name: &'static str,
    source: &'static str,
}

fn rows() -> Vec<Row> {
    ROWS.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (head, source) = line.split_once('|').unwrap();
            let (kind, name) = head.split_once(':').unwrap();
            assert!(kind == "net" || kind == "guarded", "{line}");
            Row {
                guarded: kind == "guarded",
                name,
                source,
            }
        })
        .collect()
}

fn lanes() -> [(&'static str, Options); 3] {
    [
        ("default", Options::default()),
        (
            "unsafe",
            Options {
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
        (
            "reprint",
            Options {
                deobfuscate: false,
                unminify: false,
                ..Options::default()
            },
        ),
    ]
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn observed(node: &Path, path: &Path) -> String {
    match run(node, path).unwrap() {
        Some(out) => format!("{}exit {:?}", out.stdout, out.exit),
        None => "did not finish".to_string(),
    }
}

fn netted(out: &untwist::Output) -> bool {
    out.notes.iter().any(|note| {
        note.message == SOURCE_READER_UNGUARDED || note.message == SOURCE_READER_UNGUARDED_MODULE
    })
}

fn check(node: &Path, dir: &Path, row: &Row) -> Vec<String> {
    let input = dir.join(format!("{}.js", row.name));
    std::fs::write(&input, row.source).unwrap();
    let want = observed(node, &input);
    assert!(want.ends_with("exit Some(0)"), "{}: {want}", row.name);
    let mut wrong = Vec::new();
    for (lane, options) in lanes() {
        let out = Pipeline::new(options).run(row.source);
        let path = dir.join(format!("{}.{lane}.js", row.name));
        std::fs::write(&path, out.code()).unwrap();
        let got = observed(node, &path);
        if got != want {
            wrong.push(format!(
                "{} ({lane}): node printed {got:?}, input {want:?}",
                row.name
            ));
        }
        if row.guarded && netted(&out) {
            wrong.push(format!("{} ({lane}): passed through", row.name));
        }
        let modules = out.bundle.iter().flat_map(|bundle| &bundle.modules);
        for module in modules {
            if source_readers(&module.code) == SourceReaders::Unguarded {
                wrong.push(format!(
                    "{} ({lane}): module {} reads a broken target",
                    row.name, module.path
                ));
            }
        }
    }
    wrong
}

#[test]
fn every_self_defending_net_row_runs_like_its_input_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let wrong: Vec<String> = rows()
        .iter()
        .flat_map(|row| check(&node, dir.path(), row))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
