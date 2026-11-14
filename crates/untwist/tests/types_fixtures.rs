#![cfg(feature = "types")]

use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_types::{annotate, erases, Mode, Refusal, TypeStats};

const FAMILIES: &[&str] = &[
    "unminify",
    "untranspile",
    "obfuscator-io",
    "rollup-commonjs",
    "libraries",
    "unpack-bundlers",
    "unpack-rollup-es",
    "unpack-rolldown",
    "bundle-matrix",
    "behaviour",
    "eval",
    "jsx",
];

const SAMPLED: usize = 40;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn js_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            js_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            found.push(path);
        }
    }
}

fn family_inputs(family: &str, limit: Option<usize>) -> Vec<(String, String)> {
    let root = workspace().join("fixtures").join(family);
    let mut paths = Vec::new();
    js_files(&root, &mut paths);
    let step = match limit {
        Some(limit) if paths.len() > limit => paths.len().div_ceil(limit),
        _ => 1,
    };
    paths
        .into_iter()
        .step_by(step)
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let label = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            Some((label, text))
        })
        .collect()
}

fn outputs(source: &str) -> Vec<(String, String)> {
    let output = Pipeline::new(Options::default()).run(source);
    let mut texts: Vec<(String, String)> = output
        .codes
        .iter()
        .enumerate()
        .map(|(index, code)| (format!("output{index}"), code.clone()))
        .collect();
    if let Some(bundle) = &output.bundle {
        texts.extend(
            bundle
                .modules
                .iter()
                .map(|module| (module.path.clone(), module.code.clone())),
        );
    }
    texts
}

#[test]
fn every_annotated_fixture_output_strips_back_to_the_untyped_output() {
    let mut failures = Vec::new();
    for family in FAMILIES {
        for (label, source) in family_inputs(family, Some(SAMPLED)) {
            for (name, code) in outputs(&source) {
                for mode in [Mode::Ts, Mode::Jsdoc] {
                    let typed = annotate(&code, mode);
                    if typed.refused == Some(Refusal::Erasure) {
                        failures.push(format!(
                            "{family}/{label} {name} {mode:?}: refused at erasure"
                        ));
                    }
                    if typed.code == code {
                        continue;
                    }
                    if let Err(reason) = erases(&code, &typed.code, mode, typed.jsx) {
                        failures.push(format!("{family}/{label} {name} {mode:?}: {reason}"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn typed_name(family: &str, label: &str, name: &str, extension: &str) -> String {
    let flat: String = format!("{family}__{label}__{name}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{flat}.{extension}")
}

fn write_pair(out: &Path, mode: Mode, file: &str, plain: &str, typed: &str) {
    let tree = out.join(mode.as_str());
    for (side, text) in [("annotated", typed), ("plain", plain)] {
        let dir = tree.join(side);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), text).unwrap();
    }
}

fn percent(part: usize, whole: usize) -> f64 {
    match whole {
        0 => 0.0,
        _ => 100.0 * part as f64 / whole as f64,
    }
}

fn line(label: &str, stats: &TypeStats) -> String {
    let total = stats.total();
    format!(
        "{label:<18} positions {:>6} proven {:>6} ({:>5.1}%) annotated {:>5} ({:>5.1}%) of-which-inferred {:>5} refused {:>5} | bindings {}/{} params {}/{} returns {}/{} | files {} refused {}",
        total.eligible,
        total.proven,
        percent(total.proven, total.eligible),
        total.annotated,
        percent(total.annotated, total.eligible),
        total.redundant,
        total.refused,
        stats.bindings.annotated,
        stats.bindings.eligible,
        stats.params.annotated,
        stats.params.eligible,
        stats.returns.annotated,
        stats.returns.eligible,
        stats.files,
        stats.files_refused,
    )
}

#[test]
#[ignore = "writes the tsc delta pairs and prints coverage; run with UNTWIST_TYPES_OUT=<dir>"]
fn coverage_and_tsc_pairs_over_the_fixture_corpus() {
    let out = std::env::var_os("UNTWIST_TYPES_OUT").map(PathBuf::from);
    let limit = std::env::var("UNTWIST_TYPES_LIMIT")
        .ok()
        .and_then(|n| n.parse().ok());
    let mut all = TypeStats::default();
    let mut lines = Vec::new();
    for family in FAMILIES {
        let mut stats = TypeStats::default();
        for (label, source) in family_inputs(family, limit) {
            for (name, code) in outputs(&source) {
                if let (Some(out), Some(declared)) = (&out, untwist_types::declarations(&code)) {
                    let dir = out.join("dts").join("annotated");
                    std::fs::create_dir_all(&dir).unwrap();
                    let file = typed_name(family, &label, &name, "d.ts");
                    std::fs::write(dir.join(file), declared).unwrap();
                }
                for mode in [Mode::Ts, Mode::Jsdoc] {
                    let typed = annotate(&code, mode);
                    if mode == Mode::Ts {
                        stats.absorb(typed.stats);
                    }
                    let (Some(out), false) = (&out, typed.code == code) else {
                        continue;
                    };
                    let extension = match (mode, typed.jsx) {
                        (Mode::Ts, true) => "tsx",
                        (Mode::Ts, false) => "ts",
                        (Mode::Jsdoc, true) => "jsx",
                        (Mode::Jsdoc, false) => "js",
                    };
                    let file = typed_name(family, &label, &name, extension);
                    write_pair(out, mode, &file, &code, &typed.code);
                }
            }
        }
        lines.push(line(family, &stats));
        all.absorb(stats);
    }
    lines.push(line("all", &all));
    let mut reasons: Vec<(&String, &usize)> = all.reasons.iter().collect();
    reasons.sort_by(|a, b| b.1.cmp(a.1));
    for (reason, count) in reasons.iter().take(30) {
        lines.push(format!("  refused {count:>6} {reason}"));
    }
    println!("{}", lines.join("\n"));
}

#[test]
fn rename_reserves_every_type_name_the_types_stage_writes() {
    for name in untwist_types::EMITTED_TYPE_NAMES {
        assert!(untwist_rename::reserved::is_reserved(name), "{name}");
    }
}
