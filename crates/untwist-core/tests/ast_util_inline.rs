use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_ast::builder::AstBuilder;
use oxc_codegen::Codegen;
use untwist_core::ast_util::inline::inline_function_call;
use untwist_core::ast_util::matcher::{call_parts, var_function_or_declaration};
use untwist_core::parse::parse_program;

fn inline(source: &str) -> String {
    try_inline(source).expect("inlined")
}

fn try_inline(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let declaration =
        var_function_or_declaration(&parsed.program.body[0]).expect("function declaration");
    let Statement::ExpressionStatement(stmt) = &parsed.program.body[1] else {
        panic!("expression statement");
    };
    let (_callee, arguments) = call_parts(&stmt.expression).expect("call");
    let builder = AstBuilder::new(&alloc);
    let inlined = inline_function_call(declaration.function, arguments, &builder)?;
    let mut codegen = Codegen::new();
    codegen.print_expression(&inlined);
    Some(codegen.into_source_text())
}

#[test]
fn parameters_are_substituted_positionally() {
    assert_eq!(
        inline("function w(a, b) { return b - 938; }\nw(1071, 1077);"),
        "1077 - 938"
    );
    assert_eq!(
        inline("function w(a, b) { return a(b); }\nw(f, 2);"),
        "f(2)"
    );
}

#[test]
fn a_missing_argument_becomes_void_zero() {
    assert_eq!(inline("function w(a, b) { return b; }\nw(1);"), "void 0");
}

#[test]
fn a_spread_argument_in_the_positional_form_is_not_inlined() {
    assert_eq!(
        try_inline("function w(a, b) { return a + b; }\nw(...xs);"),
        None
    );
}

#[test]
fn the_rest_form_forwards_the_spread() {
    assert_eq!(
        inline("function w(a, ...b) { return a(...b); }\nw(f, 1, 2);"),
        "f(1, 2)"
    );
}

#[test]
fn a_rest_taking_function_that_does_not_forward_is_not_inlined() {
    assert_eq!(
        try_inline("function w(a, ...b) { return a + b.length; }\nw(f, 1, 2);"),
        None
    );
}

#[test]
fn a_rest_call_to_a_callee_other_than_the_first_parameter_is_not_inlined() {
    assert_eq!(
        try_inline("function w(a, ...b) { return c(...b); }\nw(f, 1, 2);"),
        None
    );
}
