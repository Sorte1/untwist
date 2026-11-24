use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::Entry;
use untwist_core::ast_util::prop::{literal_key, member_parts};

use crate::collect::{module_export_name, reference_symbol};
use crate::human::looks_human_written;

const NAME_FIELDS: [&str; 2] = ["name", "displayName"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShorthandKind {
    PatternProperty,
    ObjectLiteralValue,
    ImportSpecifier,
}

#[derive(Clone, Debug)]
pub struct ShorthandHint {
    pub symbol: SymbolId,
    pub key: String,
    pub kind: ShorthandKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassHintKind {
    Type,
    IsFlag,
    NameField,
}

#[derive(Clone, Debug)]
pub struct ClassHint {
    pub kind: ClassHintKind,
    pub name: String,
}

#[derive(Default)]
pub struct Usage {
    pub called: FxHashSet<SymbolId>,
    pub constructed: FxHashSet<SymbolId>,
    pub props_read: FxHashMap<SymbolId, Vec<String>>,
    pub hints: Vec<ShorthandHint>,
    pub class_hints: FxHashMap<SymbolId, ClassHint>,
    pub indexed: FxHashSet<SymbolId>,
    pub numeric_only: FxHashSet<SymbolId>,
    pub passed: FxHashMap<SymbolId, Vec<(SymbolId, usize)>>,
    numeric: FxHashMap<SymbolId, NumericUses>,
    literal_keys: FxHashMap<SymbolId, usize>,
}

#[derive(Clone, Copy, Default)]
struct NumericUses {
    strong: u32,
    neutral: u32,
}

impl Usage {
    pub fn object_literal_key(&self, symbol: SymbolId) -> Option<&str> {
        let index = *self.literal_keys.get(&symbol)?;
        self.hints.get(index).map(|hint| hint.key.as_str())
    }

    fn index_literal_keys(&mut self) {
        for (index, hint) in self.hints.iter().enumerate() {
            if hint.kind == ShorthandKind::ObjectLiteralValue {
                self.literal_keys.entry(hint.symbol).or_insert(index);
            }
        }
    }

    pub fn is_numeric(&self, symbol: SymbolId) -> bool {
        self.numeric
            .get(&symbol)
            .is_some_and(|uses| uses.strong > 0)
    }

    fn settle_numeric(&mut self, scoping: &Scoping) {
        for (symbol, uses) in &self.numeric {
            if uses.strong == 0 {
                continue;
            }
            let reads = scoping
                .get_resolved_references(*symbol)
                .filter(|reference| reference.is_read())
                .count();
            if (uses.strong + uses.neutral) as usize >= reads {
                self.numeric_only.insert(*symbol);
            }
        }
    }

    pub fn class_hint(&self, symbol: SymbolId) -> Option<&str> {
        self.class_hints.get(&symbol).map(|hint| hint.name.as_str())
    }

    pub fn is_callable(&self, symbol: SymbolId) -> bool {
        self.called.contains(&symbol) || self.constructed.contains(&symbol)
    }

    pub fn is_only_constructed(&self, symbol: SymbolId) -> bool {
        self.constructed.contains(&symbol) && !self.called.contains(&symbol)
    }

    pub fn reads_any(&self, symbol: SymbolId, names: &[&str]) -> bool {
        self.props_read
            .get(&symbol)
            .is_some_and(|read| read.iter().any(|prop| names.contains(&prop.as_str())))
    }
}

pub fn collect_usage(program: &Program<'_>, scoping: &Scoping) -> Usage {
    walk_usage::<false>(program, scoping)
}

pub fn collect_naming_usage(program: &Program<'_>, scoping: &Scoping) -> Usage {
    walk_usage::<true>(program, scoping)
}

fn walk_usage<const CLASS_HINTS: bool>(program: &Program<'_>, scoping: &Scoping) -> Usage {
    let mut walker = UsageWalker::<CLASS_HINTS> {
        scoping,
        next_owner: None,
        class: None,
        owner: None,
        this_aliases: Vec::new(),
        out: Usage::default(),
    };
    walker.visit_program(program);
    let mut usage = walker.out;
    usage.index_literal_keys();
    if CLASS_HINTS {
        usage.settle_numeric(scoping);
    }
    usage
}

struct UsageWalker<'s, const CLASS_HINTS: bool> {
    scoping: &'s Scoping,
    next_owner: Option<SymbolId>,
    class: Option<SymbolId>,
    owner: Option<SymbolId>,
    this_aliases: Vec<SymbolId>,
    out: Usage,
}

pub fn is_class_hint(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && name.chars().all(|ch| ch.is_ascii_alphanumeric())
        && name.chars().any(|ch| ch.is_ascii_lowercase())
        && looks_human_written(name)
}

fn is_true(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(literal) => literal.value,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::LogicalNot
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(n) if n.value == 0.0)
        }
        _ => false,
    }
}

fn string_value<'e>(expr: &'e Expression<'_>) -> Option<&'e str> {
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked)
            .and_then(|cooked| cooked.as_str()),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Instance,
    Prototype,
    Constructor,
}

fn hint_of(property: &str, value: &Expression<'_>, target: Target) -> Option<ClassHint> {
    if target == Target::Instance && property == "type" {
        return named_hint(ClassHintKind::Type, string_value(value)?);
    }
    if target != Target::Constructor {
        if let Some(rest) = property.strip_prefix("is") {
            return is_true(value)
                .then_some(rest)
                .and_then(|rest| named_hint(ClassHintKind::IsFlag, rest));
        }
    }
    if target != Target::Instance && NAME_FIELDS.contains(&property) {
        return named_hint(ClassHintKind::NameField, string_value(value)?);
    }
    None
}

fn named_hint(kind: ClassHintKind, name: &str) -> Option<ClassHint> {
    is_class_hint(name).then(|| ClassHint {
        kind,
        name: name.to_string(),
    })
}

fn is_from_this(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ThisExpression(_) => true,
        Expression::CallExpression(call) => call
            .arguments
            .first()
            .is_some_and(|first| matches!(first, Argument::ThisExpression(_))),
        Expression::LogicalExpression(logical) => {
            logical.operator == LogicalOperator::Or
                && matches!(
                    logical.right.without_parentheses(),
                    Expression::ThisExpression(_)
                )
        }
        _ => false,
    }
}

fn declared_symbol(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn anonymous_class_or_function(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => function.id.is_none(),
        Expression::ClassExpression(class) => class.id.is_none(),
        _ => false,
    }
}

impl<const CLASS_HINTS: bool> UsageWalker<'_, CLASS_HINTS> {
    fn hint(&mut self, symbol: SymbolId, key: &str, kind: ShorthandKind) {
        self.out.hints.push(ShorthandHint {
            symbol,
            key: key.to_string(),
            kind,
        });
    }

    fn add_class_hint(&mut self, symbol: SymbolId, hint: ClassHint) {
        match self.out.class_hints.entry(symbol) {
            Entry::Occupied(mut held) if hint.kind < held.get().kind => {
                held.insert(hint);
            }
            Entry::Occupied(_) => {}
            Entry::Vacant(slot) => {
                slot.insert(hint);
            }
        }
    }

    fn identifier_symbol(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        reference_symbol(id, self.scoping)
    }

    fn is_instance(&self, object: &Expression<'_>) -> bool {
        match object {
            Expression::ThisExpression(_) => true,
            Expression::AssignmentExpression(assign) => {
                matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_))
                    && is_from_this(&assign.right)
            }
            Expression::Identifier(_) => self
                .identifier_symbol(object)
                .is_some_and(|symbol| self.this_aliases.contains(&symbol)),
            _ => false,
        }
    }

    fn track_alias(&mut self, symbol: Option<SymbolId>, value: &Expression<'_>) {
        let Some(symbol) = symbol else {
            return;
        };
        if self.owner.is_some() && is_from_this(value) {
            self.this_aliases.push(symbol);
        }
    }

    fn hinted_target(&self, object: &Expression<'_>) -> Option<(SymbolId, Target)> {
        let object = object.without_parentheses();
        if self.is_instance(object) {
            return self.owner.map(|owner| (owner, Target::Instance));
        }
        if let Some((inner, property)) = member_parts(object) {
            if property != "prototype" {
                return None;
            }
            return self
                .identifier_symbol(inner)
                .map(|symbol| (symbol, Target::Prototype));
        }
        self.identifier_symbol(object)
            .map(|symbol| (symbol, Target::Constructor))
    }

    fn assignment_hint(&mut self, it: &AssignmentExpression<'_>) {
        if it.operator != AssignmentOperator::Assign {
            return;
        }
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            self.track_alias(reference_symbol(id, self.scoping), &it.right);
            return;
        }
        let Some(member) = it.left.as_member_expression() else {
            return;
        };
        let Some(property) = member.static_property_name().and_then(|name| name.as_str()) else {
            return;
        };
        let Some((symbol, target)) = self.hinted_target(member.object()) else {
            return;
        };
        if let Some(hint) = hint_of(property, &it.right, target) {
            self.add_class_hint(symbol, hint);
        }
    }

    fn count_numeric(&mut self, operand: &Expression<'_>, strong: bool) {
        if let Some(symbol) = self.identifier_symbol(operand) {
            self.add_numeric(symbol, strong);
        }
    }

    fn add_numeric(&mut self, symbol: SymbolId, strong: bool) {
        let uses = self.out.numeric.entry(symbol).or_default();
        if strong {
            uses.strong += 1;
        } else {
            uses.neutral += 1;
        }
    }

    fn binary_operands(&mut self, it: &BinaryExpression<'_>) {
        let operator = it.operator;
        let strong = (operator.is_arithmetic() && operator != BinaryOperator::Addition)
            || operator.is_bitwise();
        let neutral = operator == BinaryOperator::Addition
            || operator.is_compare()
            || (operator.is_equality()
                && [&it.left, &it.right].iter().any(|side| {
                    matches!(side.without_parentheses(), Expression::NumericLiteral(_))
                }));
        if strong || neutral {
            self.count_numeric(&it.left, strong);
            self.count_numeric(&it.right, strong);
        }
    }

    fn compound_operand(&mut self, it: &AssignmentExpression<'_>) {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left else {
            return;
        };
        let operator = it.operator;
        let strong = (operator.is_arithmetic() && operator != AssignmentOperator::Addition)
            || operator.is_bitwise();
        if !strong && operator != AssignmentOperator::Addition {
            return;
        }
        if let Some(symbol) = reference_symbol(id, self.scoping) {
            self.add_numeric(symbol, strong);
        }
    }

    fn math_arguments(&mut self, it: &CallExpression<'_>) {
        let math = member_parts(it.callee.without_parentheses()).is_some_and(|(object, _)| {
            matches!(object.without_parentheses(), Expression::Identifier(id) if id.name == "Math")
        });
        if !math {
            return;
        }
        for argument in &it.arguments {
            if let Some(expr) = argument.as_expression() {
                self.count_numeric(expr, true);
            }
        }
    }

    fn passed_arguments(&mut self, it: &CallExpression<'_>) {
        let Some(callee) = self.identifier_symbol(&it.callee) else {
            return;
        };
        for (index, argument) in it.arguments.iter().enumerate() {
            let Some(expr) = argument.as_expression() else {
                return;
            };
            if let Some(symbol) = self.identifier_symbol(expr) {
                self.out
                    .passed
                    .entry(symbol)
                    .or_default()
                    .push((callee, index));
            }
        }
    }

    fn field_hint(&mut self, it: &PropertyDefinition<'_>) {
        let (Some(class), true, false, Some(value)) =
            (self.class, it.r#static, it.computed, it.value.as_ref())
        else {
            return;
        };
        let PropertyKey::StaticIdentifier(key) = &it.key else {
            return;
        };
        if let Some(hint) = hint_of(key.name.as_str(), value, Target::Constructor) {
            self.add_class_hint(class, hint);
        }
    }
}

impl<'a, const CLASS_HINTS: bool> VisitJs<'a> for UsageWalker<'_, CLASS_HINTS> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if !CLASS_HINTS {
            return walk_js::walk_variable_declarator(self, it);
        }
        if let Some(init) = &it.init {
            if anonymous_class_or_function(init) {
                self.next_owner = declared_symbol(&it.id);
            }
            self.track_alias(declared_symbol(&it.id), init);
        }
        walk_js::walk_variable_declarator(self, it);
        self.next_owner = None;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if !CLASS_HINTS {
            return walk_js::walk_function(self, it, flags);
        }
        let own = it.id.as_ref().and_then(|id| id.symbol_id.get());
        let owner = self.next_owner.take().or(own);
        let saved = std::mem::replace(&mut self.owner, owner);
        let saved_aliases = std::mem::take(&mut self.this_aliases);
        walk_js::walk_function(self, it, flags);
        self.owner = saved;
        self.this_aliases = saved_aliases;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if !CLASS_HINTS {
            return walk_js::walk_class(self, it);
        }
        let own = it.id.as_ref().and_then(|id| id.symbol_id.get());
        let class = self.next_owner.take().or(own);
        let saved_class = std::mem::replace(&mut self.class, class);
        let saved_owner = self.owner.take();
        let saved_aliases = std::mem::take(&mut self.this_aliases);
        walk_js::walk_class(self, it);
        self.class = saved_class;
        self.owner = saved_owner;
        self.this_aliases = saved_aliases;
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if !CLASS_HINTS {
            return walk_js::walk_method_definition(self, it);
        }
        self.next_owner = match it.kind {
            MethodDefinitionKind::Constructor => self.class,
            _ => None,
        };
        walk_js::walk_method_definition(self, it);
        self.next_owner = None;
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        if !CLASS_HINTS {
            return walk_js::walk_property_definition(self, it);
        }
        self.field_hint(it);
        walk_js::walk_property_definition(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if !CLASS_HINTS {
            return walk_js::walk_assignment_expression(self, it);
        }
        self.assignment_hint(it);
        self.compound_operand(it);
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if CLASS_HINTS {
            self.binary_operands(it);
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if CLASS_HINTS && (it.operator.is_arithmetic() || it.operator.is_bitwise()) {
            self.count_numeric(&it.argument, true);
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let (true, SimpleAssignmentTarget::AssignmentTargetIdentifier(id)) =
            (CLASS_HINTS, &it.argument)
        {
            if let Some(symbol) = reference_symbol(id, self.scoping) {
                self.add_numeric(symbol, true);
            }
        }
        walk_js::walk_update_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if CLASS_HINTS && literal_key(&it.expression).is_none() {
            if let Some(symbol) = self.identifier_symbol(&it.object) {
                self.out.indexed.insert(symbol);
            }
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(symbol) = self.identifier_symbol(&it.callee) {
            self.out.called.insert(symbol);
        }
        if CLASS_HINTS {
            self.math_arguments(it);
            self.passed_arguments(it);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if let Some(symbol) = self.identifier_symbol(&it.callee) {
            self.out.constructed.insert(symbol);
        }
        walk_js::walk_new_expression(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Some((object, property)) = member_parts(it) {
            if let Expression::Identifier(id) = object.without_parentheses() {
                if let Some(symbol) = reference_symbol(id, self.scoping) {
                    self.out
                        .props_read
                        .entry(symbol)
                        .or_default()
                        .push(property.into_owned());
                }
            }
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if !it.computed && !it.method {
            if let PropertyKey::StaticIdentifier(key) = &it.key {
                if let Expression::Identifier(id) = it.value.without_parentheses() {
                    if let Some(symbol) = reference_symbol(id, self.scoping) {
                        let name = key.name.as_str().to_string();
                        self.hint(symbol, &name, ShorthandKind::ObjectLiteralValue);
                    }
                }
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if !it.computed {
            if let PropertyKey::StaticIdentifier(key) = &it.key {
                if let BindingPattern::BindingIdentifier(id) = &it.value {
                    if let Some(symbol) = id.symbol_id.get() {
                        let name = key.name.as_str().to_string();
                        self.hint(symbol, &name, ShorthandKind::PatternProperty);
                    }
                }
            }
        }
        walk_js::walk_binding_property(self, it);
    }

    fn visit_import_specifier(&mut self, it: &ImportSpecifier<'a>) {
        if let Some(symbol) = it.local.symbol_id.get() {
            let imported = module_export_name(&it.imported).to_string();
            self.hint(symbol, &imported, ShorthandKind::ImportSpecifier);
        }
        walk_js::walk_import_specifier(self, it);
    }
}
