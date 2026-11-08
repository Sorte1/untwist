use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::{call_parts, statement_iife};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::respan::fresh_spans_statement;
use untwist_core::parse::{has_error, parse_with, SourceType};
use untwist_core::pass::Cx;

use super::legacy::{literal_arithmetic, parameters};
use crate::array_rotator::is_rotator_statement;

fn assigns_a_function(expr: &Expression<'_>, name: &str) -> bool {
    let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
        return false;
    };
    target.name == name
        && assign.operator == AssignmentOperator::Assign
        && matches!(
            assign.right.without_parentheses(),
            Expression::FunctionExpression(_)
        )
}

fn calls(expr: &Expression<'_>, name: &str) -> bool {
    call_parts(expr.without_parentheses())
        .is_some_and(|(callee, _)| identifier_name(callee) == Some(name))
}

pub(super) fn reassigns_itself(stmt: &Statement<'_>) -> bool {
    let Statement::FunctionDeclaration(function) = stmt else {
        return false;
    };
    let (Some(id), Some(body)) = (function.id.as_ref(), function.body.as_ref()) else {
        return false;
    };
    let name = id.name.as_str();
    match body.statements.as_slice() {
        [Statement::ReturnStatement(ret)] => {
            let Some(Expression::SequenceExpression(sequence)) =
                ret.argument.as_ref().map(Expression::without_parentheses)
            else {
                return false;
            };
            matches!(sequence.expressions.as_slice(),
                [assign, call] if assigns_a_function(assign, name) && calls(call, name))
        }
        [Statement::ExpressionStatement(assign), Statement::ReturnStatement(ret)] => {
            assigns_a_function(&assign.expression, name)
                && ret.argument.as_ref().is_some_and(|call| calls(call, name))
        }
        _ => false,
    }
}

pub(super) fn checksum_rotator(
    stmt: &Statement<'_>,
    name: &str,
    scoping: &Scoping,
    symbol: SymbolId,
) -> Option<Span> {
    if !is_rotator_statement(stmt, name) {
        return None;
    }
    let (function, arguments) = statement_iife(stmt)?;
    let (target, _) = parameters(function)?;
    if function.id.is_some() || target == name || sees_its_parameters(function) {
        return None;
    }
    let [first, second] = arguments.as_slice() else {
        return None;
    };
    let Some(Expression::Identifier(id)) = first.as_expression() else {
        return None;
    };
    let reference = id.reference_id.get()?;
    if scoping.get_reference(reference).symbol_id() != Some(symbol) {
        return None;
    }
    second
        .as_expression()
        .and_then(literal_arithmetic)
        .map(|_| stmt.span())
}

#[derive(Default)]
struct Names {
    seen: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for Names {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.seen.insert(it.name.to_string());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.seen.insert(it.name.to_string());
    }
}

fn without_parentheses_mut<'k, 'a>(expr: &'k mut Expression<'a>) -> &'k mut Expression<'a> {
    let negated = matches!(
        expr,
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot
    );
    if !negated && !matches!(expr, Expression::ParenthesizedExpression(_)) {
        return expr;
    }
    match expr {
        Expression::ParenthesizedExpression(inner) => {
            without_parentheses_mut(&mut inner.expression)
        }
        Expression::UnaryExpression(unary) => without_parentheses_mut(&mut unary.argument),
        other => other,
    }
}

fn rotator_function<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut Function<'a>> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let Expression::CallExpression(call) = without_parentheses_mut(&mut statement.expression)
    else {
        return None;
    };
    match without_parentheses_mut(&mut call.callee) {
        Expression::FunctionExpression(function) => Some(function),
        _ => None,
    }
}

fn sees_its_parameters(function: &Function<'_>) -> bool {
    let mut names = Names::default();
    names.visit_function(function, ScopeFlags::Function);
    ["arguments", "eval"]
        .iter()
        .any(|name| names.seen.contains(*name))
}

fn unused_name(function: &Function<'_>, base: &str) -> Option<String> {
    let mut names = Names::default();
    names.visit_function(function, ScopeFlags::Function);
    (0..64)
        .map(|n| format!("{base}{n}"))
        .find(|candidate| !names.seen.contains(candidate))
}

pub(super) fn parsed_statement<'a>(text: &str, cx: &Cx<'a, '_>) -> Option<Statement<'a>> {
    let source = cx.allocator().alloc_str(text);
    let parsed = parse_with(cx.allocator(), source, SourceType::cjs());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let mut program = parsed.program;
    let mut statement = program.body.pop()?;
    fresh_spans_statement(&mut statement).then_some(statement)
}

pub(super) fn call_the_array_first<'a>(stmt: &mut Statement<'a>, cx: &Cx<'a, '_>) -> bool {
    let Some(function) = rotator_function(stmt) else {
        return false;
    };
    let Some(BindingPattern::BindingIdentifier(param)) =
        function.params.items.first().map(|param| &param.pattern)
    else {
        return false;
    };
    let target = param.name.to_string();
    let Some(fresh) = unused_name(function, &target) else {
        return false;
    };
    let Some(created) = parsed_statement(&format!("var {target} = {fresh}();"), cx) else {
        return false;
    };
    let (Some(body), Some(param)) = (function.body.as_mut(), function.params.items.first_mut())
    else {
        return false;
    };
    let BindingPattern::BindingIdentifier(id) = &mut param.pattern else {
        return false;
    };
    id.name = Ident::from_str_in(&fresh, cx);
    body.statements.insert(0, created);
    true
}

pub(super) fn hoist_array_call<'a>(
    stmt: &mut Statement<'a>,
    name: &str,
    cx: &Cx<'a, '_>,
) -> Option<String> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    let fresh = unused_name(function, name)?;
    let mut created = parsed_statement(&format!("var {fresh} = {name}();"), cx)?;
    let body = function.body.as_mut()?;
    let inside = Span::new(body.span.start, body.span.start + 1);
    let Statement::VariableDeclaration(declaration) = &mut created else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &mut declaration.declarations.first_mut()?.id
    else {
        return None;
    };
    id.span = inside;
    body.statements.insert(0, created);
    Some(fresh)
}

struct EditAt<F> {
    span: Span,
    edit: Option<F>,
}

impl<'a, F: FnOnce(&mut Statement<'a>)> VisitMut<'a> for EditAt<F> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.edit.is_none() {
            return;
        }
        if it.span() == self.span {
            if let Some(edit) = self.edit.take() {
                edit(it);
            }
            return;
        }
        walk_mut::walk_statement(self, it);
    }
}

pub(super) fn edit_statement<'a>(
    program: &mut Program<'a>,
    span: Span,
    edit: impl FnOnce(&mut Statement<'a>),
) {
    EditAt {
        span,
        edit: Some(edit),
    }
    .visit_program(program);
}
