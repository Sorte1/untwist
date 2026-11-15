use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const MISBOUND: &str = "a factory parameter would be renamed to a name bound inside the module where the binding cannot be proven";

const SPLITS: [&str; 74] = [
    "a41r3-w0 --unsafe",
    "a41r3-w11 --unsafe",
    "a41r3-w12",
    "a41r3-w12 --unsafe",
    "a41r3-w13",
    "a41r3-w13 --unsafe",
    "a41r3-w6 --unsafe",
    "a41r3-w7 --unsafe",
    "bfy-m1 --unsafe",
    "bfy-m1s --unsafe",
    "bfy-x1 --unsafe",
    "bfy-x1s --unsafe",
    "esb-m1 --unsafe",
    "esb-m1s --unsafe",
    "esb-x1 --unsafe",
    "esb-x1s --unsafe",
    "esbS-e2s",
    "esbS-e2s --unsafe",
    "esbS-m2s",
    "esbS-m2s --unsafe",
    "mtr-m1 --unsafe --no-unminify",
    "mtr-m12 --unsafe --no-unminify",
    "mtr-m13 --unsafe --no-unminify",
    "mtr-m1s --unsafe --no-unminify",
    "mtr-m4 --unsafe --no-unminify",
    "mtr-m9 --unsafe --no-unminify",
    "mtr-x1 --unsafe --no-unminify",
    "mtr-x1s --unsafe --no-unminify",
    "pcl-m1 --unsafe",
    "pcl-m1s --unsafe",
    "pcl-n5 --unsafe",
    "pcl-n5s --unsafe",
    "pcl-x1 --unsafe",
    "pcl-x1s --unsafe",
    "wp4-m1 --unsafe",
    "wp4-m1s",
    "wp4-m1s --unsafe",
    "wp4-n1 --unsafe",
    "wp4-n1s",
    "wp4-n1s --unsafe",
    "wp4-n2 --unsafe",
    "wp4-n2s",
    "wp4-n2s --unsafe",
    "wp4-n5 --unsafe",
    "wp4-n5s",
    "wp4-n5s --unsafe",
    "wp4-n7 --unsafe",
    "wp4-n8 --unsafe",
    "wp4-x1 --unsafe",
    "wp4-x1s",
    "wp4-x1s --unsafe",
    "wp5-m1 --unsafe",
    "wp5-m1s",
    "wp5-m1s --unsafe",
    "wp5-n1 --unsafe",
    "wp5-n1s",
    "wp5-n1s --unsafe",
    "wp5-n2 --unsafe",
    "wp5-n2s",
    "wp5-n2s --unsafe",
    "wp5-n5 --unsafe",
    "wp5-n5s",
    "wp5-n5s --unsafe",
    "wp5-n7 --unsafe",
    "wp5-n8 --unsafe",
    "wp5-x1 --unsafe",
    "wp5-x1s",
    "wp5-x1s --unsafe",
    "wp5i-e1s",
    "wp5i-e1s --unsafe",
    "wp5i-n1s",
    "wp5i-n1s --unsafe",
    "wp5i-n2s",
    "wp5i-n2s --unsafe",
];

const REFUSALS: [&str; 13] = [
    "a41r3-w1 --unsafe",
    "a41r3-w2 --unsafe",
    "a41r3-w8 --unsafe",
    "a41r3-w9 --unsafe",
    "bfy-x2 --unsafe",
    "esb-x2 --unsafe",
    "pcl-x2 --unsafe",
    "wp4-n3 --unsafe",
    "wp4-n4 --unsafe",
    "wp4-x2 --unsafe",
    "wp5-n3 --unsafe",
    "wp5-n4 --unsafe",
    "wp5-x2 --unsafe",
];

const BOUND_EVAL: &str = "-u1";

fn probes() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-factory-names");
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("probes")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    found.sort();
    found
}

fn name_of(path: &Path, stem: bool) -> String {
    let name = match stem {
        true => path.file_stem(),
        false => path.file_name(),
    };
    name.map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn observed(node: &Path, file: &Path) -> (String, bool) {
    let observed = run(node, file).expect("spawn").expect("terminates");
    (observed.stdout, observed.exit == Some(0))
}

struct Written {
    entry: PathBuf,
    tree: bool,
    notes: String,
}

fn written(path: &Path, (unsafe_transforms, unminify): (bool, bool), dir: &Path) -> Written {
    let source = std::fs::read_to_string(path).expect("probe");
    let options = Options {
        unsafe_transforms,
        unminify,
        input_names: vec![name_of(path, false)],
        ..Options::default()
    };
    let out = Pipeline::new(options).run(&source);
    out.write_to(dir).expect("write");
    let tree = dir.join("bundle.json").exists();
    let entry = match dir.join("index.js").exists() {
        true => dir.join("index.js"),
        false => dir.join("deobfuscated.js"),
    };
    Written {
        entry,
        tree,
        notes: format!("{:?}", out.notes),
    }
}

fn shape_wrong(row: &str, written: &Written) -> Option<String> {
    if SPLITS.contains(&row) && !written.tree {
        return Some(format!("{row}: expected a written tree: {}", written.notes));
    }
    if row.contains(BOUND_EVAL) && written.tree {
        return Some(format!(
            "{row}: a direct eval through a local eval binding must keep the bundle whole"
        ));
    }
    let refused = !written.tree && written.notes.contains(MISBOUND);
    if REFUSALS.contains(&row) && !refused {
        return Some(format!(
            "{row}: expected the refusal {MISBOUND:?}: {}",
            written.notes
        ));
    }
    None
}

#[test]
fn every_factory_name_probe_prints_and_exits_the_same_after_untwist_in_both_lanes() {
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
        let name = name_of(&path, true);
        let metro = name.starts_with("mtr-");
        let lanes = [
            ("", (false, true)),
            ("--unsafe", (true, true)),
            ("--unsafe --no-unminify", (true, false)),
        ];
        let lanes = lanes
            .iter()
            .filter(|(flags, _)| metro || !flags.contains("no-unminify"));
        for (flags, lane) in lanes {
            let row = format!("{name} {flags}").trim().to_string();
            let dir = tempfile::tempdir().expect("temp dir");
            let written = written(&path, *lane, dir.path());
            let got = observed(&node, &written.entry);
            wrong.extend(shape_wrong(&row, &written));
            rows += 1;
            if got != expected {
                wrong.push(format!(
                    "{row}: node gave {expected:?}, untwist's output gave {got:?}"
                ));
            }
        }
    }
    assert!(rows >= 272, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
