use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::ast_util::code_text::{is_number, literal_text};

pub(crate) const NAME: &str = "name";

const KEYED_READERS: [&str; 2] = ["get", "getOwnPropertyDescriptor"];
const VALUE_READERS: [&str; 3] = ["values", "entries", "getOwnPropertyDescriptors"];
const MERGERS: [&str; 1] = ["assign"];

#[derive(Clone, Copy)]
pub(crate) enum Reader {
    Keyed,
    Values,
    Merge,
}

pub(crate) fn reader(name: &str) -> Option<Reader> {
    match name {
        _ if KEYED_READERS.contains(&name) => Some(Reader::Keyed),
        _ if VALUE_READERS.contains(&name) => Some(Reader::Values),
        _ if MERGERS.contains(&name) => Some(Reader::Merge),
        _ => None,
    }
}

pub(crate) fn is_never_a_function(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}

fn spells_no_text(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::UnaryExpression(unary) => matches!(
            unary.operator,
            UnaryOperator::Void | UnaryOperator::Typeof | UnaryOperator::LogicalNot
        ),
        Expression::BinaryExpression(binary) => {
            binary.operator.is_equality()
                || binary.operator.is_compare()
                || matches!(
                    binary.operator,
                    BinaryOperator::In | BinaryOperator::Instanceof
                )
        }
        _ => false,
    }
}

pub(crate) fn key_text<'e>(expr: &'e Expression<'_>) -> Option<Cow<'e, str>> {
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(Cow::Borrowed),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str())
            .map(Cow::Borrowed),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            literal_text(expr).map(Cow::Owned)
        }
        _ => None,
    }
}

pub(crate) fn key_may_name(key: &Expression<'_>) -> bool {
    !(is_number(key) || spells_no_text(key) || key_text(key).is_some_and(|text| text != NAME))
}

fn is_reading_name(name: &str) -> bool {
    name == NAME || reader(name).is_some()
}

pub(crate) fn property_key_may_name(key: &PropertyKey<'_>, computed: bool) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => is_reading_name(&id.name),
        PropertyKey::PrivateIdentifier(_) => false,
        _ => key.as_expression().is_none_or(|expr| match key_text(expr) {
            Some(text) => is_reading_name(&text),
            None => computed && key_may_name(expr),
        }),
    }
}

pub(crate) fn member_key<'e>(member: &'e MemberExpression<'_>) -> Option<Cow<'e, str>> {
    match member {
        MemberExpression::StaticMemberExpression(it) => Some(Cow::Borrowed(&it.property.name)),
        MemberExpression::ComputedMemberExpression(it) => key_text(&it.expression),
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn may_be_function(argument: Option<&Argument<'_>>) -> bool {
    argument.is_some_and(|argument| {
        argument
            .as_expression()
            .is_none_or(|expr| !is_never_a_function(expr))
    })
}

pub(crate) fn call_reads_name(reader: Reader, arguments: &[Argument<'_>]) -> bool {
    if arguments.iter().any(Argument::is_spread) {
        return true;
    }
    match reader {
        Reader::Keyed => {
            may_be_function(arguments.first())
                && arguments
                    .get(1)
                    .and_then(Argument::as_expression)
                    .is_some_and(key_may_name)
        }
        Reader::Values => may_be_function(arguments.first()),
        Reader::Merge => (1..arguments.len()).any(|at| may_be_function(arguments.get(at))),
    }
}

#[derive(Default)]
struct Counts {
    bindings: u32,
    numeric_bindings: u32,
    numeric_writes: u32,
}

#[derive(Default)]
pub(crate) struct CounterBook {
    counts: FxHashMap<SymbolId, Counts>,
}

pub(crate) fn numeric_assignment(operator: AssignmentOperator, right: &Expression<'_>) -> bool {
    match operator {
        AssignmentOperator::Assign | AssignmentOperator::Addition => is_number(right),
        operator => operator.is_arithmetic() || operator.is_bitwise(),
    }
}

pub(crate) fn numeric_declarator(it: &VariableDeclarator<'_>) -> Option<SymbolId> {
    match &it.id {
        BindingPattern::BindingIdentifier(id) if it.init.as_ref().is_none_or(is_number) => {
            id.symbol_id.get()
        }
        _ => None,
    }
}

pub(crate) fn head_bindings(left: &ForStatementLeft<'_>) -> Vec<SymbolId> {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return Vec::new();
    };
    declaration
        .declarations
        .iter()
        .filter_map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

impl CounterBook {
    fn entry(&mut self, symbol: SymbolId) -> &mut Counts {
        self.counts.entry(symbol).or_default()
    }

    pub(crate) fn bind(&mut self, symbol: SymbolId) {
        self.entry(symbol).bindings += 1;
    }

    pub(crate) fn bind_numeric(&mut self, symbol: SymbolId) {
        self.entry(symbol).numeric_bindings += 1;
    }

    pub(crate) fn write_numeric(&mut self, symbol: SymbolId) {
        self.entry(symbol).numeric_writes += 1;
    }

    pub(crate) fn is_counter(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        let Some(counts) = self.counts.get(&symbol) else {
            return false;
        };
        let writes = scoping
            .get_resolved_references(symbol)
            .filter(|reference| reference.is_write())
            .count();
        counts.bindings == counts.numeric_bindings
            && counts.numeric_bindings > 0
            && writes == counts.numeric_writes as usize
    }
}
