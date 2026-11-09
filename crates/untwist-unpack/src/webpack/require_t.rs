use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::GetSpan;
use oxc_str::Ident;

use crate::matcher::{argument_expression, const_member};
use crate::model::Module;

pub const PASS_THROUGH: f64 = 9.0;

fn is_require_t(expr: &Expression<'_>) -> bool {
    matches!(const_member(expr, "t"), Some(Expression::Identifier(id)) if id.name == "require")
}

fn literal_id(argument: Option<&Argument<'_>>) -> bool {
    matches!(
        argument.and_then(argument_expression),
        Some(Expression::NumericLiteral(_) | Expression::StringLiteral(_))
    )
}

fn mode(argument: Option<&Argument<'_>>) -> Option<f64> {
    match argument.and_then(argument_expression)? {
        Expression::NumericLiteral(literal) => Some(literal.value),
        _ => None,
    }
}

fn is_null(argument: Option<&Argument<'_>>) -> bool {
    matches!(
        argument.and_then(argument_expression),
        Some(Expression::NullLiteral(_))
    )
}

pub enum Form {
    Call,
    Bind,
}

pub fn form(call: &CallExpression<'_>) -> Option<(Form, f64)> {
    if call.optional {
        return None;
    }
    let arguments = &call.arguments;
    if is_require_t(&call.callee) && arguments.len() == 2 && literal_id(arguments.first()) {
        return Some((Form::Call, mode(arguments.get(1))?));
    }
    let object = const_member(&call.callee, "bind")?;
    let bound = is_require_t(object)
        && arguments.len() == 3
        && is_null(arguments.first())
        && literal_id(arguments.get(1));
    bound.then_some((Form::Bind, mode(arguments.get(2))?))
}

pub fn subject<'c, 'a>(call: &'c CallExpression<'a>, form: &Form) -> Option<&'c Expression<'a>> {
    match form {
        Form::Call => const_member(&call.callee, "t"),
        Form::Bind => const_member(const_member(&call.callee, "bind")?, "t"),
    }
}

struct Convert<'a, 'n> {
    builder: AstBuilder<'a>,
    runtime: Option<&'n str>,
    used: bool,
}

fn require<'a>(builder: &AstBuilder<'a>, span: oxc_span::Span) -> Expression<'a> {
    Expression::new_identifier(span, Ident::from_str_in("require", builder), builder)
}

pub fn pass_through<'a>(builder: &AstBuilder<'a>, call: &mut CallExpression<'a>, form: &Form) {
    match form {
        Form::Call => {
            call.callee = require(builder, call.callee.span());
            call.arguments.truncate(1);
        }
        Form::Bind => {
            if let Expression::StaticMemberExpression(bind) = &mut call.callee {
                bind.object = require(builder, bind.object.span());
            }
            call.arguments.truncate(2);
        }
    }
}

impl<'a> Convert<'a, '_> {
    fn send_to_runtime(&mut self, call: &mut CallExpression<'a>, form: &Form, runtime: &str) {
        let helper = match (form, &mut call.callee) {
            (Form::Call, callee) => callee,
            (Form::Bind, Expression::StaticMemberExpression(bind)) => &mut bind.object,
            (Form::Bind, _) => return,
        };
        if let Expression::StaticMemberExpression(member) = helper {
            if let Expression::Identifier(object) = &mut member.object {
                object.name = Ident::from_str_in(runtime, &self.builder);
                self.used = true;
            }
        }
    }
}

impl<'a> VisitMut<'a> for Convert<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        walk_mut::walk_call_expression(self, it);
        let Some((form, mode)) = form(it) else {
            return;
        };
        if mode == PASS_THROUGH {
            pass_through(&self.builder, it, &form);
        } else if let Some(runtime) = self.runtime {
            self.send_to_runtime(it, &form, runtime);
        }
    }
}

pub fn convert_require_t<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    runtime: Option<&str>,
) -> bool {
    let mut convert = Convert {
        builder: AstBuilder::new(alloc),
        runtime,
        used: false,
    };
    convert.visit_program(module.program_mut());
    convert.used
}
