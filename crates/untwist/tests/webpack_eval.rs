use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::layers::INLINE_LAYER_PASS;
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const APP_MODULES: [&str; 6] = [
    "index.js",
    "util.js",
    "legacy.js",
    "strict-cjs.js",
    "data.json",
    "shapes.js",
];

const UNPACK_PENDING: &[&str] = &[];

struct Row {
    path: String,
    flags: String,
    sha256: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/webpack-eval")
}

fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(root().join("MANIFEST")).expect("MANIFEST");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [path, _bundler, _loader, flags, sha256] = fields[..] else {
                panic!("MANIFEST line has {} fields: {line}", fields.len());
            };
            Row {
                path: path.to_string(),
                flags: flags.to_string(),
                sha256: sha256.to_string(),
            }
        })
        .collect()
}

fn source(row: &Row) -> String {
    let bytes = std::fs::read(root().join(&row.path)).expect("fixture");
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(digest, row.sha256, "{}: the hash moved", row.path);
    String::from_utf8(bytes).expect("utf-8")
}

fn unpacked(unsafe_transforms: bool, source: &str) -> Output {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source)
}

fn development(row: &Row) -> bool {
    row.flags.starts_with("mode=development")
}

fn problems(row: &Row, out: &Output) -> Vec<String> {
    let mut found = Vec::new();
    let peels = out
        .stages
        .iter()
        .filter(|stage| stage.stage == INLINE_LAYER_PASS)
        .count();
    if peels != 1 {
        found.push(format!("{peels} inline peels"));
    }
    let pending = UNPACK_PENDING
        .iter()
        .any(|prefix| row.path.starts_with(prefix));
    let Some(bundle) = &out.bundle else {
        if !pending {
            found.push("not unpacked".to_string());
        } else if out.code().contains("eval(\"") || out.code().contains("sourceURL=") {
            found.push("keeps an eval or a sourceURL".to_string());
        }
        return found;
    };
    if pending {
        found.push("unpacked: drop it from UNPACK_PENDING".to_string());
    }
    if bundle.kind != "webpack" {
        found.push(format!("unpacked as {}", bundle.kind));
    }
    for module in &bundle.modules {
        if module.code.contains("eval(\"") || module.code.contains("sourceURL=") {
            found.push(format!("{} keeps an eval or a sourceURL", module.path));
        }
    }
    if development(row) {
        let missing: Vec<&str> = APP_MODULES
            .iter()
            .copied()
            .filter(|name| {
                !bundle
                    .modules
                    .iter()
                    .any(|module| module.id.ends_with(name))
            })
            .collect();
        if !missing.is_empty() {
            found.push(format!("modules missing: {missing:?}"));
        }
    }
    found
}

#[test]
fn every_eval_devtool_bundle_is_inlined_once_and_unpacked_as_webpack_in_both_lanes() {
    let rows = rows();
    assert!(rows.len() >= 39, "{} rows", rows.len());
    let mut wrong = Vec::new();
    for row in &rows {
        let text = source(row);
        for unsafe_transforms in [false, true] {
            let out = unpacked(unsafe_transforms, &text);
            for problem in problems(row, &out) {
                wrong.push(format!(
                    "{} unsafe={unsafe_transforms}: {problem}",
                    row.path
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn every_unpacked_eval_devtool_bundle_prints_what_the_bundle_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut wrong = Vec::new();
    for row in rows() {
        let expected = run(&node, &root().join(&row.path))
            .expect("spawn")
            .expect("terminates");
        assert!(!expected.stdout.is_empty(), "{} prints nothing", row.path);
        for unsafe_transforms in [false, true] {
            let out = unpacked(unsafe_transforms, &source(&row));
            let dir = tempfile::tempdir().expect("temp dir");
            out.write_to(dir.path()).expect("write");
            let entry = match out.bundle {
                Some(_) => dir.path().join("index.js"),
                None => dir.path().join("deobfuscated.js"),
            };
            let actual = run(&node, &entry).expect("spawn").expect("terminates");
            if actual.stdout != expected.stdout || actual.exit != expected.exit {
                wrong.push(format!(
                    "{} unsafe={unsafe_transforms}: bundle printed {:?}, unpacked printed {:?} {:?}",
                    row.path, expected.stdout, actual.stdout, actual.stderr
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
