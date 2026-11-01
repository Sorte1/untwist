use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use untwist_core::parse::parse_program;
use untwist_core::semantic::Strictness;

fn program_strictness(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    Strictness::of_program(&parsed.program).is_strict()
}

fn first_function_strictness(source: &str) -> (bool, bool) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let outer = Strictness::of_program(&parsed.program);
    let inner = parsed
        .program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::FunctionDeclaration(function) => Some(outer.in_function(function)),
            Statement::ExpressionStatement(expression) => match &expression.expression {
                Expression::ArrowFunctionExpression(arrow) => Some(outer.in_arrow(arrow)),
                _ => None,
            },
            _ => None,
        });
    (outer.is_strict(), inner.is_some_and(Strictness::is_strict))
}

#[test]
fn a_script_is_sloppy_until_a_directive_or_module_syntax() {
    assert!(!program_strictness("var a = 1;"));
    assert!(program_strictness("'use strict'; var a = 1;"));
    assert!(program_strictness("export const a = 1;"));
    assert!(!program_strictness("a; 'use strict';"));
}

#[test]
fn a_function_or_arrow_directive_makes_only_its_body_strict() {
    assert_eq!(
        first_function_strictness("function f() { 'use strict'; }"),
        (false, true)
    );
    assert_eq!(
        first_function_strictness("function f() { g(); }"),
        (false, false)
    );
    assert_eq!(
        first_function_strictness("() => { 'use strict'; };"),
        (false, true)
    );
    assert_eq!(
        first_function_strictness("'use strict'; function f() {}"),
        (true, true)
    );
}

#[test]
fn a_class_body_is_strict_and_strictness_never_relaxes() {
    let sloppy = Strictness::default();
    assert!(!sloppy.is_strict());
    assert!(sloppy.in_class().is_strict());
    assert!(sloppy.in_class().in_class().is_strict());
}
