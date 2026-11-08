use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::search::statements_at;

use super::legacy::decoder_function;

fn strings(stmt: &Statement<'_>) -> Option<Vec<String>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let Some(Expression::ArrayExpression(array)) = declaration.declarations.first()?.init.as_ref()
    else {
        return None;
    };
    array
        .elements
        .iter()
        .map(|element| match element.as_expression()? {
            Expression::StringLiteral(text) => text.value.as_str().map(str::to_string),
            _ => None,
        })
        .collect()
}

fn literal_number(expr: &Expression<'_>) -> Option<f64> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(number) => Some(number.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
            Some(-literal_number(&unary.argument)?)
        }
        Expression::StringLiteral(text) => {
            let text = text.value.as_str()?.trim();
            match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                Some(hex) => i64::from_str_radix(hex, 16).ok().map(|value| value as f64),
                None => text.parse::<i64>().ok().map(|value| value as f64),
            }
        }
        _ => None,
    }
}

fn is_param(expr: &Expression<'_>, param: &str) -> bool {
    identifier_name(expr) == Some(param)
}

fn shift_of(expr: &Expression<'_>, param: &str) -> Option<f64> {
    match expr.without_parentheses() {
        other if is_param(other, param) => Some(0.0),
        Expression::AssignmentExpression(assignment) => {
            let target = matches!(&assignment.left,
                AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == param);
            if !target {
                return None;
            }
            match (assignment.operator, assignment.right.without_parentheses()) {
                (AssignmentOperator::Subtraction, right) => literal_number(right),
                (AssignmentOperator::Assign, Expression::UnaryExpression(unary))
                    if unary.operator == UnaryOperator::UnaryPlus
                        && is_param(&unary.argument, param) =>
                {
                    Some(0.0)
                }
                (AssignmentOperator::Assign, Expression::BinaryExpression(binary))
                    if binary.operator == BinaryOperator::Subtraction
                        && is_param(&binary.left, param) =>
                {
                    literal_number(&binary.right)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn read_index(expr: &Expression<'_>, array: &str, param: &str) -> Option<f64> {
    let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
        return None;
    };
    (identifier_name(&member.object) == Some(array)).then(|| shift_of(&member.expression, param))?
}

fn plain_offset(function: &Function<'_>, array: &str) -> Option<f64> {
    let BindingPattern::BindingIdentifier(param) = &function.params.items.first()?.pattern else {
        return None;
    };
    let param = param.name.as_str();
    let statements = function.body.as_ref()?.statements.as_slice();
    let (shift, rest) = match statements {
        [Statement::ExpressionStatement(first), rest @ ..] => {
            (shift_of(&first.expression, param)?, rest)
        }
        rest => (0.0, rest),
    };
    let read = match rest {
        [Statement::ReturnStatement(ret)] => read_index(ret.argument.as_ref()?, array, param)?,
        [Statement::VariableDeclaration(declaration), Statement::ReturnStatement(ret)] => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            if ret.argument.as_ref().and_then(identifier_name) != Some(id.name.as_str()) {
                return None;
            }
            read_index(declarator.init.as_ref()?, array, param)?
        }
        _ => return None,
    };
    Some(shift + read)
}

fn decoder_symbol(stmt: &Statement<'_>) -> Option<SymbolId> {
    match stmt {
        Statement::FunctionDeclaration(function) => function.id.as_ref()?.symbol_id.get(),
        Statement::VariableDeclaration(declaration) => {
            match &declaration.declarations.first()?.id {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            }
        }
        _ => None,
    }
}

struct Calls<'s> {
    scoping: &'s Scoping,
    decoder: SymbolId,
    strings: Vec<String>,
    offset: f64,
    found: FxHashMap<Span, String>,
}

impl<'a> VisitJs<'a> for Calls<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let calls_decoder = match it.callee.without_parentheses() {
            Expression::Identifier(id) => {
                id.reference_id
                    .get()
                    .and_then(|r| self.scoping.get_reference(r).symbol_id())
                    == Some(self.decoder)
            }
            _ => false,
        };
        let index = it
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(literal_number)
            .map(|value| value - self.offset);
        if let Some(index) = index.filter(|_| calls_decoder) {
            let whole = index >= 0.0 && index.fract() == 0.0;
            if let Some(text) = self.strings.get(index as usize).filter(|_| whole) {
                self.found.insert(it.span, text.clone());
            }
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub(super) fn plain_decoded_keys(
    program: &Program<'_>,
    scoping: &Scoping,
    (array, count, decoder): (Span, usize, Span),
) -> FxHashMap<Span, String> {
    let found = statements_at(program, &[array, decoder]);
    let (Some(Some(array)), Some(Some(decoder))) = (found.first(), found.get(1)) else {
        return FxHashMap::default();
    };
    let (Some(mut strings), Some(name)) = (strings(array), array_binding(array)) else {
        return FxHashMap::default();
    };
    let (Some(function), Some(symbol)) = (decoder_function(decoder), decoder_symbol(decoder))
    else {
        return FxHashMap::default();
    };
    let Some(offset) = plain_offset(function, name) else {
        return FxHashMap::default();
    };
    if !strings.is_empty() {
        let shift = count % strings.len();
        strings.rotate_left(shift);
    }
    let mut calls = Calls {
        scoping,
        decoder: symbol,
        strings,
        offset,
        found: FxHashMap::default(),
    };
    calls.visit_program(program);
    calls.found
}

fn array_binding<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    match &declaration.declarations.first()?.id {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}
