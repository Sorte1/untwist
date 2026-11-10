use oxc_allocator::{Allocator, CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::{AstBuilder, GetAstBuilder};
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::SPAN;

use crate::ast_util::literal::undefined_expression;

pub fn returned_expression<'k, 'a>(function: &'k Function<'a>) -> Option<&'k Expression<'a>> {
    let body = function.body.as_ref()?;
    if body.statements.len() != 1 {
        return None;
    }
    let Statement::ReturnStatement(statement) = body.statements.first()? else {
        return None;
    };
    statement.argument.as_ref()
}

pub fn is_plain_function(function: &Function<'_>) -> bool {
    !function.r#async && !function.generator
}

fn simple_name<'k>(item: &'k FormalParameter<'_>) -> Option<&'k str> {
    if item.initializer.is_some() {
        return None;
    }
    match &item.pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

pub fn param_names<'k, 'a>(function: &'k Function<'a>) -> Option<Vec<&'k str>> {
    if function.params.rest.is_some() {
        return None;
    }
    function.params.items.iter().map(simple_name).collect()
}

pub fn rest_param<'k, 'a>(function: &'k Function<'a>) -> Option<(&'k str, &'k str)> {
    let rest = function.params.rest.as_ref()?;
    let [first] = function.params.items.as_slice() else {
        return None;
    };
    let first = simple_name(first)?;
    let BindingPattern::BindingIdentifier(name) = &rest.rest.argument else {
        return None;
    };
    Some((first, name.name.as_str()))
}

fn forwards_rest(function: &Function<'_>) -> bool {
    let Some((first, rest)) = rest_param(function) else {
        return false;
    };
    let Some(Expression::CallExpression(call)) = returned_expression(function) else {
        return false;
    };
    if call.optional || call.arguments.len() != 1 {
        return false;
    }
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    let Some(Argument::SpreadElement(spread)) = call.arguments.first() else {
        return false;
    };
    let Expression::Identifier(argument) = &spread.argument else {
        return false;
    };
    callee.name == first && argument.name == rest
}

#[derive(Default)]
struct ReceiverProbe {
    found: bool,
}

impl<'a> VisitJs<'a> for ReceiverProbe {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.found {
            return;
        }
        if matches!(
            it,
            Expression::ThisExpression(_)
                | Expression::NewTarget(_)
                | Expression::Super(_)
                | Expression::ImportMeta(_)
        ) {
            self.found = true;
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name.as_str() == "arguments" {
            self.found = true;
        }
    }
}

pub fn reads_the_callee_receiver(expr: &Expression<'_>) -> bool {
    let mut probe = ReceiverProbe::default();
    probe.visit_expression(expr);
    probe.found
}

struct Substitute<'p, 'a, 's> {
    names: &'s [&'s str],
    arguments: &'s AVec<'p, Argument<'p>>,
    allocator: &'a Allocator,
}

impl<'a> VisitMut<'a> for Substitute<'_, 'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let Expression::Identifier(id) = &*it else {
            walk_mut::walk_expression(self, it);
            return;
        };
        let Some(index) = self.names.iter().position(|name| *name == id.name.as_str()) else {
            walk_mut::walk_expression(self, it);
            return;
        };
        let builder = AstBuilder::new(self.allocator);
        *it = match self.arguments.get(index).and_then(Argument::as_expression) {
            Some(argument) => argument.clone_in(self.allocator),
            None => undefined_expression(SPAN, &builder),
        };
    }
}

fn forwards_in_order(call: &CallExpression<'_>, names: &[&str]) -> bool {
    let Some((first, rest)) = names.split_first() else {
        return false;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    callee.name == *first
        && call.arguments.len() == rest.len()
        && call.arguments.iter().zip(rest).all(
            |(argument, name)| matches!(argument, Argument::Identifier(id) if id.name == *name),
        )
}

pub fn inline_spread_forward<'p, 'a>(
    function: &Function<'p>,
    arguments: &AVec<'p, Argument<'p>>,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
    if !is_plain_function(function) {
        return None;
    }
    let names = param_names(function)?;
    let (last, leading) = arguments.split_last()?;
    let spread_last = matches!(last, Argument::SpreadElement(_))
        && !leading
            .iter()
            .any(|argument| matches!(argument, Argument::SpreadElement(_)));
    if !spread_last || leading.is_empty() || arguments.len() != names.len() {
        return None;
    }
    let Some(Expression::CallExpression(call)) = returned_expression(function) else {
        return None;
    };
    if call.optional || !forwards_in_order(call, &names) {
        return None;
    }
    let builder = b.builder();
    let allocator = builder.allocator();
    let callee = leading.first()?.as_expression()?.clone_in(allocator);
    let forwarded: AVec<'a, Argument<'a>> = AVec::from_iter_in(
        arguments.iter().skip(1).map(|a| a.clone_in(allocator)),
        builder,
    );
    Some(Expression::new_call_expression(
        SPAN, callee, None, forwarded, false, builder,
    ))
}

pub fn inline_function_call<'p, 'a>(
    function: &Function<'p>,
    arguments: &AVec<'p, Argument<'p>>,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
    if !is_plain_function(function) {
        return None;
    }
    let builder = b.builder();
    let allocator = builder.allocator();
    if rest_param(function).is_some() {
        if !forwards_rest(function) {
            return None;
        }
        let callee = arguments.first()?.as_expression()?.clone_in(allocator);
        let rest: AVec<'a, Argument<'a>> = AVec::from_iter_in(
            arguments.iter().skip(1).map(|a| a.clone_in(allocator)),
            builder,
        );
        return Some(Expression::new_call_expression(
            SPAN, callee, None, rest, false, builder,
        ));
    }

    if arguments
        .iter()
        .any(|argument| matches!(argument, Argument::SpreadElement(_)))
    {
        return None;
    }

    let names = param_names(function)?;
    let returned = returned_expression(function)?;
    if reads_the_callee_receiver(returned) {
        return None;
    }
    let mut clone: Expression<'a> = returned.clone_in(allocator);
    let mut substitute = Substitute {
        names: &names,
        arguments,
        allocator,
    };
    substitute.visit_expression(&mut clone);
    Some(clone)
}
