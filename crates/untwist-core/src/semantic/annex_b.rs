use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, BlockStatement, Class, FormalParameters, Function,
    IfStatement, Program, StaticBlock, SwitchCase,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::node::NodeId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::OnceCell;

use super::{is_constant_binding, is_constant_binding_by, Strictness};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnexBFunction {
    pub function: SymbolId,
    pub var_scope: ScopeId,
    pub target: Option<SymbolId>,
}

#[derive(Debug, Default)]
pub struct AnnexBWrites {
    functions: Vec<AnnexBFunction>,
    written: FxHashSet<SymbolId>,
}

impl AnnexBWrites {
    pub fn of(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut scan = Scan {
            scoping,
            strictness: Strictness::default(),
            frames: Vec::new(),
            block_level: false,
            found: Self::default(),
        };
        scan.visit_program(program);
        scan.found
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    pub fn may_write(&self, symbol: SymbolId) -> bool {
        self.written.contains(&symbol)
    }

    pub fn functions(&self) -> &[AnnexBFunction] {
        &self.functions
    }

    pub fn shared_bindings(&self) -> impl Iterator<Item = (SymbolId, SymbolId)> + '_ {
        self.functions.iter().filter_map(|function| {
            function
                .target
                .filter(|target| *target != function.function)
                .map(|target| (function.function, target))
        })
    }
}

pub fn declared_in_block(scoping: &Scoping, function_scope: Option<ScopeId>) -> bool {
    function_scope
        .and_then(|scope| scoping.scope_parent_id(scope))
        .is_some_and(|parent| !scoping.scope_flags(parent).is_var())
}

fn is_function_declaration(scoping: &Scoping, symbol: SymbolId) -> bool {
    let flags = scoping.symbol_flags(symbol);
    flags.contains(SymbolFlags::Function) && !flags.contains(SymbolFlags::FunctionExpression)
}

fn bisected_owner(scoping: &Scoping, node: NodeId) -> Option<ScopeId> {
    let count = scoping.scopes_len();
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = low + (high - low) / 2;
        if scoping.get_node_id(ScopeId::new(middle)) < node {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    (low < count)
        .then(|| ScopeId::new(low))
        .filter(|scope| scoping.get_node_id(*scope) == node)
}

fn scope_owned_by(scoping: &Scoping, node: NodeId) -> Option<ScopeId> {
    bisected_owner(scoping, node).or_else(|| {
        (0..scoping.scopes_len())
            .map(ScopeId::new)
            .find(|scope| scoping.get_node_id(*scope) == node)
    })
}

pub struct ScopeOwners<'s> {
    scoping: &'s Scoping,
    index: OnceCell<FxHashMap<NodeId, ScopeId>>,
}

impl<'s> ScopeOwners<'s> {
    pub fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            index: OnceCell::new(),
        }
    }

    fn owner(&self, node: NodeId) -> Option<ScopeId> {
        bisected_owner(self.scoping, node).or_else(|| {
            self.index
                .get_or_init(|| first_owners(self.scoping))
                .get(&node)
                .copied()
        })
    }

    pub fn declaring_block(&self, symbol: SymbolId) -> Option<ScopeId> {
        declaring_block_by(self.scoping, symbol, |node| self.owner(node))
    }

    pub fn is_constant_binding(&self, symbol: SymbolId) -> bool {
        is_constant_binding_by(self.scoping, symbol, |symbol| {
            is_hoisted_by(self.scoping, symbol, |symbol| self.declaring_block(symbol))
        })
    }
}

fn first_owners(scoping: &Scoping) -> FxHashMap<NodeId, ScopeId> {
    let mut owners = FxHashMap::default();
    for scope in (0..scoping.scopes_len()).map(ScopeId::new) {
        owners.entry(scoping.get_node_id(scope)).or_insert(scope);
    }
    owners
}

pub fn declaring_block(scoping: &Scoping, symbol: SymbolId) -> Option<ScopeId> {
    declaring_block_by(scoping, symbol, |node| scope_owned_by(scoping, node))
}

fn declaring_block_by(
    scoping: &Scoping,
    symbol: SymbolId,
    owned_by: impl Fn(NodeId) -> Option<ScopeId>,
) -> Option<ScopeId> {
    if !is_function_declaration(scoping, symbol) {
        return None;
    }
    let home = scoping.symbol_scope_id(symbol);
    if !scoping.scope_flags(home).is_var() {
        return Some(home);
    }
    owned_by(scoping.symbol_declaration(symbol))
        .and_then(|scope| scoping.scope_parent_id(scope))
        .filter(|parent| !scoping.scope_flags(*parent).is_var())
}

pub fn is_hoisted_block_function(scoping: &Scoping, symbol: SymbolId) -> bool {
    is_hoisted_by(scoping, symbol, |symbol| declaring_block(scoping, symbol))
}

fn is_hoisted_by(
    scoping: &Scoping,
    symbol: SymbolId,
    declaring: impl Fn(SymbolId) -> Option<ScopeId>,
) -> bool {
    scoping
        .scope_flags(scoping.symbol_scope_id(symbol))
        .is_var()
        && declaring(symbol).is_some()
}

pub fn block_function_symbols(scoping: &Scoping) -> FxHashSet<SymbolId> {
    let in_block: FxHashSet<NodeId> = scoping
        .scope_descendants_from_root()
        .filter(|scope| {
            let flags = scoping.scope_flags(*scope);
            flags.is_function() && !flags.is_arrow() && declared_in_block(scoping, Some(*scope))
        })
        .map(|scope| scoping.get_node_id(scope))
        .collect();
    if in_block.is_empty() {
        return FxHashSet::default();
    }
    scoping
        .symbol_ids()
        .filter(|symbol| is_function_declaration(scoping, *symbol))
        .filter(|symbol| in_block.contains(&scoping.symbol_declaration(*symbol)))
        .collect()
}

pub fn is_constant_binding_in(scoping: &Scoping, writes: &AnnexBWrites, symbol: SymbolId) -> bool {
    is_constant_binding(scoping, symbol) && !writes.may_write(symbol)
}

struct Frame {
    scope: Option<ScopeId>,
    strict: bool,
    parameters: FxHashSet<SymbolId>,
}

struct Scan<'s> {
    scoping: &'s Scoping,
    strictness: Strictness,
    frames: Vec<Frame>,
    block_level: bool,
    found: AnnexBWrites,
}

#[derive(Default)]
struct Parameters(FxHashSet<SymbolId>);

impl<'a> VisitJs<'a> for Parameters {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.0.insert(symbol);
        }
    }
}

fn parameters_of(params: &FormalParameters<'_>) -> FxHashSet<SymbolId> {
    let mut scan = Parameters::default();
    scan.visit_formal_parameters(params);
    scan.0
}

impl Scan<'_> {
    fn enter(&mut self, scope: Option<ScopeId>, parameters: FxHashSet<SymbolId>) -> bool {
        self.frames.push(Frame {
            scope,
            strict: self.strictness.is_strict(),
            parameters,
        });
        std::mem::replace(&mut self.block_level, false)
    }

    fn leave(&mut self, block_level: bool) {
        self.frames.pop();
        self.block_level = block_level;
    }

    fn record(&mut self, function: &Function<'_>) {
        if !self.block_level || function.r#async || function.generator || !function.is_declaration()
        {
            return;
        }
        let Some(frame) = self.frames.last() else {
            return;
        };
        if frame.strict {
            return;
        }
        let (Some(var_scope), Some(id)) = (frame.scope, function.id.as_ref()) else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        let target = self
            .scoping
            .get_binding(var_scope, id.name)
            .filter(|target| !frame.parameters.contains(target));
        if let Some(target) = target {
            self.found.written.insert(target);
        }
        self.found.functions.push(AnnexBFunction {
            function: symbol,
            var_scope,
            target,
        });
    }

    fn in_block(&mut self, walk: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.block_level, true);
        walk(self);
        self.block_level = outer;
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.strictness = Strictness::of_program(it);
        let outer = self.enter(it.scope_id.get(), FxHashSet::default());
        walk_js::walk_program(self, it);
        self.leave(outer);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.record(it);
        let strictness = self.strictness;
        self.strictness = strictness.in_function(it);
        let outer = self.enter(it.scope_id.get(), parameters_of(&it.params));
        walk_js::walk_function(self, it, flags);
        self.leave(outer);
        self.strictness = strictness;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let strictness = self.strictness;
        self.strictness = strictness.in_arrow(it);
        let outer = self.enter(it.scope_id.get(), parameters_of(&it.params));
        walk_js::walk_arrow_function_expression(self, it);
        self.leave(outer);
        self.strictness = strictness;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let strictness = self.strictness;
        self.strictness = strictness.in_class();
        walk_js::walk_class(self, it);
        self.strictness = strictness;
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        let outer = self.enter(it.scope_id.get(), FxHashSet::default());
        walk_js::walk_static_block(self, it);
        self.leave(outer);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.in_block(|scan| walk_js::walk_block_statement(scan, it));
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        self.in_block(|scan| walk_js::walk_switch_case(scan, it));
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.in_block(|scan| walk_js::walk_if_statement(scan, it));
    }
}
