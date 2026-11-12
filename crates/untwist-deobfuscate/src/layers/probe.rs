use oxc_ast::ast::{Argument, Expression, Program, Statement};

use super::constructor::is_function_constructor;
use super::model::LayerForm;

const PACKER_ARITY: usize = 6;

const GLOBAL_OBJECTS: [&str; 2] = ["window", "globalThis"];

#[derive(Clone, Copy)]
pub struct Probe<'p, 'a> {
    pub form: LayerForm,
    pub expression: &'p Expression<'a>,
}

pub fn probe<'p, 'a>(program: &'p Program<'a>) -> Option<Probe<'p, 'a>> {
    let expression = only_expression(program)?;
    let form = whole_program_form(expression)?;
    Some(Probe { form, expression })
}

fn only_expression<'p, 'a>(program: &'p Program<'a>) -> Option<&'p Expression<'a>> {
    let mut found = None;
    for statement in &program.body {
        match statement {
            Statement::EmptyStatement(_) => {}
            Statement::ExpressionStatement(only) if found.is_none() => {
                found = Some(&only.expression);
            }
            _ => return None,
        }
    }
    found
}

fn whole_program_form(expression: &Expression<'_>) -> Option<LayerForm> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    if is_eval_callee(&call.callee) {
        let packed = call.arguments.first().is_some_and(is_packer_call);
        return Some(if packed {
            LayerForm::Packer
        } else {
            LayerForm::Eval
        });
    }
    is_function_constructor_call(&call.callee).then_some(LayerForm::Function)
}

fn is_eval_callee(callee: &Expression<'_>) -> bool {
    match callee.without_parentheses() {
        Expression::Identifier(id) => id.name == "eval",
        Expression::SequenceExpression(seq) => is_indirect_eval(&seq.expressions),
        Expression::StaticMemberExpression(member) => {
            member.property.name == "eval" && is_global_object(&member.object)
        }
        _ => false,
    }
}

fn is_indirect_eval(expressions: &[Expression<'_>]) -> bool {
    let [first, Expression::Identifier(last)] = expressions else {
        return false;
    };
    last.name == "eval" && first.is_literal()
}

fn is_global_object(object: &Expression<'_>) -> bool {
    match object.without_parentheses() {
        Expression::Identifier(id) => GLOBAL_OBJECTS.contains(&id.name.as_str()),
        Expression::ThisExpression(_) => true,
        _ => false,
    }
}

fn is_function_constructor_call(callee: &Expression<'_>) -> bool {
    let inner = match callee.without_parentheses() {
        Expression::CallExpression(inner) => &inner.callee,
        Expression::NewExpression(inner) => &inner.callee,
        _ => return false,
    };
    is_function_constructor(inner)
}

fn is_packer_call(argument: &Argument<'_>) -> bool {
    let Some(Expression::CallExpression(call)) = argument
        .as_expression()
        .map(Expression::without_parentheses)
    else {
        return false;
    };
    call.arguments.len() == PACKER_ARITY
        && matches!(
            call.callee.without_parentheses(),
            Expression::FunctionExpression(function) if function.params.items.len() == PACKER_ARITY
        )
}

#[cfg(test)]
mod tests {
    use super::probe;
    use crate::layers::model::LayerForm;
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;

    fn form_of(source: &str) -> Option<LayerForm> {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        probe(&parsed.program).map(|found| found.form)
    }

    #[test]
    fn each_whole_program_call_shape_is_probed_as_its_form() {
        let rows = [
            ("eval('x');", Some(LayerForm::Eval)),
            (";eval('x');;", Some(LayerForm::Eval)),
            ("'use strict'; eval('x')", Some(LayerForm::Eval)),
            ("(0, eval)('x')", Some(LayerForm::Eval)),
            ("window.eval('x')", Some(LayerForm::Eval)),
            ("globalThis.eval('x')", Some(LayerForm::Eval)),
            ("this.eval('x')", Some(LayerForm::Eval)),
            ("Function('x')()", Some(LayerForm::Function)),
            ("new Function('a', 'x')(1)", Some(LayerForm::Function)),
            (
                "[]['filter']['constructor']('x')()",
                Some(LayerForm::Function),
            ),
            (
                "[].constructor.constructor('x')()",
                Some(LayerForm::Function),
            ),
            (
                "(0)['constructor']['constructor']('x')()",
                Some(LayerForm::Function),
            ),
            (
                "''.constructor.constructor('x')()",
                Some(LayerForm::Function),
            ),
            (
                "({}).constructor.constructor('x')()",
                Some(LayerForm::Function),
            ),
            (
                "[][(!1+[])[0]+'ilter'].constructor('x')()",
                Some(LayerForm::Function),
            ),
            (
                "eval(function(p,a,c,k,e,d){return p}('x',2,1,'x'.split('|'),0,{}))",
                Some(LayerForm::Packer),
            ),
        ];
        for (source, form) in rows {
            assert_eq!(form_of(source), form, "{source}");
        }
    }

    #[test]
    fn anything_else_is_no_layer() {
        let rows = [
            "",
            "eval('x'); f();",
            "var a = eval('x');",
            "x.eval('x')",
            "(1, 2, eval)('x')",
            "(f, eval)('x')",
            "Function('x')",
            "new Function('x')",
            "f(eval('x'))",
            "(function(){ eval('x') })()",
            "evil('x')",
            "eval`x`",
            "[1]['filter']['constructor']('x')()",
            "[f()]['filter']['constructor']('x')()",
            "({a: 1}).constructor.constructor('x')()",
            "[]['nope']['constructor']('x')()",
            "[]['filter']['call']('x')()",
            "x['filter']['constructor']('x')()",
            "[][f()]['constructor']('x')()",
            "[]?.filter.constructor('x')()",
            "[].constructor('x')()",
        ];
        for source in rows {
            assert_eq!(form_of(source), None, "{source}");
        }
    }
}
