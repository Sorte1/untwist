mod common;

use common::parse;
use oxc_allocator::Allocator;
use oxc_span::GetSpan;
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_unpack::closed::Captured;
use untwist_unpack::model::BundleKind;
use untwist_unpack::path::PosixPath;
use untwist_unpack::runtime::{
    first_unclosed_helper, runtime_import, runtime_module, take_statements,
};

const BUNDLE: &str = "var __getOwnPropNames = Object.getOwnPropertyNames;\nvar __commonJS = (cb, mod) => function __require() {\n  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;\n};\nvar require_a = __commonJS({ \"a.js\"(exports, module) { module.exports = 1; } });\nconsole.log(require_a());\n";

#[test]
fn helpers_closed_over_globals_and_each_other_move_to_the_runtime_module_by_taking() {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, BUNDLE);
    let scoping = build_scoping(&program).scoping;
    assert_eq!(
        first_unclosed_helper(&scoping, &program.body, &[0, 1]),
        None
    );
    let spans: Vec<_> = program.body[..2].iter().map(GetSpan::span).collect();
    let taken = take_statements(&alloc, &mut program.body, &[0, 1]);
    assert_eq!(taken.iter().map(GetSpan::span).collect::<Vec<_>>(), spans);
    assert_eq!(program.body.len(), 2);
    let (module, names) =
        runtime_module(&alloc, &program, BundleKind::Esbuild, taken).expect("a runtime module");
    assert_eq!(names, ["__getOwnPropNames", "__commonJS"]);
    assert_eq!(module.path.as_str(), "_runtime/esbuild.js");
    assert_eq!(module.id, "_runtime/esbuild");
    let code = print(module.program(), PrintStyle::Pretty);
    assert!(
        code.trim_end()
            .ends_with("module.exports = {\n  __getOwnPropNames,\n  __commonJS\n};"),
        "{code}"
    );
}

#[test]
fn a_helper_that_reads_a_bundle_binding_stays_in_the_bundle() {
    let alloc = Allocator::default();
    let program = parse(&alloc, BUNDLE);
    let scoping = build_scoping(&program).scoping;
    assert_eq!(
        first_unclosed_helper(&scoping, &program.body, &[1]),
        Some((
            "__commonJS".to_string(),
            Captured::Binding("__getOwnPropNames".to_string())
        ))
    );
    assert_eq!(
        first_unclosed_helper(&scoping, &program.body, &[2]),
        Some((
            "require_a".to_string(),
            Captured::Binding("__commonJS".to_string())
        ))
    );
}

#[test]
fn a_module_imports_the_runtime_by_its_relative_path() {
    let alloc = Allocator::default();
    let from = PosixPath::parse("src/lib/a.js").expect("a path");
    let statement = runtime_import(&alloc, &from, BundleKind::Esbuild, &["__toESM".to_string()])
        .expect("an import");
    let program = parse(&alloc, "x();");
    let mut program = program;
    program.body.insert(0, statement);
    assert_eq!(
        print(&program, PrintStyle::Pretty).trim_end(),
        "const { __toESM } = require(\"../../_runtime/esbuild.js\");\nx();"
    );
}

#[test]
fn the_runtime_notes_name_the_kind_and_the_helper() {
    assert_eq!(
        untwist_unpack::notes::runtime_helper_not_closed(
            "esbuild",
            "__toESM",
            Captured::Arguments.describe()
        ),
        "esbuild helper __toESM cannot move to a runtime module: it reads the factory's own arguments"
    );
}

#[test]
fn a_helper_that_reads_a_name_each_module_has_for_itself_stays_in_the_bundle() {
    for (source, name) in [
        ("var load = () => require(\"./config.json\");", "require"),
        ("var kind = () => typeof module;", "module"),
        ("var own = () => exports.a;", "exports"),
        ("function file() { return __filename; }", "__filename"),
        ("function dir() { return __dirname; }", "__dirname"),
    ] {
        let alloc = Allocator::default();
        let program = parse(&alloc, source);
        let scoping = build_scoping(&program).scoping;

        let found = first_unclosed_helper(&scoping, &program.body, &[0])
            .map(|(_, captured)| captured.describe());

        assert_eq!(
            found,
            Some(format!(
                "it reads {name}, which every module has for itself"
            )),
            "{source}"
        );
    }
}

#[test]
fn a_helper_that_binds_a_host_name_itself_still_moves() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "var load = (require, module) => require(module);");
    let scoping = build_scoping(&program).scoping;

    let found = first_unclosed_helper(&scoping, &program.body, &[0]);

    assert_eq!(found, None);
}
