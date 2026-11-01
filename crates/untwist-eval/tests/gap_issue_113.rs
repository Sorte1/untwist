mod common;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program, Statement};
use oxc_span::SPAN;
use untwist_core::parse::parse_program;
use untwist_eval::decode::{Call, InterpEvaluator};
use untwist_eval::value::Value;

const UNIT: &str = "\
function a() { var h = ['aGVsbG8=']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var i = e[f]; return i.charAt(0); }; return b(c, d); }
var args = [0x0, 0x5];";

fn literal_args<'a>(program: &'a Program<'a>) -> Vec<&'a Expression<'a>> {
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        let Some(Expression::ArrayExpression(array)) = declaration
            .declarations
            .first()
            .and_then(|declarator| declarator.init.as_ref())
        else {
            continue;
        };
        return array
            .elements
            .iter()
            .filter_map(oxc_ast::ast::ArrayExpressionElement::as_expression)
            .collect();
    }
    Vec::new()
}

#[test]
fn issue_113_an_out_of_range_slot_bails() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, UNIT);
    let program = common::decoder_program(&parsed.program, "a", &["b"], None);
    let mut session = InterpEvaluator::default()
        .instantiate(&program)
        .expect("instantiates");

    assert!(session.element(0).is_some());
    assert!(session.element(5).is_none());

    let arguments = literal_args(&parsed.program);
    let first = Call {
        decoder: 0,
        span: SPAN,
        arguments: vec![arguments[0]],
    };
    let value = session.eval_call(&first).expect("no bail");
    assert!(matches!(&value, Value::Str(_)), "{value:?}");

    let second = Call {
        decoder: 0,
        span: SPAN,
        arguments: vec![arguments[1]],
    };
    assert!(session.eval_call(&second).is_err());
}
