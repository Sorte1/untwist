use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const BUNDLE: &str = "(function (modules) {\n  var cache = {};\n  function r(id) {\n    if (cache[id]) return cache[id].exports;\n    var m = (cache[id] = { exports: {} });\n    modules[id].call(m.exports, m, m.exports, r);\n    return m.exports;\n  }\n  r.hmd = function (m) { return m; };\n  return r(0);\n})([\n  function (e, t, i) {\n    console.log(typeof require.hmd, typeof i.hmd);\n    t.x = i(1);\n    console.log(t.x);\n  },\n  function (e, t) {\n    e.exports = 7;\n  },\n]);\n";

fn unpacked(conservative: bool) -> (tempfile::TempDir, String) {
    let options = Options {
        extended: !conservative,
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    };
    let out = Pipeline::new(options).run(BUNDLE);
    let dir = tempfile::tempdir().expect("temp dir");
    out.write_to(dir.path()).expect("write");
    let entry = std::fs::read_to_string(dir.path().join("index.js")).expect("an entry module");
    (dir, entry)
}

#[test]
fn the_default_rename_keeps_the_hosts_require_apart_from_the_runtimes() {
    let (dir, entry) = unpacked(false);
    assert!(
        entry.contains("typeof require.hmd, typeof __webpack_require__.hmd"),
        "{entry}"
    );
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let unpacked = run(&node, &dir.path().join("index.js"))
        .expect("spawn")
        .expect("terminates");
    assert_eq!(unpacked.stdout, "undefined function\n7\n");
}

#[test]
fn the_conservative_rename_folds_the_runtime_onto_the_hosts_require_and_node_sees_it() {
    let (dir, entry) = unpacked(true);
    assert!(
        entry.contains("typeof require.hmd, typeof require.hmd"),
        "{entry}"
    );
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let source = dir.path().join("bundle.js");
    std::fs::write(&source, BUNDLE).expect("write the bundle");
    let bundle = run(&node, &source).expect("spawn").expect("terminates");
    let folded = run(&node, &dir.path().join("index.js"))
        .expect("spawn")
        .expect("terminates");
    assert_eq!(bundle.stdout, "undefined function\n7\n");
    assert_eq!(folded.stdout, "undefined undefined\n7\n");
}
