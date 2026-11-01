use std::cell::RefCell;
use std::rc::Rc;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program, Statement};
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, run_to_fixpoint, Cx, Interest, Pass, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;

use super::{clear_kept_order, lend_order, take_matching, Mark};

type Marks = Rc<RefCell<Vec<Mark>>>;

fn bump(expr: &mut Expression<'_>, from: f64, cx: &mut Cx<'_, '_>) {
    let Expression::NumericLiteral(literal) = expr else {
        return;
    };
    if literal.value == from {
        literal.value = from + 1.0;
        literal.raw = None;
        cx.changed();
    }
}

struct Producer(Marks);

impl<'a> Pass<'a> for Producer {
    fn name(&self) -> &'static str {
        "test-producer"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION | Interest::EXIT_PROGRAM
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        bump(expr, 1.0, cx);
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.0.borrow_mut().push(Mark::of(program, cx));
    }
}

struct Between(bool);

impl<'a> Pass<'a> for Between {
    fn name(&self) -> &'static str {
        "test-between"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if let (true, Some(Statement::ExpressionStatement(last))) =
            (self.0, program.body.last_mut())
        {
            bump(&mut last.expression, 3.0, cx);
        }
    }
}

struct Consumer(Marks);

impl<'a> Pass<'a> for Consumer {
    fn name(&self) -> &'static str {
        "test-consumer"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.0.borrow_mut().push(Mark::of(program, cx));
    }
}

fn marks(edits_between: bool) -> Vec<Mark> {
    let alloc = Allocator::default();
    let mut program = parse_program(&alloc, "1;\n3;").program;
    let sem = build_scoping(&program);
    let marks = Marks::default();
    let passes: Vec<Box<dyn Pass<'_>>> = vec![
        Box::new(Producer(Rc::clone(&marks))),
        Box::new(Between(edits_between)),
        Box::new(Consumer(Rc::clone(&marks))),
    ];
    let mut group = PassGroup::new(passes, PassOptions::default());
    run_once(&alloc, &mut program, sem, &mut group);
    let taken = marks.borrow().clone();
    taken
}

type Reuses = Rc<RefCell<Vec<bool>>>;

struct Lender(Reuses);

impl<'a> Pass<'a> for Lender {
    fn name(&self) -> &'static str {
        "test-lender"
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let reused = take_matching(Mark::of(program, cx)).is_some();
        self.0.borrow_mut().push(reused);
        lend_order(true, program, cx).give_back_any(program, cx);
    }
}

struct Edits;

impl<'a> Pass<'a> for Edits {
    fn name(&self) -> &'static str {
        "test-edits"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        bump(expr, 1.0, cx);
    }
}

fn reuses(source: &str, rounds: usize) -> Vec<bool> {
    clear_kept_order();
    let alloc = Allocator::default();
    let mut program = parse_program(&alloc, source).program;
    let mut sem = build_scoping(&program);
    let reuses = Reuses::default();
    let passes: Vec<Box<dyn Pass<'_>>> =
        vec![Box::new(Lender(Rc::clone(&reuses))), Box::new(Edits)];
    let mut group = PassGroup::new(passes, PassOptions::default());
    for _ in 0..rounds {
        sem = run_to_fixpoint(&alloc, &mut program, sem, &mut group).0;
    }
    clear_kept_order();
    let taken = reuses.borrow().clone();
    taken
}

#[test]
fn a_kept_order_is_not_lent_after_a_round_that_edited() {
    assert_eq!(reuses("var x = 1;\nf(x);", 1), [false, false]);
}

#[test]
fn a_kept_order_is_lent_again_to_an_unedited_program() {
    assert_eq!(reuses("var x = 2;\nf(x);", 2), [false, true]);
}

#[test]
fn a_mark_moves_with_a_change_between_two_program_hooks() {
    let marks = marks(true);
    assert_eq!(marks.len(), 2);
    assert!(marks[0] != marks[1]);
}

#[test]
fn a_mark_holds_across_program_hooks_that_change_nothing() {
    let marks = marks(false);
    assert_eq!(marks.len(), 2);
    assert!(marks[0] == marks[1]);
}
