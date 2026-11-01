mod common;

use untwist_eval::budget::Budget;
use untwist_eval::decode::{
    all_decoded, decoded_to_expression, Decoded, Evaluator, InterpEvaluator,
};
use untwist_eval::value::JsString;

const PROGRAM: &str = "\
function a() { var h = ['log', 'Hello World!', void 0]; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
b(0x0);
b(0x1);
b(0x2);
b(0x1f4);";

#[test]
fn every_call_site_decodes_in_order() {
    common::with_program(PROGRAM, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::default();
        let values = evaluator.decode(&decoder_program, &calls).expect("decode");
        assert_eq!(values.len(), 4);
        assert!(matches!(&values[0], Decoded::Str(s) if s.to_lossy_string() == "log"));
        assert!(matches!(&values[1], Decoded::Str(s) if s.to_lossy_string() == "Hello World!"));
        assert!(matches!(values[2], Decoded::Undefined));
        assert!(matches!(values[3], Decoded::Undefined));
        assert!(!all_decoded(&values));
    });
}

#[test]
fn a_fully_decoding_program_reports_all_decoded() {
    let source = "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
b(0x0);";
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::default();
        let values = evaluator.decode(&decoder_program, &calls).expect("decode");
        assert!(all_decoded(&values));
    });
}

#[test]
fn a_budget_that_runs_out_bails_rather_than_returning_partial_results() {
    common::with_program(PROGRAM, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::new(Budget::new(1));
        assert_eq!(
            evaluator
                .decode(&decoder_program, &calls)
                .err()
                .map(|bail| bail.to_string()),
            Some("step budget exhausted".to_string())
        );
    });
}

#[test]
fn a_lone_surrogate_result_writes_back_as_an_escaped_literal() {
    let alloc = oxc_allocator::Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&alloc);
    let value = Decoded::Str(JsString::from_units(vec![0xD800, 0x61]));
    let expr = decoded_to_expression(&value, oxc_span::SPAN, &builder).expect("literal");
    let mut codegen =
        oxc_codegen::Codegen::new().with_options(oxc_codegen::CodegenOptions::default());
    codegen.print_expression(&expr);
    assert_eq!(codegen.into_source_text(), "\"\\ud800a\"");
}

#[test]
fn a_call_that_throws_bails_the_whole_decode() {
    let source = "\
function a() { var h = ['log', 'Hello World!', 'oops']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0x0;
    if (f === 0x2) { throw 'bad index'; }
    var i = e[f];
    return i;
  }), b(c, d);
}
b(0x0);
b(0x1);
b(0x2);";
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::default();
        let err = evaluator
            .decode(&decoder_program, &calls)
            .err()
            .map(|bail| bail.to_string());
        assert_eq!(
            err,
            Some("unsupported construct: a decoder call threw".to_string())
        );
    });
}

fn printed(alloc: &oxc_allocator::Allocator, value: &Decoded) -> Option<String> {
    let builder = oxc_ast::builder::AstBuilder::new(alloc);
    let expr = decoded_to_expression(value, oxc_span::SPAN, &builder)?;
    let mut codegen = oxc_codegen::Codegen::new();
    codegen.print_expression(&expr);
    Some(codegen.into_source_text())
}

#[test]
fn every_primitive_has_a_literal_form_and_only_an_object_has_none() {
    let alloc = oxc_allocator::Allocator::default();
    assert_eq!(
        printed(&alloc, &Decoded::Undefined).as_deref(),
        Some("void 0")
    );
    assert_eq!(printed(&alloc, &Decoded::Num(-0.0)).as_deref(), Some("-0"));
    assert_eq!(
        printed(&alloc, &Decoded::Num(f64::NAN)).as_deref(),
        Some("0 / 0")
    );
    assert_eq!(
        printed(&alloc, &Decoded::Num(f64::INFINITY)).as_deref(),
        Some("1 / 0")
    );
    assert_eq!(
        printed(&alloc, &Decoded::Num(f64::NEG_INFINITY)).as_deref(),
        Some("-1 / 0")
    );
    assert_eq!(printed(&alloc, &Decoded::Null).as_deref(), Some("null"));
    assert_eq!(
        printed(&alloc, &Decoded::Bool(true)).as_deref(),
        Some("true")
    );
    assert!(printed(&alloc, &Decoded::Object).is_none());
}
