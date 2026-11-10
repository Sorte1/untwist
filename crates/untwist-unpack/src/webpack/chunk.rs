use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, LogicalOperator};
use untwist_core::ast_util::prop::{identifier_name, member_parts, number_key};

use crate::matcher::{
    argument_expression, call_parts, const_member, expression_statement, literal_id, Factory,
};
use crate::model::{Bodies, Bundle, BundleKind};
use crate::webpack::container::{is_module_container, module_factories};
use crate::webpack::extract::extract_modules_with;
use crate::webpack::gate::{routable, Prelude};
use crate::UnpackOptions;

const GLOBAL_PREFIX: &str = "webpack";
const REGISTER: &str = "O";

#[derive(PartialEq, Eq)]
enum GlobalObject<'k> {
    Identifier(&'k str),
    This,
}

fn global_object<'k>(expr: &'k Expression<'_>) -> Option<GlobalObject<'k>> {
    match expr {
        Expression::Identifier(id) => Some(GlobalObject::Identifier(id.name.as_str())),
        Expression::ThisExpression(_) => Some(GlobalObject::This),
        _ => None,
    }
}

fn jsonp_global<'k>(expr: &'k Expression<'_>) -> Option<(GlobalObject<'k>, Cow<'k, str>)> {
    let (object, property) = member_parts(expr)?;
    if !property.starts_with(GLOBAL_PREFIX) {
        return None;
    }
    Some((global_object(object)?, property))
}

fn jsonp_global_target<'k>(
    target: &'k AssignmentTarget<'_>,
) -> Option<(GlobalObject<'k>, Cow<'k, str>)> {
    let (object, property) = match target {
        AssignmentTarget::StaticMemberExpression(member) if !member.optional => {
            (&member.object, Cow::Borrowed(member.property.name.as_str()))
        }
        AssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
            match &member.expression {
                Expression::StringLiteral(literal) if !literal.value.has_lone_surrogate() => {
                    (&member.object, Cow::Borrowed(literal.value.as_str()?))
                }
                Expression::NumericLiteral(literal) => {
                    (&member.object, Cow::Owned(number_key(literal.value)))
                }
                _ => return None,
            }
        }
        _ => return None,
    };
    if !property.starts_with(GLOBAL_PREFIX) {
        return None;
    }
    Some((global_object(object)?, property))
}

fn is_jsonp_push_target(expr: &Expression<'_>) -> bool {
    let Expression::AssignmentExpression(assignment) = expr else {
        return false;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return false;
    }
    let Some(target) = jsonp_global_target(&assignment.left) else {
        return false;
    };
    let Expression::LogicalExpression(logical) = &assignment.right else {
        return false;
    };
    if logical.operator != LogicalOperator::Or {
        return false;
    }
    let empty_array =
        matches!(&logical.right, Expression::ArrayExpression(array) if array.elements.is_empty());
    empty_array && jsonp_global(&logical.left).is_some_and(|source| source == target)
}

fn chunk_ids(expr: &Expression<'_>) -> Option<Vec<String>> {
    let Expression::ArrayExpression(array) = expr else {
        return None;
    };
    array
        .elements
        .iter()
        .map(|element| element.as_expression().and_then(literal_id))
        .collect()
}

fn registered_entry_id(expr: &Expression<'_>) -> Option<String> {
    let (callee, arguments) = call_parts(expr)?;
    let (object, name) = member_parts(callee)?;
    if name != REGISTER {
        return None;
    }
    let require = identifier_name(object)?;
    let body = arguments.get(2).and_then(Argument::as_expression)?;
    required_id(body, require)
}

fn required_id(expr: &Expression<'_>, require: &str) -> Option<String> {
    let Expression::ArrowFunctionExpression(arrow) = expr else {
        let (callee, arguments) = call_parts(expr)?;
        if identifier_name(callee)? != require {
            return None;
        }
        return literal_id(arguments.first().and_then(argument_expression)?);
    };
    if let Some(inner) = arrow.body.as_expression() {
        return required_id(inner, require);
    }
    let body = arrow.body.as_function_body()?;
    body.statements.iter().find_map(|stmt| match stmt {
        Statement::ReturnStatement(ret) => ret
            .argument
            .as_ref()
            .and_then(|argument| required_id(argument, require)),
        Statement::ExpressionStatement(inner) => required_id(&inner.expression, require),
        _ => None,
    })
}

fn program_entry_id(source: &Program<'_>) -> Option<String> {
    source.body.iter().find_map(|stmt| {
        let expr = expression_statement(stmt)?;
        match expr {
            Expression::SequenceExpression(sequence) => {
                sequence.expressions.iter().find_map(registered_entry_id)
            }
            other => registered_entry_id(other),
        }
    })
}

fn chunk<'k, 'a>(expr: &'k Expression<'a>) -> Option<(Vec<String>, &'k Expression<'a>)> {
    let (callee, arguments) = call_parts(expr)?;
    if arguments.len() != 1 {
        return None;
    }
    if !const_member(callee, "push").is_some_and(is_jsonp_push_target) {
        return None;
    }
    let Expression::ArrayExpression(payload) = argument_expression(arguments.first()?)? else {
        return None;
    };
    if payload.elements.len() < 2 {
        return None;
    }
    let ids = chunk_ids(payload.elements.first()?.as_expression()?)?;
    let container = payload.elements.get(1)?.as_expression()?;
    is_module_container(container).then_some((ids, container))
}

pub fn is_webpack_chunk(expr: &Expression<'_>) -> bool {
    chunk(expr).is_some()
}

pub fn chunk_factories<'k, 'a>(expr: &'k Expression<'a>) -> Option<Vec<(String, Factory<'k, 'a>)>> {
    chunk(expr).map(|(_, container)| module_factories(container))
}

pub fn detect_webpack_chunk<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
) -> Option<Bundle<'a>> {
    detect_webpack_chunk_with(
        alloc,
        source,
        expr,
        &mut Bodies::Cloned,
        UnpackOptions::default(),
    )
}

pub fn detect_webpack_chunk_with<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    expr: &Expression<'a>,
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
) -> Option<Bundle<'a>> {
    let (ids, container) = chunk(expr)?;
    if !matches!(bodies, Bodies::Taken(_))
        && !routable(
            source.source_text,
            container,
            &Prelude {
                runtime: None,
                mentioned: Default::default(),
            },
            options,
        )
    {
        return None;
    }
    let mut bundle = Bundle::new(BundleKind::Webpack, None);
    bundle.chunk_ids = ids;
    (bundle.modules, bundle.fresh_requires) =
        extract_modules_with(alloc, source, (container, None), bodies, options);
    if bundle.entry_id.is_none() {
        bundle.entry_id = program_entry_id(source);
    }
    Some(bundle)
}
