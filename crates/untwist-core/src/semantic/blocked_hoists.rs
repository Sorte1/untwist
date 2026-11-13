use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, CatchClause, Function, IdentifierReference, Program,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::node::NodeId;
use oxc_syntax::reference::{Reference, ReferenceId};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

use super::annex_b::{block_function_symbols, ScopeOwners};

const LEXICAL: SymbolFlags = SymbolFlags::BlockScopedVariable
    .union(SymbolFlags::ConstVariable)
    .union(SymbolFlags::Class);

#[derive(Clone, Copy)]
struct BlockFunction {
    function: SymbolId,
    block: ScopeId,
    var: ScopeId,
}

impl BlockFunction {
    fn moved(self, scoping: &Scoping) -> bool {
        scoping.symbol_scope_id(self.function) != self.block
    }
}

struct Plan {
    functions: Vec<BlockFunction>,
    declared_in: FxHashSet<(ScopeId, String)>,
}

impl Plan {
    fn of(scoping: &Scoping) -> Option<Self> {
        let symbols = block_function_symbols(scoping);
        if symbols.is_empty() {
            return None;
        }
        let owners = ScopeOwners::new(scoping);
        let mut functions: Vec<BlockFunction> = symbols
            .into_iter()
            .filter_map(|function| {
                let block = owners.declaring_block(function)?;
                let var = scoping
                    .scope_ancestors(block)
                    .find(|scope| scoping.scope_flags(*scope).is_var())?;
                Some(BlockFunction {
                    function,
                    block,
                    var,
                })
            })
            .collect();
        functions.sort_unstable_by_key(|found| found.function);
        if !functions.iter().any(|found| found.moved(scoping)) {
            return None;
        }
        let declared_in = functions
            .iter()
            .map(|found| (found.block, scoping.symbol_name(found.function).to_string()))
            .collect();
        Some(Self {
            functions,
            declared_in,
        })
    }

    fn lexical_between(&self, scoping: &Scoping, found: BlockFunction) -> bool {
        let name = scoping.symbol_ident(found.function);
        scoping
            .scope_ancestors(found.block)
            .skip(1)
            .take_while(|scope| *scope != found.var)
            .any(|scope| {
                self.declared_in.contains(&(scope, name.to_string()))
                    || scoping.get_binding(scope, name).is_some_and(|bound| {
                        !scoping
                            .symbol_flags(bound)
                            .contains(SymbolFlags::FunctionScopedVariable)
                    })
            })
    }

    fn lexical_at_var_scope(scoping: &Scoping, found: BlockFunction) -> bool {
        found.moved(scoping)
            && scoping
                .symbol_redeclarations(found.function)
                .iter()
                .any(|redeclared| redeclared.flags.intersects(LEXICAL))
    }

    fn blocked(&self, scoping: &Scoping, found: BlockFunction) -> bool {
        self.lexical_between(scoping, found) || Self::lexical_at_var_scope(scoping, found)
    }

    fn may_hoist(&self, scoping: &Scoping, found: BlockFunction) -> bool {
        let flags = scoping.symbol_flags(found.function);
        !flags.contains(SymbolFlags::AsyncOrGeneratorFunction)
            && !scoping.scope_flags(found.block).is_strict_mode()
            && !self.blocked(scoping, found)
    }
}

struct Repair {
    changed: FxHashSet<SymbolId>,
    names: FxHashSet<String>,
    split: FxHashMap<(SymbolId, Span), SymbolId>,
}

#[derive(Clone, Copy)]
struct Declared {
    span: Span,
    declaration: NodeId,
    flags: SymbolFlags,
}

struct Split {
    found: BlockFunction,
    own: Vec<Declared>,
    rest: Vec<Declared>,
}

fn partition(scoping: &Scoping, owners: &ScopeOwners<'_>, found: BlockFunction) -> Split {
    let declares_in_block = |declared: &Declared| {
        declared.flags.contains(SymbolFlags::Function)
            && !declared.flags.contains(SymbolFlags::FunctionExpression)
            && owners
                .owner(declared.declaration)
                .and_then(|scope| scoping.scope_parent_id(scope))
                == Some(found.block)
    };
    let (own, rest) = scoping
        .symbol_redeclarations(found.function)
        .iter()
        .map(|declared| Declared {
            span: declared.span,
            declaration: declared.declaration,
            flags: declared.flags,
        })
        .partition(declares_in_block);
    Split { found, own, rest }
}

fn split_redeclarations(scoping: &mut Scoping, split: &Split) -> Option<SymbolId> {
    let found = split.found;
    let first = *split.rest.first()?;
    let name = scoping.symbol_ident(found.function).to_string();
    let flags = split
        .rest
        .iter()
        .fold(SymbolFlags::empty(), |flags, declared| {
            flags | declared.flags
        });
    let held = scoping.create_symbol(
        first.span,
        Ident::from(name.as_str()),
        flags,
        found.var,
        first.declaration,
    );
    for declared in split.rest.iter().skip(1) {
        scoping.add_symbol_redeclaration(held, declared.flags, declared.declaration, declared.span);
    }
    scoping.clear_symbol_redeclarations(found.function);
    if !split.own.is_empty() {
        *scoping.symbol_flags_mut(found.function) = split
            .own
            .iter()
            .fold(SymbolFlags::empty(), |flags, declared| {
                flags | declared.flags
            });
    }
    for declared in split.own.iter().skip(1) {
        scoping.add_symbol_redeclaration(
            found.function,
            declared.flags,
            declared.declaration,
            declared.span,
        );
    }
    Some(held)
}

fn unhoist(scoping: &mut Scoping, split: &Split, repair: &mut Repair) {
    let found = split.found;
    let held = split_redeclarations(scoping, split);
    scoping.move_binding_by_symbol_id(found.var, found.block, found.function);
    let name = scoping.symbol_ident(found.function).to_string();
    if let Some(held) = held {
        scoping.add_binding(found.var, Ident::from(name.as_str()), held);
        for declared in &split.rest {
            repair.split.insert((found.function, declared.span), held);
        }
        repair.changed.insert(held);
    }
    repair.changed.insert(found.function);
    repair.names.insert(name);
}

fn promote(scoping: &mut Scoping, plan: &Plan, repair: &mut Repair) {
    for found in &plan.functions {
        let name = scoping.symbol_ident(found.function).to_string();
        if !repair.names.contains(&name)
            || found.moved(scoping)
            || scoping
                .get_binding(found.var, Ident::from(name.as_str()))
                .is_some()
            || !plan.may_hoist(scoping, *found)
        {
            continue;
        }
        scoping.move_binding_by_symbol_id(found.block, found.var, found.function);
        repair.changed.insert(found.function);
    }
}

pub fn repair_blocked_hoists(program: &Program<'_>, scoping: &mut Scoping) {
    let Some(plan) = Plan::of(scoping) else {
        return;
    };
    let splits: Vec<Split> = {
        let owners = ScopeOwners::new(scoping);
        plan.functions
            .iter()
            .copied()
            .filter(|found| found.moved(scoping) && plan.blocked(scoping, *found))
            .map(|found| partition(scoping, &owners, found))
            .collect()
    };
    if splits.is_empty() {
        return;
    }
    let mut repair = Repair {
        changed: FxHashSet::default(),
        names: FxHashSet::default(),
        split: FxHashMap::default(),
    };
    for split in &splits {
        unhoist(scoping, split, &mut repair);
    }
    promote(scoping, &plan, &mut repair);
    let mut walk = Resolve {
        scoping,
        repair: &repair,
        frames: Vec::new(),
        moves: Vec::new(),
    };
    walk.visit_program(program);
    let moves = std::mem::take(&mut walk.moves);
    for (id, name, symbol) in moves {
        rebind(scoping, id, Ident::from(name.as_str()), symbol);
    }
}

#[derive(Clone, Copy)]
struct ParameterFrame {
    scope: ScopeId,
    parameters: Span,
}

impl ParameterFrame {
    fn holds(self, span: Span) -> bool {
        self.parameters.start <= span.start && span.end <= self.parameters.end
    }
}

struct Resolve<'s> {
    scoping: &'s Scoping,
    repair: &'s Repair,
    frames: Vec<ParameterFrame>,
    moves: Vec<(ReferenceId, String, Option<SymbolId>)>,
}

impl Resolve<'_> {
    fn framed(&mut self, scope: Option<ScopeId>, parameters: Span, walk: impl FnOnce(&mut Self)) {
        let Some(scope) = scope else {
            walk(self);
            return;
        };
        self.frames.push(ParameterFrame { scope, parameters });
        walk(self);
        self.frames.pop();
    }

    fn hidden(&self, scope: ScopeId, symbol: SymbolId, at: Span, arguments: bool) -> bool {
        let flags = self.scoping.symbol_flags(symbol);
        if arguments && flags.contains(SymbolFlags::FunctionExpression) {
            return true;
        }
        let declared = self.scoping.symbol_span(symbol);
        self.frames.iter().any(|frame| {
            frame.scope == scope
                && frame.holds(at)
                && !flags.contains(SymbolFlags::FunctionExpression)
                && !frame.holds(declared)
        })
    }

    fn lookup(&self, start: ScopeId, name: Ident<'_>, at: Span) -> Option<SymbolId> {
        let arguments = name.as_str() == "arguments";
        for scope in self.scoping.scope_ancestors(start) {
            if let Some(symbol) = self.scoping.get_binding(scope, name) {
                if !self.hidden(scope, symbol, at, arguments) {
                    return Some(symbol);
                }
            }
            let flags = self.scoping.scope_flags(scope);
            if arguments && flags.is_function() && !flags.is_arrow() {
                return None;
            }
        }
        None
    }
}

impl<'a> VisitJs<'a> for Resolve<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.framed(it.scope_id.get(), it.params.span, |walk| {
            walk_js::walk_function(walk, it, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.framed(it.scope_id.get(), it.params.span, |walk| {
            walk_js::walk_arrow_function_expression(walk, it);
        });
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        let Some(parameter) = &it.param else {
            walk_js::walk_catch_clause(self, it);
            return;
        };
        self.framed(it.scope_id.get(), parameter.span, |walk| {
            walk_js::walk_catch_clause(walk, it);
        });
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let split = it
            .symbol_id
            .get()
            .and_then(|symbol| self.repair.split.get(&(symbol, it.span)));
        if let Some(held) = split {
            it.symbol_id.set(Some(*held));
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if !self.repair.names.contains(it.name.as_str()) {
            return;
        }
        let Some(id) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(id);
        let old = reference.symbol_id();
        let new = self.lookup(reference.scope_id(), it.name, it.span);
        let touched = |symbol: Option<SymbolId>| {
            symbol.is_some_and(|symbol| self.repair.changed.contains(&symbol))
        };
        if old != new && (touched(old) || touched(new)) {
            self.moves.push((id, it.name.to_string(), new));
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn rebind(scoping: &mut Scoping, id: ReferenceId, name: Ident<'_>, symbol: Option<SymbolId>) {
    let reference = scoping.get_reference(id);
    let (node, scope, flags) = (reference.node_id(), reference.scope_id(), reference.flags());
    match reference.symbol_id() {
        Some(old) => scoping.delete_resolved_reference(old, id),
        None => scoping.delete_root_unresolved_reference(name, id),
    }
    match symbol {
        Some(symbol) => {
            scoping.get_reference_mut(id).set_symbol_id(symbol);
            scoping.add_resolved_reference(symbol, id);
        }
        None => {
            *scoping.get_reference_mut(id) = Reference::new(node, scope, flags);
            scoping.add_root_unresolved_reference(name, id);
        }
    }
}

pub fn hoist_blockers(scoping: &Scoping) -> Vec<SymbolId> {
    let symbols = block_function_symbols(scoping);
    if symbols.is_empty() {
        return Vec::new();
    }
    let owners = ScopeOwners::new(scoping);
    let mut blockers: Vec<SymbolId> = symbols
        .into_iter()
        .filter(|function| {
            !scoping
                .symbol_flags(*function)
                .contains(SymbolFlags::AsyncOrGeneratorFunction)
        })
        .filter_map(|function| Some((function, owners.declaring_block(function)?)))
        .filter(|(_, block)| !scoping.scope_flags(*block).is_strict_mode())
        .flat_map(|(function, block)| blockers_of(scoping, function, block))
        .collect();
    blockers.sort_unstable();
    blockers.dedup();
    blockers
}

fn blockers_of(scoping: &Scoping, function: SymbolId, block: ScopeId) -> Vec<SymbolId> {
    let name = scoping.symbol_ident(function);
    let mut found = Vec::new();
    for scope in scoping.scope_ancestors(block).skip(1) {
        let at_var = scoping.scope_flags(scope).is_var();
        let blocking = scoping.get_binding(scope, name).filter(|bound| {
            let flags = scoping.symbol_flags(*bound);
            *bound != function
                && if at_var {
                    flags.intersects(LEXICAL)
                } else {
                    !flags.contains(SymbolFlags::FunctionScopedVariable)
                }
        });
        found.extend(blocking);
        if at_var {
            break;
        }
    }
    found
}
