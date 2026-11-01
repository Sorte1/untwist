use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::bootstrap::{self, Resolve};
pub(super) use super::defs::Defs;
use super::defs::{
    Def, DefTable, CALLABLE, CONSTRUCTS_FRESH, ESCAPES, ESCAPING_REF, FIELDS_WRITTEN,
    METHOD_CALLED, MUTATED, OPAQUE,
};
use super::live::iife_returned;
use super::settle::Written;
use super::source::Sources;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default)]
struct ThisScan {
    found: bool,
}

impl<'a> ThisScan {
    fn scan_key(&mut self, key: &PropertyKey<'a>) {
        if let Some(expr) = key.as_expression() {
            self.visit_expression(expr);
        }
    }
}

impl<'a> VisitJs<'a> for ThisScan {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(heritage) = &it.heritage {
            self.visit_expression(&heritage.expression);
        }
        for element in &it.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    for statement in &block.body {
                        self.visit_statement(statement);
                    }
                }
                ClassElement::MethodDefinition(m) if m.computed => self.scan_key(&m.key),
                ClassElement::PropertyDefinition(m) if m.computed => self.scan_key(&m.key),
                ClassElement::AccessorProperty(m) if m.computed => self.scan_key(&m.key),
                _ => {}
            }
        }
    }
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }
    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

#[derive(Default)]
struct ArgumentsScan {
    found: bool,
}

impl<'a> VisitJs<'a> for ArgumentsScan {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

pub(super) fn reads_arguments(function: &Function<'_>) -> bool {
    let mut scan = ArgumentsScan::default();
    scan.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        for statement in &body.statements {
            if scan.found {
                break;
            }
            scan.visit_statement(statement);
        }
    }
    scan.found
}

pub(super) fn function_uses_this(function: &Function<'_>) -> bool {
    let mut scan = ThisScan::default();
    scan.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        for statement in &body.statements {
            scan.visit_statement(statement);
        }
    }
    scan.found
}

#[derive(Default)]
struct Returns {
    found: bool,
}

impl<'a> VisitJs<'a> for Returns {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _it: &Class<'a>) {}
    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.found |= it.argument.is_some();
    }
}

fn body_returns(body: Option<&FunctionBody<'_>>) -> bool {
    let mut scan = Returns::default();
    if let Some(body) = body {
        for statement in &body.statements {
            scan.visit_statement(statement);
        }
    }
    scan.found
}

fn plain_param_syms(func: &Function<'_>) -> Vec<SymbolId> {
    func.params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn inlinable_func<'a>(defs: &Defs<'_, 'a>) -> Option<&'a Function<'a>> {
    if defs.opaque
        || defs.mutated
        || defs.escaping_ref
        || defs.callee_refs == 0
        || defs.refs != defs.callee_refs
    {
        return None;
    }
    let func = defs.func?;
    if func.generator || func.r#async || func.params.rest.is_some() {
        return None;
    }
    if !func.params.items.iter().all(|param| {
        param.initializer.is_none() && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
    }) {
        return None;
    }
    let body = func.body.as_ref()?;
    let mut scan = ThisScan::default();
    for statement in &body.statements {
        scan.visit_statement(statement);
    }
    if scan.found {
        return None;
    }
    Some(func)
}

fn inlinable_of(defs: &DefTable<'_>, scoping: &Scoping) -> FxHashSet<u32> {
    let mut inlinable = FxHashSet::default();
    for (_, found) in defs.present() {
        if let Some(func) = inlinable_func(&found) {
            if plain_param_syms(func).iter().all(|sym| {
                !scoping.symbol_is_mutated(*sym)
                    && defs.get(*sym).is_none_or(|d| d.writes.is_empty())
            }) {
                inlinable.insert(func.span.start);
            }
        }
    }
    inlinable
}

pub(super) fn function_constructs_fresh(function: &Function<'_>) -> bool {
    !function.generator && !function.r#async && !body_returns(function.body.as_deref())
}

pub(super) fn class_constructs_fresh(class: &Class<'_>) -> bool {
    if class.heritage.is_some() {
        return false;
    }
    class.body.body.iter().all(|element| match element {
        ClassElement::MethodDefinition(method)
            if method.kind == MethodDefinitionKind::Constructor =>
        {
            !body_returns(method.value.body.as_deref())
        }
        _ => true,
    })
}

fn bare_symbol_written(scoping: &Scoping) -> bool {
    scoping
        .root_unresolved_references()
        .get("Symbol")
        .is_some_and(|references| {
            references
                .iter()
                .any(|reference| scoping.get_reference(*reference).is_write())
        })
}

pub(super) struct Bindings<'s, 'a> {
    scoping: &'s Scoping,
    pub(super) defs: DefTable<'a>,
    handled: FxHashSet<u32>,
    plain_targets: FxHashSet<u32>,
    for_left: bool,
    pub(super) annex: FxHashSet<&'a str>,
    pub(super) inlinable: FxHashSet<u32>,
    hidden: FxHashSet<u32>,
    symbol_rebind: bool,
    pub(super) sources: Sources,
    calls_of: Vec<(SymbolId, &'a CallExpression<'a>)>,
    pub(super) params: FxHashMap<SymbolId, (SymbolId, usize)>,
    returned: FxHashSet<SymbolId>,
    tracks_calls: bool,
    candidates: Vec<&'a CallExpression<'a>>,
    owners: FxHashSet<SymbolId>,
    tables: Vec<SymbolId>,
    uncalled: Vec<u32>,
    pub(super) bootstrap_refusal: Option<&'static str>,
}

impl<'s, 'a> Bindings<'s, 'a> {
    pub(super) fn collect(
        program: &'a Program<'a>,
        scoping: &'s Scoping,
        tracks_calls: bool,
        resolve: Resolve<'_>,
    ) -> Self {
        let mut bindings = Self {
            scoping,
            defs: DefTable::new(scoping.symbols_len()),
            handled: FxHashSet::default(),
            plain_targets: FxHashSet::default(),
            for_left: false,
            annex: FxHashSet::default(),
            inlinable: FxHashSet::default(),
            hidden: FxHashSet::default(),
            symbol_rebind: false,
            sources: Sources::new(program),
            calls_of: Vec::new(),
            params: FxHashMap::default(),
            returned: FxHashSet::default(),
            tracks_calls,
            candidates: Vec::new(),
            owners: FxHashSet::default(),
            tables: Vec::new(),
            uncalled: Vec::new(),
            bootstrap_refusal: None,
        };
        bindings.visit_program(program);
        bindings.defs.finish();
        bindings.symbol_rebind |= bare_symbol_written(scoping);
        let annex = std::mem::take(&mut bindings.annex);
        bindings.defs.each_present_mut(|symbol, def| {
            def.set_to(MUTATED, scoping.symbol_is_mutated(symbol));
            if annex.contains(scoping.symbol_name(symbol)) {
                def.set(OPAQUE);
            }
        });
        bindings.annex = annex;
        bindings.inlinable = inlinable_of(&bindings.defs, scoping);
        bindings.prove_bootstraps(program, resolve);
        if tracks_calls {
            bindings.collect_params();
            bindings
                .calls_of
                .sort_unstable_by_key(|(callee, call)| (callee.index(), call.span.start));
        }
        bindings
    }

    fn collect_params(&mut self) {
        let mut params = std::mem::take(&mut self.params);
        let mut returned = FxHashSet::default();
        for (symbol, defs) in self.defs.present() {
            let func = match (defs.func, defs.inits) {
                (Some(func), _) => func,
                (None, [Some(init)]) if !defs.callable => match iife_returned(init) {
                    Some(func) => {
                        returned.insert(symbol);
                        func
                    }
                    None => continue,
                },
                _ => continue,
            };
            if func.params.rest.is_some() {
                continue;
            }
            for (index, param) in func.params.items.iter().enumerate() {
                if let BindingPattern::BindingIdentifier(id) = &param.pattern {
                    if let Some(psym) = id.symbol_id.get() {
                        params.insert(psym, (symbol, index));
                    }
                }
            }
        }
        self.params = params;
        self.returned = returned;
    }

    fn prove_bootstraps(&mut self, program: &'a Program<'a>, resolve: Resolve<'_>) {
        for call in std::mem::take(&mut self.candidates) {
            match bootstrap::prove(program, self.scoping, resolve, call) {
                Ok(proven) => {
                    self.tables.extend(proven.table);
                    self.uncalled.extend(proven.uncalled);
                    if self.tracks_calls {
                        self.params.extend(proven.params);
                        self.calls_of.extend(proven.calls);
                        self.owners.extend(proven.owners);
                    }
                }
                Err(bootstrap::NOT_A_BOOTSTRAP) => {}
                Err(reason) => {
                    self.bootstrap_refusal.get_or_insert(reason);
                }
            }
        }
    }

    pub(super) fn is_table(&self, symbol: SymbolId) -> bool {
        self.tables.contains(&symbol)
    }

    fn is_iife_owner(&self, owner: SymbolId) -> bool {
        self.params.get(&owner) == Some(&(owner, 0))
    }

    pub(super) fn param_of(&self, symbol: SymbolId) -> Option<(SymbolId, usize)> {
        let (func, index) = self.params.get(&symbol).copied()?;
        (self.is_iife_owner(func) || self.owners.contains(&func) || self.does_not_escape(func))
            .then_some((func, index))
    }

    fn note_iife(&mut self, call: &CallExpression<'a>) {
        if call.optional {
            return;
        }
        let Expression::FunctionExpression(func) = call.callee.without_parentheses() else {
            return;
        };
        if bootstrap::table_candidate(call) {
            let call = self.alloc(call);
            self.candidates.push(call);
        }
        if !self.tracks_calls {
            return;
        }
        let recursive = func.id.as_ref().is_some_and(|id| {
            id.symbol_id
                .get()
                .is_none_or(|symbol| !self.scoping.get_resolved_reference_ids(symbol).is_empty())
        });
        let spread = call
            .arguments
            .iter()
            .any(|argument| argument.as_expression().is_none());
        if func.params.items.is_empty()
            || recursive
            || spread
            || func.params.rest.is_some()
            || reads_arguments(func)
        {
            return;
        }
        let mut owner = None;
        for (index, param) in func.params.items.iter().enumerate() {
            let BindingPattern::BindingIdentifier(id) = &param.pattern else {
                continue;
            };
            let Some(psym) = id.symbol_id.get() else {
                continue;
            };
            let owner = *owner.get_or_insert(psym);
            self.params.insert(psym, (owner, index));
        }
        if let Some(owner) = owner {
            let call = self.alloc(call);
            self.calls_of.push((owner, call));
        }
    }

    pub(super) fn does_not_escape(&self, func: SymbolId) -> bool {
        self.defs.get(func).is_some_and(|defs| {
            (defs.callable || self.returned.contains(&func))
                && !defs.opaque
                && !defs.mutated
                && !defs.escaping_ref
                && !defs.method_called
                && defs.callee_refs > 0
                && defs.refs == defs.callee_refs
        })
    }

    pub(super) fn call_sites(
        &self,
        func: SymbolId,
    ) -> impl Iterator<Item = &'a CallExpression<'a>> + '_ {
        let start = self
            .calls_of
            .partition_point(|(callee, _)| callee.index() < func.index());
        self.calls_of[start..]
            .iter()
            .take_while(move |(callee, _)| *callee == func)
            .map(|(_, call)| *call)
    }

    pub(super) fn is_array_function(&self, symbol: SymbolId) -> bool {
        let Some(defs) = self.defs.get(symbol) else {
            return false;
        };
        let Some(func) = defs.func else {
            return false;
        };
        let inside = |value: &Option<&Expression<'_>>| {
            value.is_some_and(|value| {
                let span = value.span();
                func.span.start <= span.start && span.end <= func.span.end
            })
        };
        !defs.opaque
            && defs.writes.iter().all(inside)
            && crate::string_array::is_string_array_function(func, self.scoping.symbol_name(symbol))
    }

    pub(super) fn never_reassigned(&self, symbol: SymbolId) -> bool {
        !self.scoping.symbol_is_mutated(symbol)
            && self
                .defs
                .get(symbol)
                .is_none_or(|defs| defs.writes.is_empty())
    }

    fn entry(&mut self, symbol: SymbolId) -> &mut Def {
        self.defs.entry(symbol)
    }

    pub(super) fn symbol_safe(&self) -> bool {
        !self.symbol_rebind
    }

    pub(super) fn global_source(&self) -> bool {
        self.sources.found()
    }

    pub(super) fn symbol_name(&self, symbol: SymbolId) -> &'s str {
        self.scoping.symbol_name(symbol)
    }

    pub(super) fn hide_calls(&mut self, globals: &[SymbolId]) {
        for symbol in globals {
            let func = self.defs.func(*symbol);
            if let Some(func) = func.filter(|func| self.inlinable.contains(&func.span.start)) {
                self.hidden.insert(func.span.start);
            }
        }
    }

    pub(super) fn walks_in_place(&self, func: &Function<'_>) -> bool {
        let start = func.span.start;
        !self.uncalled.contains(&start)
            && (!self.inlinable.contains(&start) || self.hidden.contains(&start))
    }

    pub(super) fn is_opaque(&self, symbol: SymbolId) -> bool {
        self.defs.get(symbol).is_some_and(|defs| defs.opaque)
    }

    pub(super) fn state_written(&self, symbol: SymbolId, written: &Written<'_>) -> bool {
        let Some(defs) = self.defs.get(symbol) else {
            return false;
        };
        !defs.callable
            && defs
                .inits
                .iter()
                .flatten()
                .any(|init| match init.without_parentheses() {
                    Expression::ObjectExpression(object) => {
                        object.properties.iter().any(|property| match property {
                            ObjectPropertyKind::ObjectProperty(p) => {
                                p.key.static_name().is_none_or(|name| written.may_be(&name))
                            }
                            ObjectPropertyKind::SpreadProperty(_) => true,
                        })
                    }
                    Expression::ArrayExpression(_) => written.may_be_element(),
                    _ => false,
                })
    }

    pub(super) fn make_opaque(&mut self, symbols: &[SymbolId]) {
        for symbol in symbols {
            self.entry(*symbol).set(OPAQUE);
            if let Some(func) = self.defs.func(*symbol) {
                self.inlinable.remove(&func.span.start);
            }
        }
    }

    fn member_written(&mut self, member: &MemberExpression<'_>) {
        if let Expression::Identifier(id) = member.object().without_parentheses() {
            if let Some(symbol) = self.reference_symbol(id) {
                self.entry(symbol).set(FIELDS_WRITTEN);
            }
        }
    }

    pub(super) fn reference_symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }
}

impl<'a> VisitJs<'a> for Bindings<'_, 'a> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), false) = (&it.id, self.for_left) {
            if let Some(symbol) = id.symbol_id.get() {
                self.handled.insert(id.span.start);
                let init = it.init.as_ref().map(|init| self.alloc(init));
                if let Some(Expression::FunctionExpression(func)) =
                    init.map(Expression::without_parentheses)
                {
                    let func = self.alloc(func);
                    self.defs.set_func(symbol, func);
                }
                self.defs.push_init(symbol, init);
            }
        }
        let outer = std::mem::replace(&mut self.for_left, false);
        walk_js::walk_variable_declarator(self, it);
        self.for_left = outer;
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.for_left = true;
        self.visit_for_statement_left(&it.left);
        self.for_left = false;
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.for_left = true;
        self.visit_for_statement_left(&it.left);
        self.for_left = false;
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (Some(id), true) = (&it.id, it.is_declaration()) {
            if let Some(symbol) = id.symbol_id.get() {
                self.handled.insert(id.span.start);
                if crate::bindings::is_block_function(self.scoping, symbol) {
                    self.annex.insert(id.name.as_str());
                }
                let fresh = function_constructs_fresh(it);
                let func = self.alloc(it);
                let defs = self.entry(symbol);
                defs.set(CALLABLE);
                defs.set_to(CONSTRUCTS_FRESH, fresh);
                self.defs.set_func(symbol, func);
            }
        }
        let saved = self.sources.enter_function(it);
        walk_js::walk_function(self, it, flags);
        self.sources.leave(saved);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let (Some(id), true) = (&it.id, it.is_declaration()) {
            if let Some(symbol) = id.symbol_id.get() {
                self.handled.insert(id.span.start);
                let fresh = class_constructs_fresh(it);
                let defs = self.entry(symbol);
                defs.set(CALLABLE);
                defs.set_to(CONSTRUCTS_FRESH, fresh);
            }
        }
        let saved = self.sources.enter_class(false);
        walk_js::walk_class(self, it);
        self.sources.leave(saved);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if let Some(symbol) = self.reference_symbol(id) {
                self.plain_targets.insert(id.span.start);
                let value = match it.operator {
                    AssignmentOperator::Assign | AssignmentOperator::Addition => {
                        Some(Some(self.alloc(&it.right)))
                    }
                    operator if operator.is_arithmetic() || operator.is_bitwise() => None,
                    _ => Some(None),
                };
                if let Some(value) = value {
                    self.defs.push_write(symbol, value);
                }
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &it.argument {
            self.plain_targets.insert(id.span.start);
        }
        walk_js::walk_update_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            if !self.plain_targets.contains(&id.span.start) {
                if let Some(symbol) = self.reference_symbol(id) {
                    self.defs.push_write(symbol, None);
                }
            }
        }
        if let Some(member) = it.as_member_expression() {
            self.member_written(member);
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                self.member_written(member);
            }
        }
        if it.operator == UnaryOperator::Delete {
            self.scan_delete(it);
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.note_iife(it);
        if let Some(member) = it.callee.without_parentheses().as_member_expression() {
            if let Expression::Identifier(id) = member.object().without_parentheses() {
                if let Some(symbol) = self.reference_symbol(id) {
                    self.entry(symbol).set(METHOD_CALLED);
                }
            }
        }
        if !it.optional {
            if let Expression::Identifier(id) = it.callee.without_parentheses() {
                if let Some(symbol) = self.reference_symbol(id) {
                    let spread = it
                        .arguments
                        .iter()
                        .any(|argument| argument.as_expression().is_none());
                    let defs = self.entry(symbol);
                    defs.callee_refs += 1;
                    if spread {
                        defs.set(ESCAPING_REF);
                    }
                    if self.tracks_calls {
                        let call = self.alloc(it);
                        self.calls_of.push((symbol, call));
                    }
                }
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if let Some(member) = it.tag.without_parentheses().as_member_expression() {
            if let Expression::Identifier(id) = member.object().without_parentheses() {
                if let Some(symbol) = self.reference_symbol(id) {
                    self.entry(symbol).set(METHOD_CALLED);
                }
            }
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if let Some(symbol) = self.reference_symbol(&it.binding) {
            self.defs.push_write(symbol, None);
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.scan_member(it);
        if let Expression::Identifier(id) = it.object().without_parentheses() {
            if let MemberExpression::ComputedMemberExpression(member) = it {
                self.visit_expression(&member.expression);
            }
            if let Some(symbol) = self.reference_symbol(id) {
                self.entry(symbol).set(ESCAPING_REF);
            }
            return;
        }
        walk_js::walk_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.scan_identifier(it);
        if let Some(symbol) = self.reference_symbol(it) {
            let defs = self.entry(symbol);
            defs.set(ESCAPES);
            defs.refs += 1;
        }
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.sources.note_this();
    }

    fn visit_super(&mut self, _it: &Super) {
        self.sources.note_this();
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        for decorator in &it.decorators {
            self.visit_decorator(decorator);
        }
        self.visit_property_key(&it.key);
        let saved = self.sources.enter_class(true);
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
        self.sources.leave(saved);
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        for decorator in &it.decorators {
            self.visit_decorator(decorator);
        }
        self.visit_property_key(&it.key);
        let saved = self.sources.enter_class(true);
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
        self.sources.leave(saved);
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        let saved = self.sources.enter_class(true);
        walk_js::walk_static_block(self, it);
        self.sources.leave(saved);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.sources.note();
        walk_js::walk_with_statement(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.sources.note();
        walk_js::walk_import_expression(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.scan_pattern_key(&it.key);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.scan_pattern_key(&it.name);
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.handled.contains(&it.span.start) {
            return;
        }
        if let Some(symbol) = it.symbol_id.get() {
            self.entry(symbol).set(OPAQUE);
        }
    }
}
