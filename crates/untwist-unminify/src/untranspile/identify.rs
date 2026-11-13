use std::cell::Cell;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use untwist_core::pass::{ExpressionKind, StatementKind};

use super::fingerprint::{self, Externals, Print};
use super::shapes::{self, Alias, Kind, ALIASES};

const VERIFY_DEPTH: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Definition {
    Function,
    Declarator,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Unknown,
    Pending,
    Holds,
}

#[derive(Debug)]
pub struct Helper {
    pub kind: Kind,
    pub definition: Definition,
    pub external: Externals,
    pub own_references: u32,
    expected: &'static [Kind],
    verdict: Cell<Verdict>,
}

#[derive(Default)]
pub struct Table {
    helpers: FxHashMap<SymbolId, Helper>,
    consumers: bool,
    destructurers: bool,
    classes: bool,
    field_definers: bool,
    machines: bool,
    runners: bool,
    iterators: bool,
}

impl Table {
    pub fn clear(&mut self) {
        self.helpers.clear();
        self.consumers = false;
        self.destructurers = false;
        self.classes = false;
        self.field_definers = false;
        self.machines = false;
        self.runners = false;
        self.iterators = false;
    }

    pub fn is_empty(&self) -> bool {
        self.helpers.is_empty()
    }

    pub fn has_consumers(&self) -> bool {
        self.consumers
    }

    pub fn has_destructurers(&self) -> bool {
        self.destructurers
    }

    pub fn has_classes(&self) -> bool {
        self.classes
    }

    pub fn has_field_definers(&self) -> bool {
        self.field_definers
    }

    pub fn has_machines(&self) -> bool {
        self.machines
    }

    pub fn has_runners(&self) -> bool {
        self.runners
    }

    pub fn has_iterators(&self) -> bool {
        self.iterators
    }

    pub fn kinds(&self) -> impl Iterator<Item = Kind> + '_ {
        self.helpers.values().map(|helper| helper.kind)
    }

    pub fn insert(&mut self, symbol: SymbolId, helper: Helper) {
        self.consumers |= shapes::is_consumer(helper.kind);
        self.destructurers |= shapes::is_destructurer(helper.kind);
        self.classes |= shapes::is_class_trigger(helper.kind);
        self.field_definers |= shapes::is_field_definer(helper.kind);
        self.machines |= shapes::is_machine_runtime(helper.kind);
        self.runners |= shapes::is_async_runner(helper.kind);
        self.iterators |= shapes::is_iterator_helper(helper.kind);
        self.helpers.insert(symbol, helper);
    }

    pub fn symbols(&self) -> impl Iterator<Item = SymbolId> + '_ {
        self.helpers.keys().copied()
    }

    pub fn get(&self, symbol: SymbolId) -> Option<&Helper> {
        self.helpers.get(&symbol)
    }

    pub fn kind(&self, symbol: SymbolId) -> Option<Kind> {
        let helper = self.helpers.get(&symbol)?;
        self.holds(symbol, VERIFY_DEPTH).then_some(helper.kind)
    }

    pub fn kind_of_callee(&self, callee: &Expression<'_>, scoping: &Scoping) -> Option<Kind> {
        let symbol = resolved(callee, scoping)?;
        self.kind(symbol)
    }

    fn holds(&self, symbol: SymbolId, depth: u32) -> bool {
        let Some(helper) = self.helpers.get(&symbol) else {
            return false;
        };
        match helper.verdict.get() {
            Verdict::Holds => return true,
            Verdict::Pending => return false,
            Verdict::Unknown => {}
        }
        if depth == 0 {
            return false;
        }
        helper.verdict.set(Verdict::Pending);
        let holds = helper.external.count() == helper.expected.len()
            && helper
                .external
                .iter()
                .zip(helper.expected)
                .all(|((external, _), expected)| {
                    self.helpers
                        .get(&external)
                        .is_some_and(|sub| sub.kind == *expected)
                        && self.holds(external, depth - 1)
                });
        helper.verdict.set(if holds {
            Verdict::Holds
        } else {
            Verdict::Unknown
        });
        holds
    }
}

pub fn resolved(expression: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::Identifier(id) = expression.without_parentheses() else {
        return None;
    };
    let reference = id.reference_id.get()?;
    scoping.get_reference(reference).symbol_id()
}

fn count(value: usize) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

const EXPRESSION_BODY: u8 = u8::MAX;

const NO_FOCUS: u8 = u8::MAX;

const DECLARED_FOCUS: u8 = u8::MAX - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub params: u8,
    pub statements: u8,
    pub lead: u8,
    pub focus: u8,
}

fn kind_of(expression: Option<&Expression<'_>>) -> u8 {
    expression.map_or(NO_FOCUS, |expression| ExpressionKind::of(expression) as u8)
}

fn focus_of(statement: &Statement<'_>) -> u8 {
    match statement {
        Statement::ReturnStatement(it) => kind_of(it.argument.as_ref()),
        Statement::IfStatement(it) => kind_of(Some(&it.test)),
        Statement::ExpressionStatement(it) => kind_of(Some(&it.expression)),
        Statement::ThrowStatement(it) => kind_of(Some(&it.argument)),
        Statement::VariableDeclaration(it) => kind_of(
            it.declarations
                .first()
                .and_then(|first| first.init.as_ref()),
        ),
        Statement::ForStatement(it) => match &it.init {
            Some(ForStatementInit::VariableDeclaration(_)) => DECLARED_FOCUS,
            Some(init) => kind_of(init.as_expression()),
            None => NO_FOCUS,
        },
        Statement::ForInStatement(it) => kind_of(Some(&it.right)),
        _ => NO_FOCUS,
    }
}

fn lead_of(statements: &[Statement<'_>]) -> (u8, u8) {
    statements.first().map_or((0, NO_FOCUS), |statement| {
        (StatementKind::of(statement) as u8, focus_of(statement))
    })
}

fn function_size(function: &Function<'_>) -> Option<Size> {
    if function.generator || function.r#async || function.params.rest.is_some() {
        return None;
    }
    let body = function.body.as_ref()?;
    let (lead, focus) = lead_of(&body.statements);
    Some(Size {
        params: count(function.params.items.len()),
        statements: count(body.statements.len()),
        lead,
        focus,
    })
}

fn arrow_size(arrow: &ArrowFunctionExpression<'_>) -> Option<Size> {
    if arrow.r#async || arrow.params.rest.is_some() {
        return None;
    }
    let (statements, (lead, focus)) = match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => (body.statements.len(), lead_of(&body.statements)),
        body => (1, (EXPRESSION_BODY, kind_of(body.as_expression()))),
    };
    Some(Size {
        params: count(arrow.params.items.len()),
        statements: count(statements),
        lead,
        focus,
    })
}

#[inline]
fn is_object_root(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(id) => id.name.as_str() == "Object",
        Expression::StaticMemberExpression(member) => {
            matches!(member.object.without_parentheses(), Expression::Identifier(id)
                if id.name.as_str() == "Object")
        }
        _ => false,
    }
}

#[inline]
pub fn may_declare_helper(declarator: &VariableDeclarator<'_>) -> bool {
    if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
        return false;
    }
    match declarator
        .init
        .as_ref()
        .map(Expression::without_parentheses)
    {
        Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)) => true,
        Some(Expression::LogicalExpression(logical)) => {
            logical.operator == LogicalOperator::Or
                && matches!(logical.left.without_parentheses(),
                    Expression::LogicalExpression(left)
                        if matches!(left.left.without_parentheses(), Expression::ThisExpression(_)))
        }
        Some(Expression::StaticMemberExpression(member)) => is_object_root(&member.object),
        _ => false,
    }
}

fn is_constant(scoping: &Scoping, symbol: SymbolId, print: &Print) -> bool {
    let writes = scoping
        .get_resolved_references(symbol)
        .filter(|reference| reference.is_write())
        .count();
    scoping.symbol_redeclarations(symbol).is_empty() && writes == print.own_writes as usize
}

fn helper_from(
    print: Print,
    size: Size,
    definition: Definition,
    scoping: &Scoping,
    symbol: SymbolId,
) -> Option<Helper> {
    #[cfg(feature = "test-hooks")]
    record(scoping, symbol, &print, size);
    let variant = shapes::known(print.hash, size)?;
    if !is_constant(scoping, symbol, &print) || variant.external.len() != print.external.count() {
        return None;
    }
    Some(Helper {
        kind: variant.kind,
        definition,
        external: print.external,
        own_references: print.own_reads + print.own_writes,
        expected: variant.external,
        verdict: Cell::new(Verdict::Unknown),
    })
}

pub fn function_declaration(
    function: &Function<'_>,
    scoping: &Scoping,
) -> Option<(SymbolId, Helper)> {
    if !function.is_declaration() {
        return None;
    }
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    let size = function_size(function)?;
    let cap = cap(size)?;
    let outer = scoping.scope_parent_id(function.scope_id.get()?)?;
    let print = fingerprint::of_function(function, Some(symbol), outer, scoping, cap)?;
    let helper = helper_from(print, size, Definition::Function, scoping, symbol)?;
    Some((symbol, helper))
}

fn function_part(init: &Expression<'_>) -> Option<(Size, ScopeId)> {
    match init.without_parentheses() {
        Expression::FunctionExpression(function) => {
            Some((function_size(function)?, function.scope_id.get()?))
        }
        Expression::ArrowFunctionExpression(arrow) => {
            Some((arrow_size(arrow)?, arrow.scope_id.get()?))
        }
        Expression::CallExpression(call) if call.arguments.is_empty() && !call.optional => {
            match call.callee.without_parentheses() {
                Expression::FunctionExpression(function) if function.params.items.is_empty() => {
                    Some((function_size(function)?, function.scope_id.get()?))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_this_member(expression: &Expression<'_>, name: &str) -> bool {
    let Expression::LogicalExpression(logical) = expression.without_parentheses() else {
        return false;
    };
    if logical.operator != LogicalOperator::And
        || !matches!(
            logical.left.without_parentheses(),
            Expression::ThisExpression(_)
        )
    {
        return false;
    }
    matches!(
        logical.right.without_parentheses(),
        Expression::StaticMemberExpression(member)
            if matches!(member.object.without_parentheses(), Expression::ThisExpression(_))
                && member.property.name.as_str() == name
    )
}

fn shared_part<'k, 'a>(
    init: &'k Expression<'a>,
    name: &str,
    at_module_root: bool,
) -> Option<&'k Expression<'a>> {
    let Expression::LogicalExpression(logical) = init.without_parentheses() else {
        return None;
    };
    let shared = logical.operator == LogicalOperator::Or
        && at_module_root
        && is_this_member(&logical.left, name);
    shared.then_some(&logical.right)
}

fn alias_of(init: &Expression<'_>, scoping: &Scoping) -> Option<Alias> {
    let Expression::StaticMemberExpression(member) = init.without_parentheses() else {
        return None;
    };
    let name = member.property.name.as_str();
    let (_, alias) = ALIASES.iter().find(|(alias, _)| *alias == name)?;
    let on_prototype = matches!(*alias, Alias::HasOwnProperty | Alias::PropertyIsEnumerable);
    let object = if on_prototype {
        let Expression::StaticMemberExpression(prototype) = member.object.without_parentheses()
        else {
            return None;
        };
        if prototype.property.name.as_str() != "prototype" {
            return None;
        }
        &prototype.object
    } else {
        &member.object
    };
    let Expression::Identifier(root) = object.without_parentheses() else {
        return None;
    };
    let global = root.name.as_str() == "Object"
        && root
            .reference_id
            .get()
            .is_some_and(|id| scoping.get_reference(id).symbol_id().is_none());
    global.then_some(*alias)
}

pub fn declarator(
    declarator: &VariableDeclarator<'_>,
    scoping: &Scoping,
    module: bool,
) -> Option<(SymbolId, Helper)> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let symbol = id.symbol_id.get()?;
    let init = declarator.init.as_ref()?;
    if let Some(alias) = alias_of(init, scoping) {
        return aliased(symbol, alias, scoping);
    }
    let at_module_root = module && scoping.symbol_scope_id(symbol) == scoping.root_scope_id();
    let (function, definition) = match shared_part(init, id.name.as_str(), at_module_root) {
        Some(function) => (function, Definition::Shared),
        None => (init, Definition::Declarator),
    };
    let (size, scope) = function_part(function)?;
    let cap = cap(size)?;
    let outer = scoping.scope_parent_id(scope)?;
    let print = fingerprint::of_expression(init, Some(symbol), outer, scoping, cap)?;
    let helper = helper_from(print, size, definition, scoping, symbol)?;
    Some((symbol, helper))
}

fn aliased(symbol: SymbolId, alias: Alias, scoping: &Scoping) -> Option<(SymbolId, Helper)> {
    let constant = scoping.symbol_redeclarations(symbol).is_empty()
        && !scoping
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write());
    constant.then(|| {
        (
            symbol,
            Helper {
                kind: Kind::Alias(alias),
                definition: Definition::Declarator,
                external: Externals::EMPTY,
                own_references: 0,
                expected: &[],
                verdict: Cell::new(Verdict::Holds),
            },
        )
    })
}

#[cfg(feature = "test-hooks")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeenPrint {
    pub name: String,
    pub hash: u128,
    pub params: u8,
    pub statements: u8,
    pub lead: u8,
    pub focus: u8,
    pub tokens: u32,
    pub external: Vec<String>,
}

#[cfg(feature = "test-hooks")]
thread_local! {
    static RECORDING: std::cell::RefCell<Option<Vec<SeenPrint>>> = const { std::cell::RefCell::new(None) };
}

fn cap(size: Size) -> Option<u32> {
    if recording() {
        Some(fingerprint::RECORDING_CAP)
    } else {
        shapes::sized_cap(size)
    }
}

#[cfg(feature = "test-hooks")]
pub fn recording() -> bool {
    RECORDING.with(|recording| recording.borrow().is_some())
}

#[cfg(not(feature = "test-hooks"))]
pub fn recording() -> bool {
    false
}

#[cfg(feature = "test-hooks")]
pub fn record_prints_for_tests(on: bool) {
    RECORDING.with(|recording| *recording.borrow_mut() = on.then(Vec::new));
}

#[cfg(feature = "test-hooks")]
pub fn take_prints_for_tests() -> Vec<SeenPrint> {
    RECORDING.with(|recording| {
        recording
            .borrow_mut()
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    })
}

#[cfg(feature = "test-hooks")]
fn record(scoping: &Scoping, symbol: SymbolId, print: &Print, size: Size) {
    RECORDING.with(|recording| {
        if let Some(seen) = recording.borrow_mut().as_mut() {
            seen.push(SeenPrint {
                name: scoping.symbol_name(symbol).to_string(),
                hash: print.hash,
                params: size.params,
                statements: size.statements,
                lead: size.lead,
                focus: size.focus,
                tokens: print.tokens,
                external: print
                    .external
                    .iter()
                    .map(|(external, _)| scoping.symbol_name(external).to_string())
                    .collect(),
            });
        }
    });
}
