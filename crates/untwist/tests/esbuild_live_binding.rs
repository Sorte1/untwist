use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const BUNDLE: &str = "var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);
var value;
var greet;
var counter;
var init_value = __esm(() => {
  value = 42;
  greet = function () { return this === undefined || this === globalThis; };
  counter = 0;
});
var bump = () => { counter++; };
var require_main = __commonJS((exports, module) => {
  var before = typeof value;
  init_value();
  bump();
  var shadow = (value) => value + 1;
  module.exports = { before, value, called: greet(), tagged: greet`x`, counter, shadow: shadow(1) };
});
console.log(JSON.stringify(require_main()), value, counter);
bump();
console.log(counter, require_main().counter);
";

const PRINTED: &str = "{\"before\":\"undefined\",\"value\":42,\"called\":true,\"tagged\":true,\"counter\":1,\"shadow\":2} 42 1\n2 1\n";

#[test]
fn a_runtime_binding_assigned_by_a_lazy_initialiser_reads_alike_before_and_after_unpacking() {
    let options = Options {
        deobfuscate: false,
        unminify: false,
        ..unsafe_lane()
    };
    let dir = tempfile::tempdir().expect("temp dir");

    let out = Pipeline::new(options).run(BUNDLE);
    out.write_to(dir.path()).expect("write");

    let runtime = std::fs::read_to_string(dir.path().join("_runtime/esbuild.js"))
        .expect("the bundle is unpacked with a runtime module");
    assert!(runtime.contains("get value()"), "{runtime}");
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
    let unpacked = run(&node, &dir.path().join("index.js"))
        .expect("spawn")
        .expect("terminates");
    assert_eq!(bundle.stdout, PRINTED);
    assert_eq!(unpacked.stdout, PRINTED);
}

fn unsafe_lane() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

const PROTO_BUNDLE: &str = "var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);
var __proto__;
var value;
var init_value = __esm(() => {
  __proto__ = { z: 9 };
  value = 42;
});
var require_main = __commonJS((exports, module) => {
  init_value();
  var o = { __proto__, value };
  module.exports = Object.keys(o).join() + \" \" + o.z + \" \" + value;
});
console.log(require_main());
";

#[test]
fn a_runtime_binding_named_proto_read_through_a_shorthand_property_keeps_an_own_key() {
    let options = Options {
        deobfuscate: false,
        unminify: false,
        ..unsafe_lane()
    };
    let dir = tempfile::tempdir().expect("temp dir");
    Pipeline::new(options)
        .run(PROTO_BUNDLE)
        .write_to(dir.path())
        .expect("write");
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
    assert_eq!(unpacked.stdout, "__proto__,value undefined 42\n");
}
