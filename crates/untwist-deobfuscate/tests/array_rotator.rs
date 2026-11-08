use oxc_allocator::Allocator;
use oxc_ast::ast::{Program, Statement};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::array_rotator::{find_rotator, rotator_unit};
use untwist_deobfuscate::notes;
use untwist_deobfuscate::proof::{rotator_refusal_at, Refusal, RotatorSite};

struct Sites {
    unit: usize,
    found: Vec<RotatorSite>,
}

impl<'a> Visit<'a> for Sites {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if rotator_unit(it) == Some(self.unit) {
            self.found.push(RotatorSite::of(it));
        }
        walk::walk_statement(self, it);
    }
}

fn rotator_refusal(
    program: &Program<'_>,
    scoping: &Scoping,
    unit: usize,
    allowed: &FxHashSet<SymbolId>,
) -> Option<Refusal> {
    let mut sites = Sites {
        unit,
        found: Vec::new(),
    };
    sites.visit_program(program);
    rotator_refusal_at(program, scoping, &sites.found, allowed, None)
}

const WHILE_FORM: &str = "\
(function (c, d) {
  var k = __DECODE_0__, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) / 0x1;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(__STRING_ARRAY__, 0x6);";

const UNARY_FOR_FORM: &str = "\
!(function (c, d) {
  var e = c();
  for (; [];) {
    try {
      var f = parseInt(__DECODE_0__(0xd8)) / 0x1;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) { e['push'](e['shift']()); }
  }
}(__STRING_ARRAY__, 0xc0943));";

const NO_PARSE_INT: &str = "\
(function (c, d) {
  var f = c();
  while (!![]) {
    try {
      if (f[0] === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(__STRING_ARRAY__, 0x6);";

fn found(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    find_rotator(&parsed.program, "__STRING_ARRAY__").is_some()
}

#[test]
fn both_emitted_rotator_forms_are_found() {
    assert!(found(WHILE_FORM));
    assert!(found(UNARY_FOR_FORM));
}

const HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['a', 'b']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c]; }
";

fn refusal(body: &str) -> Option<&'static str> {
    let alloc = Allocator::default();
    let source = format!("{HEADER}{body}");
    let parsed = parse_program(&alloc, &source);
    let semantics = build_scoping(&parsed.program);
    let allowed: FxHashSet<_> = semantics
        .scoping
        .symbol_ids()
        .filter(|s| semantics.scoping.symbol_name(*s).starts_with("__"))
        .collect();
    rotator_refusal(&parsed.program, &semantics.scoping, 0, &allowed).map(|r| r.note)
}

#[test]
fn a_guarded_loop_without_parse_int_is_a_rotator() {
    assert!(found(NO_PARSE_INT));
}

#[test]
fn the_decoder_corpus_rotator_is_found() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/eval/none-rot1-shift0.js"
    ))
    .unwrap();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    assert!(find_rotator(&parsed.program, "a").is_some());
}

#[test]
fn a_loop_whose_try_never_breaks_out_is_not_a_rotator() {
    assert!(!found(
        &NO_PARSE_INT.replace("if (f[0] === d) break;", "if (f[0] === d) f.length;")
    ));
    assert!(!found(&NO_PARSE_INT.replace(
        "if (f[0] === d) break;",
        "if (f[0] === d) { for (;;) { break; } }"
    )));
}

#[test]
fn a_sealed_top_level_rotator_is_trusted() {
    assert_eq!(refusal(NO_PARSE_INT), None);
}

#[test]
fn a_rotator_that_is_not_a_program_statement_is_refused() {
    let nested = format!("function never() {{\n{NO_PARSE_INT}\n}}");
    assert_eq!(refusal(&nested), Some(notes::ROTATOR_NOT_TOP_LEVEL));
    let branch = format!("if (false) {{\n{NO_PARSE_INT}\n}}");
    assert_eq!(refusal(&branch), Some(notes::ROTATOR_NOT_TOP_LEVEL));
}

#[test]
fn a_rotator_beside_its_machinery_in_one_function_body_is_trusted() {
    let body = format!("(function () {{\n{HEADER}{NO_PARSE_INT}\n}})();");
    assert_eq!(refusal_without_header(&body), None);
    let before =
        format!("(function () {{\n{HEADER}console.log(__DECODE_0__(0));\n{NO_PARSE_INT}\n}})();");
    assert_eq!(
        refusal_without_header(&before),
        Some(notes::CODE_BEFORE_ROTATOR)
    );
    let deeper = format!("(function () {{\n{HEADER}function spin() {{\n{NO_PARSE_INT}\n}}\n}})();");
    assert_eq!(
        refusal_without_header(&deeper),
        Some(notes::ROTATOR_NOT_TOP_LEVEL)
    );
}

#[test]
fn code_before_the_rotator_is_refused() {
    let before = format!("console.log(__DECODE_0__(0));\n{NO_PARSE_INT}");
    assert_eq!(refusal(&before), Some(notes::CODE_BEFORE_ROTATOR));
    let inert = format!(
        "var k = 1, s = 'x', t = -k + 2, o = {{ a: [k, 'x', /r/], b: null }};\n{NO_PARSE_INT}"
    );
    assert_eq!(refusal(&inert), None);
}

#[test]
fn inert_blocks_and_expression_statements_before_the_rotator_are_trusted() {
    for prefix in [
        "{}",
        "void 0;",
        "{ { var t = 1; } let u = 'a'; }",
        "'note';",
        "-1;",
    ] {
        assert_eq!(
            refusal(&format!("{prefix}\n{NO_PARSE_INT}")),
            None,
            "{prefix}"
        );
    }
    for prefix in [
        "{ console.log(__DECODE_0__(0)); }",
        "{ function g() {} }",
        "f();",
        "-{ valueOf: function () { return 0; } };",
    ] {
        assert_eq!(
            refusal(&format!("{prefix}\n{NO_PARSE_INT}")),
            Some(notes::CODE_BEFORE_ROTATOR),
            "{prefix}"
        );
    }
}

#[test]
fn only_an_allowlisted_expression_before_the_rotator_runs_no_code() {
    for prefix in [
        "var n = -{ valueOf: function () { return 0; } };",
        "var n = -[{ toString: function () { return '0'; } }];",
        "var o = { get x() { return 1; } };",
        "var o = { f() {} };",
        "var o = { ['k']: 1 };",
        "var a = [...[1]];",
        "var k = 1; k = 2; var t = -k;",
        "var w = window;",
        "var t = typeof x;",
        "var s = `a${1}`;",
    ] {
        let source = format!("{prefix}\n{NO_PARSE_INT}");
        assert_eq!(
            refusal(&source),
            Some(notes::CODE_BEFORE_ROTATOR),
            "{prefix}"
        );
    }
}

#[test]
fn a_function_expression_bound_before_the_rotator_runs_no_code() {
    for prefix in [
        "var f = function () {};",
        "let f = function () {};",
        "const f = function () {}, g = function () {};",
    ] {
        let source = format!("{prefix}\n{NO_PARSE_INT}");
        assert_eq!(refusal(&source), None, "{prefix}");
    }
}

#[test]
fn a_rotator_that_reaches_outside_itself_is_refused() {
    for addition in [
        "seen = f;",
        "c.done = 1;",
        "Object.assign(box, [1]);",
        "function g(seen) { return seen; } seen = 1;",
        "new Date();",
        "this.x;",
    ] {
        let body = NO_PARSE_INT.replace("var f = c();", &format!("var f = c(); {addition}"));
        assert_eq!(
            refusal(&body),
            Some(notes::ROTATOR_NOT_SEALED),
            "{addition}"
        );
    }
}

#[test]
fn a_rotator_that_never_mentions_the_array_function_is_not_ours() {
    assert!(!found(
        &WHILE_FORM.replace("__STRING_ARRAY__", "somethingElse")
    ));
}

#[test]
fn a_rotator_that_may_create_the_array_twice_is_refused() {
    for (from, to) in [
        (
            "(function (c, d) {",
            "(function (c, d, z = __DECODE_0__(1)) {",
        ),
        ("var f = c();", "var x = __DECODE_0__(1); var f = c();"),
        ("var f = c();", "var x = __DECODE_0__(1), f = c();"),
        ("var f = c();", "var k = __STRING_ARRAY__, f = c();"),
        ("var f = c();", "c(); var f = c();"),
        ("var f = c();", "var f = c(), g = c();"),
        ("var f = c();", "var f = c(); function w() { return c(); }"),
        ("(function (c, d) {", "(function (c, ...d) {"),
    ] {
        let body = NO_PARSE_INT.replace(from, to);
        assert_eq!(refusal(&body), Some(notes::ROTATOR_NOT_SEALED), "{to}");
    }
}

#[test]
fn a_rotator_that_creates_the_array_by_name_once_and_first_is_trusted() {
    let direct = "!function () {\n  var k = __DECODE_0__, f = __STRING_ARRAY__();\n  for (;;) try {\n    if (k(0) === 'b') break;\n    f['push'](f['shift']());\n  } catch (h) { f['push'](f['shift']()); }\n}();";
    assert!(found(direct));
    assert_eq!(refusal(direct), None);
    for (from, to) in [
        (
            "f = __STRING_ARRAY__();",
            "g = __STRING_ARRAY__, f = __STRING_ARRAY__();",
        ),
        (
            "var k = __DECODE_0__, f = __STRING_ARRAY__();",
            "k = __DECODE_0__(1); var f = __STRING_ARRAY__();",
        ),
        (
            "f['push'](f['shift']());\n  }",
            "f['push'](f['shift']());\n  __STRING_ARRAY__();\n  }",
        ),
    ] {
        let body = direct.replace(from, to);
        assert_ne!(body, direct, "{to}");
        assert_eq!(refusal(&body), Some(notes::ROTATOR_NOT_SEALED), "{to}");
    }
}

#[test]
fn a_rotator_whose_first_call_creates_the_array_is_trusted() {
    for (from, to) in [
        ("var f = c();", "var k = __DECODE_0__, n = 1, f = c();"),
        ("var f = c();", "var f = c(); var x = __DECODE_0__(1);"),
        (
            "var f = c();",
            "var g; function w(x) { return __DECODE_0__(x); } var f = c();",
        ),
    ] {
        let body = NO_PARSE_INT.replace(from, to);
        assert_eq!(refusal(&body), None, "{to}");
    }
}

fn refusal_without_header(source: &str) -> Option<&'static str> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let allowed: FxHashSet<_> = semantics
        .scoping
        .symbol_ids()
        .filter(|s| semantics.scoping.symbol_name(*s).starts_with("__"))
        .collect();
    rotator_refusal(&parsed.program, &semantics.scoping, 0, &allowed).map(|r| r.note)
}

#[test]
fn machinery_bound_after_the_rotator_is_refused() {
    let array = "function __STRING_ARRAY__() { var h = ['a', 'b']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }\n";
    let declared = "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c]; }\n";
    let var_decoder =
        "var __DECODE_0__ = function (c) { var e = __STRING_ARRAY__(); return e[c]; };\n";
    assert_eq!(
        refusal_without_header(&format!("{array}{NO_PARSE_INT}\n{declared}")),
        None
    );
    assert_eq!(
        refusal_without_header(&format!("{array}{UNARY_FOR_FORM}\n{var_decoder}")),
        Some(notes::MACHINERY_NOT_INITIALISED)
    );
    let var_array = array.replacen(
        "function __STRING_ARRAY__() {",
        "var __STRING_ARRAY__ = function () {",
        1,
    );
    assert_eq!(
        refusal_without_header(&format!("{declared}{NO_PARSE_INT}\n{var_array}")),
        Some(notes::MACHINERY_NOT_INITIALISED)
    );
    let let_decoder = var_decoder.replacen("var ", "let ", 1);
    assert_eq!(
        refusal_without_header(&format!("{array}{UNARY_FOR_FORM}\n{let_decoder}")),
        Some(notes::MACHINERY_NOT_INITIALISED)
    );
}

#[test]
fn an_import_anywhere_in_a_module_runs_before_the_rotator() {
    for import in [
        "import './polyfill.js';",
        "import { x } from './polyfill.js';",
        "export * from './polyfill.js';",
        "export { x } from './polyfill.js';",
    ] {
        let after = format!("{NO_PARSE_INT}\n{import}");
        assert_eq!(
            refusal(&after),
            Some(notes::CODE_BEFORE_ROTATOR),
            "{import}"
        );
    }
    let local_export = format!("{NO_PARSE_INT}\nexport var y = 1;");
    assert_eq!(refusal(&local_export), None);
}
