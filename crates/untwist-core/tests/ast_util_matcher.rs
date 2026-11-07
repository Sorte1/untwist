use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use untwist_core::ast_util::matcher::{
    infinite_loop_body, is_literal_argument, statement_iife, var_function_or_declaration,
};
use untwist_core::ast_util::prop::member_parts;
use untwist_core::parse::parse_program;

fn with<R>(source: &str, f: impl FnOnce(&[Statement<'_>]) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    f(&parsed.program.body)
}

#[test]
fn var_function_and_declaration_are_the_same_shape() {
    with("function a() {}\nvar b = function () {};", |body| {
        assert_eq!(
            var_function_or_declaration(&body[0]).map(|d| d.name),
            Some("a")
        );
        assert_eq!(
            var_function_or_declaration(&body[1]).map(|d| d.name),
            Some("b")
        );
    });
    with(
        "let d = function () {};\nconst e = function () {};",
        |body| {
            assert_eq!(
                var_function_or_declaration(&body[0]).map(|d| d.name),
                Some("d")
            );
            assert_eq!(
                var_function_or_declaration(&body[1]).map(|d| d.name),
                Some("e")
            );
        },
    );
    with("var c = 1;\nlet f = 2, g = function () {};", |body| {
        assert!(var_function_or_declaration(&body[0]).is_none());
        assert!(var_function_or_declaration(&body[1]).is_none());
    });
}

#[test]
fn a_bang_prefixed_iife_is_still_an_iife() {
    with("(function (c, d) {})(a, 1);", |body| {
        assert!(statement_iife(&body[0])
            .is_some_and(|(f, args)| f.params.items.len() == 2 && args.len() == 2));
    });
    with("!(function (c, d) {})(a, 1);", |body| {
        assert!(statement_iife(&body[0]).is_some());
    });
    with("(() => {})();", |body| {
        assert!(statement_iife(&body[0]).is_none());
    });
}

#[test]
fn every_infinite_loop_head_the_obfuscator_emits_is_recognised() {
    for source in [
        "while (!![]) { x(); }",
        "while (true) { x(); }",
        "for (;;) { x(); }",
        "for (; [];) { x(); }",
        "for (; !![];) { x(); }",
    ] {
        with(source, |body| {
            assert!(infinite_loop_body(&body[0]).is_some(), "{source}");
        });
    }
    with("while (x) { y(); }", |body| {
        assert!(infinite_loop_body(&body[0]).is_none());
    });
}

#[test]
fn const_member_matches_dot_and_bracket_but_not_optional() {
    with("a.push; a['push']; a?.push; a[b];", |body| {
        let names: Vec<Option<String>> = body
            .iter()
            .map(|stmt| {
                let Statement::ExpressionStatement(stmt) = stmt else {
                    return None;
                };
                member_parts(&stmt.expression).map(|(_, name)| name.into_owned())
            })
            .collect();
        assert_eq!(
            names,
            vec![
                Some("push".to_string()),
                Some("push".to_string()),
                None,
                None
            ]
        );
    });
}

#[test]
fn literal_arguments_are_recursive_over_arithmetic() {
    with("f(5 - -678 - 625, -'0xa6', 'DiKO', x);", |body| {
        let Statement::ExpressionStatement(stmt) = &body[0] else {
            panic!("expression statement");
        };
        let Expression::CallExpression(call) = &stmt.expression else {
            panic!("call");
        };
        let flags: Vec<bool> = call
            .arguments
            .iter()
            .map(|argument| argument.as_expression().is_some_and(is_literal_argument))
            .collect();
        assert_eq!(flags, vec![true, true, true, false]);
    });
}
