mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{Pass, Safety};
use untwist_deobfuscate::removal::{machinery, outstanding_references};

const DECODED: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));";

const UNDECODABLE: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(unknownGlobal));";

const NESTED: &str = "\
(function () {
  function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
  function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
  console[b(0x0)](b(0x1));
})();";

const STAMPED_WITH_ROTATOR: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { return e[f]; }; return __DECODE_0__(c, d); }
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

const STAMPED_WITH_LEFTOVERS: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { return e[f]; }; return __DECODE_0__(c, d); }
typeof __DECODE_0__;
var x = __DECODE_0__;
__DECODE_0__?.(0x1);
f(__STRING_ARRAY__);";

#[test]
fn the_machinery_is_removed_once_every_call_site_is_gone() {
    let (code, _notes) = common::run_passes(DECODED);
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert_eq!(code.trim(), "console[\"log\"](\"Hello World!\");");
}

#[test]
fn a_surviving_reference_blocks_removal_and_is_reported() {
    let (code, notes) = common::run_passes(UNDECODABLE);
    assert!(code.contains("function a()"), "{code}");
    assert!(code.contains("function b(c, d)"), "{code}");
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("decoder machinery kept")),
        "{notes:?}"
    );
}

#[test]
fn an_undefined_result_still_allows_removal() {
    let (code, notes) = common::run_passes(
        "\
function a() { var h = ['log', void 0]; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));",
    );
    assert_eq!(code.trim(), "console[\"log\"](void 0);");
    assert!(notes
        .iter()
        .any(|note| note.contains("untwist:decode_error")));
}

#[test]
fn machinery_nested_inside_a_function_expression_is_removed_too() {
    let (code, _notes) = common::run_passes(NESTED);
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(
        code.contains("console[\"log\"](\"Hello World!\")"),
        "{code}"
    );
}

#[test]
fn a_second_run_over_cleaned_code_changes_nothing_and_says_nothing() {
    let cleaned = common::run_passes(DECODED).0;
    let alloc = Allocator::default();
    let (code, notes) = common::run_ctors(&alloc, &cleaned, &[untwist_deobfuscate::removal::boxed]);
    assert_eq!(code.trim(), cleaned.trim());
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn machinery_reports_the_array_its_decoders_and_the_rotator() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, STAMPED_WITH_ROTATOR);
    let found = machinery(&parsed.program);
    assert!(found.array.is_some());
    assert_eq!(found.decoders.len(), 1);
    assert!(found.rotator.is_some());
    assert!(outstanding_references(&parsed.program).is_empty());
}

#[test]
fn every_reference_outside_the_machinery_is_outstanding() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, STAMPED_WITH_LEFTOVERS);
    let outstanding = outstanding_references(&parsed.program);
    let texts: Vec<&str> = outstanding
        .iter()
        .map(|span| span.source_text(STAMPED_WITH_LEFTOVERS))
        .collect();
    assert_eq!(
        texts,
        vec![
            "__DECODE_0__",
            "__DECODE_0__",
            "__DECODE_0__",
            "__STRING_ARRAY__"
        ]
    );
}

#[test]
fn the_pass_is_named_and_declares_its_safety() {
    let pass: Box<dyn Pass<'_>> = untwist_deobfuscate::removal::boxed();
    assert_eq!(pass.name(), "remove-decoder-machinery");
    assert_eq!(pass.safety(), Safety::Safe);
}

const ARRAY_WITHOUT_DECODERS: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
f(0x1);";

const DECODER_OUTSIDE_A_STATEMENT_LIST: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
if (q) function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { return e[f]; }; return __DECODE_0__(c, d); }";

fn removal_only(source: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    common::run_ctors(&alloc, source, &[untwist_deobfuscate::removal::boxed])
}

#[test]
fn a_string_array_without_any_decoder_is_left_alone() {
    let (code, notes) = removal_only(ARRAY_WITHOUT_DECODERS);
    assert!(code.contains("__STRING_ARRAY__"), "{code}");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_machinery_statement_outside_a_statement_list_gives_up_whole() {
    let (code, notes) = removal_only(DECODER_OUTSIDE_A_STATEMENT_LIST);
    assert!(code.contains("__STRING_ARRAY__"), "{code}");
    assert!(code.contains("__DECODE_0__"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("not a member of a statement list")),
        "{notes:?}"
    );
}
