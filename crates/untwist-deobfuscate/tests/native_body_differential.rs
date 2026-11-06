use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_eval::test_hooks::{native_body_runs, record_units, set_native_bodies, UnitRecord};

const GROUPS: [(&str, &[&str]); 5] = [
    (
        "fixtures/obfuscator-io",
        &[
            "encoding-",
            "pair-rc4+",
            "pair-cff+rc4",
            "pair-dead+rc4",
            "preset-",
        ],
    ),
    ("fixtures/eval", &["base64-", "rc4-"]),
    ("fixtures/corpus-npm", &["case-"]),
    ("fixtures/obfuscated", &[""]),
    ("fixtures/reference/deobfuscate", &["obfuscator.io-"]),
];

const ENCODED: [&str; 4] = ["encoding-", "pair-rc4+", "pair-cff+rc4", "pair-dead+rc4"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn inputs() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for (dir, prefixes) in GROUPS {
        let Ok(entries) = std::fs::read_dir(root().join(dir)) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                name.ends_with(".js") && prefixes.iter().any(|prefix| name.starts_with(prefix))
            })
            .collect();
        paths.sort();
        found.extend(paths);
    }
    found
}

struct Run {
    code: Vec<String>,
    notes: String,
    stats: untwist::RunStats,
    decoded: String,
    heap_peak: u64,
    units: Vec<UnitRecord>,
    native: u64,
}

fn run(source: &str, options: &Options, native: bool) -> Run {
    set_native_bodies(native);
    let before = native_body_runs();
    let mut output = None;
    let units = record_units(|| output = Some(Pipeline::new(options.clone()).run(source)));
    let native = native_body_runs() - before;
    set_native_bodies(true);
    let output = output.expect("a pipeline output");
    Run {
        code: output.codes.clone(),
        notes: format!("{:?}", output.notes),
        stats: output.stats,
        decoded: format!("{:?}", output.decoded_strings),
        heap_peak: output.heap_peak,
        units,
        native,
    }
}

fn assert_same(native: &Run, interpreted: &Run, context: &str) {
    assert_eq!(native.code, interpreted.code, "{context}: code");
    assert_eq!(native.notes, interpreted.notes, "{context}: notes");
    assert_eq!(native.stats, interpreted.stats, "{context}: stats");
    assert_eq!(
        native.decoded, interpreted.decoded,
        "{context}: decoded strings"
    );
    assert!(
        native.heap_peak <= interpreted.heap_peak,
        "{context}: heap peak"
    );
    assert_eq!(
        native.units.len(),
        interpreted.units.len(),
        "{context}: units"
    );
    for (index, (one, other)) in native.units.iter().zip(&interpreted.units).enumerate() {
        assert!(
            one.heap_peak <= other.heap_peak,
            "{context}: unit {index} heap peak"
        );
        let mut levelled = one.clone();
        levelled.heap_peak = other.heap_peak;
        assert_eq!(&levelled, other, "{context}: unit {index}");
    }
}

fn lanes() -> [(&'static str, Options); 2] {
    [
        ("default", Options::default()),
        (
            "unsafe",
            Options {
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
    ]
}

fn cell(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.split("--").next().unwrap_or(name).to_string()
}

#[test]
fn native_and_interpreted_decoder_bodies_give_the_same_pipeline_output() {
    let mut cells: BTreeMap<String, u64> = BTreeMap::new();
    let paths = inputs();
    assert!(paths.len() > 150, "{} inputs", paths.len());
    for path in &paths {
        let source = std::fs::read_to_string(path).expect("read an input");
        for (lane, options) in lanes() {
            let context = format!("{} [{lane}]", path.display());
            let interpreted = run(&source, &options, false);
            let native = run(&source, &options, true);
            assert_eq!(interpreted.native, 0, "{context}");
            assert_same(&native, &interpreted, &context);
            *cells.entry(format!("{} [{lane}]", cell(path))).or_default() += native.native;
        }
    }
    for (cell, runs) in &cells {
        let encoded = ENCODED.iter().any(|prefix| cell.starts_with(prefix));
        if encoded && cell.ends_with("[unsafe]") {
            assert!(*runs > 0, "{cell}: no native body ran");
        }
    }
    let total: u64 = cells.values().sum();
    println!(
        "{} inputs, {total} native loop runs, all equal: {cells:?}",
        paths.len()
    );
}
