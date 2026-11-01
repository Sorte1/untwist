use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use rustc_hash::FxHashMap;
use untwist_core::parse::parse_program;
use untwist_eval::fold::{fold_literal_chain, fold_to_const, Const, FoldOptions};

fn with_expr<R>(source: &str, body: impl FnOnce(&Expression<'_>) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let Statement::ExpressionStatement(stmt) = &parsed.program.body[0] else {
        panic!("expression statement");
    };
    body(&stmt.expression)
}

fn fold_chain(source: &str) -> Option<Const> {
    with_expr(source, fold_literal_chain)
}

fn fold_default(source: &str) -> Option<Const> {
    with_expr(source, |expr| fold_to_const(expr, &FoldOptions::default()))
}

#[test]
fn issue_131_a_split_reverse_join_chain_folds() {
    assert_eq!(
        fold_chain(r#""!dlroW olleH".split("").reverse().join("");"#),
        Some(Const::Str("Hello World!".to_string()))
    );
}

#[test]
fn the_default_folder_still_refuses_a_mutating_native() {
    assert_eq!(fold_default(r#""ab".split("").reverse().join("");"#), None);
}

#[test]
fn an_array_literal_chain_folds() {
    assert_eq!(
        fold_chain(r#"["a", "b", "c"].reverse().join("-");"#),
        Some(Const::Str("c-b-a".to_string()))
    );
}

#[test]
fn local_mutation_is_refused_beside_a_binding() {
    let mut bindings = FxHashMap::default();
    bindings.insert("arr".to_string(), Const::Str("a".to_string()));
    let options = FoldOptions {
        allow_pure_calls: true,
        allow_local_mutation: true,
        bindings,
    };
    let folded = with_expr(r#""ab".split("").reverse().join("");"#, |expr| {
        fold_to_const(expr, &options)
    });
    assert_eq!(folded, None);
}
