use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use untwist_core::pass::Cx;

use super::class_ctor::{field_of, field_run, simple_params};
use super::identify::Table;

pub struct Moved {
    pub derived: bool,
    pub fields: usize,
}

struct Run {
    at: usize,
    start: usize,
    count: usize,
}

fn constructor_of<'k, 'a>(element: &'k ClassElement<'a>) -> Option<&'k Function<'a>> {
    match element {
        ClassElement::MethodDefinition(method)
            if method.kind == MethodDefinitionKind::Constructor =>
        {
            Some(&method.value)
        }
        _ => None,
    }
}

fn super_call<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k CallExpression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    match &statement.expression {
        Expression::CallExpression(call)
            if !call.optional && matches!(call.callee, Expression::Super(_)) =>
        {
            Some(call)
        }
        _ => None,
    }
}

fn is_unresolved_arguments(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    matches!(expression, Expression::Identifier(id)
        if id.name.as_str() == "arguments"
            && id.reference_id.get().is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none()))
}

fn forwards_arguments(statement: &Statement<'_>, scoping: &Scoping) -> bool {
    super_call(statement).is_some_and(|call| {
        matches!(call.arguments.as_slice(), [Argument::SpreadElement(spread)]
            if is_unresolved_arguments(&spread.argument, scoping))
    })
}

fn run_of(class: &Class<'_>, table: &Table, scoping: &Scoping) -> Option<Run> {
    let (at, function) = class
        .body
        .body
        .iter()
        .enumerate()
        .find_map(|(at, element)| constructor_of(element).map(|function| (at, function)))?;
    let statements = function.body.as_ref()?.statements.as_slice();
    let scope = function.scope_id.get()?;
    let start = if class.heritage.is_some() {
        super_call(statements.first()?)?;
        1
    } else {
        if !simple_params(function) {
            return None;
        }
        0
    };
    let count = field_run(statements, start, None, None, scope, table, scoping);
    (count > 0).then_some(Run { at, start, count })
}

fn is_instance_field(element: &ClassElement<'_>) -> bool {
    match element {
        ClassElement::PropertyDefinition(property) => !property.r#static,
        ClassElement::AccessorProperty(property) => !property.r#static,
        _ => false,
    }
}

fn insert_at(body: &ClassBody<'_>, at: usize) -> usize {
    body.body
        .iter()
        .rposition(is_instance_field)
        .filter(|index| *index > at)
        .map_or(at, |index| index + 1)
}

fn take_fields<'a>(
    function: &mut Function<'a>,
    run: &Run,
    derived: bool,
    cx: &Cx<'a, '_>,
) -> (AVec<'a, ClassElement<'a>>, bool) {
    let mut fields = AVec::with_capacity_in(run.count, cx);
    let Some(body) = function.body.as_mut() else {
        return (fields, false);
    };
    for statement in body.statements.drain(run.start..run.start + run.count) {
        fields.extend(field_of(statement, cx));
    }
    let no_params = function.params.items.is_empty() && function.params.rest.is_none();
    let statements = body.statements.as_slice();
    let default = no_params
        && body.directives.is_empty()
        && match (derived, statements) {
            (false, []) => true,
            (true, [only]) => forwards_arguments(only, cx.scoping()),
            _ => false,
        };
    (fields, default)
}

pub fn rewrite<'a>(class: &mut Class<'a>, table: &Table, cx: &Cx<'a, '_>) -> Option<Moved> {
    let run = run_of(class, table, cx.scoping())?;
    let derived = class.heritage.is_some();
    let insert = insert_at(&class.body, run.at);
    let ClassElement::MethodDefinition(method) = class.body.body.get_mut(run.at)? else {
        return None;
    };
    let (fields, default) = take_fields(&mut method.value, &run, derived, cx);
    let moved = fields.len();
    let old = std::mem::replace(&mut class.body.body, AVec::with_capacity_in(0, cx));
    let mut body = AVec::with_capacity_in(old.len() + moved, cx);
    let mut fields = Some(fields);
    for (index, element) in old.into_iter().enumerate() {
        if index == insert {
            body.extend(fields.take().into_iter().flatten());
        }
        if !(default && index == run.at) {
            body.push(element);
        }
    }
    body.extend(fields.take().into_iter().flatten());
    class.body.body = body;
    Some(Moved {
        derived,
        fields: moved,
    })
}

pub fn has_run(class: &Class<'_>, table: &Table, scoping: &Scoping) -> bool {
    run_of(class, table, scoping).is_some()
}
