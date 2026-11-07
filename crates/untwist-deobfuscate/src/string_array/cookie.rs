use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator, UpdateOperator};
use untwist_core::ast_util::matcher::{call_parts, iife, statement_iife};
use untwist_core::ast_util::prop::{identifier_name, member_parts};

use super::legacy::is_countdown;

const FORMAT_CHECK: &str = "\\w+ *\\(\\) *{\\w+ *['|\"].+['|\"];? *}";

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_quote(c: char) -> bool {
    matches!(c, '\'' | '|' | '"')
}

fn is_line_end(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn skip(text: &[char], mut at: usize, wanted: impl Fn(char) -> bool) -> usize {
    while text.get(at).copied().is_some_and(&wanted) {
        at += 1;
    }
    at
}

fn literal(text: &[char], at: usize, c: char) -> Option<usize> {
    (text.get(at) == Some(&c)).then_some(at + 1)
}

fn closes_at(text: &[char], at: usize) -> bool {
    let at = literal(text, at, ';').unwrap_or(at);
    literal(text, skip(text, at, |c| c == ' '), '}').is_some()
}

fn format_check_at(text: &[char], start: usize) -> bool {
    let word = skip(text, start, is_word);
    if word == start {
        return false;
    }
    let Some(at) = literal(text, skip(text, word, |c| c == ' '), '(')
        .and_then(|at| literal(text, at, ')'))
        .and_then(|at| literal(text, skip(text, at, |c| c == ' '), '{'))
    else {
        return false;
    };
    let word = skip(text, at, is_word);
    if word == at {
        return false;
    }
    let at = skip(text, word, |c| c == ' ');
    if !text.get(at).copied().is_some_and(is_quote) {
        return false;
    }
    let body = at + 1;
    let line = skip(text, body, |c| !is_line_end(c));
    (body + 1..line).any(|end| is_quote(text[end]) && closes_at(text, end + 1))
}

pub(super) fn passes_format_check(source: &str) -> bool {
    let text: Vec<char> = source.chars().collect();
    (0..text.len()).any(|start| format_check_at(&text, start))
}

fn property<'k, 'a>(object: &'k ObjectExpression<'a>, name: &str) -> Option<&'k Function<'a>> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        let key = property.key.static_name();
        if property.computed || key.as_ref().and_then(|key| key.as_str()) != Some(name) {
            return None;
        }
        match &property.value {
            Expression::FunctionExpression(function) => Some(&**function),
            _ => None,
        }
    })
}

fn declared<'k, 'a>(stmt: &'k Statement<'a>) -> Option<(&'k str, &'k Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.name.as_str(), declarator.init.as_ref()?))
}

fn method_call<'k, 'a>(
    expr: &'k Expression<'a>,
    holder: &str,
    name: &str,
) -> Option<&'k [Argument<'a>]> {
    let (callee, arguments) = call_parts(expr.without_parentheses())?;
    let (object, key) = member_parts(callee.without_parentheses())?;
    (identifier_name(object) == Some(holder) && key == name).then_some(arguments.as_slice())
}

fn tests_removal_text(function: &Function<'_>, holder: &str) -> bool {
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return false;
    };
    let Some(argument) = ret.argument.as_ref() else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(argument.without_parentheses()) else {
        return false;
    };
    let Some((pattern, key)) = member_parts(callee.without_parentheses()) else {
        return false;
    };
    let Expression::NewExpression(regexp) = pattern.without_parentheses() else {
        return false;
    };
    let [source] = regexp.arguments.as_slice() else {
        return false;
    };
    let fixed = matches!(source.as_expression(), Some(Expression::StringLiteral(text)) if text.value == FORMAT_CHECK);
    let [tested] = arguments.as_slice() else {
        return false;
    };
    let Some(tested) = tested.as_expression() else {
        return false;
    };
    let Some((removal, to_string)) = call_parts(tested.without_parentheses())
        .filter(|(_, arguments)| arguments.is_empty())
        .and_then(|(callee, _)| member_parts(callee.without_parentheses()))
    else {
        return false;
    };
    let reads_removal =
        member_parts(removal.without_parentheses()).is_some_and(|(object, name)| {
            identifier_name(object) == Some(holder) && name == "removeCookie"
        });
    identifier_name(&regexp.callee) == Some("RegExp")
        && fixed
        && key == "test"
        && to_string == "toString"
        && reads_removal
}

fn counted_call(set: &Expression<'_>, call: &Expression<'_>, rotate: &str) -> Option<usize> {
    let Expression::AssignmentExpression(set) = set.without_parentheses() else {
        return None;
    };
    let Expression::CallExpression(call) = call.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(counter) = &set.left else {
        return None;
    };
    let Expression::NumericLiteral(count) = &set.right else {
        return None;
    };
    let [argument] = call.arguments.as_slice() else {
        return None;
    };
    let Some(Expression::UpdateExpression(update)) = argument.as_expression() else {
        return None;
    };
    let SimpleAssignmentTarget::AssignmentTargetIdentifier(bumped) = &update.argument else {
        return None;
    };
    let whole = count.value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&count.value);
    let shaped = set.operator == AssignmentOperator::Assign
        && update.prefix
        && update.operator == UpdateOperator::Increment
        && bumped.name == counter.name
        && identifier_name(&call.callee) == Some(rotate)
        && whole;
    shaped.then_some(count.value as usize)
}

fn tail_expressions<'k, 'a>(statements: &'k [Statement<'a>]) -> Option<Vec<&'k Expression<'a>>> {
    let (Statement::ReturnStatement(ret), before) = statements.split_last()? else {
        return None;
    };
    let mut tail = Vec::new();
    for stmt in before {
        let Statement::ExpressionStatement(stmt) = stmt else {
            return None;
        };
        tail.push(&stmt.expression);
    }
    match ret.argument.as_ref()?.without_parentheses() {
        Expression::SequenceExpression(sequence) => tail.extend(sequence.expressions.iter()),
        other => tail.push(other),
    }
    Some(tail)
}

fn pattern_text(expr: &Expression<'_>, key: &str) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_) => true,
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            pattern_text(&binary.left, key) && pattern_text(&binary.right, key)
        }
        other => call_parts(other).is_some_and(|(callee, arguments)| {
            let escapes = matches!(arguments.as_slice(), [pattern, replacement]
                if matches!(pattern.as_expression(), Some(Expression::RegExpLiteral(_)))
                && matches!(replacement.as_expression(), Some(Expression::StringLiteral(_))));
            member_parts(callee.without_parentheses()).is_some_and(|(object, name)| {
                identifier_name(object) == Some(key) && name == "replace"
            }) && escapes
        }),
    }
}

fn identity(expr: &Expression<'_>) -> bool {
    let Expression::FunctionExpression(function) = expr.without_parentheses() else {
        return false;
    };
    let Some(param) = function
        .params
        .items
        .first()
        .and_then(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
    else {
        return false;
    };
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    function.params.items.len() == 1
        && matches!(body.statements.as_slice(), [Statement::ReturnStatement(ret)]
            if ret.argument.as_ref().and_then(identifier_name) == Some(param))
}

fn safe_lookup(stmt: &Statement<'_>, (matcher, key): (&str, &str)) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    let init = declaration
        .declarations
        .iter()
        .filter_map(|declarator| declarator.init.as_ref())
        .collect::<Vec<_>>();
    let [init] = init.as_slice() else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(init.without_parentheses()) else {
        return false;
    };
    let Expression::AssignmentExpression(defaulted) = callee.without_parentheses() else {
        return false;
    };
    let Expression::LogicalExpression(fallback) = defaulted.right.without_parentheses() else {
        return false;
    };
    let target = match &defaulted.left {
        AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    };
    let Some(Expression::NewExpression(regexp)) =
        arguments.first().and_then(Argument::as_expression)
    else {
        return false;
    };
    target == Some(matcher)
        && identifier_name(&fallback.left) == Some(matcher)
        && identity(&fallback.right)
        && arguments.len() == 1
        && identifier_name(&regexp.callee) == Some("RegExp")
        && matches!(regexp.arguments.as_slice(), [source]
            if source.as_expression().is_some_and(|source| pattern_text(source, key)))
}

fn rotation_in_reader(function: &Function<'_>, rotate: &str) -> Option<usize> {
    let names: Vec<&str> = function
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect();
    let [matcher, key] = names.as_slice() else {
        return None;
    };
    let body = function.body.as_ref()?;
    let statements = body.statements.as_slice();
    let split = statements
        .iter()
        .position(|stmt| !matches!(stmt, Statement::VariableDeclaration(_)))?;
    let (lookups, rest) = statements.split_at(split);
    if !lookups.iter().all(|stmt| match stmt {
        Statement::VariableDeclaration(declaration) => {
            declaration
                .declarations
                .iter()
                .all(|declarator| declarator.init.is_none())
                || safe_lookup(stmt, (matcher, key))
        }
        _ => false,
    }) {
        return None;
    }
    let tail = tail_expressions(rest)?;
    let [set, call, _] = tail.as_slice() else {
        return None;
    };
    counted_call(set, call, rotate)
}

fn chosen_by_check<'k, 'a>(stmt: &'k Statement<'a>, check: &str, holder: &str) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Expression::ConditionalExpression(outer) = stmt.expression.without_parentheses() else {
        return false;
    };
    let Expression::ConditionalExpression(inner) = outer.consequent.without_parentheses() else {
        return false;
    };
    let reads = method_call(&inner.consequent, holder, "getCookie").is_some_and(|arguments| {
        matches!(arguments, [first, second]
            if matches!(first.as_expression(), Some(Expression::NullLiteral(_)))
            && matches!(second.as_expression(), Some(Expression::StringLiteral(_))))
    });
    identifier_name(&outer.test) == Some(check)
        && identifier_name(&inner.test) == Some(check)
        && reads
        && method_call(&inner.alternate, holder, "removeCookie").is_some()
        && method_call(&outer.alternate, holder, "setCookie").is_some()
}

fn guarded_rotation(inner: &Function<'_>, rotate: &str, source: &str) -> Option<usize> {
    let body = inner.body.as_ref()?;
    let [object, update, check, chosen] = body.statements.as_slice() else {
        return None;
    };
    let (holder, Expression::ObjectExpression(cookie)) = declared(object)? else {
        return None;
    };
    let Statement::ExpressionStatement(update) = update else {
        return None;
    };
    let Expression::AssignmentExpression(assigned) = update.expression.without_parentheses() else {
        return None;
    };
    let target = assigned
        .left
        .as_simple_assignment_target()?
        .as_member_expression()?;
    let updated = identifier_name(target.object()) == Some(holder)
        && target.static_property_name().and_then(|name| name.as_str()) == Some("updateCookie");
    let Expression::FunctionExpression(format_check) = &assigned.right else {
        return None;
    };
    let (flag, init) = declared(check)?;
    let checked = method_call(init, holder, "updateCookie").is_some_and(<[_]>::is_empty);
    if !updated || !checked || !tests_removal_text(format_check, holder) {
        return None;
    }
    if !chosen_by_check(chosen, flag, holder) {
        return None;
    }
    let removal = property(cookie, "removeCookie")?;
    let reader = property(cookie, "getCookie")?;
    let count = rotation_in_reader(reader, rotate)?;
    let text = source.get(removal.span.start as usize..removal.span.end as usize)?;
    passes_format_check(text).then_some(count)
}

fn references(scoping: &Scoping, function: &Function<'_>, name: &str) -> Option<usize> {
    let scope = function.scope_id.get()?;
    let symbol = scoping.find_binding(scope, name.into())?;
    Some(scoping.get_resolved_reference_ids(symbol).len())
}

pub(super) fn cookie_rotation(
    stmt: &Statement<'_>,
    array: &str,
    (scoping, source): (&Scoping, &str),
) -> Option<usize> {
    let Statement::ExpressionStatement(outer) = stmt else {
        return None;
    };
    let call = match outer.expression.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    let (function, arguments) = iife(call)?;
    let first = arguments
        .first()?
        .as_expression()
        .and_then(identifier_name)?;
    let BindingPattern::BindingIdentifier(target) = &function.params.items.first()?.pattern else {
        return None;
    };
    if first != array || function.params.rest.is_some() || arguments.len() > 2 {
        return None;
    }
    let body = function.body.as_ref()?;
    let [rotator, guard] = body.statements.as_slice() else {
        return None;
    };
    let (rotate, Expression::FunctionExpression(countdown)) = declared(rotator)? else {
        return None;
    };
    let (inner, inner_arguments) = statement_iife(guard)?;
    let target = target.name.as_str();
    let sealed = is_countdown(countdown, target)
        && inner_arguments.is_empty()
        && inner.params.items.is_empty()
        && references(scoping, function, rotate) == Some(1)
        && references(scoping, function, target) == Some(2);
    if !sealed {
        return None;
    }
    guarded_rotation(inner, rotate, source)
}

#[cfg(test)]
mod tests {
    use super::passes_format_check;

    #[test]
    fn the_format_check_accepts_the_minified_template_only() {
        assert!(passes_format_check("function(){return'dev';}"));
        assert!(passes_format_check("function () {return \"dev\";}"));
        assert!(!passes_format_check("function () {\n  return 'dev';\n}"));
        assert!(!passes_format_check("function(){return 1;}"));
    }
}
