use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const PRELUDE: &str = "(function (modules) {\n  var installedModules = {};\n  function __webpack_require__(moduleId) {\n    if (installedModules[moduleId]) return installedModules[moduleId].exports;\n    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });\n    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);\n    module.l = true;\n    return module.exports;\n  }\n  return __webpack_require__((__webpack_require__.s = \"./src/index.js\"));\n})";

const NESTED: &str = "({\n  \"./src/index.js\": function (module, exports, __webpack_require__) {\n    console.log(__webpack_require__(\"./a/main.js\"));\n  },\n  \"./a/main.js\": function (module, exports, __webpack_require__) {\n    module.exports = __webpack_require__(\"./node_modules/y.js\");\n  },\n  \"./node_modules/y.js\": function (module, exports) {\n    module.exports = \"root y\";\n  },\n  \"./a/node_modules/y.js\": function (module, exports) {\n    module.exports = \"nested y\";\n  },\n});\n";

const NESTED_PACKAGE: &str = "({\n  \"./src/index.js\": function (module, exports, __webpack_require__) {\n    console.log(__webpack_require__(\"./a/b/main.js\"));\n  },\n  \"./a/b/main.js\": function (module, exports, __webpack_require__) {\n    module.exports = __webpack_require__(\"./node_modules/y/index.js\");\n  },\n  \"./node_modules/y/index.js\": function (module, exports) {\n    module.exports = \"root y\";\n  },\n  \"./a/node_modules/y/index.js\": function (module, exports) {\n    module.exports = \"nested y\";\n  },\n});\n";

const CORE_NAME: &str = "({\n  \"./src/index.js\": function (module, exports, __webpack_require__) {\n    console.log(__webpack_require__(\"./node_modules/events.js\").marker);\n    console.log(__webpack_require__(\"./node_modules/path\").marker);\n  },\n  \"./node_modules/events.js\": function (module, exports) {\n    exports.marker = \"bundled events\";\n  },\n  \"./node_modules/path\": function (module, exports) {\n    exports.marker = \"bundled path\";\n  },\n});\n";

const UNCAPTURED: &str = "({\n  \"./src/index.js\": function (module, exports, __webpack_require__) {\n    console.log(__webpack_require__(\"./a/main.js\"));\n  },\n  \"./a/main.js\": function (module, exports, __webpack_require__) {\n    module.exports = __webpack_require__(\"./node_modules/y.js\");\n  },\n  \"./node_modules/y.js\": function (module, exports) {\n    module.exports = \"root y\";\n  },\n  \"./b/node_modules/y.js\": function (module, exports) {\n    module.exports = \"other y\";\n  },\n});\n";

struct Unpacked {
    dir: tempfile::TempDir,
    bundle: String,
}

impl Unpacked {
    fn file(&self, path: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(path))
            .unwrap_or_else(|error| panic!("{path}: {error}"))
    }
}

fn unpacked(modules: &str, extended: bool) -> Unpacked {
    let bundle = format!("{PRELUDE}{modules}");
    let options = Options {
        extended,
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    };
    let out = Pipeline::new(options).run(&bundle);
    let dir = tempfile::tempdir().expect("temp dir");
    out.write_to(dir.path()).expect("write");
    Unpacked { dir, bundle }
}

fn runs_like_the_bundle(unpacked: &Unpacked, expected: &str) {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let source = unpacked.dir.path().join("bundle.js");
    std::fs::write(&source, &unpacked.bundle).expect("write the bundle");
    let bundle = run(&node, &source).expect("spawn").expect("terminates");
    let tree = run(&node, &unpacked.dir.path().join("index.js"))
        .expect("spawn")
        .expect("terminates");
    assert_eq!(bundle.stdout, expected, "{}", bundle.stderr);
    assert_eq!(tree.stdout, expected, "{}", tree.stderr);
}

#[test]
fn a_nearer_node_modules_file_does_not_capture_a_require_of_the_root_one() {
    let unpacked = unpacked(NESTED, true);
    assert!(
        unpacked
            .file("a/main.js")
            .contains("require(\"../node_modules/y.js\")"),
        "{}",
        unpacked.file("a/main.js")
    );
    runs_like_the_bundle(&unpacked, "root y\n");
}

#[test]
fn a_nearer_node_modules_package_does_not_capture_a_require_of_the_root_one() {
    let unpacked = unpacked(NESTED_PACKAGE, true);
    assert!(
        unpacked
            .file("a/b/main.js")
            .contains("require(\"../../node_modules/y/index.js\")"),
        "{}",
        unpacked.file("a/b/main.js")
    );
    runs_like_the_bundle(&unpacked, "root y\n");
}

#[test]
fn a_bundled_module_named_like_a_node_core_module_is_the_one_loaded() {
    let unpacked = unpacked(CORE_NAME, true);
    runs_like_the_bundle(&unpacked, "bundled events\nbundled path\n");
}

#[test]
fn a_node_modules_file_no_ancestor_can_capture_keeps_the_bare_form() {
    let unpacked = unpacked(UNCAPTURED, true);
    assert!(
        unpacked.file("a/main.js").contains("require(\"y.js\")"),
        "{}",
        unpacked.file("a/main.js")
    );
    runs_like_the_bundle(&unpacked, "root y\n");
}

#[test]
fn the_reference_lane_keeps_the_bare_form() {
    let unpacked = unpacked(NESTED, false);
    assert!(
        unpacked.file("a/main.js").contains("require(\"y.js\")"),
        "{}",
        unpacked.file("a/main.js")
    );
}
