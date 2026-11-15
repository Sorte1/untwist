use oxc_allocator::Allocator;
use untwist_core::parse::{
    early_errors, has_error, parse_failure, parse_named_module, parse_named_script, parse_program,
    strip_bookmarklet, ParseFailure,
};
use untwist_core::semantic::build_scoping;

#[test]
fn parses_module_syntax() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "import a from 'a'; export default a;");
    assert!(p.diagnostics.is_empty());
    assert!(p.source_type.is_module());
    assert_eq!(p.program.body.len(), 2);
}

#[test]
fn falls_back_to_commonjs_for_module_only_reserved_word() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "var await = 1; return 1;");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert!(p.source_type.is_commonjs());
}

#[test]
fn parses_jsx() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "const x = <div className=\"a\">{y}</div>;");
    assert!(p.diagnostics.is_empty());
}

#[test]
fn recovers_from_syntax_errors_with_diagnostics() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "if (true) const a = 1;\nlet b = 2;");
    assert!(!p.diagnostics.is_empty());
    assert!(!p.program.body.is_empty());
}

#[test]
fn strips_bookmarklet_prefix_and_decodes() {
    assert_eq!(
        strip_bookmarklet("javascript:alert(%22hi%22)"),
        "alert(\"hi\")"
    );
    assert_eq!(strip_bookmarklet("javascript:v%F"), "v%F");
    assert_eq!(strip_bookmarklet("plain()"), "plain()");
}

#[test]
fn a_file_without_module_syntax_is_a_classic_script() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "var a = 1; function f() { return this; }");
    assert!(p.diagnostics.is_empty());
    assert!(!p.source_type.is_module());
    assert!(!p.program.source_type.is_module());
}

#[test]
fn import_meta_and_top_level_await_make_a_module() {
    for source in [
        "console.log(import.meta.url);",
        "function f() { return import.meta; }",
        "await Promise.resolve(1);",
        "for await (const x of xs) {}",
        "export {};",
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        assert!(p.diagnostics.is_empty(), "{source}: {:?}", p.diagnostics);
        assert!(p.source_type.is_module(), "{source}");
        assert!(p.program.source_type.is_module(), "{source}");
    }
}

#[test]
fn await_inside_an_async_function_does_not_make_a_module() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "async function f() { await g(); for await (const x of y) {} }",
    );
    assert!(!p.source_type.is_module());
}

#[test]
fn a_sloppy_script_hoists_a_block_function_to_its_function() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "function outer() { { function atob(s) { return s; } } return atob(\"x\"); }",
    );
    let sem = build_scoping(&p.program);
    assert!(!sem
        .scoping
        .root_unresolved_references()
        .contains_key("atob"));
}

#[test]
fn a_strict_script_keeps_a_block_function_in_its_block() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "\"use strict\"; function outer() { { function atob(s) { return s; } } return atob(\"x\"); }",
    );
    let sem = build_scoping(&p.program);
    assert!(sem
        .scoping
        .root_unresolved_references()
        .contains_key("atob"));
}

fn reprinted(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    untwist_core::print::pretty_codegen(&parsed.program)
}

#[test]
fn a_parenthesised_name_keeps_an_anonymous_function_unnamed() {
    for (source, kept) in [
        ("(f) = function () {};", "f = (0, function() {});"),
        ("((C)) = class {};", "C = (0, class {});"),
        ("( f /* c */ ) /* d */ ??= () => 1;", "f ??= (0, () => 1);"),
        (
            "[(a) = function () {}] = [];",
            "[a = (0, function() {})] = [];",
        ),
        ("({ x: (b) = class {} } = {});", "b = (0, class {})"),
        (
            "g((f) = function () {}, (h) == 1);",
            "f = (0, function() {})",
        ),
    ] {
        assert!(
            reprinted(source).contains(kept),
            "{source}: {}",
            reprinted(source)
        );
    }
}

#[test]
fn a_bare_or_compound_target_is_left_as_written() {
    for (source, kept) in [
        ("f = function () {};", "f = function() {};"),
        ("(f) += function () {};", "f += function() {};"),
        ("(f) = function named() {};", "f = function named() {};"),
        ("(o.f) = function () {};", "o.f = function() {};"),
        ("if (a) f = function () {};", "f = function() {};"),
        ("(f = function () {});", "f = function() {};"),
    ] {
        assert!(
            reprinted(source).contains(kept),
            "{source}: {}",
            reprinted(source)
        );
    }
}

#[test]
fn an_html_open_comment_in_a_script_stays_a_comment() {
    let alloc = Allocator::default();
    let source = "var a = 5, b = 3;\nvar x = a<!--b\n;\nlog(x, b);";
    let p = parse_program(&alloc, source);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert!(!p.source_type.is_module());
    assert_eq!(p.program.body.len(), 3);
}

#[test]
fn module_syntax_and_top_level_await_still_parse_as_a_module() {
    for source in [
        "export const a = 1;",
        "import 'a';",
        "await Promise.resolve(1);",
        "console.log(import.meta.url);",
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        assert!(p.diagnostics.is_empty(), "{source}: {:?}", p.diagnostics);
        assert!(p.source_type.is_module(), "{source}");
    }
}

const AWAIT_CALL: &str = "var f = \"outer\";\nfunction t(){ { function f(){ return 1 } } return typeof f; }\nconsole.log(await (async () => t())());";

#[test]
fn a_script_that_parses_with_an_await_call_stays_a_script_as_in_node() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, AWAIT_CALL);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert!(!p.source_type.is_module());
}

#[test]
fn a_named_module_is_parsed_as_a_module_from_the_start() {
    let alloc = Allocator::default();
    let p = parse_named_module(&alloc, AWAIT_CALL);
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert!(p.source_type.is_module());
}

#[test]
fn a_named_module_that_only_parses_as_a_script_reports_the_module_error_as_node_does() {
    let alloc = Allocator::default();
    let p = parse_named_module(&alloc, "var await = 1;");
    assert!(has_error(&p.diagnostics));
    assert!(p.source_type.is_module());
}

#[test]
fn a_top_level_return_is_an_error_in_a_module_and_allowed_in_a_script() {
    let alloc = Allocator::default();
    assert!(has_error(
        &parse_named_module(&alloc, "return;").diagnostics
    ));
    assert!(!has_error(&parse_program(&alloc, "return;").diagnostics));
}

#[test]
fn a_lexical_declaration_of_a_commonjs_wrapper_name_takes_the_module_goal_as_in_node() {
    for source in [
        "const require = 1;",
        "let exports;",
        "let module = 2;",
        "const __filename = 1;",
        "let __dirname;",
        "class module {}",
        "class exports {}",
        "let [exports] = [1];",
        "let { a: require } = {};",
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        assert!(p.diagnostics.is_empty(), "{source}: {:?}", p.diagnostics);
        assert!(p.source_type.is_module(), "{source}");
    }
}

#[test]
fn other_declarations_of_a_wrapper_name_stay_commonjs_as_in_node() {
    for source in [
        "var module;",
        "function __filename() {}",
        "{ let module; }",
        "let arguments;",
        "function f() { let exports; }",
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        assert!(p.diagnostics.is_empty(), "{source}: {:?}", p.diagnostics);
        assert!(!p.source_type.is_module(), "{source}");
    }
}

#[test]
fn a_redeclared_wrapper_name_that_does_not_parse_as_a_module_is_an_error() {
    for source in [
        "const exports = 0; with ({}) {}",
        "const exports = 0; return;",
        "const exports = 0; var a = 010;",
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        let rejected = has_error(&p.diagnostics) || !early_errors(&p.program).is_clean();
        assert!(rejected, "{source}");
    }
}

#[test]
fn early_errors_are_reported_for_strict_code_and_not_for_sloppy_code() {
    for (source, rejected) in [
        ("\"use strict\"; var a = 010;", true),
        ("var a = 010;", false),
        ("\"use strict\"; with ({}) {}", true),
        ("with ({}) {}", false),
        ("function f(a, a) { \"use strict\"; }", true),
        ("function f(a, a) {}", false),
        ("let a; let a;", true),
        ("{ function f() {} function f() {} }", false),
        ("\"use strict\"; { function f() {} function f() {} }", true),
        ("class A { m() { return 010; } }", true),
        ("class A { x = 010; }", false),
        ("class A { static { var o = { 010: 1 }; } }", false),
        ("class A { x = () => 010; }", true),
        ("\"use strict\"; class A { x = 010; }", true),
        ("{ function g() {} } { var g; }", false),
        ("{ function g() {} } let g;", false),
        ("{ function g() {} var g; }", true),
        ("{ function g() {} } let g; let g;", true),
        ("try {} catch (e) { function e() {} }", true),
        ("if (0) { /(/; }", true),
        ("/(?<a>x)|(?<a>y)/;", true),
        ("/(?i:a)/;", true),
        ("var a; [(a = 1)] = [];", true),
        ("var a; [(a)] = [1];", false),
        ("((a)) => 1;", true),
        ("if (0) { using x = null; }", true),
        ("var z = 1 || @dec class {};", true),
        ("class A { accessor x = 1; }", true),
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        assert!(p.diagnostics.is_empty(), "{source}: {:?}", p.diagnostics);
        assert_eq!(!early_errors(&p.program).is_clean(), rejected, "{source}");
    }
}

#[test]
fn an_html_comment_opener_in_module_code_is_an_early_error_as_in_node() {
    let alloc = Allocator::default();
    let module = parse_named_module(&alloc, "var a = 1, b = 5; a<!--b\n;");
    assert!(module.diagnostics.is_empty());
    assert!(!early_errors(&module.program).is_clean());
    let script = parse_program(&alloc, "var a = 1, b = 5; a<!--b\n;");
    assert!(early_errors(&script.program).is_clean());
}

#[test]
fn a_script_that_parses_only_as_a_module_without_module_syntax_keeps_its_script_error() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var a = 1, b = 5; var c = (a<!--b);");
    assert!(has_error(&parsed.diagnostics));
    assert!(!parsed.program.source_type.is_module());
}

#[test]
fn a_cjs_input_never_takes_the_module_goal() {
    let alloc = Allocator::default();
    for source in [
        "const exports = 0;",
        "await 0;",
        "import.meta;",
        "export {};",
    ] {
        let parsed = parse_named_script(&alloc, source);
        assert!(has_error(&parsed.diagnostics), "{source}");
        assert!(!parsed.program.source_type.is_module(), "{source}");
    }
    let parsed = parse_named_script(&alloc, "var exports = 1;");
    assert!(!has_error(&parsed.diagnostics));
}

#[test]
fn a_parse_failure_node_runs_is_labelled_a_parser_limit() {
    for (source, failure) in [
        ("function f() {} if (0) f() = 1;", ParseFailure::Unsupported),
        (
            "function f() {} if (0) for (f() in {}) ;",
            ParseFailure::Unsupported,
        ),
        ("var yield; for (yield of [1]) ;", ParseFailure::Unsupported),
        (
            "function f() {} if (0) [f()] = [];",
            ParseFailure::SyntaxError,
        ),
        (
            "function f() {} if (0) f() &&= 1;",
            ParseFailure::SyntaxError,
        ),
        (
            "function f() {} if (0) new f() = 1;",
            ParseFailure::SyntaxError,
        ),
        ("var a = 1 +;", ParseFailure::SyntaxError),
    ] {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        assert!(has_error(&parsed.diagnostics), "{source}");
        assert_eq!(
            parse_failure(source, &parsed.diagnostics),
            failure,
            "{source}"
        );
    }
}

#[test]
fn a_function_named_await_in_a_script_stays_a_script() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "function await(x) { return x; }\nvar y = 1<!--y\n;\nawait(2);",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert!(!p.source_type.is_module());
}
