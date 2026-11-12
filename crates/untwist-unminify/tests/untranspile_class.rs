use untwist_core::pass::{Note, PassCtor, PassOptions};
use untwist_testing::{transform_report, Allocator};
use untwist_unminify::notes::{UNTRANSPILE_CLASS_PROTOTYPE_OPEN, UNTRANSPILE_UNKNOWN_HELPER};
use untwist_unminify::untranspile::PASS;
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let path = root().join("fixtures").join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn run_with(source: &str, unsafe_transforms: bool, extended: bool) -> (String, Vec<Note>) {
    let alloc = Allocator::default();
    let options = PassOptions {
        unsafe_transforms,
        extended,
        ..PassOptions::default()
    };
    let (code, report) = transform_report(&alloc, source, &ctors(), options);
    (code, report.notes)
}

fn unsafe_run(path: &str) -> String {
    run_with(&read(path), true, true).0
}

fn notes_of(path: &str) -> Vec<String> {
    run_with(&read(path), true, true)
        .1
        .into_iter()
        .filter(|note| note.pass == PASS)
        .map(|note| note.message)
        .collect()
}

fn kept(code: &str, name: &str) -> bool {
    code.contains(&format!("_classCallCheck(this, {name})"))
}

#[test]
fn babel_classes_become_class_expressions_under_unsafe() {
    let code = unsafe_run("untranspile/babel/classes.7.29.js");
    assert!(code.contains("var Base = class Base {"), "{code}");
    assert!(code.contains("  get val() {"), "{code}");
    assert!(code.contains("  static get kind() {"), "{code}");
    assert!(
        code.contains("var Child = class Child extends Base {"),
        "{code}"
    );
    assert!(
        code.contains("    super(v);\n    this.w = w;\n  }"),
        "{code}"
    );
    assert!(
        code.contains("var FromLegacy = class FromLegacy extends Legacy {\n  get() {"),
        "{code}"
    );
    assert!(code.contains("var Anon = class Anon {};"), "{code}");
    assert!(!code.contains("_classCallCheck(this"), "{code}");
}

#[test]
fn the_default_mode_and_conservative_keep_every_class() {
    let source = read("untranspile/babel/classes.7.29.js");
    let (default_mode, _) = run_with(&source, false, true);
    let (conservative, _) = run_with(&source, true, false);
    for code in [default_mode, conservative] {
        assert!(kept(&code, "Base") && kept(&code, "Child"), "{code}");
        assert!(!code.contains("= class "), "{code}");
    }
}

#[test]
fn minified_and_terser_inlined_helpers_are_recognised() {
    let minified = unsafe_run("untranspile/babel/classes.7.29.min.js");
    assert!(minified.contains("var Base = class t {"), "{minified}");
    assert!(
        minified.contains("var Child = class e extends Base {"),
        "{minified}"
    );
    let terser = unsafe_run("untranspile/terser/classes.safari9.js");
    assert!(
        terser.contains("var Child = class Child extends Base {"),
        "{terser}"
    );
    assert!(
        terser.contains("      super(v);\n      this.w = w;"),
        "{terser}"
    );
}

#[test]
fn leading_define_property_calls_become_fields_in_order() {
    let code = unsafe_run("behaviour/class-shapes.js");
    assert!(
        code.contains("  tag = \"base\";\n  empty;\n  constructor(v) {"),
        "{code}"
    );
    assert!(
        code.contains("    var doubled = w * 2;\n    super(v);"),
        "{code}"
    );
    assert!(
        code.contains("_defineProperty(this, \"kid\", doubled);"),
        "{code}"
    );
    let fields = unsafe_run("untranspile/babel/class-fields.7.29.js");
    assert!(fields.contains("  a = 1;\n  b = this.a + 1;"), "{fields}");
    assert!(
        fields.contains("_defineProperty(this, _ref, \"dyn\")"),
        "{fields}"
    );
}

#[test]
fn a_super_call_nested_in_a_statement_is_split_out() {
    let code = unsafe_run("behaviour/class-shapes.js");
    assert!(
        code.contains("super(v);\n    this.extra = this.v + 1;"),
        "{code}"
    );
    assert!(code.contains("super(7);\n    var self = this;"), "{code}");
    assert!(
        code.contains("var Forward = class Forward extends Base {};"),
        "{code}"
    );
    assert!(
        code.contains("super(...args);\n    _defineProperty(this, \"count\", args.length);"),
        "{code}"
    );
}

#[test]
fn esbuild_public_field_calls_become_fields() {
    let code = unsafe_run("untranspile/terser/class-fields.esbuild.ie11.js");
    assert!(
        code.contains("return class _Fields {\n      a = 1;\n      b = this.a + 1;"),
        "{code}"
    );
    assert!(code.contains("__publicField(this, _a, \"dyn\")"), "{code}");
}

#[test]
fn a_field_that_reads_a_constructor_binding_stays_in_the_constructor() {
    let code = unsafe_run("behaviour/class-field-reads-parameter.js");
    assert!(code.contains("  first = 0;\n  constructor(x) {"), "{code}");
    assert!(code.contains("_defineProperty(this, \"x\", x);"), "{code}");
    let ordered = unsafe_run("behaviour/class-field-after-statement.js");
    assert!(
        ordered.contains("_defineProperty(this, \"field\", note(\"field\"))"),
        "{ordered}"
    );
}

#[test]
fn every_guard_keeps_its_class() {
    for (path, name) in [
        ("behaviour/class-sloppy-method.js", "Kind"),
        ("behaviour/class-inner-rebind.js", "Counter"),
        ("behaviour/class-key-reads-class.js", "Keyed"),
        ("behaviour/class-name-capture.js", "Label"),
        ("behaviour/class-this-before-super.js", "Early"),
        ("behaviour/class-this-in-function.js", "Boxed"),
    ] {
        let code = unsafe_run(path);
        assert!(kept(&code, name), "{path}: {code}");
    }
}

#[test]
fn swc_classes_convert_only_where_nothing_reaches_the_prototype() {
    let code = unsafe_run("untranspile/swc/classes.es5.js");
    assert!(code.contains("_class_call_check(this, Base)"), "{code}");
    assert!(code.contains("var Counter = class Counter {"), "{code}");
    assert!(
        code.contains("var FromLegacy = class FromLegacy extends Legacy {"),
        "{code}"
    );
    let notes = notes_of("untranspile/swc/classes.es5.js");
    let open = notes
        .iter()
        .filter(|note| note.as_str() == UNTRANSPILE_CLASS_PROTOTYPE_OPEN)
        .count();
    assert_eq!(open, 1, "{notes:?}");
}

#[test]
fn swc_classes_whose_prototype_nothing_reaches_convert() {
    let code = unsafe_run("untranspile/hand/class-proto-closed.js");
    assert!(code.contains("var Shape = class Shape {"), "{code}");
    assert!(
        code.contains("var Square = class Square extends Shape {"),
        "{code}"
    );
    assert!(!code.contains("_class_call_check(this,"), "{code}");
}

#[test]
fn swc_classes_stay_open_without_unsafe() {
    let (code, _) = run_with(&read("untranspile/hand/class-proto-closed.js"), false, true);
    assert!(code.contains("_class_call_check(this, Shape)"), "{code}");
}

#[test]
fn every_prototype_reach_keeps_its_swc_class() {
    for name in [
        "write",
        "escape",
        "reflect",
        "define",
        "this",
        "this-write",
        "alias",
        "computed",
    ] {
        let path = format!("untranspile/hand/class-proto-{name}.js");
        let code = unsafe_run(&path);
        assert!(
            code.contains("_class_call_check(this, K)"),
            "{path}: {code}"
        );
        let open = notes_of(&path)
            .iter()
            .filter(|note| note.as_str() == UNTRANSPILE_CLASS_PROTOTYPE_OPEN)
            .count();
        assert_eq!(open, 1, "{path}");
    }
}

#[test]
fn a_class_helper_with_another_body_is_noted() {
    let source =
        "function _classCallCheck(a, n) { if (!(a instanceof n)) throw new Error(\"x\"); }\n\
        function Box() { _classCallCheck(this, Box); }\nnew Box();\n";
    let notes: Vec<String> = run_with(source, true, true)
        .1
        .into_iter()
        .filter(|note| note.pass == PASS)
        .map(|note| note.message)
        .collect();
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with(UNTRANSPILE_UNKNOWN_HELPER)),
        "{notes:?}"
    );
}

#[test]
fn native_class_public_fields_move_out_of_the_constructor() {
    let code = unsafe_run("untranspile/esbuild/class-fields.0.28.1-es2015.js");
    assert!(
        code.contains("class _Fields {\n  a = 1;\n  b = this.a + 1;\n  constructor() {"),
        "{code}"
    );
    assert!(code.contains("__publicField(this, _a, \"dyn\")"), "{code}");
    assert!(
        code.contains("class Kid extends Parent {\n  x = 1;\n  init() {"),
        "{code}"
    );
    assert!(!code.contains("super(...arguments)"), "{code}");
}

#[test]
fn native_class_public_fields_stay_without_unsafe() {
    let source = read("untranspile/esbuild/class-fields.0.28.1-es2015.js");
    let (code, _) = run_with(&source, false, true);
    assert!(code.contains("__publicField(this, \"a\", 1)"), "{code}");
}

#[test]
fn native_class_guards_keep_their_public_field_calls() {
    for (path, kept_call) in [
        (
            "behaviour/class-native-param-order.js",
            "__publicField(this, \"x\", log(\"field\"))",
        ),
        (
            "behaviour/class-native-body-first.js",
            "__publicField(this, \"x\", log(\"field\"))",
        ),
    ] {
        let code = unsafe_run(path);
        assert!(code.contains(kept_call), "{path}: {code}");
    }
}

#[test]
fn moved_fields_follow_the_existing_instance_fields() {
    let code = unsafe_run("behaviour/class-native-field-order.js");
    assert!(
        code.contains("  y = log(\"y\");\n  z = log(\"z\");\n  x = log(\"x\");\n}"),
        "{code}"
    );
}

#[test]
fn a_constructor_with_parameters_stays_after_its_fields_move() {
    let code = unsafe_run("behaviour/class-native-ctor-length.js");
    assert!(
        code.contains("  x = 1;\n  constructor(a) {\n    super(...arguments);\n  }"),
        "{code}"
    );
    assert!(code.contains("  y = 2;\n  constructor(a) {}"), "{code}");
}

#[test]
fn tslib_classes_are_kept_with_one_note() {
    let code = unsafe_run("untranspile/ts/classes.es5.js");
    assert!(code.contains("__extends(Child, _super)"), "{code}");
    let notes = notes_of("untranspile/ts/classes.es5.js");
    let open = notes
        .iter()
        .filter(|note| note.as_str() == UNTRANSPILE_CLASS_PROTOTYPE_OPEN)
        .count();
    assert_eq!(open, 1, "{notes:?}");
}

const ESBUILD_FIELD_HELPERS: &str = "var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== \"symbol\" ? key + \"\" : key, value);
";

#[test]
fn an_anonymous_function_moved_into_a_field_keeps_its_empty_name() {
    let source = format!(
        "{ESBUILD_FIELD_HELPERS}class A {{ constructor() {{ __publicField(this, \"x\", 1); __publicField(this, \"f\", () => {{}}); __publicField(this, \"g\", class {{}}); __publicField(this, \"h\", function () {{}}); }} }}\nconst a = new A();\nconsole.log(JSON.stringify([a.x, a.f.name, a.g.name, a.h.name]));\n"
    );
    let (code, _) = run_with(&source, true, true);
    assert!(code.contains("f = (0, () => {});"), "{code}");
    assert!(code.contains("x = 1;"), "{code}");
    untwist_testing::node::assert_node_agrees(&source, &code);
}

#[test]
fn a_parent_called_as_a_function_keeps_every_class_of_the_program() {
    for path in [
        "untranspile/matrix/babel-7.0/ie11/class-extends.js",
        "untranspile/matrix/babel-7.0/ie11/class-fields.js",
    ] {
        let code = unsafe_run(path);
        assert!(!code.contains(" = class "), "{path}: {code}");
    }
    for path in [
        "untranspile/matrix/babel-7.24/ie11/class-extends.js",
        "untranspile/matrix/babel-7.28/ie11/class-extends.js",
    ] {
        let code = unsafe_run(path);
        assert!(
            code.contains("var Dog = class Dog extends Animal {"),
            "{path}: {code}"
        );
    }
}
