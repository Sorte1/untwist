use std::path::{Path, PathBuf};

use untwist::{parse_modern_baseline, Options, Pipeline};
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

fn fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/modern");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort();
    files
}

fn options(baseline: Option<&str>, unsafe_transforms: bool) -> Options {
    Options {
        unpack: false,
        unsafe_transforms,
        modern: baseline.map(|text| parse_modern_baseline(text).unwrap()),
        ..Options::default()
    }
}

struct Ran {
    code: String,
    notes: Vec<String>,
}

fn untwist(source: &str, options: &Options) -> Ran {
    let output = Pipeline::new(options.clone()).run(source);
    Ran {
        code: output.code().to_string(),
        notes: output
            .notes
            .iter()
            .filter(|note| note.pass == "modern")
            .map(|note| format!("modern: {}", note.message))
            .collect(),
    }
}

fn printed(node: &Path, dir: &Path, code: &str) -> String {
    let path = dir.join("program.js");
    std::fs::write(&path, code).unwrap();
    let observed = run(node, &path).unwrap().expect("node finished");
    format!("{}exit {:?}\n", observed.stdout, observed.exit)
}

fn removed(notes: &[String]) -> usize {
    notes
        .iter()
        .filter(|note| note.starts_with("modern: removed core-js"))
        .count()
}

#[test]
fn modern_output_prints_what_the_babel_build_prints_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for file in fixtures() {
        let source = std::fs::read_to_string(&file).unwrap();
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        let before = printed(&node, dir.path(), &source);
        let rollup = name.ends_with(".rollup-cjs.js") || name.ends_with(".rollup-cjs.terser.js");
        let lanes: &[(&str, bool)] = match rollup {
            true => &[
                ("es2017", false),
                ("es2022", false),
                ("es2024", true),
                ("node 22", true),
            ],
            false => &[("es2022", false)],
        };
        for &(baseline, unsafe_transforms) in lanes {
            let ran = untwist(&source, &options(Some(baseline), unsafe_transforms));
            let after = printed(&node, dir.path(), &ran.code);
            if after != before {
                wrong.push(format!("{name} --modern {baseline}: node printed {before:?}, the output printed {after:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_mangled_terser_builds_lose_their_native_polyfills() {
    for file in fixtures()
        .iter()
        .filter(|file| file.to_string_lossy().ends_with(".rollup-cjs.terser.js"))
    {
        let source = std::fs::read_to_string(file).unwrap();
        for unsafe_transforms in [false, true] {
            let ran = untwist(&source, &options(Some("es2022"), unsafe_transforms));
            let count = removed(&ran.notes);
            assert!(
                count >= 50,
                "{} (unsafe {unsafe_transforms}): removed {count}\n{}",
                file.display(),
                ran.notes.join("\n")
            );
        }
    }
}

#[test]
fn the_rollup_builds_lose_their_native_polyfills() {
    for file in fixtures()
        .iter()
        .filter(|file| file.to_string_lossy().ends_with(".rollup-cjs.js"))
    {
        let source = std::fs::read_to_string(file).unwrap();
        let ran = untwist(&source, &options(Some("es2022"), true));
        let count = removed(&ran.notes);
        assert!(
            count >= 20,
            "{}: removed {count}\n{}",
            file.display(),
            ran.notes.join("\n")
        );
        assert!(
            ran.code.len() * 4 < source.len() * 3,
            "{}: {} -> {} bytes",
            file.display(),
            source.len(),
            ran.code.len()
        );
    }
}

#[test]
fn modern_mode_is_off_by_default_and_under_unsafe() {
    let file = fixtures()
        .into_iter()
        .find(|file| {
            file.to_string_lossy()
                .contains("usage.core-js-3.39.0.rollup-cjs.js")
        })
        .unwrap();
    let source = std::fs::read_to_string(file).unwrap();
    for unsafe_transforms in [false, true] {
        let ran = untwist(&source, &options(None, unsafe_transforms));
        assert!(
            ran.notes.iter().all(|note| !note.starts_with("modern:")),
            "{:?}",
            ran.notes
        );
        assert!(!ran.code.contains("untwist --modern"));
    }
}

#[test]
fn the_report_says_the_output_assumes_a_modern_engine() {
    let file = fixtures()
        .into_iter()
        .find(|file| {
            file.to_string_lossy()
                .contains("usage.core-js-3.39.0.rollup-cjs.js")
        })
        .unwrap();
    let source = std::fs::read_to_string(file).unwrap();
    let ran = untwist(&source, &options(Some("es2022"), false));
    assert!(
        ran.notes.iter().any(|note| note
            .starts_with("modern: the output assumes an engine at least as new as chrome 94")),
        "{:?}",
        ran.notes
    );
    assert!(ran.code.contains("// untwist --modern es2022:"));
}

#[test]
fn proposals_and_features_newer_than_the_baseline_are_kept() {
    let file = fixtures()
        .into_iter()
        .find(|file| {
            file.to_string_lossy()
                .contains("usage.core-js-3.39.0.rollup-cjs.js")
        })
        .unwrap();
    let source = std::fs::read_to_string(file).unwrap();
    let ran = untwist(&source, &options(Some("es2017"), true));
    let kept = |module: &str| {
        ran.notes
            .iter()
            .any(|note| note.starts_with("modern: kept") && note.contains(module))
    };
    assert!(kept("modules/es.array.at.js"), "{}", ran.notes.join("\n"));
    assert!(
        kept("modules/es.object.group-by.js"),
        "{}",
        ran.notes.join("\n")
    );
    assert!(
        kept("modules/es.set.union.v2.js") || kept("modules/es.set.union"),
        "{}",
        ran.notes.join("\n")
    );
}

#[test]
fn every_helper_rewrite_runs_on_the_rollup_builds() {
    let file = fixtures()
        .into_iter()
        .find(|file| {
            file.to_string_lossy()
                .ends_with("babel-usage.core-js-3.39.0.rollup-cjs.js")
        })
        .unwrap();
    let source = std::fs::read_to_string(file).unwrap();
    let ran = untwist(&source, &options(Some("node 22"), true));
    for kind in [
        "wellKnownSymbol(name) -> Symbol.name",
        "getBuiltIn(...) -> the global",
        "hasOwn(it, key) -> Object.hasOwn(it, key)",
        "uncurryThis(method)(that, ...) -> Proto.method.call(that, ...)",
        "isCallable(it) -> typeof it == \"function\"",
    ] {
        assert!(
            ran.notes
                .iter()
                .any(|note| note.starts_with("modern: rewrote") && note.ends_with(kind)),
            "{kind}\n{}",
            ran.notes.join("\n")
        );
    }
    assert!(ran.code.contains("Symbol.iterator"));
    assert!(ran.code.contains("Object.hasOwn("));
}

#[test]
fn is_callable_stays_a_helper_call_when_a_browser_is_in_the_baseline() {
    let file = fixtures()
        .into_iter()
        .find(|file| {
            file.to_string_lossy()
                .ends_with("babel-usage.core-js-3.39.0.rollup-cjs.js")
        })
        .unwrap();
    let source = std::fs::read_to_string(file).unwrap();
    let ran = untwist(&source, &options(Some("node 22, chrome 125"), true));
    assert!(
        !ran.notes
            .iter()
            .any(|note| note.contains("isCallable(it) ->")),
        "{}",
        ran.notes.join("\n")
    );
}
