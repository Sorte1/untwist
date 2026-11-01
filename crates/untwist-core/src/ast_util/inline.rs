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

pub fn param_names<'k, 'a>(function: &'k Function<'a>) -> Option<Vec<&'k str>> {
    if function.params.rest.is_some() {
        return None;
    }
    let mut names = Vec::with_capacity(function.params.items.len());
    for item in &function.params.items {
        let BindingPattern::BindingIdentifier(id) = &item.pattern else {
            return None;
        };
        names.push(id.name.as_str());
    }
    Some(names)
}

pub fn rest_param<'k, 'a>(function: &'k Function<'a>) -> Option<(&'k str, &'k str)> {
    let rest = function.params.rest.as_ref()?;
    if function.params.items.len() != 1 {
        return None;
    }
    let BindingPattern::BindingIdentifier(first) = &function.params.items[0].pattern else {
        return None;
    };
    let BindingPattern::BindingIdentifier(name) = &rest.rest.argument else {
        return None;
    };
    Some((first.name.as_str(), name.name.as_str()))
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

pub fn inline_function_call<'p, 'a>(
    function: &Function<'p>,
    arguments: &AVec<'p, Argument<'p>>,
    b: &impl GetAstBuilder<'a>,
) -> Option<Expression<'a>> {
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
