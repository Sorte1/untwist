use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::globals::Globals;
use crate::calls::agnostic::bound_once::BoundOnce;
use crate::calls::agnostic::text::closed_runner_read;

const SEEN: u8 = 1;
const NON_CALLEE: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RetFate {
    Routes,
    Sink,
    Inert,
}

pub(super) struct Func {
    pub(super) params: Vec<Option<SymbolId>>,
    pub(super) opaque: bool,
    pub(super) span: Span,
    pub(super) fate: RetFate,
}

pub(super) struct Pre {
    pub(super) funcs: FxHashMap<u32, Func>,
    pub(super) by_symbol: FxHashMap<SymbolId, u32>,
    pub(super) globals: Globals,
    pub(super) exposed: FxHashSet<SymbolId>,
    pub(super) accessor_names: FxHashSet<String>,
    pub(super) closed_reads: FxHashSet<u32>,
}

impl Pre {
    pub(super) fn fate_of(&self, start: u32) -> Option<RetFate> {
        self.funcs.get(&start).map(|func| func.fate)
    }
}

pub(super) fn param_symbol(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn params_opaque(params: &FormalParameters<'_>) -> bool {
    params.rest.is_some()
        || params
            .items
            .iter()
            .any(|param| param_symbol(param).is_none())
}

struct Raw {
    params: Vec<Option<SymbolId>>,
    opaque: bool,
    span: Span,
    async_or_gen: bool,
}

pub(super) struct Prescan<'s> {
    scoping: &'s Scoping,
    script: bool,
    funcs: FxHashMap<u32, Raw>,
    by_symbol: FxHashMap<SymbolId, u32>,
    holders: FxHashSet<u32>,
    uses_arguments: FxHashSet<u32>,
    callee_next: Option<u32>,
    refs: Vec<u8>,
    iife_starts: FxHashSet<u32>,
    globals: Globals,
    accessor_names: FxHashSet<String>,
    closed_reads: FxHashSet<u32>,
    stack: Vec<u32>,
    this_global: Vec<bool>,
    strict: Vec<bool>,
}

impl<'s> Prescan<'s> {
    pub(super) fn new(scoping: &'s Scoping, program: &Program<'_>) -> Self {
        let script = !untwist_core::semantic::has_module_syntax(program);
        Self {
            scoping,
            script,
            funcs: FxHashMap::default(),
            by_symbol: FxHashMap::default(),
            holders: FxHashSet::default(),
            uses_arguments: FxHashSet::default(),
            callee_next: None,
            refs: vec![0; scoping.symbols_len()],
            iife_starts: FxHashSet::default(),
            globals: Globals::default(),
            accessor_names: FxHashSet::default(),
            closed_reads: FxHashSet::default(),
            stack: Vec::new(),
            this_global: vec![script],
            strict: vec![untwist_core::semantic::program_is_strict(program)],
        }
    }

    fn exposed(&self, symbol: SymbolId) -> bool {
        self.script
            && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            && self.globals.may_reach(self.scoping.symbol_name(symbol))
    }

    pub(super) fn finish(self) -> Pre {
        let bound = BoundOnce::new(self.scoping);
        let exposed: FxHashSet<SymbolId> = self
            .by_symbol
            .keys()
            .copied()
            .chain(self.scoping.iter_bindings_in(self.scoping.root_scope_id()))
            .filter(|symbol| self.exposed(*symbol))
            .collect();
        let resolvable = |symbol: &SymbolId| bound.holds(*symbol) && !exposed.contains(symbol);
        let mut fates: FxHashMap<u32, (u8, bool)> = FxHashMap::default();
        for (symbol, start) in &self.by_symbol {
            let flags = self.refs.get(symbol.index()).copied().unwrap_or(0);
            let entry = fates.entry(*start).or_insert((0, true));
            entry.0 |= flags;
            entry.1 &= resolvable(symbol);
        }
        let by_symbol: FxHashMap<SymbolId, u32> = self
            .by_symbol
            .iter()
            .filter(|(symbol, _)| resolvable(symbol))
            .map(|(symbol, start)| (*symbol, *start))
            .collect();
        let mut resolved = FxHashMap::default();
        for (start, raw) in self.funcs {
            let fate = if raw.async_or_gen || self.uses_arguments.contains(&start) {
                RetFate::Sink
            } else {
                Self::fate(
                    fates.get(&start).copied(),
                    self.holders.contains(&start),
                    self.iife_starts.contains(&start),
                )
            };
            let opaque = raw.opaque || self.uses_arguments.contains(&start);
            resolved.insert(
                start,
                Func {
                    params: raw.params,
                    opaque,
                    span: raw.span,
                    fate,
                },
            );
        }
        Pre {
            funcs: resolved,
            by_symbol,
            globals: self.globals,
            exposed,
            accessor_names: self.accessor_names,
            closed_reads: self.closed_reads,
        }
    }

    fn fate(names: Option<(u8, bool)>, held: bool, iife: bool) -> RetFate {
        let (flags, resolvable) = names.unwrap_or((0, true));
        if !resolvable || flags & NON_CALLEE != 0 {
            return RetFate::Sink;
        }
        if iife {
            return RetFate::Routes;
        }
        if !held {
            return RetFate::Sink;
        }
        if flags & SEEN == 0 {
            RetFate::Inert
        } else {
            RetFate::Routes
        }
    }

    fn record(&mut self, span: Span, params: &FormalParameters<'_>, async_or_gen: bool) {
        self.funcs.insert(
            span.start,
            Raw {
                params: params.items.iter().map(param_symbol).collect(),
                opaque: params_opaque(params),
                span,
                async_or_gen,
            },
        );
    }

    fn note_iife(&mut self, callee: &Expression<'_>) {
        match callee.without_parentheses() {
            Expression::FunctionExpression(func) => {
                self.iife_starts.insert(func.span.start);
            }
            Expression::ArrowFunctionExpression(arrow) => {
                self.iife_starts.insert(arrow.span.start);
            }
            _ => {}
        }
    }

    fn this_global(&self) -> bool {
        self.this_global.last().copied().unwrap_or(true)
    }

    fn enter(&mut self, strict: bool, this_global: Option<bool>) {
        let strict = strict || self.strict.last().copied().unwrap_or(false);
        let this_global = this_global.unwrap_or(!strict);
        self.strict.push(strict);
        self.this_global.push(this_global);
    }

    fn leave(&mut self) {
        self.strict.pop();
        self.this_global.pop();
    }
}

impl<'a> VisitJs<'a> for Prescan<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.record(it.span, &it.params, it.r#async || it.generator);
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            self.by_symbol.insert(symbol, it.span.start);
            if it.is_declaration() {
                self.holders.insert(it.span.start);
            }
        }
        self.stack.push(it.span.start);
        self.enter(it.has_use_strict_directive(), None);
        walk_js::walk_function(self, it, flags);
        self.leave();
        self.stack.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.record(it.span, &it.params, it.r#async);
        self.stack.push(it.span.start);
        let this_global = self.this_global();
        self.enter(it.body.has_use_strict_directive(), Some(this_global));
        walk_js::walk_arrow_function_expression(self, it);
        self.leave();
        self.stack.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.enter(true, Some(false));
        walk_js::walk_class(self, it);
        self.leave();
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            let init = it.init.as_ref().map(Expression::without_parentheses);
            let start = match init {
                Some(Expression::FunctionExpression(func)) => Some(func.span.start),
                Some(Expression::ArrowFunctionExpression(arrow)) => Some(arrow.span.start),
                _ => None,
            };
            if let (Some(symbol), Some(start)) = (id.symbol_id.get(), start) {
                self.by_symbol.insert(symbol, start);
                self.holders.insert(start);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "arguments" {
            for start in &self.stack {
                self.uses_arguments.insert(*start);
            }
        }
        self.globals.note_identifier(self.scoping, it);
        if let Some(symbol) = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        {
            let is_callee = self.callee_next.take() == Some(it.span.start);
            if let Some(flags) = self.refs.get_mut(symbol.index()) {
                *flags |= if is_callee { SEEN } else { SEEN | NON_CALLEE };
            }
        }
        walk_js::walk_identifier_reference(self, it);
    }

    fn visit_this_expression(&mut self, it: &ThisExpression) {
        self.globals.note_this(it.span, self.this_global());
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.globals.note_member_value(it);
        self.globals
            .note_member(self.scoping, it, self.this_global());
        walk_js::walk_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.note_iife(&it.callee);
        if let Expression::Identifier(id) = it.callee.without_parentheses() {
            self.callee_next = Some(id.span.start);
            if id.name == "eval" && closed_runner_read(it, self.scoping).is_some() {
                self.closed_reads.insert(it.span.start);
                self.globals.note_closed_eval(id.span.start);
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.globals
            .note_target(self.scoping, it, self.this_global());
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.globals.note_name_write(self.scoping, &it.binding);
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            match it.argument.without_parentheses() {
                Expression::Identifier(id) => self.globals.note_name_write(self.scoping, id),
                other => {
                    if let Some(member) = other.as_member_expression() {
                        self.globals
                            .note_member_write(self.scoping, member, self.this_global());
                    }
                }
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.globals.with = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if matches!(it.kind, PropertyKind::Get | PropertyKind::Set) {
            if let Some(name) = it.key.static_name().and_then(|name| name.into_cow_str()) {
                self.accessor_names.insert(name.into_owned());
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if matches!(
            it.kind,
            MethodDefinitionKind::Get | MethodDefinitionKind::Set
        ) {
            if let Some(name) = it.key.static_name().and_then(|name| name.into_cow_str()) {
                self.accessor_names.insert(name.into_owned());
            }
        }
        walk_js::walk_method_definition(self, it);
    }
}
