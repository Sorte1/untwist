use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack_program_with, UnpackOptions};

const SPLIT: UnpackOptions = UnpackOptions {
    split_hoisted: true,
    assume_modules: false,
    conservative: false,
    trust_free_loader: false,
};
const ASSUMED: UnpackOptions = UnpackOptions {
    assume_modules: true,
    ..SPLIT
};

const STAND_IN: &str =
    "var stand = typeof document != \"undefined\" ? document.currentScript : null;\n";
const WRAPPER: &str = "var helper = {};
var hasRequiredHelper;
function requireHelper() {
  if (hasRequiredHelper) return helper;
  hasRequiredHelper = 1;
  helper.read = function () { return typeof hostValue; };
  return helper;
}
";
const USE: &str = "var main = requireHelper();\nconsole.log(main.read(), stand);\n";

fn bundle(extra: &str) -> String {
    format!("(function () {{\n\"use strict\";\n{STAND_IN}{extra}{WRAPPER}{USE}}})();\n")
}

fn wrapped(directive: &str, pre: &str, inner: &str, post: &str) -> String {
    format!("(function () {{\n{directive}{pre}{inner}{post}}})();\n")
}

fn modules(source: &str, options: UnpackOptions) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let bundle = unpack_program_with(&alloc, &parsed.program, options)?;
    Some(
        bundle
            .modules
            .iter()
            .map(|(id, module)| (id.clone(), pretty_codegen(module.program())))
            .collect(),
    )
}

fn ids(source: &str) -> Option<Vec<String>> {
    modules(source, ASSUMED).map(|found| found.into_iter().map(|(id, _)| id).collect())
}

fn text_of(found: &[(String, String)], id: &str) -> String {
    found
        .iter()
        .find(|(known, _)| known == id)
        .map(|(_, text)| text.clone())
        .unwrap_or_else(|| panic!("no module {id}: {found:?}"))
}

#[test]
fn the_current_script_stand_in_keeps_refusing_without_the_flag() {
    assert!(modules(&bundle(""), SPLIT).is_none());
}

#[test]
fn a_commonjs_wrapper_becomes_its_own_module_and_the_stand_in_stays_in_the_entry() {
    let found = modules(&bundle(""), ASSUMED).expect("split");
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["chunk_helper", "index"]);
    let chunk = text_of(&found, "chunk_helper");
    assert!(chunk.contains("function requireHelper()"), "{chunk}");
    assert!(chunk.contains("export { requireHelper }"), "{chunk}");
    let entry = text_of(&found, "index");
    assert!(entry.contains("document.currentScript"), "{entry}");
    assert!(entry.contains("import { requireHelper } from"), "{entry}");
}

#[test]
fn outer_wrapper_bindings_a_wrapper_reads_come_from_the_shell_module() {
    let inner = bundle("");
    let source = wrapped("", "var hostValue;\n", &inner, "");
    let found = modules(&source, ASSUMED).expect("split");
    let shell = text_of(&found, "shell");
    assert!(shell.contains("export var hostValue;"), "{shell}");
    let chunk = text_of(&found, "chunk_helper");
    assert!(
        chunk.contains("import { hostValue } from \"shell\""),
        "{chunk}"
    );
    let entry = text_of(&found, "index");
    assert!(entry.contains("var hostValue;"), "{entry}");
}

#[test]
fn a_strict_outer_wrapper_routes_later_assignments_through_a_setter() {
    let inner = bundle("");
    let source = wrapped(
        "\"use strict\";\n",
        "var hostValue;\n",
        &inner,
        "hostValue = function () {};\n",
    );
    let found = modules(&source, ASSUMED).expect("split");
    let shell = text_of(&found, "shell");
    assert!(shell.contains("function set_hostValue(value)"), "{shell}");
    let entry = text_of(&found, "index");
    assert!(
        entry.contains("set_hostValue(hostValue = function() {})")
            || entry.contains("set_hostValue(hostValue = function () {})"),
        "{entry}"
    );
}

#[test]
fn shapes_that_would_change_what_a_wrapper_module_sees_keep_it_in_the_entry() {
    let inner = bundle("");
    let cases = [
        (
            "an outer binding with a value",
            wrapped("", "var hostValue = 1;\n", &inner, ""),
        ),
        (
            "a bundle write to an outer binding",
            wrapped("", "var hostValue;\n", &bundle("hostValue = 2;\n"), ""),
        ),
        (
            "an outer name that shadows a wrapper export",
            wrapped("", "var hostValue;\nvar requireHelper;\n", &inner, ""),
        ),
        (
            "a holder read before its line",
            bundle("var early = typeof helper;\n"),
        ),
        (
            "a wrapper that reads entry code",
            bundle("").replace("typeof hostValue", "stand"),
        ),
        (
            "a holder written from outside the wrapper",
            bundle("").replace(
                "var main = requireHelper();",
                "helper = {};\nvar main = requireHelper();",
            ),
        ),
    ];
    for (what, source) in cases {
        assert_eq!(ids(&source), None, "{what}:\n{source}");
    }
}

#[test]
fn outer_wrappers_that_cannot_be_kept_as_written_are_refused() {
    let inner = bundle("");
    let cases = [
        (
            "sloppy outer code after the bundle",
            wrapped("", "var hostValue;\n", &inner, "hostValue = 3;\n"),
        ),
        (
            "outer code before the bundle",
            wrapped(
                "\"use strict\";\n",
                "var hostValue;\nhostValue = 3;\n",
                &inner,
                "",
            ),
        ),
        (
            "a direct eval in the outer wrapper",
            wrapped(
                "\"use strict\";\n",
                "var hostValue;\n",
                &inner,
                "eval(\"hostValue = 4\");\n",
            ),
        ),
        ("an outer let", wrapped("", "let hostValue;\n", &inner, "")),
        (
            "outer code that reads a host name",
            wrapped(
                "\"use strict\";\n",
                "var hostValue;\n",
                &inner,
                "console.log(typeof module);\n",
            ),
        ),
        (
            "outer code that reads a wrapper name",
            wrapped(
                "\"use strict\";\n",
                "var hostValue;\n",
                &inner,
                "console.log(typeof requireHelper);\n",
            ),
        ),
        (
            "a function after the bundle that a wrapper reads",
            wrapped("\"use strict\";\n", "", &inner, "function hostValue() {}\n"),
        ),
        (
            "a quiet value that reads a shadowed undefined",
            bundle("").replace(
                "var helper = {};",
                "var helper = { v: undefined };\nvar undefined = 1;",
            ),
        ),
    ];
    for (what, source) in cases {
        let found = ids(&source);
        assert!(found.is_none(), "{what}: {found:?}\n{source}");
    }
}
