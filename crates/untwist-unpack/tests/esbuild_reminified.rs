use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, BundleKind, UnpackResult};

const GETTER: &str = "var g = Object.getOwnPropertyNames;\n";

fn files(result: Option<UnpackResult<'_>>) -> Option<Vec<(String, String)>> {
    let result = result?;
    assert_eq!(result.bundle.kind, BundleKind::Esbuild);
    Some(
        result
            .bundle
            .modules
            .values()
            .map(|module| {
                (
                    module.path.as_str().to_string(),
                    pretty_codegen(module.program()),
                )
            })
            .collect(),
    )
}

fn split(source: &str) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{source}");
    files(unpack(&alloc, &parsed.program, &[]))
}

fn paths(files: &[(String, String)]) -> Vec<&str> {
    files.iter().map(|(path, _)| path.as_str()).collect()
}

fn code<'f>(files: &'f [(String, String)], path: &str) -> &'f str {
    files
        .iter()
        .find(|(at, _)| at == path)
        .map_or_else(|| panic!("no {path}"), |(_, code)| code.as_str())
}

fn helper(catch_param: &str, name: &str) -> String {
    format!(
        "var c = (e, o) => function {name}() {{
  try {{
    if (!o) {{
      (0, e[g(e)[0]])((o = {{ exports: {{}} }}).exports, o);
    }}
    return o.exports;
  }} catch ({catch_param}) {{
    o = 0;
    throw {catch_param};
  }}
}};
"
    )
}

fn two_modules(helper: &str) -> String {
    format!(
        "{GETTER}{helper}var a = c({{ \"a.js\"(e, o) {{ o.exports = 1; }} }});
var b = c({{ \"b.js\"(e, o) {{ o.exports = a() + 1; }} }});
console.log(b());
"
    )
}

#[test]
fn a_catch_parameter_named_like_the_callback_parameter_is_not_shadowing() {
    let files = split(&two_modules(&helper("e", ""))).expect("terser's helper is proven");
    assert_eq!(paths(&files), ["index.js", "a.js", "b.js"]);
    assert_eq!(
        code(&files, "b.js"),
        "module.exports = require(\"./a.js\") + 1;"
    );
}

#[test]
fn a_catch_parameter_named_like_the_module_parameter_is_refused() {
    assert_eq!(split(&two_modules(&helper("o", ""))), None);
}

#[test]
fn a_helper_function_named_like_a_parameter_is_refused() {
    assert_eq!(split(&two_modules(&helper("x", "e"))), None);
}

#[test]
fn a_hoisted_var_named_like_a_factory_parameter_stays_in_the_entry() {
    let source = format!(
        "var e;\nvar o;\n{}e = 1;\no = 2;\n",
        two_modules(&helper("e", ""))
    );
    let files = split(&source).expect("the hoisted vars are entry code");
    assert_eq!(paths(&files), ["index.js", "a.js", "b.js"]);
    let entry = code(&files, "index.js");
    assert!(entry.starts_with("var e;\nvar o;\n"), "{entry}");
    assert!(entry.contains("e = 1;\no = 2;"), "{entry}");
}

#[test]
fn bun_destructured_object_helpers_count_as_effect_free() {
    let source = "var { defineProperty: r, getOwnPropertyNames: p } = Object;
var h = (o, l) => () => (l || o((l = { exports: {} }).exports, l), l.exports);
var a = h((e, m) => { m.exports = 1; });
var b = h((e, m) => { m.exports = a() + 1; });
console.log(b());
";
    let files = split(source).expect("bun --minify keeps the shape");
    assert_eq!(paths(&files), ["index.js", "a.js", "b.js"]);
}

#[test]
fn a_destructure_of_something_else_than_object_stops_the_proof() {
    let source = "var { defineProperty: r } = makeObject();
var h = (o, l) => () => (l || o((l = { exports: {} }).exports, l), l.exports);
var a = h((e, m) => { m.exports = 1; });
var b = h((e, m) => { m.exports = a() + 1; });
console.log(b());
";
    assert_eq!(split(source), None);
}

fn object_keys(extra: &str) -> String {
    format!(
        "{GETTER}{}const m = {{ \"a.js\": function (e, o) {{ o.exports = 1; }} }};
var a = c(m);
var b = c({{ \"b.js\"(e, o) {{ o.exports = a() + 1; }} }});
{extra}console.log(b());
",
        helper("e", "")
    )
}

#[test]
fn a_module_map_moved_into_a_const_before_the_call_is_read_back() {
    let files = split(&object_keys("")).expect("the map is read through its const");
    assert_eq!(paths(&files), ["index.js", "a.js", "b.js"]);
    assert_eq!(code(&files, "a.js"), "module.exports = 1;");
    assert!(!code(&files, "index.js").contains("const m"));
}

#[test]
fn a_module_map_read_twice_is_refused() {
    assert_eq!(split(&object_keys("console.log(m);\n")), None);
}

#[test]
fn a_module_map_written_later_is_refused() {
    let source = object_keys("").replace("const m", "var m") + "m = 0;\n";
    assert_eq!(split(&source), None);
}
