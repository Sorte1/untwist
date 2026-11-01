use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_span::SPAN;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::bridge::{
    build_request, build_requests, decoded_expression, primitive_value,
};
use untwist_deobfuscate::notes;
use untwist_eval::decode::{all_decoded, Decoded, Evaluator, InterpEvaluator};
use untwist_eval::value::JsString;

const PROGRAM: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'Hello World!', void 0]; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) {
  var e = __STRING_ARRAY__();
  __DECODE_0__ = function (f, g) { f = f - 0x0; var i = e[f]; return i; };
  return __DECODE_0__(c, d);
}
console[__DECODE_0__(0x0)](__DECODE_0__(0x1));
__DECODE_0__(0x2);
__DECODE_0__(0x1f4);";

#[test]
fn the_request_carries_the_machinery_and_every_call_in_source_order() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, PROGRAM);
    let semantics = build_scoping(&parsed.program);
    let request = build_request(&parsed.program, &semantics.scoping).expect("request");
    assert_eq!(request.program.string_array.name, "__STRING_ARRAY__");
    assert_eq!(request.program.decoders.len(), 1);
    assert!(request.program.rotator.is_none());
    assert_eq!(request.calls.len(), 4);
    assert!(request
        .calls
        .windows(2)
        .all(|w| w[0].span.start < w[1].span.start));
}

#[test]
fn decoding_the_request_yields_the_plaintext_and_reports_the_undecodable() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, PROGRAM);
    let semantics = build_scoping(&parsed.program);
    let request = build_request(&parsed.program, &semantics.scoping).expect("request");
    let mut evaluator = InterpEvaluator::default();
    let values = evaluator
        .decode(&request.program, &request.calls)
        .expect("decode");
    assert_eq!(values.len(), 4);
    assert!(matches!(&values[0], Decoded::Str(s) if s.to_lossy_string() == "log"));
    assert!(matches!(&values[1], Decoded::Str(s) if s.to_lossy_string() == "Hello World!"));
    assert!(matches!(values[2], Decoded::Undefined));
    assert!(matches!(values[3], Decoded::Undefined));
    assert!(!all_decoded(&values));
    assert!(primitive_value(values[0].clone()).is_some());
}

#[test]
fn a_primitive_value_widens_and_prints_as_a_literal() {
    let alloc = Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&alloc);
    let value =
        primitive_value(Decoded::Str(JsString::from_units(vec![0x48, 0x69]))).expect("primitive");
    let expr = decoded_expression(&value, SPAN, &builder).expect("string");
    let mut codegen = Codegen::new();
    codegen.print_expression(&expr);
    assert_eq!(codegen.into_source_text(), "\"Hi\"");

    let expr = decoded_expression(&Decoded::Undefined, SPAN, &builder).expect("undefined");
    let mut codegen = Codegen::new();
    codegen.print_expression(&expr);
    assert_eq!(codegen.into_source_text(), "void 0");
}

#[test]
fn a_program_without_a_stamped_decoder_yields_no_request() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "console.log(1);");
    let semantics = build_scoping(&parsed.program);
    assert!(build_request(&parsed.program, &semantics.scoping).is_none());
}

const TWO_DECODERS_OUT_OF_ORDER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'Hello World!']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_1__(c, d) {
  var e = __STRING_ARRAY__();
  __DECODE_1__ = function (f, g) { return e[f - 0x0]; };
  return __DECODE_1__(c, d);
}
function __DECODE_0__(c, d) {
  var e = __STRING_ARRAY__();
  __DECODE_0__ = function (f, g) { return e[f - 0x0]; };
  return __DECODE_0__(c, d);
}
__DECODE_1__(0x1);";

const WITH_ROTATOR: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'Hello World!']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) {
  var e = __STRING_ARRAY__();
  __DECODE_0__ = function (f, g) { return e[f - 0x0]; };
  return __DECODE_0__(c, d);
}
(function (c, d) {
  var k = __DECODE_0__, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) / 0x1;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(__STRING_ARRAY__, 0x6);
__DECODE_0__(0x0);";

const GAPPED_DECODER_INDICES: &str = "\
let __DECODE_0__ = 1;
function __STRING_ARRAY__() { var h = ['log', 'Hello World!']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_1__(c, d) {
  var e = __STRING_ARRAY__();
  __DECODE_1__ = function (f, g) { return e[f - 0x0]; };
  return __DECODE_1__(c, d);
}
__DECODE_1__(0x1);";

#[test]
fn decoders_land_at_the_position_their_name_numbers_them() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, TWO_DECODERS_OUT_OF_ORDER);
    let semantics = build_scoping(&parsed.program);
    let request = build_request(&parsed.program, &semantics.scoping).expect("request");
    assert_eq!(request.program.decoders.len(), 2);
    assert_eq!(request.program.decoders[0].name, "__DECODE_0__");
    assert_eq!(request.program.decoders[1].name, "__DECODE_1__");
    assert_eq!(request.calls.len(), 1);
    assert_eq!(request.calls[0].decoder, 1);
}

#[test]
fn a_rotator_beside_the_array_reaches_the_request() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, WITH_ROTATOR);
    let semantics = build_scoping(&parsed.program);
    let request = build_request(&parsed.program, &semantics.scoping).expect("request");
    assert!(request.program.rotator.is_some());
}

#[test]
fn a_gap_in_the_stamped_decoder_indices_yields_no_request() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, GAPPED_DECODER_INDICES);
    let semantics = build_scoping(&parsed.program);
    assert!(build_request(&parsed.program, &semantics.scoping).is_none());
}

fn printed_value(value: &Decoded) -> Option<String> {
    let alloc = Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&alloc);
    let expr = decoded_expression(value, SPAN, &builder)?;
    let mut codegen = Codegen::new();
    codegen.print_expression(&expr);
    Some(codegen.into_source_text())
}

#[test]
fn every_non_representable_primitive_prints_as_itself() {
    assert_eq!(
        printed_value(&Decoded::Undefined).as_deref(),
        Some("void 0")
    );
    assert_eq!(
        printed_value(&Decoded::Num(f64::NAN)).as_deref(),
        Some("0 / 0")
    );
    assert_eq!(
        printed_value(&Decoded::Num(f64::INFINITY)).as_deref(),
        Some("1 / 0")
    );
    assert_eq!(
        printed_value(&Decoded::Num(f64::NEG_INFINITY)).as_deref(),
        Some("-1 / 0")
    );
    assert_eq!(printed_value(&Decoded::Num(-0.0)).as_deref(), Some("-0"));
    assert_eq!(printed_value(&Decoded::Num(1.5)).as_deref(), Some("1.5"));
}

const ESCAPING: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { f = f - 0x0; return e[f]; }; return __DECODE_0__(c, d); }
__STRING_ARRAY__().reverse();
console[__DECODE_0__(0x0)](__DECODE_0__(0x1));";

fn refusals_of(source: &str) -> (usize, Vec<(&'static str, String)>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let plan = build_requests(&parsed.program, &semantics.scoping);
    let spans = plan
        .refusals
        .iter()
        .map(|r| {
            (
                r.note,
                source[r.span.start as usize..r.span.end as usize].to_string(),
            )
        })
        .collect();
    (plan.requests.len(), spans)
}

#[test]
fn an_array_read_outside_its_machinery_refuses_the_unit() {
    let (requests, refusals) = refusals_of(ESCAPING);
    assert_eq!(requests, 0);
    assert_eq!(
        refusals,
        vec![(notes::STRING_ARRAY_ESCAPES, "__STRING_ARRAY__".to_string())]
    );
}

#[test]
fn without_the_escaping_statement_the_unit_is_decoded() {
    let (requests, refusals) =
        refusals_of(&ESCAPING.replace("__STRING_ARRAY__().reverse();\n", ""));
    assert_eq!(requests, 1);
    assert!(refusals.is_empty());
}

#[test]
fn a_decoder_passed_as_a_value_refuses_the_unit() {
    let source = ESCAPING.replace("__STRING_ARRAY__().reverse();", "[1].map(__DECODE_0__);");
    let (requests, refusals) = refusals_of(&source);
    assert_eq!(requests, 0);
    assert_eq!(
        refusals,
        vec![(notes::DECODER_USED_AS_VALUE, "__DECODE_0__".to_string())]
    );
}

#[test]
fn a_with_statement_refuses_every_unit() {
    let source = ESCAPING.replace("__STRING_ARRAY__().reverse();", "with ({}) {}");
    let (requests, refusals) = refusals_of(&source);
    assert_eq!(requests, 0);
    assert_eq!(
        refusals,
        vec![(notes::WITH_STATEMENT, "with ({}) {}".to_string())]
    );
}
