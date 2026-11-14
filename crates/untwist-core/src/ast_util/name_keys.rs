use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

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

struct Counters<'s> {
    scoping: &'s Scoping,
    counts: FxHashMap<SymbolId, Counts>,
}

pub(crate) fn numeric_counters(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut counters = Counters {
        scoping,
        counts: FxHashMap::default(),
    };
    counters.visit_program(program);
    counters
        .counts
        .into_iter()
        .filter(|(symbol, counts)| {
            let writes = scoping
                .get_resolved_references(*symbol)
                .filter(|reference| reference.is_write())
                .count();
            counts.bindings == counts.numeric_bindings
                && counts.numeric_bindings > 0
                && writes == counts.numeric_writes as usize
        })
        .map(|(symbol, _)| symbol)
        .collect()
}

fn numeric_assignment(operator: AssignmentOperator, right: &Expression<'_>) -> bool {
    match operator {
        AssignmentOperator::Assign | AssignmentOperator::Addition => is_number(right),
        operator => operator.is_arithmetic() || operator.is_bitwise(),
    }
}

impl Counters<'_> {
    fn entry(&mut self, symbol: Option<SymbolId>) -> Option<&mut Counts> {
        symbol.map(|symbol| self.counts.entry(symbol).or_default())
    }

    fn referenced(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn numeric_write(&mut self, target: Option<&IdentifierReference<'_>>) {
        let symbol = target.and_then(|id| self.referenced(id));
        if let Some(counts) = self.entry(symbol) {
            counts.numeric_writes += 1;
        }
    }
}

impl<'a> VisitJs<'a> for Counters<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(counts) = self.entry(it.symbol_id.get()) {
            counts.bindings += 1;
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            if it.init.as_ref().is_none_or(is_number) {
                if let Some(counts) = self.entry(id.symbol_id.get()) {
                    counts.numeric_bindings += 1;
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.visit_for_head(&it.left);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.visit_for_head(&it.left);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if numeric_assignment(it.operator, &it.right) {
                self.numeric_write(Some(id));
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &it.argument {
            self.numeric_write(Some(id));
        }
        walk_js::walk_update_expression(self, it);
    }
}

impl Counters<'_> {
    fn visit_for_head(&mut self, left: &ForStatementLeft<'_>) {
        let ForStatementLeft::VariableDeclaration(declaration) = left else {
            return;
        };
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                if let Some(counts) = self.entry(id.symbol_id.get()) {
                    counts.bindings += 1;
                }
            }
        }
    }
}
