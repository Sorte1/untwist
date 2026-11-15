use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast_util::code_text::{is_number, literal_text};

pub(crate) const NAME: &str = "name";

const PROTO: &str = "__proto__";
const ARGUMENTS: &str = "arguments";

const CHAIN_READERS: [&str; 1] = ["get"];
const OWN_KEYED_READERS: [&str; 1] = ["getOwnPropertyDescriptor"];
const VALUE_READERS: [&str; 3] = ["values", "entries", "getOwnPropertyDescriptors"];
const MERGERS: [&str; 1] = ["assign"];

#[derive(Clone, Copy)]
pub(crate) enum Reader {
    Chain,
    OwnKeyed,
    Values,
    Merge,
}

pub(crate) fn reader(name: &str) -> Option<Reader> {
    match name {
        _ if CHAIN_READERS.contains(&name) => Some(Reader::Chain),
        _ if OWN_KEYED_READERS.contains(&name) => Some(Reader::OwnKeyed),
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

fn sets_prototype<'p, 'a>(property: &'p ObjectPropertyKind<'a>) -> Option<&'p Expression<'a>> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    let proto_key = match &property.key {
        PropertyKey::StaticIdentifier(id) => id.name == PROTO,
        PropertyKey::StringLiteral(literal) => literal.value == PROTO,
        _ => false,
    };
    let setter = property.kind == PropertyKind::Init
        && !property.computed
        && !property.shorthand
        && !property.method
        && proto_key;
    setter.then_some(&property.value)
}

pub(crate) fn never_reaches_a_function(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NullLiteral(_) => true,
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .filter_map(sets_prototype)
            .all(never_reaches_a_function),
        _ => false,
    }
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

fn may_reach_function(argument: Option<&Argument<'_>>) -> bool {
    argument.is_some_and(|argument| {
        argument
            .as_expression()
            .is_none_or(|expr| !never_reaches_a_function(expr))
    })
}

fn keyed_by_name(arguments: &[Argument<'_>]) -> bool {
    arguments
        .get(1)
        .and_then(Argument::as_expression)
        .is_some_and(key_may_name)
}

pub(crate) fn call_reads_name(reader: Reader, arguments: &[Argument<'_>]) -> bool {
    if arguments.iter().any(Argument::is_spread) {
        return true;
    }
    match reader {
        Reader::Chain => may_reach_function(arguments.first()) && keyed_by_name(arguments),
        Reader::OwnKeyed => may_be_function(arguments.first()) && keyed_by_name(arguments),
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
    block_functions: FxHashSet<String>,
}

pub(crate) fn counter_symbols(
    scoping: &Scoping,
    key: &Expression<'_>,
    found: &mut Vec<SymbolId>,
) -> bool {
    let mut shaped = Vec::new();
    let counter = counter_shaped(scoping, key, &mut shaped);
    if counter {
        found.extend(shaped);
    }
    counter
}

fn counter_shaped(scoping: &Scoping, key: &Expression<'_>, found: &mut Vec<SymbolId>) -> bool {
    match key.without_parentheses() {
        Expression::Identifier(id) => {
            let symbol = id
                .reference_id
                .get()
                .and_then(|reference| scoping.get_reference(reference).symbol_id());
            symbol.map(|symbol| found.push(symbol)).is_some()
        }
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            [&binary.left, &binary.right]
                .into_iter()
                .all(|operand| is_number(operand) || counter_shaped(scoping, operand, found))
        }
        _ => false,
    }
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

    pub(crate) fn block_function(&mut self, name: &str) {
        self.block_functions.insert(name.to_string());
    }

    fn written_unseen(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        let name = scoping.symbol_name(symbol);
        scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
            || name == ARGUMENTS
            || self.block_functions.contains(name)
    }

    pub(crate) fn all_counters(&self, scoping: &Scoping, symbols: &[SymbolId]) -> bool {
        symbols.is_empty()
            || (Self::every_write_seen(scoping)
                && symbols
                    .iter()
                    .all(|symbol| self.is_counter(scoping, *symbol)))
    }

    fn every_write_seen(scoping: &Scoping) -> bool {
        !scoping
            .scope_descendants_from_root()
            .any(|scope| scoping.scope_flags(scope).contains_direct_eval())
    }

    pub(crate) fn is_counter(&self, scoping: &Scoping, symbol: SymbolId) -> bool {
        let Some(counts) = self.counts.get(&symbol) else {
            return false;
        };
        if self.written_unseen(scoping, symbol) {
            return false;
        }
        let writes = scoping
            .get_resolved_references(symbol)
            .filter(|reference| reference.is_write())
            .count();
        counts.bindings == counts.numeric_bindings
            && counts.numeric_bindings > 0
            && writes == counts.numeric_writes as usize
    }
}
