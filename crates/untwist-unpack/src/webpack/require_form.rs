use oxc_ast::ast::*;

use crate::matcher::{const_member, single_param_name};

const EXPORTS: &str = "exports";
const LOADED: [&str; 2] = ["l", "loaded"];
const ID: [&str; 2] = ["i", "id"];

enum Step<'k, 'a> {
    Effect(&'k Expression<'a>),
    Bind(&'k str, &'k Expression<'a>),
    Guard(&'k Expression<'a>, &'k Expression<'a>),
    Return(&'k Expression<'a>),
}

fn sequence<'k, 'a>(expr: &'k Expression<'a>) -> Vec<&'k Expression<'a>> {
    match expr.without_parentheses() {
        Expression::SequenceExpression(list) => list
            .expressions
            .iter()
            .flat_map(sequence)
            .collect(),
        other => vec![other],
    }
}

fn only_return<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    let statement = match statement {
        Statement::BlockStatement(block) => match &block.body[..] {
            [only] => only,
            _ => return None,
        },
        other => other,
    };
    match statement {
        Statement::ReturnStatement(ret) => ret.argument.as_ref().map(Expression::without_parentheses),
        _ => None,
    }
}

fn steps<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<Vec<Step<'k, 'a>>> {
    let mut steps = Vec::new();
    for statement in statements {
        match statement {
            Statement::ExpressionStatement(statement) => {
                steps.extend(
                    sequence(&statement.expression)
                        .into_iter()
                        .map(|part| Step::Effect(part.without_parentheses())),
                );
            }
            Statement::ReturnStatement(ret) => {
                let mut parts = sequence(ret.argument.as_ref()?);
                let last = parts.pop()?;
                steps.extend(parts.into_iter().map(|part| Step::Effect(part.without_parentheses())));
                steps.push(Step::Return(last.without_parentheses()));
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                        return None;
                    };
                    steps.push(Step::Bind(id.name.as_str(), declarator.init.as_ref()?.without_parentheses()));
                }
            }
            Statement::IfStatement(branch) if branch.alternate.is_none() => {
                steps.push(Step::Guard(branch.test.without_parentheses(), only_return(&branch.consequent)?));
            }
            _ => return None,
        }
    }
    Some(steps)
}

fn name_of<'k>(expr: &'k Expression<'_>) -> Option<&'k str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn is_name(expr: &Expression<'_>, name: &str) -> bool {
    name_of(expr) == Some(name)
}

fn indexed<'k>(expr: &'k Expression<'_>, index: &str) -> Option<&'k str> {
    let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
        return None;
    };
    (!member.optional && is_name(&member.expression, index)).then(|| name_of(&member.object))?
}

fn exports_of(expr: &Expression<'_>, object: &str) -> bool {
    const_member(expr.without_parentheses(), EXPORTS).is_some_and(|inner| is_name(inner, object))
}

fn exports_of_slot(expr: &Expression<'_>, cache: &str, index: &str) -> bool {
    const_member(expr.without_parentheses(), EXPORTS).is_some_and(|inner| indexed(inner, index) == Some(cache))
}

fn is_undefined(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name == "undefined",
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

fn is_false(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(value) => !value.value,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::LogicalNot
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(one) if one.value == 1.0)
        }
        _ => false,
    }
}

fn is_true(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(value) => value.value,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::LogicalNot
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(zero) if zero.value == 0.0)
        }
        _ => false,
    }
}

fn defined(test: &Expression<'_>, cached: &str) -> bool {
    let Expression::BinaryExpression(compare) = test else {
        return false;
    };
    compare.operator == BinaryOperator::StrictInequality
        && ((is_name(&compare.left, cached) && is_undefined(&compare.right))
            || (is_name(&compare.right, cached) && is_undefined(&compare.left)))
}

struct Names<'k> {
    index: &'k str,
    require: &'k str,
    container: &'k str,
}

fn field_fits(key: &str, value: &Expression<'_>, names: &Names<'_>) -> bool {
    let value = value.without_parentheses();
    match key {
        EXPORTS => matches!(value, Expression::ObjectExpression(empty) if empty.properties.is_empty()),
        key if ID.contains(&key) => is_name(value, names.index),
        key if LOADED.contains(&key) => is_false(value),
        _ => false,
    }
}

fn fresh_record<'k>(expr: &'k Expression<'_>, names: &Names<'_>) -> Option<(&'k str, Vec<&'k str>)> {
    let Expression::AssignmentExpression(assignment) = expr else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::ComputedMemberExpression(slot) = &assignment.left else {
        return None;
    };
    if slot.optional || !is_name(&slot.expression, names.index) {
        return None;
    }
    let cache = name_of(&slot.object)?;
    let Expression::ObjectExpression(record) = assignment.right.without_parentheses() else {
        return None;
    };
    let mut keys = Vec::new();
    for property in &record.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.kind != PropertyKind::Init || property.method {
            return None;
        }
        let key = property.key.static_name()?;
        let key = [EXPORTS, ID[0], ID[1], LOADED[0], LOADED[1]]
            .into_iter()
            .find(|known| Some(*known) == key.as_str())?;
        if keys.contains(&key) || !field_fits(key, &property.value, names) {
            return None;
        }
        keys.push(key);
    }
    Some((cache, keys))
}

fn field_set<'k>(expr: &'k Expression<'_>, module: &str, cache: &str, names: &Names<'_>) -> Option<&'k str> {
    let Expression::AssignmentExpression(assignment) = expr else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
        return None;
    };
    let target = is_name(&member.object, module) || indexed(&member.object, names.index) == Some(cache);
    let key = member.property.name.as_str();
    let fits = assignment.operator == AssignmentOperator::Assign
        && target
        && !member.optional
        && field_fits(key, &assignment.right, names);
    fits.then_some(key)
}

fn dispatch(expr: &Expression<'_>, module: &str, names: &Names<'_>) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    if call.optional {
        return false;
    }
    let arguments: Vec<&Expression<'_>> = call
        .arguments
        .iter()
        .filter_map(Argument::as_expression)
        .collect();
    if arguments.len() != call.arguments.len() {
        return false;
    }
    let standard = |rest: &[&Expression<'_>]| {
        matches!(rest, [m, e, r] if is_name(m, module) && exports_of(e, module) && is_name(r, names.require))
    };
    if indexed(&call.callee, names.index) == Some(names.container) {
        return standard(&arguments);
    }
    let Some(target) = const_member(&call.callee, "call") else {
        return false;
    };
    indexed(target, names.index) == Some(names.container)
        && matches!(&arguments[..], [this, rest @ ..] if exports_of(this, module) && standard(rest))
}

fn loaded(expr: &Expression<'_>, module: &str) -> bool {
    let Expression::AssignmentExpression(assignment) = expr else {
        return false;
    };
    let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
        return false;
    };
    assignment.operator == AssignmentOperator::Assign
        && is_name(&member.object, module)
        && LOADED.contains(&member.property.name.as_str())
        && is_true(&assignment.right)
}

fn cached_return<'s, 'k, 'a>(steps: &'s [Step<'k, 'a>], names: &Names<'_>) -> Option<(&'k str, &'s [Step<'k, 'a>])> {
    match steps {
        [Step::Guard(test, value), rest @ ..] => {
            let cache = indexed(test, names.index)?;
            exports_of_slot(value, cache, names.index).then_some((cache, rest))
        }
        [Step::Bind(cached, read), Step::Guard(test, value), rest @ ..] => {
            let cache = indexed(read, names.index)?;
            (defined(test, cached) && exports_of(value, cached)).then_some((cache, rest))
        }
        _ => None,
    }
}

pub fn caching_cache<'k>(function: &'k Function<'_>, require: &str, container: &str) -> Option<&'k str> {
    if function.generator || function.r#async {
        return None;
    }
    let index = single_param_name(&function.params)?;
    let body = function.body.as_deref()?;
    let steps = steps(&body.statements)?;
    let names = Names {
        index,
        require,
        container,
    };
    let (cache, rest) = cached_return(&steps, &names)?;
    let [Step::Bind(module, record), rest @ ..] = rest else {
        return None;
    };
    let (record_cache, mut keys) = fresh_record(record, &names)?;
    let mut rest = rest;
    while let [Step::Effect(set), tail @ ..] = rest {
        let Some(key) = field_set(set, module, cache, &names) else {
            break;
        };
        if keys.contains(&key) {
            return None;
        }
        keys.push(key);
        rest = tail;
    }
    let [Step::Effect(call), middle @ .., Step::Return(result)] = rest else {
        return None;
    };
    let distinct = ![index, require, container, cache].contains(module) && cache != index;
    let fits = distinct
        && record_cache == cache
        && keys.contains(&EXPORTS)
        && dispatch(call, module, &names)
        && exports_of(result, module)
        && match middle {
            [] => true,
            [Step::Effect(flag)] => loaded(flag, module),
            _ => false,
        };
    fits.then_some(cache)
}
