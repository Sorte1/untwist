use untwist_core::pass::{Note, PassCtor, PassOptions};
use untwist_testing::{transform_report, Allocator};
use untwist_unminify::notes::UNTRANSPILE_OBJECT_REST_KEPT;
use untwist_unminify::untranspile::{merge_member_declarations, PASS};
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> String {
    let path = root().join("fixtures/untranspile").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn options(unsafe_transforms: bool) -> PassOptions {
    PassOptions {
        unsafe_transforms,
        extended: true,
        ..PassOptions::default()
    }
}

fn helper_ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn run_report(source: &str, merge_only: bool, unsafe_transforms: bool) -> (String, Vec<Note>) {
    let alloc = Allocator::default();
    let ctors = if merge_only {
        merge_ctors()
    } else {
        helper_ctors()
    };
    let (code, report) = transform_report(&alloc, source, &ctors, options(unsafe_transforms));
    (code, report.notes)
}

fn merge_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![merge_member_declarations::boxed]
}

fn helpers(source: &str, unsafe_transforms: bool) -> String {
    run_report(source, false, unsafe_transforms).0
}

fn merge(source: &str) -> String {
    run_report(source, true, false).0
}

#[test]
fn babel_sliced_to_array_becomes_an_array_pattern_under_unsafe() {
    let code = helpers(&fixture("babel/destructure.7.29.js"), true);
    assert!(code.contains("var [m, n] = _Set;"), "{code}");
    assert!(
        code.contains("var [key, value] = _Object$entries[_i];"),
        "{code}"
    );
    assert!(code.contains("var [c1, c2] = str;"), "{code}");
    assert!(code.contains("_slicedToArray(_ref5, 2)"), "{code}");
}

#[test]
fn swc_and_typescript_destructuring_helpers_become_array_patterns() {
    let swc = helpers(&fixture("swc/destructure.es5.js"), true);
    assert!(swc.contains("var [c1, c2] = str;"), "{swc}");
    assert!(!swc.contains("_sliced_to_array(str"), "{swc}");
    let ts = helpers(&fixture("ts/destructure.es5-iter.js"), true);
    assert!(ts.contains("var [m, n] = new Set("), "{ts}");
    assert!(ts.contains("var [c1, c2] = str;"), "{ts}");
}

#[test]
fn the_default_mode_rewrites_only_literal_sources() {
    let code = helpers(&fixture("hand/destructure-temp.js"), false);
    assert!(code.contains("var [one, , three] = [1, 2, 3];"), "{code}");
    assert!(code.contains("var [x, y] = \"xy\";"), "{code}");
    let babel = helpers(&fixture("babel/destructure.7.29.js"), false);
    assert!(babel.contains("_slicedToArray(_Set, 2)"), "{babel}");
}

#[test]
fn a_temporary_read_outside_the_run_keeps_the_helper_call() {
    let code = helpers(&fixture("hand/destructure-temp.js"), true);
    assert!(
        code.contains("_slicedToArray(new Set([\"a\", \"b\", \"c\"]), 2)"),
        "{code}"
    );
}

#[test]
fn object_rest_helpers_are_kept_with_one_note_per_kind() {
    let (code, notes) = run_report(&fixture("babel/rest.7.29.js"), false, true);
    assert!(
        code.contains("_objectWithoutProperties(src, [\"a\"])"),
        "{code}"
    );
    let kept: Vec<&str> = notes
        .iter()
        .filter(|note| note.pass == PASS && note.message.starts_with(UNTRANSPILE_OBJECT_REST_KEPT))
        .map(|note| note.message.as_str())
        .collect();
    assert_eq!(kept.len(), 1, "{notes:?}");
    assert!(kept[0].ends_with("ObjectWithoutProperties"), "{kept:?}");
}

#[test]
fn consecutive_member_reads_of_a_constant_merge() {
    let code = merge("function f(e) { var a = e.x; var b = e[\"y\"]; var x = e.x; g(a, b, x); }");
    assert!(code.contains("var { x: a, \"y\": b, x } = e;"), "{code}");
}

#[test]
fn a_written_base_does_not_merge() {
    let code = merge("var e = o; function f() { var a = e.x; var b = e.y; e = p; }");
    assert!(code.contains("var a = e.x;\n"), "{code}");
}

#[test]
fn a_target_used_as_the_next_base_ends_the_run() {
    let code = merge("function f(e) { var a = e.x; var b = a.y; var c = a.z; g(b, c); }");
    assert!(code.contains("var a = e.x;"), "{code}");
    assert!(code.contains("var { y: b, z: c } = a;"), "{code}");
}

#[test]
fn a_parameter_of_a_function_that_reads_arguments_does_not_merge() {
    let code = merge("function f(e) { var t = arguments; var a = e.x; var b = e.y; g(a, b, t); }");
    assert!(code.contains("var a = e.x;"), "{code}");
}

#[test]
fn different_kinds_and_repeated_targets_do_not_merge() {
    let code =
        merge("function f(e) { const a = e.x; let b = e.y; b = 1; var c = e.z; var c = e.w; }");
    assert!(code.contains("const a = e.x;"), "{code}");
    assert!(code.contains("var c = e.z;"), "{code}");
}

#[test]
fn the_merge_pass_is_named_and_extended() {
    let pass = merge_member_declarations::boxed();
    assert_eq!(pass.name(), "merge-member-declarations");
    assert!(pass.requires_extended());
}

#[test]
fn a_base_declared_at_the_top_of_a_script_does_not_merge() {
    let code = merge("var e = { get x() { globalThis.e = { y: 2 }; return 1; }, y: 1 };\nvar a = e.x;\nvar b = e.y;\nconsole.log(a, b);");
    assert!(code.contains("var a = e.x;\nvar b = e.y;"), "{code}");
}

#[test]
fn an_imported_base_does_not_merge() {
    let code = merge("import e from \"m\";\nvar a = e.x;\nvar b = e.y;\nexport { a, b };");
    assert!(code.contains("var a = e.x;\nvar b = e.y;"), "{code}");
}

#[test]
fn a_base_declared_at_the_top_of_a_module_still_merges() {
    let code = merge("const e = g();\nvar a = e.x;\nvar b = e.y;\nexport { a, b };");
    assert!(code.contains("var { x: a, y: b } = e;"), "{code}");
}
