use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;

use super::values::{is_naming_property, is_word_like, key_name, method_name};
use crate::human::looks_human_written;

const PREDICATE: &str = "predicate";

const BOOLEAN_METHODS: [&str; 10] = [
    "has",
    "includes",
    "hasOwnProperty",
    "test",
    "startsWith",
    "endsWith",
    "some",
    "every",
    "contains",
    "isArray",
];

const BOOLEAN_GLOBALS: [&str; 2] = ["isNaN", "isFinite"];

const MAX_RETURNS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgName {
    Named(String),
    Mixed,
}

#[derive(Default)]
pub struct CallFacts {
    pub calls: usize,
    pub constructed: usize,
    pub args: Vec<ArgName>,
}

impl CallFacts {
    pub fn record(&mut self, arguments: &[Argument<'_>], constructed: bool) {
        let names: Vec<ArgName> = argument_names(arguments);
        if self.calls == 0 {
            self.args = names;
        } else {
            for (index, slot) in self.args.iter_mut().enumerate() {
                if names.get(index) != Some(slot) {
                    *slot = ArgName::Mixed;
                }
            }
        }
        self.calls += 1;
        if constructed {
            self.constructed += 1;
        }
    }
}

fn argument_names(arguments: &[Argument<'_>]) -> Vec<ArgName> {
    let mut names = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let Some(expr) = argument.as_expression() else {
            break;
        };
        names.push(argument_name(expr).map_or(ArgName::Mixed, ArgName::Named));
    }
    names
}

fn argument_name(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            (looks_human_written(name) && is_word_like(name) && name.len() > 2)
                .then(|| name.to_string())
        }
        Expression::StaticMemberExpression(member) => {
            let name = member.property.name.as_str();
            is_naming_property(name).then(|| name.to_string())
        }
        _ => None,
    }
}

pub struct FunctionFacts {
    pub params: Vec<Option<SymbolId>>,
    pub predicate: Option<String>,
}

fn simple(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

pub fn param_symbols(params: &FormalParameters<'_>) -> Vec<Option<SymbolId>> {
    params
        .items
        .iter()
        .map(|param| simple(&param.pattern))
        .collect()
}

pub fn function_facts(function: &Function<'_>) -> FunctionFacts {
    let predicate = function
        .body
        .as_ref()
        .filter(|_| !function.generator && !function.r#async)
        .and_then(|body| predicate_name(&body.statements));
    FunctionFacts {
        params: param_symbols(&function.params),
        predicate,
    }
}

pub fn arrow_facts(arrow: &ArrowFunctionExpression<'_>) -> FunctionFacts {
    let predicate = if arrow.r#async {
        None
    } else {
        match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => predicate_name(&body.statements),
            body => body.as_expression().and_then(boolean_name),
        }
    };
    FunctionFacts {
        params: param_symbols(&arrow.params),
        predicate,
    }
}

fn predicate_name(statements: &[Statement<'_>]) -> Option<String> {
    if !matches!(statements.last(), Some(Statement::ReturnStatement(_))) {
        return None;
    }
    let mut returns = Vec::new();
    if !collect_returns(statements, &mut returns) || returns.is_empty() {
        return None;
    }
    let mut specific: Option<String> = None;
    for expr in returns {
        let name = boolean_name(expr)?;
        if name == PREDICATE {
            continue;
        }
        match &specific {
            Some(current) if *current != name => return Some(PREDICATE.to_string()),
            _ => specific = Some(name),
        }
    }
    Some(specific.unwrap_or_else(|| PREDICATE.to_string()))
}

fn collect_returns<'k, 'a>(
    statements: &'k [Statement<'a>],
    out: &mut Vec<&'k Expression<'a>>,
) -> bool {
    statements
        .iter()
        .all(|statement| collect_from(statement, out))
}

fn collect_from<'k, 'a>(statement: &'k Statement<'a>, out: &mut Vec<&'k Expression<'a>>) -> bool {
    if out.len() > MAX_RETURNS {
        return false;
    }
    match statement {
        Statement::ReturnStatement(statement) => match &statement.argument {
            Some(argument) => {
                out.push(argument);
                true
            }
            None => false,
        },
        Statement::BlockStatement(block) => collect_returns(&block.body, out),
        Statement::IfStatement(statement) => {
            collect_from(&statement.consequent, out)
                && statement
                    .alternate
                    .as_ref()
                    .is_none_or(|alternate| collect_from(alternate, out))
        }
        Statement::TryStatement(_)
        | Statement::SwitchStatement(_)
        | Statement::LabeledStatement(_)
        | Statement::WithStatement(_) => false,
        Statement::ForStatement(_)
        | Statement::ForInStatement(_)
        | Statement::ForOfStatement(_)
        | Statement::WhileStatement(_)
        | Statement::DoWhileStatement(_) => !has_return(statement),
        _ => true,
    }
}

fn has_return(statement: &Statement<'_>) -> bool {
    let body = match statement {
        Statement::ForStatement(statement) => &statement.body,
        Statement::ForInStatement(statement) => &statement.body,
        Statement::ForOfStatement(statement) => &statement.body,
        Statement::WhileStatement(statement) => &statement.body,
        Statement::DoWhileStatement(statement) => &statement.body,
        _ => return true,
    };
    let mut found = Vec::new();
    !collect_from(body, &mut found) || !found.is_empty()
}

fn title(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn is_name(word: &str) -> Option<String> {
    let base = key_name(word)?;
    Some(format!("is{}", title(&base)))
}

fn boolean_name(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(_) => Some(PREDICATE.to_string()),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            Some(PREDICATE.to_string())
        }
        Expression::BinaryExpression(binary) => binary_name(binary),
        Expression::LogicalExpression(logical) => {
            if logical.operator == LogicalOperator::Coalesce {
                return None;
            }
            let left = boolean_name(&logical.left)?;
            boolean_name(&logical.right)?;
            Some(left)
        }
        Expression::CallExpression(call) => call_name(call),
        _ => None,
    }
}

fn binary_name(binary: &BinaryExpression<'_>) -> Option<String> {
    match binary.operator {
        BinaryOperator::Instanceof => match binary.right.without_parentheses() {
            Expression::Identifier(id) if looks_human_written(&id.name) => is_name(&id.name),
            Expression::StaticMemberExpression(member) => is_name(&member.property.name),
            _ => Some(PREDICATE.to_string()),
        },
        BinaryOperator::StrictEquality | BinaryOperator::Equality => {
            Some(compared_string(binary).unwrap_or_else(|| PREDICATE.to_string()))
        }
        operator if operator.is_compare() || operator.is_equality() => Some(PREDICATE.to_string()),
        BinaryOperator::In => Some(PREDICATE.to_string()),
        _ => None,
    }
}

fn compared_string(binary: &BinaryExpression<'_>) -> Option<String> {
    let literal = match (&binary.left, &binary.right) {
        (Expression::StringLiteral(literal), _) | (_, Expression::StringLiteral(literal)) => {
            literal.value.as_str()?
        }
        _ => return None,
    };
    is_name(literal)
}

fn call_name(call: &CallExpression<'_>) -> Option<String> {
    if let Expression::Identifier(id) = call.callee.without_parentheses() {
        return BOOLEAN_GLOBALS
            .contains(&id.name.as_str())
            .then(|| PREDICATE.to_string());
    }
    let method = method_name(&call.callee)?;
    if !BOOLEAN_METHODS.contains(&method.as_ref()) {
        return None;
    }
    if method == "isArray" {
        return Some("isArray".to_string());
    }
    Some(PREDICATE.to_string())
}
