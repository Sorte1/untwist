use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::Ancestor;
use untwist_core::ast_util::global::is_global_name_at;
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::pass::Cx;

use super::{is_arguments, is_arguments_length, literal_index};

pub struct Host {
    scope: ScopeId,
    own_name: Option<SymbolId>,
    starts_lazily: bool,
    has_fixed_arity: bool,
    is_safe_lane: bool,
}

pub struct Moved<'r, 'a> {
    pub index: usize,
    pub pattern: Option<&'r BindingPattern<'a>>,
    pub default: Option<&'r Expression<'a>>,
    pub rest: &'r [Statement<'a>],
}

pub fn host(function: &Function<'_>, cx: &Cx<'_, '_>) -> Option<Host> {
    let scope = function.scope_id.get()?;
    let body = function.body.as_deref()?;
    let is_plain = function.params.rest.is_none()
        && body.directives.is_empty()
        && has_unique_names(&function.params)
        && is_global_name_at(cx.scoping(), scope, "arguments");
    is_plain.then(|| Host {
        scope,
        own_name: own_name(function, cx.scoping()),
        starts_lazily: function.generator,
        has_fixed_arity: is_accessor(cx),
        is_safe_lane: !cx.opts.unsafe_transforms,
    })
}

fn own_name(function: &Function<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    scoping
        .symbol_redeclarations(symbol)
        .is_empty()
        .then_some(symbol)
}

fn has_unique_names(params: &FormalParameters<'_>) -> bool {
    let names: Vec<&str> = params
        .items
        .iter()
        .flat_map(|item| item.pattern.get_binding_identifiers())
        .map(|identifier| identifier.name.as_str())
        .collect();
    names
        .iter()
        .enumerate()
        .all(|(position, name)| !names[..position].contains(name))
}

fn is_accessor(cx: &Cx<'_, '_>) -> bool {
    match cx.tcx.parent() {
        Ancestor::MethodDefinitionValue(method) => matches!(
            *method.kind(),
            MethodDefinitionKind::Get | MethodDefinitionKind::Set
        ),
        Ancestor::ObjectPropertyValue(property) => *property.kind() != PropertyKind::Init,
        _ => false,
    }
}

fn is_plain_parameter(item: &FormalParameter<'_>) -> bool {
    item.initializer.is_none() && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
}

fn later_are_plain(params: &FormalParameters<'_>, index: usize) -> bool {
    params.items.iter().skip(index + 1).all(is_plain_parameter)
}

pub fn declared_slot_fits(
    declaration: &VariableDeclaration<'_>,
    moved: &Moved<'_, '_>,
    params: &FormalParameters<'_>,
    host: &Host,
    cx: &Cx<'_, '_>,
) -> bool {
    let scoping = cx.scoping();
    let Some(pattern) = moved.pattern else {
        return false;
    };
    match params.items.get(moved.index) {
        Some(slot) => {
            is_plain_parameter(slot)
                && later_are_plain(params, moved.index)
                && binding_symbol(pattern).is_some_and(|symbol| {
                    binding_symbol(&slot.pattern) == Some(symbol)
                        && scoping.symbol_redeclarations(symbol).len() == 2
                        && !parameters_mention(params, symbol, scoping)
                })
        }
        None => {
            !host.has_fixed_arity
                && declares_fresh_names(declaration, pattern, scoping)
                && !hoists_block_function(moved.rest, &bound_names(pattern))
        }
    }
}

pub fn loose_slot_fits(
    symbol: SymbolId,
    position: usize,
    params: &FormalParameters<'_>,
    cx: &Cx<'_, '_>,
) -> bool {
    later_are_plain(params, position) && cx.scoping().symbol_redeclarations(symbol).is_empty()
}

fn bound_names<'a>(pattern: &BindingPattern<'a>) -> Vec<&'a str> {
    pattern
        .get_binding_identifiers()
        .iter()
        .map(|identifier| identifier.name.as_str())
        .collect()
}

fn declares_fresh_names(
    declaration: &VariableDeclaration<'_>,
    pattern: &BindingPattern<'_>,
    scoping: &Scoping,
) -> bool {
    let is_constant = match declaration.kind {
        VariableDeclarationKind::Var | VariableDeclarationKind::Let => false,
        VariableDeclarationKind::Const => true,
        VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => return false,
    };
    pattern.get_binding_identifiers().iter().all(|identifier| {
        identifier.symbol_id.get().is_some_and(|symbol| {
            scoping.symbol_redeclarations(symbol).is_empty()
                && !(is_constant && is_written(symbol, scoping))
        })
    })
}

fn is_written(symbol: SymbolId, scoping: &Scoping) -> bool {
    scoping
        .get_resolved_references(symbol)
        .any(|reference| reference.is_write())
}

struct Mentions<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.reference_id.get().is_none_or(|reference| {
            self.scoping.get_reference(reference).symbol_id() == Some(self.symbol)
        });
    }
}

fn parameters_mention(params: &FormalParameters<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    let mut mentions = Mentions {
        scoping,
        symbol,
        found: false,
    };
    mentions.visit_formal_parameters(params);
    mentions.found
}

pub fn default_fits(
    moved: &Moved<'_, '_>,
    params: &FormalParameters<'_>,
    host: &Host,
    cx: &Cx<'_, '_>,
) -> bool {
    if host.is_safe_lane && host.starts_lazily && !runs_no_code(moved) {
        return false;
    }
    if host.is_safe_lane && !keeps_arguments_view(moved, params) {
        return false;
    }
    reads_what_parameters_see(moved, params, host, cx.scoping())
}

fn runs_no_code(moved: &Moved<'_, '_>) -> bool {
    moved
        .pattern
        .is_none_or(|pattern| matches!(pattern, BindingPattern::BindingIdentifier(_)))
        && moved.default.is_none_or(Expression::is_literal)
}

struct ArgumentsUses {
    mapped: usize,
    aliased: bool,
}

impl ArgumentsUses {
    fn is_plain_read(&self, expr: &Expression<'_>) -> bool {
        if is_arguments_length(expr) {
            return true;
        }
        let Expression::ComputedMemberExpression(member) = expr else {
            return false;
        };
        !member.optional
            && is_arguments(&member.object)
            && literal_index(&member.expression).is_some_and(|index| index >= self.mapped)
    }
}

fn is_arguments_member(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => is_arguments(&member.object),
        Expression::ComputedMemberExpression(member) => is_arguments(&member.object),
        _ => false,
    }
}

impl<'a> VisitJs<'a> for ArgumentsUses {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.aliased |= it.name.as_str() == "arguments";
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.is_plain_read(it) {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.aliased |= is_arguments_member(&it.callee);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.aliased |= is_arguments_member(&it.tag);
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
}

fn keeps_arguments_view(moved: &Moved<'_, '_>, params: &FormalParameters<'_>) -> bool {
    if !params.items.iter().all(is_plain_parameter) {
        return true;
    }
    let mut uses = ArgumentsUses {
        mapped: params.items.len().max(moved.index + 1),
        aliased: false,
    };
    for statement in moved.rest {
        uses.visit_statement(statement);
    }
    if let Some(default) = moved.default {
        uses.visit_expression(default);
    }
    if let Some(pattern) = moved.pattern {
        uses.visit_binding_pattern(pattern);
    }
    !uses.aliased
}

struct DefaultReads<'s, 'a> {
    scoping: &'s Scoping,
    scope: ScopeId,
    earlier: Vec<SymbolId>,
    own_name: Option<SymbolId>,
    outer: Vec<&'a str>,
    is_sound: bool,
}

impl<'a> VisitJs<'a> for DefaultReads<'_, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            self.is_sound = false;
            return;
        };
        match self.scoping.get_reference(reference).symbol_id() {
            Some(symbol) if Some(symbol) == self.own_name => self.outer.push(it.name.as_str()),
            Some(symbol) if self.scoping.symbol_scope_id(symbol) == self.scope => {
                self.is_sound &= self.earlier.contains(&symbol);
            }
            _ => self.outer.push(it.name.as_str()),
        }
    }

    fn visit_await_expression(&mut self, _it: &AwaitExpression<'a>) {
        self.is_sound = false;
    }

    fn visit_yield_expression(&mut self, _it: &YieldExpression<'a>) {
        self.is_sound = false;
    }
}

fn earlier_parameters(
    params: &FormalParameters<'_>,
    index: usize,
    scoping: &Scoping,
) -> Vec<SymbolId> {
    params
        .items
        .iter()
        .take(index)
        .flat_map(|item| item.pattern.get_binding_identifiers())
        .filter_map(|identifier| identifier.symbol_id.get())
        .filter(|symbol| scoping.symbol_redeclarations(*symbol).is_empty())
        .collect()
}

fn reads_what_parameters_see(
    moved: &Moved<'_, '_>,
    params: &FormalParameters<'_>,
    host: &Host,
    scoping: &Scoping,
) -> bool {
    let mut reads = DefaultReads {
        scoping,
        scope: host.scope,
        earlier: earlier_parameters(params, moved.index, scoping),
        own_name: host.own_name,
        outer: Vec::new(),
        is_sound: true,
    };
    if let Some(default) = moved.default {
        reads.visit_expression(default);
    }
    if let Some(pattern) = moved.pattern {
        reads.visit_binding_pattern(pattern);
    }
    reads.is_sound && !hoists_block_function(moved.rest, &reads.outer)
}

struct BlockFunctions<'n> {
    names: &'n [&'n str],
    found: bool,
}

impl<'a> VisitJs<'a> for BlockFunctions<'_> {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        self.found |= it.is_declaration()
            && it
                .id
                .as_ref()
                .is_some_and(|id| self.names.contains(&id.name.as_str()));
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
}

fn hoists_block_function(rest: &[Statement<'_>], names: &[&str]) -> bool {
    if names.is_empty() {
        return false;
    }
    let mut functions = BlockFunctions {
        names,
        found: false,
    };
    for statement in rest {
        if !matches!(statement, Statement::FunctionDeclaration(_)) {
            functions.visit_statement(statement);
        }
    }
    functions.found
}
