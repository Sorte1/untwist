use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard, PoisonError};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use untwist_core::parse::parse_program;
use untwist_core::pass::{
    probe_variants_for_tests, run_to_fixpoint, take_variant_violations_for_tests, Cx,
    ExpressionKind, ExpressionVariants, Interest, Pass, PassGroup, PassOptions, StatementKind,
    StatementVariants,
};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

type Seen = Rc<RefCell<Vec<String>>>;

type Run = (String, Vec<(&'static str, u32)>);

struct RecordsExpressions {
    name: &'static str,
    variants: ExpressionVariants,
    seen: Seen,
}

impl<'a> Pass<'a> for RecordsExpressions {
    fn name(&self) -> &'static str {
        self.name
    }
    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(self.variants)
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        let kind = format!("{}:{:?}", self.name, ExpressionKind::of(expr));
        self.seen.borrow_mut().push(kind);
    }
}

struct RecordsStatements {
    variants: StatementVariants,
    seen: Seen,
}

impl<'a> Pass<'a> for RecordsStatements {
    fn name(&self) -> &'static str {
        "records-statements"
    }
    fn interest(&self) -> Interest {
        (Interest::ENTER_STATEMENT | Interest::EXIT_STATEMENT).on_statements(self.variants)
    }
    fn enter_statement(&mut self, stmt: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        let kind = format!("enter:{:?}", StatementKind::of(stmt));
        self.seen.borrow_mut().push(kind);
    }
    fn exit_statement(&mut self, stmt: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        let kind = format!("exit:{:?}", StatementKind::of(stmt));
        self.seen.borrow_mut().push(kind);
    }
}

struct NotZeroToTrue {
    variants: ExpressionVariants,
}

impl<'a> Pass<'a> for NotZeroToTrue {
    fn name(&self) -> &'static str {
        "not-zero-to-true"
    }
    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(self.variants)
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::UnaryExpression(unary) = &*expr else {
            return;
        };
        if unary.operator != UnaryOperator::LogicalNot
            || !matches!(&unary.argument, Expression::NumericLiteral(n) if n.value == 0.0)
        {
            return;
        }
        let span = unary.span;
        *expr = Expression::new_boolean_literal(span, true, cx);
        cx.changed();
    }
}

fn unary() -> ExpressionVariants {
    ExpressionVariants::of(&[ExpressionKind::UnaryExpression])
}

fn recorder(name: &'static str, kinds: &[ExpressionKind], seen: &Seen) -> Box<dyn Pass<'static>> {
    Box::new(RecordsExpressions {
        name,
        variants: ExpressionVariants::of(kinds),
        seen: Rc::clone(seen),
    })
}

fn opts() -> PassOptions {
    PassOptions {
        max_iterations: Some(1),
        ..PassOptions::default()
    }
}

fn run(source: &'static str, group: &mut PassGroup<'static>) -> Run {
    let alloc: &'static Allocator = Box::leak(Box::new(Allocator::default()));
    let mut parsed = parse_program(alloc, source);
    let sem = build_scoping(&parsed.program);
    let (_sem, report) = run_to_fixpoint(alloc, &mut parsed.program, sem, group);
    (print(&parsed.program, PrintStyle::Compact), report.per_pass)
}

fn run_probed(source: &'static str, passes: Vec<Box<dyn Pass<'static>>>) -> (Run, Vec<String>) {
    take_variant_violations_for_tests();
    probe_variants_for_tests(true);
    let mut group = PassGroup::new(passes, opts());
    probe_variants_for_tests(false);
    let out = run(source, &mut group);
    (out, take_variant_violations_for_tests())
}

const MIXED: &str = "x = a + !0 ? f(b) : [c]; if (d) { e(); } else while (g) h();";

#[test]
fn a_declared_set_limits_the_calls_to_its_variants() {
    let _serial = serial();
    let seen: Seen = Rc::default();
    let kinds = [
        ExpressionKind::BinaryExpression,
        ExpressionKind::CallExpression,
    ];
    let mut group = PassGroup::new(vec![recorder("r", &kinds, &seen)], opts());
    run(MIXED, &mut group);
    assert_eq!(
        *seen.borrow(),
        [
            "r:BinaryExpression",
            "r:CallExpression",
            "r:CallExpression",
            "r:CallExpression"
        ]
    );
}

#[test]
fn the_default_set_calls_a_pass_on_every_variant() {
    let _serial = serial();
    let by_variant: Seen = Rc::default();
    let every: Seen = Rc::default();
    let declared = RecordsExpressions {
        name: "r",
        variants: ExpressionVariants::ALL,
        seen: Rc::clone(&by_variant),
    };
    let exhaustive = RecordsExpressions {
        name: "r",
        variants: ExpressionVariants::ALL,
        seen: Rc::clone(&every),
    };
    run(MIXED, &mut PassGroup::new(vec![Box::new(declared)], opts()));
    run(
        MIXED,
        &mut PassGroup::exhaustive_for_tests(vec![Box::new(exhaustive)], opts()),
    );
    assert!(by_variant.borrow().len() >= 15, "{:?}", by_variant.borrow());
    assert_eq!(*by_variant.borrow(), *every.borrow());
}

#[test]
fn an_exhaustive_group_ignores_the_declared_sets() {
    let _serial = serial();
    let seen: Seen = Rc::default();
    let mut group = PassGroup::exhaustive_for_tests(
        vec![recorder("r", &[ExpressionKind::CallExpression], &seen)],
        opts(),
    );
    run(MIXED, &mut group);
    assert!(seen.borrow().len() >= 15, "{:?}", seen.borrow());
    assert!(seen.borrow().iter().any(|seen| seen == "r:Identifier"));
}

fn rewrite_chain(seen: &Seen) -> Vec<Box<dyn Pass<'static>>> {
    vec![
        recorder("early-boolean", &[ExpressionKind::BooleanLiteral], seen),
        Box::new(NotZeroToTrue { variants: unary() }),
        recorder("late-boolean", &[ExpressionKind::BooleanLiteral], seen),
        recorder("late-unary", &[ExpressionKind::UnaryExpression], seen),
    ]
}

#[test]
fn a_variant_change_dispatches_the_later_passes_of_the_new_variant() {
    let _serial = serial();
    let by_variant: Seen = Rc::default();
    let every: Seen = Rc::default();
    let out = run(
        "y = !0, !1;",
        &mut PassGroup::new(rewrite_chain(&by_variant), opts()),
    );
    let exhaustive = run(
        "y = !0, !1;",
        &mut PassGroup::exhaustive_for_tests(rewrite_chain(&every), opts()),
    );
    assert_eq!(out.0, "y=true,!1;");
    assert_eq!(
        *by_variant.borrow(),
        ["late-boolean:BooleanLiteral", "late-unary:UnaryExpression"]
    );
    let relevant: Vec<String> = every
        .borrow()
        .iter()
        .filter(|seen| {
            matches!(
                seen.as_str(),
                "early-boolean:BooleanLiteral"
                    | "late-boolean:BooleanLiteral"
                    | "late-unary:UnaryExpression"
            )
        })
        .cloned()
        .collect();
    assert_eq!(relevant, *by_variant.borrow());
    assert_eq!(out, exhaustive);
}

#[test]
fn statement_hooks_dispatch_by_statement_variant() {
    let _serial = serial();
    let seen: Seen = Rc::default();
    let pass = RecordsStatements {
        variants: StatementVariants::of(&[
            StatementKind::IfStatement,
            StatementKind::WhileStatement,
        ]),
        seen: Rc::clone(&seen),
    };
    run(MIXED, &mut PassGroup::new(vec![Box::new(pass)], opts()));
    assert_eq!(
        *seen.borrow(),
        [
            "enter:IfStatement",
            "enter:WhileStatement",
            "exit:WhileStatement",
            "exit:IfStatement"
        ]
    );
}

#[test]
fn a_group_wider_than_the_mask_calls_every_listed_pass_on_every_variant() {
    let _serial = serial();
    let seen: Seen = Rc::default();
    let names: Vec<&'static str> = (0..65)
        .map(|index| &*Box::leak(format!("r{index}").into_boxed_str()))
        .collect();
    let passes = names
        .iter()
        .map(|name| recorder(name, &[ExpressionKind::CallExpression], &seen))
        .collect();
    let (_, per_pass) = run(MIXED, &mut PassGroup::new(passes, opts()));
    let calls = seen
        .borrow()
        .iter()
        .filter(|seen| seen.ends_with(":CallExpression"))
        .count();
    assert_eq!(calls, 65 * 3);
    assert_eq!(seen.borrow().len() % 65, 0);
    assert!(seen.borrow().len() / 65 >= 15);
    assert!(per_pass.iter().all(|(_, count)| *count == 0));
}

#[test]
fn the_probe_reports_a_pass_that_acts_outside_its_declared_set() {
    let _serial = serial();
    let mutant = || -> Vec<Box<dyn Pass<'static>>> {
        vec![Box::new(NotZeroToTrue {
            variants: ExpressionVariants::of(&[ExpressionKind::BinaryExpression]),
        })]
    };
    let missed = run("y = !0;", &mut PassGroup::new(mutant(), opts()));
    assert_eq!(missed.0, "y=!0;", "the mutant's set hides its one edit");
    let (probed, violations) = run_probed("y = !0;", mutant());
    assert_eq!(probed.0, "y=true;");
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(
        violations[0],
        "not-zero-to-true acted in exit_expression on UnaryExpression, outside its declared \
         variants"
    );
}

#[test]
fn the_probe_reports_nothing_when_every_acted_on_variant_is_declared() {
    let _serial = serial();
    let passes =
        || -> Vec<Box<dyn Pass<'static>>> { vec![Box::new(NotZeroToTrue { variants: unary() })] };
    let by_variant = run(MIXED, &mut PassGroup::new(passes(), opts()));
    let (probed, violations) = run_probed(MIXED, passes());
    assert_eq!(violations, Vec::<String>::new());
    assert_eq!(probed, by_variant);
    assert_eq!(by_variant.1, [("not-zero-to-true", 1)]);
}

#[derive(Default)]
struct KindNames {
    checked: usize,
}

impl<'a> Visit<'a> for KindNames {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        let debug = format!("{it:?}");
        let kind = format!("{:?}(", ExpressionKind::of(it));
        assert!(debug.starts_with(&kind), "{kind} for {debug}");
        self.checked += 1;
        walk::walk_expression(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        let debug = format!("{it:?}");
        let kind = format!("{:?}(", StatementKind::of(it));
        assert!(debug.starts_with(&kind), "{kind} for {debug}");
        self.checked += 1;
        walk::walk_statement(self, it);
    }
}

#[test]
fn every_kind_is_named_after_the_variant_it_reads() {
    let source = "import d from 'm'; export const e = 1; export default f; \
        label: for (const k in o) { if (k) continue label; else break; } \
        for (const v of o) debugger; for (;;) {} do ; while (0); switch (s) { case 1: throw t; } \
        try { with (o) {} } catch {} class C { #p; m() { return #p in this && super.m?.(this.#p); } } \
        async function* g() { yield await import('x'), new.target, import.meta; } \
        var r = [/re/g, 1n, null, `t${s}`, (p), ++q, -q, q **= 2, a ?? b, c ? d : e, x => x, \
        function () {}, class {}, { k: 1 }, tag`x`, o[k], o.k];";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let mut names = KindNames::default();
    names.visit_program(&parsed.program);
    assert!(names.checked >= 60, "{}", names.checked);
}

#[test]
fn an_interest_keeps_every_variant_until_narrowed_and_unions_its_sets() {
    let binary = ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);
    let call = ExpressionVariants::of(&[ExpressionKind::CallExpression]);
    let blocks = StatementVariants::of(&[StatementKind::BlockStatement]);
    assert_eq!(
        Interest::EXIT_EXPRESSION.expressions(),
        ExpressionVariants::ALL
    );
    assert_eq!(
        Interest::EXIT_STATEMENT.statements(),
        StatementVariants::ALL
    );
    let narrowed = Interest::EXIT_EXPRESSION.on_expressions(binary);
    let joined = narrowed | Interest::ENTER_EXPRESSION.on_expressions(call);
    assert!(joined
        .expressions()
        .includes(ExpressionKind::BinaryExpression));
    assert!(joined
        .expressions()
        .includes(ExpressionKind::CallExpression));
    assert!(!joined.expressions().includes(ExpressionKind::Identifier));
    assert_eq!(
        (narrowed | Interest::EXIT_STATEMENT).expressions(),
        ExpressionVariants::ALL
    );
    let statements = Interest::EXIT_STATEMENT.on_statements(blocks);
    assert!(statements
        .statements()
        .includes(StatementKind::BlockStatement));
    assert!(!statements
        .statements()
        .includes(StatementKind::ExportFromDeclaration));
    assert!(statements.includes(Interest::EXIT_STATEMENT));
    assert_eq!(statements.named(), ["EXIT_STATEMENT"]);
}

struct Retires {
    retire_on: &'static [u32],
    retires: Interest,
    traversal: u32,
    seen: Seen,
}

impl<'a> Pass<'a> for Retires {
    fn name(&self) -> &'static str {
        "retires"
    }
    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM | Interest::EXIT_PROGRAM | Interest::EXIT_EXPRESSION)
            .on_expressions(ExpressionVariants::of(&[ExpressionKind::CallExpression]))
    }
    fn retired(&self) -> Interest {
        if self.retire_on.contains(&self.traversal) {
            self.retires
        } else {
            Interest::NONE
        }
    }
    fn enter_program(&mut self, _node: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {
        self.traversal += 1;
        let entered = format!("enter:{}", self.traversal);
        self.seen.borrow_mut().push(entered);
    }
    fn exit_program(&mut self, _node: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {
        let exited = format!("exit:{}", self.traversal);
        self.seen.borrow_mut().push(exited);
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        let kind = format!("call:{}:{:?}", self.traversal, ExpressionKind::of(expr));
        self.seen.borrow_mut().push(kind);
    }
}

fn retiring(retire_on: &'static [u32], retires: Interest, seen: &Seen) -> Box<dyn Pass<'static>> {
    Box::new(Retires {
        retire_on,
        retires,
        traversal: 0,
        seen: Rc::clone(seen),
    })
}

const CALLS: &str = "a(); b(); c();";

fn retire_group(
    retire_on: &'static [u32],
    retires: Interest,
    exhaustive: bool,
) -> (PassGroup<'static>, Seen, Seen) {
    let own: Seen = Rc::default();
    let mate: Seen = Rc::default();
    let passes = vec![
        retiring(retire_on, retires, &own),
        recorder("mate", &[ExpressionKind::CallExpression], &mate),
    ];
    let group = if exhaustive {
        PassGroup::exhaustive_for_tests(passes, opts())
    } else {
        PassGroup::new(passes, opts())
    };
    (group, own, mate)
}

#[test]
fn a_retired_hook_is_not_dispatched_for_the_rest_of_the_traversal() {
    let _serial = serial();
    for exhaustive in [false, true] {
        let (mut group, own, mate) = retire_group(&[1], Interest::EXIT_EXPRESSION, exhaustive);
        run(CALLS, &mut group);
        assert_eq!(*own.borrow(), ["enter:1", "exit:1"], "{exhaustive}");
        let mate_calls = mate
            .borrow()
            .iter()
            .filter(|seen| seen.ends_with(":CallExpression"))
            .count();
        assert_eq!(mate_calls, 3, "{exhaustive}: {:?}", mate.borrow());
    }
}

#[test]
fn a_retired_hook_comes_back_on_the_next_traversal() {
    let _serial = serial();
    for exhaustive in [false, true] {
        let (mut group, own, _) = retire_group(&[1], Interest::EXIT_EXPRESSION, exhaustive);
        let first = run(CALLS, &mut group);
        let second = run(CALLS, &mut group);
        assert_eq!(first, second);
        let own: Vec<String> = own
            .borrow()
            .iter()
            .filter(|seen| !seen.ends_with(":Identifier"))
            .cloned()
            .collect();
        assert_eq!(
            own,
            [
                "enter:1",
                "exit:1",
                "enter:2",
                "call:2:CallExpression",
                "call:2:CallExpression",
                "call:2:CallExpression",
                "exit:2"
            ],
            "{exhaustive}"
        );
    }
}

#[test]
fn enter_program_is_never_retired_and_exit_program_may_be() {
    let _serial = serial();
    let (mut group, own, _) = retire_group(&[1, 2], Interest::ALL, false);
    run(CALLS, &mut group);
    run(CALLS, &mut group);
    assert_eq!(*own.borrow(), ["enter:1", "enter:2"]);
}

#[test]
fn a_late_site_still_fires_for_a_pass_that_did_not_retire() {
    let _serial = serial();
    let own: Seen = Rc::default();
    let passes: Vec<Box<dyn Pass<'static>>> = vec![
        retiring(&[1], Interest::EXIT_EXPRESSION, &own),
        Box::new(NotZeroToTrue { variants: unary() }),
    ];
    let (code, per_pass) = run("a(); b(); y = !0;", &mut PassGroup::new(passes, opts()));
    assert_eq!(code, "a();b();y=true;");
    assert_eq!(per_pass, [("retires", 0), ("not-zero-to-true", 1)]);
    assert_eq!(*own.borrow(), ["enter:1", "exit:1"]);
}
