use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const PRELUDE: &str = "(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c=\"function\"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error(\"Cannot find module '\"+i+\"'\");throw a.code=\"MODULE_NOT_FOUND\",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u=\"function\"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()";

const TWO_VERSIONS_A_SHIM_AND_TWO_ENTRIES: &str = "({\n1:[function(require,module,exports){\nconsole.log(require('./a')(), require('./b')(), require('events').marker);\n},{\"./a\":2,\"./b\":3,\"events\":6}],\n2:[function(require,module,exports){\nvar dep = require('dep');\nmodule.exports = function(){ return \"a uses \" + dep.version; };\n},{\"dep\":4}],\n3:[function(require,module,exports){\nvar dep = require('dep');\nmodule.exports = function(){ return \"b uses \" + dep.version; };\n},{\"dep\":5}],\n4:[function(require,module,exports){\nexports.version = \"1.0\";\n},{}],\n5:[function(require,module,exports){\nexports.version = \"2.0\";\n},{}],\n6:[function(require,module,exports){\nexports.marker = \"bundled events shim\";\n},{}],\n7:[function(require,module,exports){\nconsole.log(\"second entry ran\");\n},{}]\n},{},[1,7]);\n";

const TWO_ROUTES: &str = "({\n1:[function(require,module,exports){\nconsole.log(require('./lib/a'), require('./lib/shared').count);\n},{\"./lib/a\":2,\"./lib/shared\":3}],\n2:[function(require,module,exports){\nmodule.exports = ++require('./shared.js').count;\n},{\"./shared.js\":3}],\n3:[function(require,module,exports){\nexports.count = 0;\n},{}]\n},{},[1]);\n";

const LOCAL_REQUIRE: &str = "({\n1:[function(require,module,exports){\nfunction load(require){ return require('./a'); }\nconsole.log(load(function(name){ return \"local \" + name; }), require('./a'));\n},{\"./a\":2}],\n2:[function(require,module,exports){\nmodule.exports = \"module a\";\n},{}]\n},{},[1]);\n";

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
fn two_versions_of_a_package_a_core_module_shim_and_a_second_entry_load_as_in_the_bundle() {
    let unpacked = unpacked(TWO_VERSIONS_A_SHIM_AND_TWO_ENTRIES, true);
    runs_like_the_bundle(
        &unpacked,
        "a uses 1.0 b uses 2.0 bundled events shim\nsecond entry ran\n",
    );
}

#[test]
fn a_module_reached_by_two_routes_is_loaded_once() {
    let unpacked = unpacked(TWO_ROUTES, true);
    runs_like_the_bundle(&unpacked, "1 1\n");
}

#[test]
fn a_local_function_named_require_keeps_its_argument() {
    let unpacked = unpacked(LOCAL_REQUIRE, true);
    assert!(
        unpacked
            .file("index.js")
            .contains("return require(\"./a\")"),
        "{}",
        unpacked.file("index.js")
    );
    runs_like_the_bundle(&unpacked, "local ./a module a\n");
}

#[test]
fn the_reference_lane_keeps_the_requests_as_written() {
    let unpacked = unpacked(TWO_VERSIONS_A_SHIM_AND_TWO_ENTRIES, false);
    let entry = unpacked.file("index.js");
    assert!(entry.contains("require(\"events\")"), "{entry}");
    assert!(!entry.contains("7.js"), "{entry}");
}

const REQUEST_BY_MODULE_ID: &str = "({\n1:[function(require,module,exports){\nconsole.log(require('./a')(), require('./b')());\n},{\"./a\":2,\"./b\":3}],\n2:[function(require,module,exports){\nmodule.exports = function(){ return \"a uses \" + require(4).version + \" and \" + require(\"4\").version; };\n},{}],\n3:[function(require,module,exports){\nvar dep = require('dep'); module.exports = function(){ return \"b uses \" + dep.version; };\n},{\"dep\":5}],\n4:[function(require,module,exports){\nexports.version = \"1.0\";\n},{}],\n5:[function(require,module,exports){\nexports.version = \"2.0\";\n},{}]\n},{},[1]);\n";

#[test]
fn a_request_outside_the_map_that_names_a_module_id_loads_that_module() {
    let unpacked = unpacked(REQUEST_BY_MODULE_ID, true);
    runs_like_the_bundle(&unpacked, "a uses 1.0 and 1.0 b uses 2.0\n");
}
