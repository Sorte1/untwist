use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, SYNTAX_ERROR_REFUSED};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const CLASS_AWAIT: [&str; 6] = [
    "p1",
    "p1 --unsafe",
    "p10",
    "p10 --unsafe",
    "p15",
    "p15 --unsafe",
];

const SPLITS: [&str; 14] = [
    "s1",
    "s1 --unsafe",
    "s2",
    "s2 --unsafe",
    "s3",
    "s3 --unsafe",
    "s4",
    "s4 --unsafe",
    "s5",
    "s5 --unsafe",
    "s11",
    "s11 --unsafe",
    "p11",
    "p11 --unsafe",
];

const PLACEMENT: &str = "the bundle sits inside a top-level statement";

const REFUSALS: [&str; 38] = [
    "w1",
    "w1 --unsafe",
    "w2",
    "w2 --unsafe",
    "w3",
    "w3 --unsafe",
    "w4",
    "w4 --unsafe",
    "w7",
    "w7 --unsafe",
    "w14",
    "w14 --unsafe",
    "w15",
    "w15 --unsafe",
    "u1",
    "u1 --unsafe",
    "u5",
    "u5 --unsafe",
    "u6",
    "u6 --unsafe",
    "u7",
    "u7 --unsafe",
    "s6",
    "s6 --unsafe",
    "s7",
    "s7 --unsafe",
    "s8",
    "s8 --unsafe",
    "s10",
    "s10 --unsafe",
    "s12",
    "s12 --unsafe",
    "s13",
    "s13 --unsafe",
    "s9",
    "s9 --unsafe",
    "s14",
    "s14 --unsafe",
];

const SYNTAX_ERRORS: [&str; 71] = [
    "ee01", "ee02", "ee03", "ee04", "ee05", "ee06", "ee07", "ee08", "ee09", "ee10", "ee11", "ee12",
    "ee13", "ee14", "ee15", "ee16", "ee17", "ee18", "ee19", "ee20", "ee21", "ee22", "ee23", "ee24",
    "ee25", "ee26", "ee27", "ee28", "ee29", "ee30", "ee31", "ee32", "ee33", "ee34", "ee35", "ee37",
    "ee38", "ee39", "ee40", "ee41", "ee42", "ee43", "ee44", "ee45", "ee46", "ee47", "ee48", "ee49",
    "ee50", "ee51", "ee52", "ee53", "ee54", "ee56", "ee57", "ee58", "ee59", "ee60", "ee61", "ee62",
    "ee63", "ee65", "ee67", "ee68", "ee69", "ee70", "g17", "g18", "g19", "g21", "g22",
];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn probes() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures().join("unpack-a37rr4"))
        .expect("probes")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "js" || ext == "mjs")
        })
        .collect();
    found.sort();
    found
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn lanes(path: &Path) -> Vec<(String, Options)> {
    let options = |unsafe_transforms: bool| Options {
        unsafe_transforms,
        input_names: vec![file_name(path)],
        ..Options::default()
    };
    vec![
        (String::new(), options(false)),
        ("--unsafe".to_string(), options(true)),
    ]
}

fn observed(node: &Path, file: &Path) -> (String, bool) {
    let observed = run(node, file).expect("spawn").expect("terminates");
    (observed.stdout, observed.exit == Some(0))
}

struct Written {
    entry: PathBuf,
    tree: bool,
    notes: String,
    code: String,
    parse_errors: usize,
}

fn written(path: &Path, options: Options, dir: &Path) -> Written {
    let source = std::fs::read_to_string(path).expect("probe");
    let out = Pipeline::new(options).run(&source);
    let notes = format!("{:?}", out.notes);
    let code = out.code().to_string();
    let parse_errors = out.parse_errors;
    if path.extension().is_some_and(|ext| ext == "mjs") {
        let entry = dir.join(file_name(path));
        std::fs::write(&entry, &code).expect("write");
        return Written {
            entry,
            tree: false,
            notes,
            code,
            parse_errors,
        };
    }
    out.write_to(dir).expect("write");
    let tree = dir.join("bundle.json").exists();
    let entry = match dir.join("index.js").exists() {
        true => dir.join("index.js"),
        false => dir.join("deobfuscated.js"),
    };
    Written {
        entry,
        tree,
        notes,
        code,
        parse_errors,
    }
}

fn checked_row(
    row: &str,
    (name, source): (&str, &str),
    written: &Written,
    wrong: &mut Vec<String>,
) {
    if SPLITS.contains(&row) && !written.tree {
        wrong.push(format!("{row}: expected a written tree: {}", written.notes));
    }
    if REFUSALS.contains(&row) && (written.tree || !written.notes.contains(PLACEMENT)) {
        wrong.push(format!(
            "{row}: expected the refusal {PLACEMENT:?}: {}",
            written.notes
        ));
    }
    let syntax = SYNTAX_ERRORS.contains(&name);
    let refused = written.parse_errors > 0
        && written.code == source
        && written.notes.contains(SYNTAX_ERROR_REFUSED);
    if syntax != refused {
        wrong.push(format!(
            "{row}: expected a syntax-error refusal: {syntax}, got {} errors: {}",
            written.parse_errors, written.notes
        ));
    }
}

#[test]
fn every_a37rr4_probe_prints_and_exits_the_same_after_untwist_in_both_lanes() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let mut wrong = Vec::new();
    let mut rows = 0;
    for path in probes() {
        let expected = observed(&node, &path);
        let source = std::fs::read_to_string(&path).expect("probe");
        let name = stem(&path);
        for (flags, options) in lanes(&path) {
            let row = format!("{name} {flags}").trim().to_string();
            let dir = tempfile::tempdir().expect("temp dir");
            let written = written(&path, options, dir.path());
            let got = observed(&node, &written.entry);
            checked_row(&row, (&name, &source), &written, &mut wrong);
            rows += 1;
            let pending = CLASS_AWAIT.contains(&row.as_str());
            if got != expected && !pending {
                wrong.push(format!(
                    "{row}: node gave {expected:?}, untwist's output gave {got:?}"
                ));
            }
        }
    }
    assert!(rows >= 398, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
