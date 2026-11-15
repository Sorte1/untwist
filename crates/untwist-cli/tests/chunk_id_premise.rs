mod common;

use std::path::{Path, PathBuf};

use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/webpack-variants/w4-async-node"
);
const EXPECTED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/webpack-variants/w4-async-node.stdout"
);
const FILES: [&str; 4] = ["index.js", "1.chunk.js", "2.chunk.js", "3.chunk.js"];
const PATCH: &str = "Object.prototype[1] = 0;\n";
const LANES: [&[&str]; 2] = [&[], &["--unsafe"]];

fn patched_copy(dir: &Path) -> Vec<PathBuf> {
    FILES
        .iter()
        .map(|name| {
            let text = std::fs::read_to_string(Path::new(FIXTURE).join(name)).expect("fixture");
            let text = match *name == FILES[0] {
                true => format!("{PATCH}{text}"),
                false => text,
            };
            let path = dir.join(name);
            std::fs::write(&path, text).expect("write input");
            path
        })
        .collect()
}

#[test]
fn an_inherited_chunk_id_is_a_premise_row_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let inputs_dir = work.path().join("in");
    std::fs::create_dir(&inputs_dir).expect("inputs dir");
    let inputs = patched_copy(&inputs_dir);
    let bundle = run(&node, &inputs[0]).expect("run node").expect("node ran");
    let unpatched = std::fs::read_to_string(EXPECTED).expect("expected stdout");
    assert_ne!(bundle.stdout, unpatched, "the patch must change the bundle");
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("out-{lane}"));
        let status = common::bin()
            .args(*flags)
            .arg("-f")
            .arg("-o")
            .arg(&out)
            .args(&inputs)
            .output()
            .expect("run untwist");
        assert!(status.status.success(), "{flags:?}: untwist failed");
        assert!(
            out.join("bundle.json").exists(),
            "{flags:?}: no tree written"
        );
        let tree = run(&node, &out.join("index.js"))
            .expect("run node")
            .expect("node ran");
        assert_eq!(
            tree.stdout, unpatched,
            "{flags:?}: the tree loads every chunk"
        );
    }
}
