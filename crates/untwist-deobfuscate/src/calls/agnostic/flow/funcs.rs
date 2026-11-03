use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

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
    pub(super) global_writes: FxHashSet<String>,
    pub(super) global_unknown_write: bool,
    pub(super) written_keys: FxHashSet<String>,
    pub(super) accessor_names: FxHashSet<String>,
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
    funcs: FxHashMap<u32, Raw>,
    by_symbol: FxHashMap<SymbolId, u32>,
    uses_arguments: FxHashSet<u32>,
    callee_next: Option<u32>,
    refs: Vec<u8>,
    iife_starts: FxHashSet<u32>,
    global_writes: FxHashSet<String>,
    global_unknown_write: bool,
    written_keys: FxHashSet<String>,
    accessor_names: FxHashSet<String>,
    stack: Vec<u32>,
}

impl<'s> Prescan<'s> {
    pub(super) fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            funcs: FxHashMap::default(),
            by_symbol: FxHashMap::default(),
            uses_arguments: FxHashSet::default(),
            callee_next: None,
            refs: vec![0; scoping.symbols_len()],
            iife_starts: FxHashSet::default(),
            global_writes: FxHashSet::default(),
            global_unknown_write: false,
            written_keys: FxHashSet::default(),
            accessor_names: FxHashSet::default(),
            stack: Vec::new(),
        }
    }

    pub(super) fn finish(self) -> Pre {
        let Prescan {
            funcs,
            by_symbol,
            uses_arguments,
            refs,
            iife_starts,
            global_writes,
            global_unknown_write,
            written_keys,
            accessor_names,
            ..
        } = self;
        let named_starts: FxHashSet<u32> = by_symbol.values().copied().collect();
        let mut start_refs: FxHashMap<u32, u8> = FxHashMap::default();
        for (symbol, start) in &by_symbol {
            let flags = refs.get(symbol.index()).copied().unwrap_or(0);
            if flags & SEEN != 0 {
                *start_refs.entry(*start).or_insert(0) |= flags;
            }
        }
        let mut resolved = FxHashMap::default();
        for (start, raw) in funcs {
            let fate = if raw.async_or_gen || uses_arguments.contains(&start) {
                RetFate::Sink
            } else if named_starts.contains(&start) {
                match start_refs.get(&start) {
                    None => RetFate::Inert,
                    Some(flags) if flags & NON_CALLEE != 0 => RetFate::Sink,
                    Some(_) => RetFate::Routes,
                }
            } else if iife_starts.contains(&start) {
                RetFate::Routes
            } else {
                RetFate::Sink
            };
            let opaque = raw.opaque || uses_arguments.contains(&start);
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
            global_writes,
            global_unknown_write,
            written_keys,
            accessor_names,
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

    fn unresolved(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_none()
    }

    fn global_base(&self, object: &Expression<'_>) -> bool {
        match object.without_parentheses() {
            Expression::Identifier(id) => self.unresolved(id),
            Expression::ThisExpression(_) => true,
            _ => false,
        }
    }

    fn key_name(member: &MemberExpression<'_>) -> Option<String> {
        match member {
            MemberExpression::StaticMemberExpression(m) => Some(m.property.name.to_string()),
            MemberExpression::ComputedMemberExpression(m) => {
                match m.expression.without_parentheses() {
                    Expression::StringLiteral(text) => Some(text.value.to_string()),
                    Expression::NumericLiteral(num) => Some(num.value.to_string()),
                    _ => None,
                }
            }
            MemberExpression::PrivateFieldExpression(_) => None,
        }
    }

    fn note_target(&mut self, target: &AssignmentTarget<'_>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
            if self.unresolved(id) {
                self.global_writes.insert(id.name.to_string());
            }
            return;
        }
        let Some(member) = target.as_member_expression() else {
            return;
        };
        let global = self.global_base(member.object());
        match Self::key_name(member) {
            Some(name) => {
                if global {
                    self.global_writes.insert(name.clone());
                }
                self.written_keys.insert(name);
            }
            None => {
                if global {
                    self.global_unknown_write = true;
                }
            }
        }
    }
}

impl<'a> VisitJs<'a> for Prescan<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.record(it.span, &it.params, it.r#async || it.generator);
        if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
            self.by_symbol.insert(symbol, it.span.start);
        }
        self.stack.push(it.span.start);
        walk_js::walk_function(self, it, flags);
        self.stack.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.record(it.span, &it.params, it.r#async);
        self.stack.push(it.span.start);
        walk_js::walk_arrow_function_expression(self, it);
        self.stack.pop();
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

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.note_iife(&it.callee);
        if let Expression::Identifier(id) = it.callee.without_parentheses() {
            self.callee_next = Some(id.span.start);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.note_target(&it.left);
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if matches!(it.kind, PropertyKind::Get | PropertyKind::Set) {
            if let Some(name) = it.key.static_name() {
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
            if let Some(name) = it.key.static_name() {
                self.accessor_names.insert(name.into_owned());
            }
        }
        walk_js::walk_method_definition(self, it);
    }
}
