use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard, PoisonError};

use oxc_allocator::{Allocator, CloneIn, GetAllocator, TakeIn};
use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::AssignmentOperator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, Cx, Interest, Pass, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::{
    build_scoping, check_patches_for_tests, forget_failures, refresh_counts_for_tests,
    skip_one_deletion_for_tests, RefreshCounts, STALE_PATCH,
};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

type Surgery = for<'a> fn(&mut Program<'a>, &mut Cx<'a, '_>);

struct Once {
    surgery: Surgery,
    done: bool,
}

impl<'a> Pass<'a> for Once {
    fn name(&self) -> &'static str {
        "surgery"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if !self.done {
            self.done = true;
            (self.surgery)(program, cx);
            cx.changed();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Patched,
    Rebuilt,
}

fn refresh(source: &str, surgery: Surgery) -> (Outcome, String) {
    forget_failures();
    let before = refresh_counts_for_tests();
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let pass: Box<dyn Pass<'_>> = Box::new(Once {
        surgery,
        done: false,
    });
    let mut group = PassGroup::new(vec![pass], PassOptions::default());
    check_patches_for_tests(true);
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    check_patches_for_tests(false);
    let after = refresh_counts_for_tests();
    let outcome = outcome(before, after);
    (outcome, print(&parsed.program, PrintStyle::Compact))
}

fn outcome(before: RefreshCounts, after: RefreshCounts) -> Outcome {
    assert_eq!(after.stale, before.stale, "{STALE_PATCH}");
    assert_eq!(
        after.patched - before.patched,
        after.checked - before.checked,
        "a patched scoping went unchecked"
    );
    match (
        after.patched - before.patched,
        after.rebuilt - before.rebuilt,
    ) {
        (1, 0) => Outcome::Patched,
        (0, 1) => Outcome::Rebuilt,
        counts => panic!("one refresh expected, got {counts:?}"),
    }
}

fn zero<'a>(cx: &Cx<'a, '_>) -> Expression<'a> {
    Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, cx)
}

fn statement_call<'p, 'a>(
    program: &'p mut Program<'a>,
    index: usize,
) -> &'p mut CallExpression<'a> {
    let Some(Statement::ExpressionStatement(statement)) = program.body.get_mut(index) else {
        panic!("statement {index} is not an expression statement");
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        panic!("statement {index} is not a call");
    };
    call
}

fn argument<'p, 'a>(call: &'p mut CallExpression<'a>) -> &'p mut Expression<'a> {
    call.arguments
        .first_mut()
        .and_then(Argument::as_expression_mut)
        .expect("a first argument")
}

fn function_body<'p, 'a>(program: &'p mut Program<'a>, index: usize) -> &'p mut Function<'a> {
    let Some(Statement::FunctionDeclaration(function)) = program.body.get_mut(index) else {
        panic!("statement {index} is not a function declaration");
    };
    function
}

fn inner_call<'p, 'a>(function: &'p mut Function<'a>, index: usize) -> &'p mut CallExpression<'a> {
    let body = function.body.as_mut().expect("a body");
    let Some(Statement::ExpressionStatement(statement)) = body.statements.get_mut(index) else {
        panic!("body statement {index} is not an expression statement");
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        panic!("body statement {index} is not a call");
    };
    call
}

fn first_default<'p, 'a>(function: &'p mut Function<'a>) -> &'p mut Expression<'a> {
    function
        .params
        .items
        .first_mut()
        .and_then(|param| param.initializer.as_deref_mut())
        .expect("a default")
}

#[test]
fn a_deleted_reference_is_patched_out_of_its_symbol() {
    let _serial = serial();
    let (outcome, code) = refresh("var kept = 1;\nf(kept);\ng(kept);\n", |program, cx| {
        let replacement = zero(cx);
        *argument(statement_call(program, 2)) = replacement;
    });
    assert_eq!(outcome, Outcome::Patched);
    assert!(code.contains("g(0)"), "{code}");
}

#[test]
fn a_deleted_free_reference_is_patched_out_of_the_root_list() {
    let _serial = serial();
    let (outcome, _) = refresh("f(free);\ng(free);\nh(free);\n", |program, cx| {
        let replacement = zero(cx);
        *argument(statement_call(program, 1)) = replacement;
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_cloned_reference_gets_a_reference_of_its_own() {
    let _serial = serial();
    let (outcome, code) = refresh("var x = 1;\nf(x);\ng(0);\n", |program, cx| {
        let copy = argument(statement_call(program, 1)).clone_in(cx.allocator());
        *argument(statement_call(program, 2)) = copy;
    });
    assert_eq!(outcome, Outcome::Patched);
    assert!(code.contains("g(x)"), "{code}");
}

#[test]
fn a_shared_reference_cell_is_split() {
    let _serial = serial();
    let (outcome, _) = refresh("var x = 1;\nf(x);\ng(0);\n", |program, cx| {
        let Expression::Identifier(original) = argument(statement_call(program, 1)) else {
            panic!("an identifier");
        };
        let id = original.reference_id.get().expect("a reference");
        let shared = Expression::new_identifier_with_reference_id(SPAN, "x", id, cx);
        *argument(statement_call(program, 2)) = shared;
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_reference_moved_under_a_shadowing_binding_is_resolved_again() {
    let _serial = serial();
    let source = "var x = 1;\nfunction f() {\n  var x = 2;\n  g(0);\n}\nh(x);\n";
    let (outcome, code) = refresh(source, |program, cx| {
        let moved = argument(statement_call(program, 2)).take_in(&*cx);
        *argument(inner_call(function_body(program, 1), 1)) = moved;
    });
    assert_eq!(outcome, Outcome::Patched);
    assert!(code.contains("g(x)"), "{code}");
}

#[test]
fn a_reference_to_a_removed_binding_resolves_to_the_outer_binding() {
    let _serial = serial();
    let source = "var x = 1;\nfunction f() {\n  var x = 2;\n  g(x);\n}\nh(x);\n";
    let (outcome, code) = refresh(source, |program, _| {
        let body = function_body(program, 1).body.as_mut().expect("a body");
        body.statements.remove(0);
    });
    assert_eq!(outcome, Outcome::Patched);
    assert!(code.contains("g(x)"), "{code}");
}

#[test]
fn a_changed_operator_patches_the_target_flags() {
    let _serial = serial();
    let (outcome, code) = refresh("var x = 1;\nx = 2;\n", |program, _| {
        let Some(Statement::ExpressionStatement(statement)) = program.body.get_mut(1) else {
            panic!("an expression statement");
        };
        let Expression::AssignmentExpression(assignment) = &mut statement.expression else {
            panic!("an assignment");
        };
        assignment.operator = AssignmentOperator::Addition;
    });
    assert_eq!(outcome, Outcome::Patched);
    assert!(code.contains("x+=2"), "{code}");
}

#[test]
fn a_default_moved_into_the_body_sees_the_body_binding() {
    let _serial = serial();
    let source = "var b = 0;\nfunction f(a = b) {\n  var b = 1;\n  g(0);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| {
        let function = function_body(program, 1);
        let moved = first_default(function).take_in(&*cx);
        *argument(inner_call(function, 1)) = moved;
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_body_reference_moved_into_a_default_skips_the_body_binding() {
    let _serial = serial();
    let source = "var b = 0;\nfunction f(a = 0) {\n  var b = 1;\n  g(b);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| {
        let function = function_body(program, 1);
        let moved = argument(inner_call(function, 1)).take_in(&*cx);
        *first_default(function) = moved;
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn an_arrow_moved_out_of_a_default_sees_the_body_binding() {
    let _serial = serial();
    let source = "var b = 0;\nfunction f(a = () => b) {\n  var b = 1;\n  g(0);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| {
        let function = function_body(program, 1);
        let moved = first_default(function).take_in(&*cx);
        *argument(inner_call(function, 1)) = moved;
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_renamed_binding_rebuilds() {
    let _serial = serial();
    let (outcome, _) = refresh("var x = 1;\nf(x);\n", |program, _| {
        let Some(Statement::VariableDeclaration(declaration)) = program.body.get_mut(0) else {
            panic!("a declaration");
        };
        let BindingPattern::BindingIdentifier(id) = &mut declaration.declarations[0].id else {
            panic!("an identifier");
        };
        id.name = "y".into();
        let Expression::Identifier(reference) = argument(statement_call(program, 1)) else {
            panic!("an identifier");
        };
        reference.name = "y".into();
    });
    assert_eq!(outcome, Outcome::Rebuilt);
}

#[test]
fn a_new_direct_eval_is_patched() {
    let _serial = serial();
    let (outcome, _) = refresh("function f() {\n  g(0);\n}\n", |program, cx| {
        let call = inner_call(function_body(program, 0), 0);
        call.callee = Expression::new_identifier(SPAN, "eval", cx);
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_deleted_catch_reference_is_patched() {
    let _serial = serial();
    let source = "try {\n  f();\n} catch (e) {\n  g(e);\n  h(e);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| {
        let Some(Statement::TryStatement(statement)) = program.body.get_mut(0) else {
            panic!("a try statement");
        };
        let handler = statement.handler.as_mut().expect("a handler");
        let Some(Statement::ExpressionStatement(call)) = handler.body.body.get_mut(1) else {
            panic!("an expression statement");
        };
        let Expression::CallExpression(call) = &mut call.expression else {
            panic!("a call");
        };
        *argument(call) = zero(cx);
    });
    assert_eq!(outcome, Outcome::Patched);
}

const EVERY_FORM: &str = r#"var a = {}, b = 1, c = 2, o = {}, k = "x";
a.b = 1; a[k] = 2; a.b += 3; a.b++; delete a.b; delete b;
({ b, c = a } = o); [b, c = b] = [c];
for (b in o) {} for (a.b of []) {}
(b ? a : o).c = 1; this[k] = 2; (a, o).c = 3;
b = c; b ||= c; b ??= 1; b++; --c;
class K extends (b ? a : o) { [k]() { return K; } static [b] = c; m() { return b; } }
var obj = { get p() { return b; }, set p(v) { c = v; }, q: function q() { return eval("b"); } };
function f(p = b, { q = c } = o, ...r) { var b; return p + q + r.length + b; }
try { f(); } catch ({ message = b }) { c = message; }
label: for (;;) { break label; }
`template ${b}`; f`tag${c}`; (() => { "use strict"; return arguments; });
"#;

#[test]
fn every_reference_form_survives_a_literal_only_round() {
    let _serial = serial();
    let (outcome, _) = refresh(EVERY_FORM, |program, cx| {
        let Some(Statement::VariableDeclaration(declaration)) = program.body.get_mut(0) else {
            panic!("a declaration");
        };
        declaration.declarations[1].init = Some(zero(cx));
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_patch_that_skips_one_deletion_differs_from_a_fresh_build() {
    let _serial = serial();
    skip_one_deletion_for_tests(true);
    let caught = catch_unwind(AssertUnwindSafe(|| {
        refresh("var kept = 1;\nf(kept);\ng(kept);\n", |program, cx| {
            let replacement = zero(cx);
            *argument(statement_call(program, 2)) = replacement;
        })
    }));
    skip_one_deletion_for_tests(false);
    check_patches_for_tests(false);
    let Err(payload) = caught else {
        panic!("the skipped deletion was not caught");
    };
    let text = payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(text.contains(STALE_PATCH), "{text}");
}

fn zero_first_initializer<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) {
    let Some(Statement::VariableDeclaration(declaration)) = program.body.get_mut(0) else {
        panic!("statement 0 is not a declaration");
    };
    declaration.declarations[0].init = Some(zero(cx));
}

fn swap_first_arguments<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>, one: usize, other: usize) {
    let first = argument(statement_call(program, one)).take_in(cx);
    let second = argument(statement_call(program, other)).take_in(cx);
    *argument(statement_call(program, one)) = second;
    *argument(statement_call(program, other)) = first;
}

#[test]
fn a_catch_default_resolves_to_a_sibling_parameter() {
    let _serial = serial();
    let source = "var z = 1;\ntry {\n  f();\n} catch ({ a, b = a }) {\n  g(b);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| zero_first_initializer(program, cx));
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn an_arrow_in_a_catch_default_resolves_to_a_sibling_parameter() {
    let _serial = serial();
    let source = "var z = 1;\ntry {\n  f();\n} catch ([a, b = () => a]) {\n  g(b);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| zero_first_initializer(program, cx));
    assert_eq!(outcome, Outcome::Patched);
}

const SIBLING_DEFAULTS: &str = "var z = 1;
function f(a, b = a, c = () => b) { return c; }
var h = (a, b = a, { c = () => b } = {}) => c;
class K {
  constructor(a, b = a) { this.b = b; }
  m(a, [b = a] = []) { return b; }
  static s(a, { b = () => a } = {}) { return b; }
}
var o = { n(a, b = a) { return b; }, set p(v = z) { z = v; } };
";

#[test]
fn parameter_defaults_resolve_to_sibling_parameters() {
    let _serial = serial();
    let (outcome, _) = refresh(SIBLING_DEFAULTS, |program, cx| {
        zero_first_initializer(program, cx);
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_changed_declaration_kind_rebuilds() {
    let _serial = serial();
    let (outcome, code) = refresh("let x = 1;\nf(x);\n", |program, _| {
        let Some(Statement::VariableDeclaration(declaration)) = program.body.get_mut(0) else {
            panic!("a declaration");
        };
        declaration.kind = VariableDeclarationKind::Const;
    });
    assert_eq!(outcome, Outcome::Rebuilt);
    assert!(code.contains("const x"), "{code}");
}

#[test]
fn a_changed_redeclaration_kind_rebuilds() {
    let _serial = serial();
    let (outcome, _) = refresh("var x = 1;\nvar x = 2;\nf(x);\n", |program, _| {
        let Some(Statement::VariableDeclaration(declaration)) = program.body.get_mut(1) else {
            panic!("a declaration");
        };
        declaration.kind = VariableDeclarationKind::Let;
    });
    assert_eq!(outcome, Outcome::Rebuilt);
}

#[test]
fn a_redeclaration_that_takes_the_first_kind_rebuilds() {
    let _serial = serial();
    let (outcome, _) = refresh("const x = 1;\nlet x = 2;\nf(x);\n", |program, _| {
        let Some(Statement::VariableDeclaration(declaration)) = program.body.get_mut(1) else {
            panic!("a declaration");
        };
        declaration.kind = VariableDeclarationKind::Const;
    });
    assert_eq!(outcome, Outcome::Rebuilt);
}

#[test]
fn swapped_references_are_relisted_in_walk_order() {
    let _serial = serial();
    let source = "var x = 1;\nf(x);\ng(x);\nh(u);\nk(u);\n";
    let (outcome, _) = refresh(source, |program, cx| {
        swap_first_arguments(program, cx, 1, 2);
        swap_first_arguments(program, cx, 3, 4);
    });
    assert_eq!(outcome, Outcome::Patched);
}

#[test]
fn a_new_free_name_is_listed_as_unresolved() {
    let _serial = serial();
    let (outcome, code) = refresh("var x = 1;\nf(0);\n", |program, cx| {
        let fresh = Expression::new_identifier(SPAN, "brandNew", cx);
        *argument(statement_call(program, 1)) = fresh;
    });
    assert_eq!(outcome, Outcome::Patched);
    assert!(code.contains("f(brandNew)"), "{code}");
}

#[test]
fn a_stray_binding_rebuilds() {
    let _serial = serial();
    let source = "var x = 1;\nfunction f() {\n  g(x);\n}\n";
    let (outcome, _) = refresh(source, |program, cx| {
        let Some(Statement::VariableDeclaration(declaration)) = program.body.first() else {
            panic!("a declaration");
        };
        let BindingPattern::BindingIdentifier(id) = &declaration.declarations[0].id else {
            panic!("an identifier");
        };
        let symbol = id.symbol_id.get().expect("a symbol");
        let scope = function_body(program, 1).scope_id.get().expect("a scope");
        cx.scoping_mut().add_binding(scope, "x".into(), symbol);
    });
    assert_eq!(outcome, Outcome::Rebuilt);
}
