mod common;

use common::eval;
use oxc_allocator::{Allocator, Vec as ArenaVec};
use oxc_ast::ast::Statement;
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use untwist_core::parse::parse_program;
use untwist_eval::env::hoist_var_names;
use untwist_eval::fold::{fold_to_const, Const, FoldOptions};

fn nested_blocks_source(depth: usize) -> String {
    let mut source = String::from("function f() {\n");
    source.push_str(&"{".repeat(depth));
    source.push_str("return 1;");
    source.push_str(&"}".repeat(depth));
    source.push_str("\n}\nf()");
    source
}

fn left_deep_addition_source(depth: usize) -> String {
    let mut source = String::from("1");
    for _ in 1..depth {
        source.push_str("+1");
    }
    source
}

#[test]
fn deeply_nested_blocks_evaluate_without_a_stack_overflow() {
    let source = nested_blocks_source(2_000);
    assert_eq!(eval(&source), Ok("number:1".to_string()));
}

#[test]
fn a_left_deep_addition_chain_evaluates_without_a_stack_overflow() {
    let depth = 5_000;
    let source = left_deep_addition_source(depth);
    assert_eq!(eval(&source), Ok(format!("number:{depth}")));
}

fn with_left_deep_chain<R>(
    depth: usize,
    body: impl FnOnce(&oxc_ast::ast::Expression<'_>) -> R,
) -> R {
    let source = left_deep_addition_source(depth);
    common::with_program(&source, |program| {
        let Some(Statement::ExpressionStatement(node)) = program.body.first() else {
            panic!("expected one expression statement");
        };
        body(&node.expression)
    })
}

#[test]
fn folding_a_left_deep_chain_does_not_overflow_the_stack() {
    let depth = 10_000;
    let folded = with_left_deep_chain(depth, |expr| fold_to_const(expr, &FoldOptions::default()));
    assert_eq!(folded, Some(Const::Num(depth as f64)));
}

#[test]
fn hoisting_a_deeply_nested_block_tree_does_not_overflow_the_stack() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let parsed = parse_program(&alloc, "var deep;");
    let mut body = parsed.program.body;
    let Some(mut current) = body.pop() else {
        panic!("expected one statement");
    };
    for _ in 0..10_000 {
        let nested = ArenaVec::from_array_in([current], &&alloc);
        current = Statement::new_block_statement(SPAN, nested, &builder);
    }
    let outer: &Statement<'_> = alloc.alloc(current);
    let mut names = Vec::new();
    assert!(hoist_var_names(std::slice::from_ref(outer), &mut names).is_ok());
    assert_eq!(names, vec!["deep"]);
}
