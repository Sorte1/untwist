use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::reach::REACHING_KEYS;

const PLAIN_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];

const STRING_METHODS: [&str; 30] = [
    "split",
    "match",
    "matchAll",
    "exec",
    "concat",
    "join",
    "slice",
    "substr",
    "substring",
    "charAt",
    "at",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "replace",
    "replaceAll",
    "repeat",
    "padStart",
    "padEnd",
    "trim",
    "trimStart",
    "trimEnd",
    "normalize",
    "fromCharCode",
    "fromCodePoint",
    "raw",
    "toString",
    "reverse",
    "toLocaleString",
];

const STRING_FUNCTIONS: [&str; 9] = [
    "String",
    "atob",
    "btoa",
    "unescape",
    "escape",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Class {
    Plain,
    Opaque,
    Spelling,
}

pub(super) fn classify(
    scoping: &Scoping,
    expression: &Expression<'_>,
    deps: &mut Vec<SymbolId>,
) -> Class {
    let mut both = |left: &Expression<'_>, right: &Expression<'_>| {
        classify(scoping, left, deps).max(classify(scoping, right, deps))
    };
    match expression.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::UpdateExpression(_)
        | Expression::UnaryExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => Class::Plain,
        Expression::BinaryExpression(binary) if binary.operator != BinaryOperator::Addition => {
            Class::Plain
        }
        Expression::BinaryExpression(binary)
            if is_numeric_shape(&binary.left) || is_numeric_shape(&binary.right) =>
        {
            Class::Plain
        }
        Expression::BinaryExpression(_) => Class::Spelling,
        Expression::ConditionalExpression(conditional) => {
            both(&conditional.consequent, &conditional.alternate)
        }
        Expression::LogicalExpression(logical) => both(&logical.left, &logical.right),
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .last()
            .map_or(Class::Plain, |last| classify(scoping, last, deps)),
        Expression::AssignmentExpression(assignment) => classify(scoping, &assignment.right, deps),
        Expression::Identifier(id) => {
            match id
                .reference_id
                .get()
                .and_then(|reference| scoping.get_reference(reference).symbol_id())
            {
                Some(symbol) => {
                    deps.push(symbol);
                    Class::Plain
                }
                None if PLAIN_GLOBALS.contains(&id.name.as_str()) => Class::Plain,
                None => Class::Opaque,
            }
        }
        Expression::StaticMemberExpression(member) if is_global_symbol(scoping, &member.object) => {
            Class::Plain
        }
        Expression::StaticMemberExpression(member) => {
            classify(scoping, &member.object, deps);
            Class::Opaque
        }
        Expression::ComputedMemberExpression(member) => {
            classify(scoping, &member.object, deps);
            Class::Opaque
        }
        Expression::CallExpression(call) if produces_strings(&call.callee) => Class::Spelling,
        Expression::StringLiteral(literal) => text_class(literal.value.as_str()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => text_class(
            template
                .quasis
                .first()
                .and_then(|quasi| quasi.value.cooked.as_ref())
                .and_then(|cooked| cooked.as_str()),
        ),
        Expression::TemplateLiteral(_)
        | Expression::TaggedTemplateExpression(_)
        | Expression::ArrayExpression(_)
        | Expression::ObjectExpression(_) => Class::Spelling,
        _ => Class::Opaque,
    }
}

pub(super) fn is_numeric_shape(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UpdateExpression(_) => true,
        Expression::UnaryExpression(unary) => matches!(
            unary.operator,
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
        ),
        Expression::BinaryExpression(binary) => {
            binary.operator.is_arithmetic() && binary.operator != BinaryOperator::Addition
                || binary.operator.is_bitwise()
        }
        _ => false,
    }
}

fn text_class(text: Option<&str>) -> Class {
    match text {
        Some(text) if !REACHING_KEYS.contains(&text) => Class::Plain,
        _ => Class::Spelling,
    }
}

fn is_global_symbol(scoping: &Scoping, object: &Expression<'_>) -> bool {
    matches!(object.without_parentheses(), Expression::Identifier(id)
        if id.name == "Symbol"
            && id
                .reference_id
                .get()
                .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none()))
}

fn produces_strings(callee: &Expression<'_>) -> bool {
    match callee.without_parentheses() {
        Expression::Identifier(id) => STRING_FUNCTIONS.contains(&id.name.as_str()),
        Expression::StaticMemberExpression(member) => {
            STRING_METHODS.contains(&member.property.name.as_str())
                || FORWARDERS.contains(&member.property.name.as_str())
        }
        Expression::ComputedMemberExpression(_) => true,
        _ => false,
    }
}

const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];

pub(super) fn spelling_symbols(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut writes = Writes {
        scoping,
        spelling: FxHashSet::default(),
        deps: Vec::new(),
    };
    writes.visit_program(program);
    let Writes {
        mut spelling, deps, ..
    } = writes;
    loop {
        let before = spelling.len();
        for (symbol, dep) in &deps {
            if spelling.contains(dep) {
                spelling.insert(*symbol);
            }
        }
        if spelling.len() == before {
            return spelling;
        }
    }
}

struct Writes<'s> {
    scoping: &'s Scoping,
    spelling: FxHashSet<SymbolId>,
    deps: Vec<(SymbolId, SymbolId)>,
}

impl Writes<'_> {
    fn apply(&mut self, symbols: &[SymbolId], value: &Expression<'_>) {
        let mut deps = Vec::new();
        let class = classify(self.scoping, value, &mut deps);
        self.apply_class(symbols, class, &deps);
    }

    fn apply_class(&mut self, symbols: &[SymbolId], class: Class, deps: &[SymbolId]) {
        for symbol in symbols {
            if class == Class::Spelling {
                self.spelling.insert(*symbol);
            }
            self.deps.extend(deps.iter().map(|dep| (*symbol, *dep)));
        }
    }

    fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn pattern_symbols(&self, pattern: &BindingPattern<'_>) -> Vec<SymbolId> {
        let mut found = PatternSymbols {
            writes: self,
            symbols: Vec::new(),
        };
        found.visit_binding_pattern(pattern);
        found.symbols
    }

    fn target_symbols(&self, target: &AssignmentTarget<'_>) -> Vec<SymbolId> {
        let mut found = PatternSymbols {
            writes: self,
            symbols: Vec::new(),
        };
        found.visit_assignment_target(target);
        found.symbols
    }

    fn left_symbols(&self, left: &ForStatementLeft<'_>) -> Vec<SymbolId> {
        match left {
            ForStatementLeft::VariableDeclaration(declaration) => declaration
                .declarations
                .iter()
                .flat_map(|declarator| self.pattern_symbols(&declarator.id))
                .collect(),
            other => other
                .as_assignment_target()
                .map(|target| self.target_symbols(target))
                .unwrap_or_default(),
        }
    }
}

struct PatternSymbols<'w, 's> {
    writes: &'w Writes<'s>,
    symbols: Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for PatternSymbols<'_, '_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.symbols.extend(it.symbol_id.get());
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.symbols.extend(self.writes.symbol_of(it));
    }

    fn visit_expression(&mut self, _it: &Expression<'a>) {}
}

impl<'a> VisitJs<'a> for Writes<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            let symbols = self.pattern_symbols(&it.id);
            self.apply(&symbols, init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        let symbols = self.pattern_symbols(&it.left);
        self.apply(&symbols, &it.right);
        walk_js::walk_assignment_pattern(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        let symbols = self.target_symbols(&it.binding);
        self.apply(&symbols, &it.init);
        walk_js::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if let (Some(init), Some(symbol)) = (&it.init, self.symbol_of(&it.binding)) {
            self.apply(&[symbol], init);
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        let symbols = self.left_symbols(&it.left);
        self.apply(&symbols, &it.right);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let symbols = self.target_symbols(&it.left);
        match it.operator {
            AssignmentOperator::Addition if is_numeric_shape(&it.right) => {}
            AssignmentOperator::Addition => self.apply_class(&symbols, Class::Spelling, &[]),
            operator if operator.is_arithmetic() || operator.is_bitwise() => {}
            _ => self.apply(&symbols, &it.right),
        }
        walk_js::walk_assignment_expression(self, it);
    }
}

pub(super) fn names_a_reaching_key(program: &Program<'_>, keys: &[&str]) -> bool {
    let mut texts = Texts { keys, found: false };
    texts.visit_program(program);
    texts.found
}

struct Texts<'k> {
    keys: &'k [&'k str],
    found: bool,
}

impl Texts<'_> {
    fn note(&mut self, text: &str) {
        self.found |= self.keys.contains(&text);
    }
}

impl<'a> VisitJs<'a> for Texts<'_> {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if let Some(text) = it.value.as_str() {
            self.note(text);
        }
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        let text = it.value.cooked.as_ref().and_then(|cooked| cooked.as_str());
        self.note(text.unwrap_or(it.value.raw.as_str()));
    }
}
