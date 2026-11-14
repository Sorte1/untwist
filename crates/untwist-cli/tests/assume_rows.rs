mod common;

use std::path::Path;

use untwist_testing::node::run_with;
use untwist_testing::node_rows::node_or_skip;

const FLAGS: [&str; 2] = ["--split-hoisted", "--split-assume-modules"];
const PACKAGE: &str = "{\"type\":\"module\"}\n";

const WRAPPER: &str = "var hasRequiredA, a;
  function requireA() { if (hasRequiredA) return a; hasRequiredA = 1; a = READ; return a; }";

struct Row {
    name: &'static str,
    source: String,
    splits: Option<bool>,
}

fn nested(pre: &str, read: &str, post: &str) -> String {
    let wrapper = WRAPPER.replace("READ", read);
    format!(
        "(function () {{ \"use strict\";\n{pre}\n(function () {{ \"use strict\";\n  {wrapper}\n  console.log(requireA()); }})();\n{post}\n}})();\n"
    )
}

fn rows() -> Vec<Row> {
    vec![
        Row {
            name: "outer-later-function",
            source: nested(
                "",
                "helper(\"x\")",
                "function helper(v) { console.log(\"helper\", v); return [v]; }",
            ),
            splits: None,
        },
        Row {
            name: "outer-later-function-over-var",
            source: nested("var cfg;", "typeof cfg", "function cfg() {}"),
            splits: None,
        },
        Row {
            name: "outer-later-function-in-block-var",
            source: nested("", "typeof flag", "if (true) { var flag = 1; }"),
            splits: None,
        },
        Row {
            name: "outer-free-read-of-wrapper-name",
            source: nested("", "1", "console.log(typeof requireA, typeof a);"),
            splits: Some(false),
        },
        Row {
            name: "outer-free-read-in-function",
            source: nested(
                "function probe() { return typeof hasRequiredA; }",
                "1",
                "console.log(probe());",
            ),
            splits: Some(false),
        },
        Row {
            name: "outer-host-names",
            source: nested(
                "",
                "1",
                "console.log(typeof module, typeof require, typeof exports);",
            ),
            splits: Some(false),
        },
        Row {
            name: "outer-free-read-of-other-global",
            source: nested("", "1", "console.log(typeof somethingElse);"),
            splits: Some(true),
        },
        Row {
            name: "shadowed-undefined",
            source: "(function () { \"use strict\"; var hasRequiredA, a = { v: undefined };\n  function requireA() { if (hasRequiredA) return a; hasRequiredA = 1; return a; }\n  var undefined = 5;\n  console.log(requireA().v, undefined); })();\n".to_string(),
            splits: None,
        },
        Row {
            name: "shadowed-infinity",
            source: "(function () { \"use strict\"; var hasRequiredA, a = [-Infinity, NaN];\n  function requireA() { if (hasRequiredA) return a; hasRequiredA = 1; return a; }\n  var Infinity = 5, NaN = 6;\n  console.log(String(requireA()), Infinity, NaN); })();\n".to_string(),
            splits: None,
        },
        Row {
            name: "outer-undefined",
            source: "(function () { \"use strict\";\nvar undefined = 7;\n(function () { \"use strict\"; var hasRequiredA, a = { v: undefined };\n  function requireA() { if (hasRequiredA) return a; hasRequiredA = 1; return a; }\n  console.log(requireA().v); })();\n})();\n".to_string(),
            splits: None,
        },
        Row {
            name: "global-undefined",
            source: "(function () { \"use strict\"; var hasRequiredA, a = { v: undefined, w: -Infinity };\n  function requireA() { if (hasRequiredA) return a; hasRequiredA = 1; return a; }\n  console.log(requireA().v, requireA().w); })();\n".to_string(),
            splits: Some(true),
        },
    ]
}

fn split_tree(out: &Path) -> bool {
    out.join("index.js").exists()
}

fn run_tree(node: &Path, out: &Path) -> Option<String> {
    let entry = match split_tree(out) {
        true => {
            std::fs::write(out.join("package.json"), PACKAGE).expect("package.json");
            out.join("index.js")
        }
        false => out.join("deobfuscated.js"),
    };
    let observed = run_with(node, &[], &entry).expect("run node")?;
    Some(format!("{:?}\n{}", observed.exit, observed.stdout))
}

#[test]
fn outer_wrapper_shapes_under_assumed_modules_run_like_the_bundle() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let mut wrong = Vec::new();
    for row in rows() {
        let input = work.path().join(format!("{}.js", row.name));
        std::fs::write(&input, &row.source).expect("write input");
        let expected = run_with(&node, &[], &input)
            .expect("run node")
            .map(|observed| format!("{:?}\n{}", observed.exit, observed.stdout));
        let out = work.path().join(format!("{}-out", row.name));
        let status = common::bin()
            .args(FLAGS)
            .arg("-f")
            .arg("-o")
            .arg(&out)
            .arg(&input)
            .output()
            .expect("run untwist");
        assert!(
            status.status.success(),
            "{}: untwist failed: {}",
            row.name,
            String::from_utf8_lossy(&status.stderr)
        );
        let observed = run_tree(&node, &out);
        if observed != expected {
            wrong.push(format!(
                "{}: bundle {expected:?}, tree {observed:?}",
                row.name
            ));
        }
        if let Some(splits) = row.splits {
            assert_eq!(split_tree(&out), splits, "{}: split", row.name);
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const LANES: [&[&str]; 7] = [
    &[],
    &["--split-hoisted"],
    &["--split-hoisted", "--split-assume-modules"],
    &["--split-hoisted", "--split-assume-modules", "--unsafe"],
    &[
        "--split-hoisted",
        "--split-assume-modules",
        "--rename",
        "smart",
    ],
    &[
        "--split-hoisted",
        "--split-assume-modules",
        "--rename",
        "mangle",
    ],
    &[
        "--split-hoisted",
        "--split-assume-modules",
        "--rename",
        "simple",
    ],
];

const REVIEW_INPUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assume_review");

fn tree_output(node: &Path, input: &Path, out: &Path, flags: &[&str]) -> Option<String> {
    let status = common::bin()
        .args(flags)
        .arg("-f")
        .arg("-o")
        .arg(out)
        .arg(input)
        .output()
        .expect("run untwist");
    assert!(
        status.status.success(),
        "{}: untwist failed: {}",
        input.display(),
        String::from_utf8_lossy(&status.stderr)
    );
    run_tree(node, out)
}

#[test]
fn review_inputs_run_like_the_bundle_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut inputs: Vec<_> = std::fs::read_dir(REVIEW_INPUTS)
        .expect("review inputs")
        .map(|entry| entry.expect("entry").path())
        .collect();
    inputs.sort();
    assert!(inputs.len() >= 11, "found {} inputs", inputs.len());
    let work = tempfile::tempdir().expect("tempdir");
    let mut wrong = Vec::new();
    for input in &inputs {
        let name = input.file_stem().expect("stem").to_string_lossy();
        let expected = run_with(&node, &[], input)
            .expect("run node")
            .map(|observed| format!("{:?}\n{}", observed.exit, observed.stdout));
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("{name}-{lane}"));
            let observed = tree_output(&node, input, &out, flags);
            if observed != expected {
                wrong.push(format!(
                    "{name} {flags:?}: bundle {expected:?}, tree {observed:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
