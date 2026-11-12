use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program};
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;
use std::collections::HashSet;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, run_to_fixpoint, Cx, Pass, PassGroup, PassOptions, RunReport};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::calls::{collect_calls, normalize_calls};

const HEAD: &str = "function __STRING_ARRAY__() {\n  var h = ['log'];\n  __STRING_ARRAY__ = function () { return h; };\n  return __STRING_ARRAY__();\n}\nfunction __DECODE_0__(a, b) {\n  var e = __STRING_ARRAY__();\n  return e[a];\n}\n";

#[derive(Default)]
struct NormalizeOnly;

impl<'a> Pass<'a> for NormalizeOnly {
    fn name(&self) -> &'static str {
        "normalize-calls-under-test"
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let changed = normalize_calls(program, cx);
        cx.changed_by(changed);
    }
}

fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(NormalizeOnly)
}

fn normalize(body: &str) -> String {
    let source = format!("{HEAD}{body}");
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(&[boxed], PassOptions::default());
    let _ = run_once(&alloc, &mut parsed.program, sem, &mut group);
    print(&parsed.program, PrintStyle::Compact)
}

fn normalize_to_fixpoint(body: &str) -> (String, RunReport) {
    let source = format!("{HEAD}{body}");
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(&[boxed], PassOptions::default());
    let (_, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    (print(&parsed.program, PrintStyle::Compact), report)
}

fn normalize_and_collect(body: &str) -> (String, Vec<Span>) {
    let source = format!("{HEAD}{body}");
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(&[boxed], PassOptions::default());
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    let spans: Vec<Span> = collect_calls(&parsed.program)
        .iter()
        .map(|call| call.span)
        .collect();
    (print(&parsed.program, PrintStyle::Compact), spans)
}

#[test]
fn a_ternary_argument_stays_whole_and_each_branch_is_collected() {
    let (out, spans) = normalize_and_collect("console.log(__DECODE_0__(p ? 0xc7 : 0xc8));");
    assert!(
        out.contains("console.log(__DECODE_0__(p?199:200));"),
        "{out}"
    );
    assert_eq!(spans.len(), 2, "{out}");
}

#[test]
fn external_variables_are_inlined_into_the_call() {
    let out =
        normalize(
        "function t() {\nlet x;\nconst y = 0xd3;\nx = 0xd6;\nconsole[__DECODE_0__(x)](__DECODE_0__(y));\n}",
    );
    assert!(out.contains("__DECODE_0__(214)"), "{out}");
    assert!(out.contains("__DECODE_0__(211)"), "{out}");
    assert!(!out.contains("x="), "{out}");
}

#[test]
fn a_bare_decoder_reference_statement_is_removed() {
    let out = normalize("__DECODE_0__;\nconsole.log(1);");
    assert!(!out.contains("__DECODE_0__;"), "{out}");
}

#[test]
fn calls_are_collected_in_source_order_and_only_when_literal() {
    let source =
        format!("{HEAD}__DECODE_0__(1);\n__DECODE_0__(unknown);\n__DECODE_0__(3 - 1, 'k');");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let calls = collect_calls(&parsed.program);
    assert_eq!(calls.len(), 2);
    assert!(calls[0].span.start < calls[1].span.start);
    assert_eq!(calls[0].decoder, 0);
    assert_eq!(calls[1].arguments.len(), 2);
    let Expression::NumericLiteral(one) = calls[0].arguments[0] else {
        panic!("the first call's argument should be a numeric literal");
    };
    assert_eq!(one.value, 1.0);
    let Expression::BinaryExpression(difference) = calls[1].arguments[0] else {
        panic!("the second call's first argument should be a binary expression");
    };
    assert_eq!(difference.operator, BinaryOperator::Subtraction);
    let Expression::StringLiteral(key) = calls[1].arguments[1] else {
        panic!("the second call's second argument should be a string literal");
    };
    assert_eq!(key.value.as_str(), Some("k"));
}

#[test]
fn spread_and_empty_call_sites_are_not_collected_but_an_optional_decoder_call_is() {
    let source = format!(
        "{HEAD}__DECODE_0__(...args);\n__DECODE_0__?.(1);\n__DECODE_0__();\n__DECODE_0__(2);"
    );
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let calls = collect_calls(&parsed.program);
    let texts: Vec<&str> = calls
        .iter()
        .map(|call| &source[call.span.start as usize..call.span.end as usize])
        .collect();
    assert_eq!(texts, ["__DECODE_0__?.(1)", "__DECODE_0__(2)"], "{texts:?}");
}

#[test]
fn a_second_normalisation_pass_changes_nothing() {
    let (out, report) = normalize_to_fixpoint(
        "function t() {\nlet x;\nx = 0xd6;\n__DECODE_0__;\nconsole.log(__DECODE_0__(x), __DECODE_0__(q ? 0xc7 : 0xc8));\n}",
    );
    assert!(report.converged(), "{:?}", report.stop);
    assert_eq!(report.iterations, 2, "{out}");
    assert!(out.contains("__DECODE_0__(214)"), "{out}");
    assert!(out.contains("__DECODE_0__(q?199:200)"), "{out}");
}

#[test]
fn a_declaration_with_several_declarators_is_not_removed() {
    let out = normalize("const y = 0xd3, z = side();\nconsole.log(__DECODE_0__(y), z);");
    assert!(out.contains("__DECODE_0__(211)"), "{out}");
    assert!(out.contains("z=side()"), "{out}");
}

#[test]
fn an_assignment_sharing_a_statement_is_not_removed() {
    let out =
        normalize("function t() {\nlet x;\nx = 0xd6, side();\nconsole.log(__DECODE_0__(x));\n}");
    assert!(out.contains("__DECODE_0__(214)"), "{out}");
    assert!(out.contains("side()"), "{out}");
}

#[test]
fn a_scoped_ternary_argument_is_left_alone() {
    let out = normalize("console.log(__DECODE_0__(p ? function () {} : 0xc8));");
    assert!(out.contains("__DECODE_0__(p?function"), "{out}");
}

#[test]
fn a_nested_ternary_argument_collects_every_branch() {
    let (out, spans) =
        normalize_and_collect("console.log(__DECODE_0__(p ? (q ? 0x1 : 0x2) : 0x3));");
    assert!(out.contains("__DECODE_0__(p?q?1:2:3)"), "{out}");
    assert_eq!(spans.len(), 3, "{out}");
    let starts: HashSet<u32> = spans.iter().map(|span| span.start).collect();
    assert_eq!(starts.len(), 3, "{spans:?}");
}

#[test]
fn a_forward_reference_blocks_the_assignment_removal() {
    let (out, report) =
        normalize_to_fixpoint("let x;\nfunction f() { return __DECODE_0__(x); }\nx = 0xd6;\nf();");
    assert!(out.contains("let x;"), "{out}");
    assert!(out.contains("x=214;"), "{out}");
    assert!(out.contains("__DECODE_0__(x)"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn an_assignment_outside_a_statement_list_leaves_the_site_alone() {
    let (out, report) =
        normalize_to_fixpoint("let x;\nif (c) x = 0xd6;\nconsole.log(__DECODE_0__(x));");
    assert!(out.contains("let x;"), "{out}");
    assert!(out.contains("if(c)x=214;"), "{out}");
    assert!(out.contains("__DECODE_0__(x)"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn an_assignment_in_a_nested_block_leaves_the_site_alone() {
    let (out, report) =
        normalize_to_fixpoint("let x;\nif (c) { x = 0xd6; }\nconsole.log(__DECODE_0__(x));");
    assert!(out.contains("let x;"), "{out}");
    assert!(out.contains("if(c){x=214}"), "{out}");
    assert!(out.contains("__DECODE_0__(x)"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_non_literal_write_refuses_the_whole_symbol() {
    let (out, report) =
        normalize_to_fixpoint("let x;\nx = 0xd6;\nx = g();\nconsole.log(__DECODE_0__(x));");
    assert!(out.contains("let x;"), "{out}");
    assert!(out.contains("x=214;"), "{out}");
    assert!(out.contains("x=g();"), "{out}");
    assert!(out.contains("__DECODE_0__(x)"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_decoder_call_branch_leaves_the_ternary_unsplit_this_iteration() {
    let out = normalize("console.log(__DECODE_0__(p ? __DECODE_0__(0x1) : 0xc8));");
    assert!(out.contains("__DECODE_0__(p?__DECODE_0__(1):200)"), "{out}");
}

#[test]
fn an_identifier_branch_is_inlined_by_the_fixpoint() {
    let (out, report) =
        normalize_to_fixpoint("const y = 0xd3;\nconsole.log(__DECODE_0__(p ? y : 0xc8));");
    assert!(report.converged(), "{:?}", report.stop);
    assert!(out.contains("__DECODE_0__(p?211:200)"), "{out}");
}

#[test]
fn a_constant_read_before_its_initialiser_runs_is_left_alone() {
    let (out, report) = normalize_to_fixpoint(
        "function g() { return __DECODE_0__(k); }\nconsole.log(g());\nvar k = 0x1;",
    );
    assert!(out.contains("__DECODE_0__(k)"), "{out}");
    assert!(out.contains("var k=1;"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_constant_read_inside_a_function_declared_after_an_inert_prefix_is_inlined() {
    let (out, _report) = normalize_to_fixpoint(
        "var k = 0x1;\nfunction g() { return __DECODE_0__(k); }\nconsole.log(g());",
    );
    assert!(out.contains("__DECODE_0__(1)"), "{out}");
    assert!(!out.contains("var k"), "{out}");
}

#[test]
fn a_constant_read_in_an_earlier_statement_is_left_alone() {
    let (out, report) = normalize_to_fixpoint("console.log(__DECODE_0__(k));\nvar k = 0x1;");
    assert!(out.contains("__DECODE_0__(k)"), "{out}");
    assert!(out.contains("var k=1;"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_variable_assigned_twice_around_a_function_is_left_alone() {
    let (out, report) = normalize_to_fixpoint(
        "var k;\nk = 0x1;\nfunction g() { return __DECODE_0__(k); }\nk = 0x0;\nconsole.log(g());",
    );
    assert!(out.contains("var k;"), "{out}");
    assert!(out.contains("k=1;"), "{out}");
    assert!(out.contains("k=0;"), "{out}");
    assert!(out.contains("__DECODE_0__(k)"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_reference_inside_a_function_expression_is_left_alone() {
    let (out, report) = normalize_to_fixpoint(
        "var k;\nk = 0x1;\nvar f = function () { return __DECODE_0__(k); };\nf();",
    );
    assert!(out.contains("var k;"), "{out}");
    assert!(out.contains("k=1;"), "{out}");
    assert!(out.contains("__DECODE_0__(k)"), "{out}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
}

#[test]
fn a_write_in_the_reading_statement_leaves_the_earlier_read_alone() {
    let (out, _report) = normalize_to_fixpoint(
        "function t() {\nvar k;\nk = 0;\nconsole.log(__DECODE_0__(k), k = 2, __DECODE_0__(k));\n}",
    );
    assert!(out.contains("__DECODE_0__(k),k=2,__DECODE_0__(2)"), "{out}");
}

#[test]
fn the_well_ordered_assignment_is_still_normalised() {
    let (out, _report) = normalize_to_fixpoint(
        "function t() {\nvar k;\nk = 0xd6;\nconsole.log(__DECODE_0__(k));\n}",
    );
    assert!(out.contains("__DECODE_0__(214)"), "{out}");
    assert!(!out.contains("var k"), "{out}");
}
