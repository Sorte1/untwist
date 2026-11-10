use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, BlockStatement, Class, FormalParameters, Function,
    IfStatement, Program, StaticBlock, SwitchCase,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::{is_constant_binding, Strictness};

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
