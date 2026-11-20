use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

use super::reach::{key_of, Key};
use super::GLOBAL_OBJECTS;
use crate::host_globals::{is_host_global, is_host_global_property};
use crate::semantic::{is_foreign_global, AnnexBWrites};

pub(super) type NumericBindings = FxHashSet<SymbolId>;

pub(super) struct KeyBindings {
    numeric: NumericBindings,
    led: NumericBindings,
}

const ARGUMENTS: &str = "arguments";

const EVAL: &str = "eval";

fn reference_symbol(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn is_numeric_compound(operator: AssignmentOperator) -> bool {
    matches!(
        operator,
        AssignmentOperator::Subtraction
            | AssignmentOperator::Multiplication
            | AssignmentOperator::Division
            | AssignmentOperator::Remainder
            | AssignmentOperator::Exponential
            | AssignmentOperator::ShiftLeft
            | AssignmentOperator::ShiftRight
            | AssignmentOperator::ShiftRightZeroFill
            | AssignmentOperator::BitwiseOR
            | AssignmentOperator::BitwiseXOR
            | AssignmentOperator::BitwiseAnd
    )
}

fn assigned_symbol(target: &AssignmentTarget<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => reference_symbol(id, scoping),
        _ => None,
    }
}

pub(super) fn is_numeric(
    expression: &Expression<'_>,
    scoping: &Scoping,
    numeric: &NumericBindings,
) -> bool {
    let expression = expression.without_parentheses();
    if matches!(key_of(expression), Key::Other) {
        return true;
    }
    match expression {
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
        Expression::BinaryExpression(binary) => {
            binary.operator == BinaryOperator::Addition
                && is_numeric(&binary.left, scoping, numeric)
                && is_numeric(&binary.right, scoping, numeric)
        }
        Expression::Identifier(id) => {
            reference_symbol(id, scoping).is_some_and(|symbol| numeric.contains(&symbol))
        }
        Expression::AssignmentExpression(assignment) => match assignment.operator {
            AssignmentOperator::Assign => is_numeric(&assignment.right, scoping, numeric),
            AssignmentOperator::Addition => {
                assigned_symbol(&assignment.left, scoping)
                    .is_some_and(|symbol| numeric.contains(&symbol))
                    && is_numeric(&assignment.right, scoping, numeric)
            }
            operator => is_numeric_compound(operator),
        },
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .last()
            .is_some_and(|last| is_numeric(last, scoping, numeric)),
        Expression::ConditionalExpression(conditional) => {
            is_numeric(&conditional.consequent, scoping, numeric)
                && is_numeric(&conditional.alternate, scoping, numeric)
        }
        _ => false,
    }
}

fn is_led(
    expression: &Expression<'_>,
    scoping: &Scoping,
    numeric: &NumericBindings,
    led: &NumericBindings,
) -> bool {
    if is_numeric(expression, scoping, numeric) {
        return true;
    }
    match expression.without_parentheses() {
        Expression::BinaryExpression(binary) => {
            binary.operator == BinaryOperator::Addition
                && is_led(&binary.left, scoping, numeric, led)
        }
        Expression::Identifier(id) => {
            reference_symbol(id, scoping).is_some_and(|symbol| led.contains(&symbol))
        }
        Expression::AssignmentExpression(assignment) => match assignment.operator {
            AssignmentOperator::Assign => is_led(&assignment.right, scoping, numeric, led),
            AssignmentOperator::Addition => assigned_symbol(&assignment.left, scoping)
                .is_some_and(|symbol| led.contains(&symbol)),
            _ => false,
        },
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .last()
            .is_some_and(|last| is_led(last, scoping, numeric, led)),
        Expression::ConditionalExpression(conditional) => {
            is_led(&conditional.consequent, scoping, numeric, led)
                && is_led(&conditional.alternate, scoping, numeric, led)
        }
        _ => false,
    }
}

pub(super) fn is_harmless_key(
    expression: &Expression<'_>,
    scoping: &Scoping,
    keys: &KeyBindings,
) -> bool {
    is_led(expression, scoping, &keys.numeric, &keys.led)
}

const MAX_PASSES: usize = 8;

struct Collect<'s> {
    scoping: &'s Scoping,
    annex_b: &'s AnnexBWrites,
    numeric: &'s NumericBindings,
    led: Option<&'s NumericBindings>,
    module: bool,
    bindings: FxHashMap<SymbolId, usize>,
    plain_bindings: FxHashMap<SymbolId, usize>,
    plain_values: FxHashSet<SymbolId>,
    numeric_writes: FxHashSet<ReferenceId>,
    refused: FxHashSet<SymbolId>,
    params: FxHashMap<SymbolId, ReferenceId>,
    strict: Vec<bool>,
    global_keys: FxHashSet<String>,
    global_unknown: bool,
    direct_eval: bool,
}

fn uses_arguments(function: &Function<'_>) -> bool {
    #[derive(Default)]
    struct Arguments {
        found: bool,
    }
    impl<'a> VisitJs<'a> for Arguments {
        fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
            self.found |= it.name == ARGUMENTS;
        }
    }
    let mut arguments = Arguments::default();
    arguments.visit_function(function, ScopeFlags::Function);
    arguments.found
}

fn first_assignment<'k, 'a>(
    function: &'k Function<'a>,
) -> Option<(&'k IdentifierReference<'a>, &'k Expression<'a>)> {
    let first = function.body.as_ref()?.statements.first()?;
    let Statement::ExpressionStatement(statement) = first else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign).then_some((&**target, &assignment.right))
}

fn subtracts_from(right: &Expression<'_>, name: &str) -> bool {
    let Expression::BinaryExpression(binary) = right.without_parentheses() else {
        return false;
    };
    binary.operator == BinaryOperator::Subtraction
        && matches!(binary.left.without_parentheses(), Expression::Identifier(id) if id.name == name)
        && matches!(key_of(&binary.right), Key::Other)
}

fn has_simple_params(function: &Function<'_>) -> bool {
    function.params.rest.is_none()
        && function.params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        })
}

impl Collect<'_> {
    fn is_strict(&self) -> bool {
        self.strict.last().copied().unwrap_or(self.module)
    }

    fn numeric(&self, expression: &Expression<'_>) -> bool {
        match self.led {
            Some(led) => is_led(expression, self.scoping, self.numeric, led),
            None => is_numeric(expression, self.scoping, self.numeric),
        }
    }

    fn adds_to(&self, symbol: SymbolId, right: &Expression<'_>) -> bool {
        match self.led {
            Some(led) => led.contains(&symbol),
            None => self.numeric.contains(&symbol) && self.numeric(right),
        }
    }

    fn decoder_param(&mut self, function: &Function<'_>) {
        let Some((target, right)) = first_assignment(function) else {
            return;
        };
        let sloppy_reads = !self.is_strict() && uses_arguments(function);
        if !has_simple_params(function) || sloppy_reads || !subtracts_from(right, &target.name) {
            return;
        }
        let param = function
            .params
            .items
            .iter()
            .find_map(|item| match &item.pattern {
                BindingPattern::BindingIdentifier(id) if id.name == target.name => {
                    id.symbol_id.get()
                }
                _ => None,
            });
        let (Some(param), Some(reference)) = (param, target.reference_id.get()) else {
            return;
        };
        if reference_symbol(target, self.scoping) == Some(param) {
            self.params.insert(param, reference);
        }
    }

    fn refuse_loop_left(&mut self, left: &ForStatementLeft<'_>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            for declarator in &declaration.declarations {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                    self.refused.extend(id.symbol_id.get());
                }
            }
        }
    }

    fn shared(&self, symbol: SymbolId) -> bool {
        if self.module || self.scoping.symbol_scope_id(symbol) != self.scoping.root_scope_id() {
            return false;
        }
        let name = self.scoping.symbol_name(symbol);
        self.global_unknown || self.global_keys.contains(name) || is_foreign_global(name)
    }

    fn is_global_object(&self, object: &Expression<'_>) -> bool {
        match object.without_parentheses() {
            Expression::ThisExpression(_) => true,
            Expression::Identifier(id) => {
                GLOBAL_OBJECTS.contains(&id.name.as_str())
                    && reference_symbol(id, self.scoping).is_none()
            }
            _ => false,
        }
    }

    fn global_write(&mut self, member: &MemberExpression<'_>) {
        if !self.is_global_object(member.object()) {
            return;
        }
        let key = match member {
            MemberExpression::StaticMemberExpression(member) => {
                Key::Name(member.property.name.as_str())
            }
            MemberExpression::ComputedMemberExpression(member) => key_of(&member.expression),
            MemberExpression::PrivateFieldExpression(_) => Key::Other,
        };
        match key {
            Key::Name(name) => {
                self.global_keys.insert(name.to_string());
            }
            Key::Other => {}
            Key::Unknown(_) => self.global_unknown = true,
        }
    }

    fn host_provided(&self, symbol: SymbolId) -> bool {
        let name = self.scoping.symbol_name(symbol);
        name == ARGUMENTS
            || (self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
                && (is_host_global(name) || is_host_global_property(name)))
    }

    fn holds(&self, symbol: SymbolId) -> bool {
        if self.direct_eval || self.annex_b.may_write(symbol) || self.host_provided(symbol) {
            return false;
        }
        let flags = self.scoping.symbol_flags(symbol);
        let variable = flags.is_variable()
            && !flags.intersects(
                SymbolFlags::Function
                    | SymbolFlags::Class
                    | SymbolFlags::CatchVariable
                    | SymbolFlags::Import,
            );
        if !variable || self.refused.contains(&symbol) || self.shared(symbol) {
            return false;
        }
        let count = self.bindings.get(&symbol).copied().unwrap_or(0);
        let writes = self
            .scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .filter(|&&reference| self.scoping.get_reference(reference).flags().is_write());
        if let Some(&only) = self.params.get(&symbol) {
            return count == 1 && writes.clone().all(|&reference| reference == only);
        }
        let plain = self.plain_bindings.get(&symbol).copied().unwrap_or(0);
        count > 0
            && count == plain
            && self.plain_values.contains(&symbol)
            && writes
                .clone()
                .all(|reference| self.numeric_writes.contains(reference))
    }
}

impl<'a> VisitJs<'a> for Collect<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            *self.bindings.entry(symbol).or_default() += 1;
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            if let Some(symbol) = id.symbol_id.get() {
                *self.plain_bindings.entry(symbol).or_default() += 1;
                let numeric = it.init.as_ref().is_none_or(|init| self.numeric(init));
                if !numeric {
                    self.refused.insert(symbol);
                }
                self.plain_values.insert(symbol);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.refuse_loop_left(&it.left);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.refuse_loop_left(&it.left);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
            let numeric = match it.operator {
                AssignmentOperator::Assign
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish => self.numeric(&it.right),
                AssignmentOperator::Addition => reference_symbol(target, self.scoping)
                    .is_some_and(|symbol| self.adds_to(symbol, &it.right)),
                operator => is_numeric_compound(operator),
            };
            if let (true, Some(reference)) = (numeric, target.reference_id.get()) {
                self.numeric_writes.insert(reference);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = it.as_member_expression() {
            self.global_write(member);
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = it.callee.without_parentheses() {
            self.direct_eval |= callee.name == EVAL;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &it.argument {
            self.numeric_writes.extend(target.reference_id.get());
        }
        walk_js::walk_update_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let strict = self.is_strict() || it.has_use_strict_directive();
        self.strict.push(strict);
        self.decoder_param(it);
        walk_js::walk_function(self, it, flags);
        self.strict.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.strict.push(true);
        walk_js::walk_class(self, it);
        self.strict.pop();
    }
}

struct Sources<'s, 'a> {
    program: &'s Program<'a>,
    scoping: &'s Scoping,
    annex_b: &'s AnnexBWrites,
    module: bool,
}

fn narrowed(
    sources: &Sources<'_, '_>,
    (numeric, led): (&NumericBindings, Option<&NumericBindings>),
) -> NumericBindings {
    let Sources {
        program,
        scoping,
        annex_b,
        module,
    } = *sources;
    let mut collect = Collect {
        scoping,
        annex_b,
        numeric,
        led,
        module,
        bindings: FxHashMap::default(),
        plain_bindings: FxHashMap::default(),
        plain_values: FxHashSet::default(),
        numeric_writes: FxHashSet::default(),
        refused: FxHashSet::default(),
        params: FxHashMap::default(),
        strict: Vec::new(),
        global_keys: FxHashSet::default(),
        global_unknown: false,
        direct_eval: false,
    };
    collect
        .strict
        .push(module || crate::semantic::program_is_strict(program));
    collect.visit_program(program);
    led.unwrap_or(numeric)
        .iter()
        .copied()
        .filter(|&symbol| collect.holds(symbol))
        .collect()
}

fn fixpoint(sources: &Sources<'_, '_>, numeric: Option<&NumericBindings>) -> NumericBindings {
    let mut current: NumericBindings = sources.scoping.symbol_ids().collect();
    for _ in 0..MAX_PASSES {
        let next = match numeric {
            Some(numeric) => narrowed(sources, (numeric, Some(&current))),
            None => narrowed(sources, (&current, None)),
        };
        if next.len() == current.len() {
            return next;
        }
        current = next;
    }
    NumericBindings::default()
}

pub(super) fn key_bindings(program: &Program<'_>, scoping: &Scoping, module: bool) -> KeyBindings {
    let annex_b = AnnexBWrites::of(program, scoping);
    let sources = Sources {
        program,
        scoping,
        annex_b: &annex_b,
        module,
    };
    let numeric = fixpoint(&sources, None);
    let led = fixpoint(&sources, Some(&numeric));
    KeyBindings { numeric, led }
}
