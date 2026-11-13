use oxc_ast::ast::{BindingIdentifier, IdentifierReference, Program};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::reference::{Reference, ReferenceId};
use oxc_syntax::scope::ScopeId;
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
    split: FxHashMap<SymbolId, SymbolId>,
}

fn split_redeclarations(scoping: &mut Scoping, found: BlockFunction) -> Option<SymbolId> {
    let declarations = scoping.symbol_redeclarations(found.function).to_vec();
    let own_span = scoping.symbol_span(found.function);
    let (own, rest): (Vec<_>, Vec<_>) = declarations
        .into_iter()
        .partition(|declared| declared.span == own_span);
    let first = rest.first()?.clone();
    let name = scoping.symbol_ident(found.function).to_string();
    let flags = rest.iter().fold(SymbolFlags::empty(), |flags, declared| {
        flags | declared.flags
    });
    let held = scoping.create_symbol(
        first.span,
        Ident::from(name.as_str()),
        flags,
        found.var,
        first.declaration,
    );
    for declared in rest.iter().skip(1) {
        scoping.add_symbol_redeclaration(held, declared.flags, declared.declaration, declared.span);
    }
    scoping.clear_symbol_redeclarations(found.function);
    if let Some(declared) = own.first() {
        *scoping.symbol_flags_mut(found.function) = declared.flags;
    }
    Some(held)
}

fn unhoist(scoping: &mut Scoping, found: BlockFunction, repair: &mut Repair) {
    let split = split_redeclarations(scoping, found);
    scoping.move_binding_by_symbol_id(found.var, found.block, found.function);
    let name = scoping.symbol_ident(found.function).to_string();
    if let Some(held) = split {
        scoping.add_binding(found.var, Ident::from(name.as_str()), held);
        repair.split.insert(found.function, held);
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
    let blocked: Vec<BlockFunction> = plan
        .functions
        .iter()
        .copied()
        .filter(|found| found.moved(scoping) && plan.blocked(scoping, *found))
        .collect();
    if blocked.is_empty() {
        return;
    }
    let mut repair = Repair {
        changed: FxHashSet::default(),
        names: FxHashSet::default(),
        split: FxHashMap::default(),
    };
    for found in blocked {
        unhoist(scoping, found, &mut repair);
    }
    promote(scoping, &plan, &mut repair);
    let mut walk = Resolve {
        scoping,
        repair: &repair,
        moves: Vec::new(),
    };
    walk.visit_program(program);
    let moves = std::mem::take(&mut walk.moves);
    for (id, name, symbol) in moves {
        rebind(scoping, id, Ident::from(name.as_str()), symbol);
    }
}

struct Resolve<'s> {
    scoping: &'s Scoping,
    repair: &'s Repair,
    moves: Vec<(ReferenceId, String, Option<SymbolId>)>,
}

impl<'a> VisitJs<'a> for Resolve<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let split = it.symbol_id.get().and_then(|symbol| {
            self.repair
                .split
                .get(&symbol)
                .filter(|_| self.scoping.symbol_span(symbol) != it.span)
        });
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
        let new = self.scoping.find_binding(reference.scope_id(), it.name);
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
