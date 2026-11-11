use std::time::Duration;

use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use untwist_core::parse::parse_program;
use untwist_core::pass::{
    note_survives_retraction, run_to_fixpoint, Cx, Interest, IterationTiming, Note, NoteKind, Pass,
    PassGroup, PassOptions, Safety, StopReason, CYCLE_CHECK_START, ITERATION_CEILING,
};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;

struct NotZeroToTrue;
impl<'a> Pass<'a> for NotZeroToTrue {
    fn name(&self) -> &'static str {
        "not-zero-to-true"
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::UnaryExpression(u) = &*expr else {
            return;
        };
        if u.operator != UnaryOperator::LogicalNot {
            return;
        }
        let Expression::NumericLiteral(n) = &u.argument else {
            return;
        };
        if n.value != 0.0 {
            return;
        }
        let span = u.span;
        *expr = Expression::new_boolean_literal(span, true, cx);
        cx.changed();
    }
}

struct SplitSequences;
impl<'a> Pass<'a> for SplitSequences {
    fn name(&self) -> &'static str {
        "split-sequences"
    }
    fn enter_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        let has_seq = stmts.iter().any(|s| {
            matches!(s, Statement::ExpressionStatement(e) if matches!(e.expression, Expression::SequenceExpression(_)))
        });
        if !has_seq {
            return;
        }
        let old = std::mem::replace(stmts, AVec::new_in(cx));
        for stmt in old {
            match stmt {
                Statement::ExpressionStatement(mut es)
                    if matches!(es.expression, Expression::SequenceExpression(_)) =>
                {
                    let placeholder = Expression::new_null_literal(es.span, cx);
                    let Expression::SequenceExpression(seq) =
                        std::mem::replace(&mut es.expression, placeholder)
                    else {
                        unreachable!()
                    };
                    for e in seq.unbox().expressions {
                        stmts.push(Statement::new_expression_statement(e.span(), e, cx));
                    }
                    cx.changed();
                }
                other => stmts.push(other),
            }
        }
    }
}

struct SleepsInEnterProgram;
impl<'a> Pass<'a> for SleepsInEnterProgram {
    fn name(&self) -> &'static str {
        "sleeps-in-enter-program"
    }
    fn enter_program(&mut self, _p: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {
        std::thread::sleep(Duration::from_millis(20));
    }
}

const PROGRAM_HOOK_SLEEP: Duration = Duration::from_millis(20);

struct UnsafeMarker;
impl<'a> Pass<'a> for UnsafeMarker {
    fn name(&self) -> &'static str {
        "unsafe-marker"
    }
    fn safety(&self) -> Safety {
        Safety::Unsafe
    }
    fn exit_program(&mut self, _p: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        cx.note("ran", None);
    }
}

fn run(
    source: &str,
    passes: Vec<Box<dyn Pass<'static> + 'static>>,
    opts: PassOptions,
) -> (String, untwist_core::pass::RunReport) {
    let alloc = Box::leak(Box::new(Allocator::default()));
    let mut parsed = parse_program(alloc, Box::leak(source.to_string().into_boxed_str()));
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::new(passes, opts);
    let (_sem, report) = run_to_fixpoint(alloc, &mut parsed.program, sem, &mut group);
    (print(&parsed.program, PrintStyle::Pretty), report)
}

#[test]
fn merged_traversal_reaches_fixpoint_in_two_iterations() {
    let (out, report) = run(
        "var a = !0; x(), y();",
        vec![Box::new(NotZeroToTrue), Box::new(SplitSequences)],
        PassOptions::default(),
    );
    assert_eq!(out, "var a = true;\nx();\ny();");
    assert_eq!(report.iterations, 2);
    assert_eq!(report.total_changes, 2);
    assert_eq!(
        report.per_pass,
        vec![("not-zero-to-true", 1), ("split-sequences", 1)]
    );
    assert!(report.converged());
}

#[test]
fn the_report_times_the_program_hooks_per_pass() {
    let (_, report) = run(
        "var a = !0;",
        vec![Box::new(SleepsInEnterProgram), Box::new(NotZeroToTrue)],
        PassOptions::default(),
    );
    let names: Vec<&str> = report
        .per_pass_elapsed
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(names, vec!["sleeps-in-enter-program", "not-zero-to-true"]);
    assert_eq!(
        names,
        report
            .per_pass
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        "elapsed rows align with the change rows"
    );
    let slept = report.per_pass_elapsed[0].1;
    assert!(
        slept >= PROGRAM_HOOK_SLEEP * report.iterations,
        "{slept:?} over {} iterations",
        report.iterations
    );
    let untimed = report.per_pass_elapsed[1].1;
    assert!(
        untimed < PROGRAM_HOOK_SLEEP,
        "a pass that works only in exit_expression is not timed: {untimed:?}"
    );
}

#[test]
fn unsafe_passes_are_skipped_unless_enabled() {
    let (_, report) = run("1;", vec![Box::new(UnsafeMarker)], PassOptions::default());
    assert!(report.notes.is_empty());
    let (_, report) = run(
        "1;",
        vec![Box::new(UnsafeMarker)],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    assert_eq!(report.notes.len(), 1);
    assert_eq!(report.notes[0].pass, "unsafe-marker");
}

#[test]
fn blocks_created_by_passes_survive_a_second_traversal() {
    struct WrapIfBodies;
    impl<'a> Pass<'a> for WrapIfBodies {
        fn name(&self) -> &'static str {
            "wrap-if"
        }
        fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
            let Statement::IfStatement(if_stmt) = stmt else {
                return;
            };
            if matches!(if_stmt.consequent, Statement::BlockStatement(_)) {
                return;
            }
            let placeholder = Statement::new_empty_statement(if_stmt.span, cx);
            let body = std::mem::replace(&mut if_stmt.consequent, placeholder);
            if_stmt.consequent = cx.block_of(body);
            cx.changed();
        }
    }
    let (out, report) = run(
        "if (a) b();",
        vec![Box::new(WrapIfBodies)],
        PassOptions::default(),
    );
    assert_eq!(out, "if (a) {\n  b();\n}");
    assert_eq!(report.iterations, 2);
}

#[test]
fn iteration_cap_is_reported() {
    struct AlwaysChanges;
    impl<'a> Pass<'a> for AlwaysChanges {
        fn name(&self) -> &'static str {
            "always"
        }
        fn exit_program(&mut self, _p: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
            cx.changed();
        }
    }
    let (_, report) = run(
        "1;",
        vec![Box::new(AlwaysChanges)],
        PassOptions {
            max_iterations: Some(3),
            ..PassOptions::default()
        },
    );
    assert_eq!(report.iterations, 3);
    assert_eq!(report.stop, StopReason::IterationCap);
    assert!(report
        .notes
        .iter()
        .any(|note| note.pass == "driver" && note.message.contains("iteration cap 3")));
}

struct Countdown;
impl<'a> Pass<'a> for Countdown {
    fn name(&self) -> &'static str {
        "countdown"
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NumericLiteral(n) = expr else {
            return;
        };
        if n.value <= 0.0 {
            return;
        }
        n.value -= 1.0;
        n.raw = None;
        cx.changed();
    }
}

#[test]
fn the_driver_runs_past_the_old_cap_of_ten_until_nothing_changes() {
    let (out, report) = run("12;", vec![Box::new(Countdown)], PassOptions::default());
    assert_eq!(out, "0;");
    assert_eq!(report.iterations, 13);
    assert_eq!(report.stop, StopReason::Converged);
    assert!(report.notes.is_empty());
}

struct Toggle;
impl<'a> Pass<'a> for Toggle {
    fn name(&self) -> &'static str {
        "toggle"
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NumericLiteral(n) = expr else {
            return;
        };
        n.value = if n.value == 1.0 { 2.0 } else { 1.0 };
        n.raw = None;
        cx.changed();
    }
}

#[test]
fn oscillating_passes_stop_with_a_cycle_note() {
    let (_, report) = run("1;", vec![Box::new(Toggle)], PassOptions::default());
    assert_eq!(report.stop, StopReason::Cycle);
    assert_eq!(report.iterations, CYCLE_CHECK_START + 2);
    assert_eq!(report.notes.len(), 1);
    assert_eq!(report.notes[0].pass, "driver");
    assert!(report.notes[0].message.contains("oscillate"));
}

#[test]
fn a_user_cap_wins_over_cycle_detection() {
    let (_, report) = run(
        "1;",
        vec![Box::new(Toggle)],
        PassOptions {
            max_iterations: Some(3),
            ..PassOptions::default()
        },
    );
    assert_eq!(report.stop, StopReason::IterationCap);
    assert_eq!(report.iterations, 3);
}

struct Increment;
impl<'a> Pass<'a> for Increment {
    fn name(&self) -> &'static str {
        "increment"
    }
    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NumericLiteral(n) = expr else {
            return;
        };
        n.value += 1.0;
        n.raw = None;
        cx.changed();
    }
}

#[test]
fn the_ceiling_bounds_a_program_that_never_repeats() {
    let (_, report) = run("0;", vec![Box::new(Increment)], PassOptions::default());
    assert_eq!(report.stop, StopReason::IterationCap);
    assert_eq!(report.iterations, ITERATION_CEILING);
}

#[test]
fn a_note_repeated_every_iteration_is_reported_once() {
    let (_, report) = run(
        "3;",
        vec![Box::new(Countdown), Box::new(UnsafeMarker)],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    assert!(report.iterations > 1, "{:?}", report.iterations);
    assert_eq!(
        report
            .notes
            .iter()
            .filter(|note| note.message == "ran")
            .count(),
        1,
        "{:?}",
        report.notes
    );
}

fn note(kind: NoteKind, span: Option<Span>) -> Note {
    Note {
        pass: "test",
        kind,
        message: "anything at all".to_string(),
        span,
    }
}

#[test]
fn an_unspanned_note_survives_retraction_whatever_its_kind() {
    assert!(note_survives_retraction(&note(NoteKind::Refusal, None)));
    assert!(note_survives_retraction(&note(NoteKind::Progress, None)));
}

#[test]
fn a_spanned_progress_note_survives_and_a_spanned_refusal_does_not() {
    let span = Some(Span::new(0, 1));
    assert!(note_survives_retraction(&note(NoteKind::Progress, span)));
    assert!(!note_survives_retraction(&note(NoteKind::Refusal, span)));
}

#[test]
fn the_classifier_no_longer_reads_the_message() {
    let a = note(NoteKind::Progress, Some(Span::new(0, 1)));
    let mut b = a.clone();
    b.message = "decoder machinery kept: 3 references did not decode".to_string();
    assert_eq!(
        note_survives_retraction(&a),
        note_survives_retraction(&b),
        "the retention rule still depends on the wording"
    );
}

#[derive(Default)]
struct CountsResets {
    seen: u32,
    resets: u32,
}

impl<'a> Pass<'a> for CountsResets {
    fn name(&self) -> &'static str {
        "test-counts-resets"
    }

    fn reset(&mut self) {
        self.resets += 1;
        self.seen = 0;
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NumericLiteral(literal) = expr else {
            return;
        };
        if literal.value >= 8.0 {
            return;
        }
        literal.value += 1.0;
        literal.raw = None;
        self.seen += 1;
        cx.changed();
    }

    fn exit_program(&mut self, _program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        cx.note(format!("seen {} resets {}", self.seen, self.resets), None);
    }
}

#[test]
fn the_driver_resets_every_pass_between_iterations() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "x(1);");
    let sem = build_scoping(&parsed.program);
    let passes: Vec<Box<dyn Pass<'_> + '_>> = vec![Box::new(CountsResets::default())];
    let mut group = PassGroup::new(passes, PassOptions::default());
    let (_sem, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    let last = report
        .notes
        .iter()
        .rfind(|note| note.message.starts_with("seen "))
        .expect("the pass reported");
    assert_eq!(last.message, "seen 0 resets 7");
}

struct CountsEveryHook {
    calls: std::rc::Rc<std::cell::Cell<u32>>,
}
impl<'a> Pass<'a> for CountsEveryHook {
    fn name(&self) -> &'static str {
        "counts-every-hook"
    }
    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION
    }
    fn exit_expression(&mut self, _e: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {
        self.calls.set(self.calls.get() + 1);
    }
    fn exit_statement(&mut self, _s: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {
        self.calls.set(self.calls.get() + 1000);
    }
}

#[test]
fn a_hook_outside_the_declared_interest_is_never_dispatched() {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let pass = CountsEveryHook {
        calls: std::rc::Rc::clone(&calls),
    };
    let (_, _) = run("a; b;", vec![Box::new(pass)], PassOptions::default());
    assert_eq!(
        calls.get(),
        2,
        "two identifier expressions, no statement hook"
    );
}

#[test]
fn the_exhaustive_group_dispatches_every_hook_regardless_of_interest() {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let pass = CountsEveryHook {
        calls: std::rc::Rc::clone(&calls),
    };
    let alloc = Box::leak(Box::new(Allocator::default()));
    let mut parsed = parse_program(alloc, "a; b;");
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::exhaustive_for_tests(vec![Box::new(pass)], PassOptions::default());
    let _ = run_to_fixpoint(alloc, &mut parsed.program, sem, &mut group);
    assert_eq!(calls.get(), 2002, "two expressions and two statements");
}

#[test]
fn interest_constants_are_distinct_single_bits_in_hook_order() {
    let all = [
        Interest::ENTER_PROGRAM,
        Interest::EXIT_PROGRAM,
        Interest::ENTER_STATEMENTS,
        Interest::EXIT_STATEMENTS,
        Interest::ENTER_STATEMENT,
        Interest::EXIT_STATEMENT,
        Interest::ENTER_EXPRESSION,
        Interest::EXIT_EXPRESSION,
        Interest::EXIT_FUNCTION,
        Interest::EXIT_ARROW_FUNCTION_EXPRESSION,
        Interest::EXIT_VARIABLE_DECLARATION,
        Interest::EXIT_OBJECT_PROPERTY,
        Interest::EXIT_METHOD_DEFINITION,
        Interest::EXIT_PROPERTY_DEFINITION,
        Interest::EXIT_TEMPLATE_LITERAL,
        Interest::EXIT_ASSIGNMENT_TARGET,
    ];
    let mut union = Interest::NONE;
    for (slot, bit) in all.iter().enumerate() {
        assert_eq!(bit.slot(), slot);
        assert!(!union.includes(*bit));
        union = union | *bit;
    }
    assert_eq!(union, Interest::ALL);
    assert_eq!(
        (Interest::ENTER_PROGRAM | Interest::EXIT_STATEMENTS).named(),
        vec!["ENTER_PROGRAM", "EXIT_STATEMENTS"]
    );
}

#[test]
fn every_iteration_reports_its_traverse_and_the_changed_ones_their_rebuild() {
    let (_, report) = run(
        "var a = !0; x(), y();",
        vec![Box::new(NotZeroToTrue), Box::new(SplitSequences)],
        PassOptions::default(),
    );
    assert_eq!(report.iteration_elapsed.len(), report.iterations as usize);
    let (last, changed) = report
        .iteration_elapsed
        .split_last()
        .unwrap_or_else(|| panic!("no iteration was timed"));
    assert!(changed.iter().all(|timing| timing.rebuild().is_some()));
    assert_eq!(
        last.rebuild(),
        None,
        "the converged iteration rebuilds nothing"
    );
    assert!(report
        .iteration_elapsed
        .iter()
        .all(|timing| timing.fingerprint().is_none()));
}

#[test]
fn only_the_iterations_that_print_a_fingerprint_report_one() {
    let (_, report) = run("1;", vec![Box::new(Toggle)], PassOptions::default());
    assert_eq!(report.stop, StopReason::Cycle);
    let fingerprinted: Vec<usize> = report
        .iteration_elapsed
        .iter()
        .enumerate()
        .filter(|(_, timing)| timing.fingerprint().is_some())
        .map(|(index, _)| index + 1)
        .collect();
    let first = CYCLE_CHECK_START as usize;
    let expected: Vec<usize> = (first..=report.iterations as usize).collect();
    assert_eq!(fingerprinted, expected);
}

#[test]
fn a_capped_run_prints_no_fingerprint_on_its_last_iteration() {
    let (_, report) = run(
        "1;",
        vec![Box::new(Toggle)],
        PassOptions {
            max_iterations: Some(CYCLE_CHECK_START),
            ..PassOptions::default()
        },
    );
    assert_eq!(report.stop, StopReason::IterationCap);
    assert!(report
        .iteration_elapsed
        .iter()
        .all(|timing| timing.fingerprint().is_none()));
}

#[test]
fn an_iteration_timing_is_three_words() {
    assert_eq!(std::mem::size_of::<IterationTiming>(), 24);
}

#[test]
fn an_iteration_timing_gives_back_every_duration_it_was_given() {
    for nanos in [0, 1, 999, 1_000_000_007, u64::MAX - 1] {
        let elapsed = Duration::from_nanos(nanos);
        let mut timing = IterationTiming::traversed(elapsed);
        assert_eq!(timing.traverse(), elapsed);
        assert_eq!((timing.rebuild(), timing.fingerprint()), (None, None));
        timing.set_rebuild(elapsed);
        timing.set_fingerprint(elapsed);
        assert_eq!(timing.rebuild(), Some(elapsed));
        assert_eq!(timing.fingerprint(), Some(elapsed));
    }
}

#[test]
fn an_iteration_timing_saturates_instead_of_wrapping() {
    let mut timing = IterationTiming::traversed(Duration::MAX);
    timing.set_rebuild(Duration::MAX);
    let ceiling = Duration::from_nanos(u64::MAX - 1);
    assert_eq!(timing.traverse(), ceiling);
    assert_eq!(timing.rebuild(), Some(ceiling));
}

struct AsksOnce {
    asked: u32,
}
impl<'a> Pass<'a> for AsksOnce {
    fn name(&self) -> &'static str {
        "asks-once"
    }
    fn exit_program(&mut self, _p: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.asked == 0 {
            cx.request_round();
        }
        self.asked += 1;
        cx.note(format!("round {}", self.asked), None);
    }
}

#[test]
fn a_requested_round_runs_once_more_without_a_change() {
    let (out, report) = run(
        "1;",
        vec![Box::new(AsksOnce { asked: 0 })],
        PassOptions::default(),
    );
    assert_eq!(out, "1;");
    assert_eq!(report.iterations, 2);
    assert_eq!(report.total_changes, 0);
    assert!(report.converged());
    assert_eq!(report.iteration_elapsed.len(), 2);
    assert!(report
        .iteration_elapsed
        .iter()
        .all(|t| t.rebuild().is_none()));
}

#[test]
fn a_requested_round_never_passes_the_iteration_cap() {
    struct AlwaysAsks;
    impl<'a> Pass<'a> for AlwaysAsks {
        fn name(&self) -> &'static str {
            "always-asks"
        }
        fn exit_program(&mut self, _p: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
            cx.request_round();
        }
    }
    let (_, report) = run(
        "1;",
        vec![Box::new(AlwaysAsks)],
        PassOptions {
            max_iterations: Some(3),
            ..PassOptions::default()
        },
    );
    assert_eq!(report.iterations, 3);
    assert!(report.converged());
}

#[test]
fn a_long_run_prints_the_program_for_the_cycle_check_a_logarithmic_number_of_times() {
    let (out, report) = run("4096;", vec![Box::new(Countdown)], PassOptions::default());
    assert_eq!(out, "0;");
    let printed = report
        .iteration_elapsed
        .iter()
        .filter(|timing| timing.fingerprint().is_some())
        .count();
    assert!(
        printed <= 32 * 8,
        "{printed} cycle-check prints over {} iterations",
        report.iterations
    );
}

#[test]
fn a_cycle_that_starts_late_is_still_found() {
    struct LateToggle;
    impl<'a> Pass<'a> for LateToggle {
        fn name(&self) -> &'static str {
            "late-toggle"
        }
        fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
            let Expression::NumericLiteral(n) = expr else {
                return;
            };
            n.value = if n.value > 1.0 {
                n.value - 1.0
            } else if n.value == 1.0 {
                -1.0
            } else {
                1.0
            };
            n.raw = None;
            cx.changed();
        }
    }
    let (_, report) = run("300;", vec![Box::new(LateToggle)], PassOptions::default());
    assert_eq!(report.stop, StopReason::Cycle);
    assert!(report.iterations < 600, "{}", report.iterations);
}
