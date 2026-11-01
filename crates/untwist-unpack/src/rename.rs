use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::build_scoping;

const MAX_UID_ATTEMPTS: u32 = 1000;

pub fn generate_uid(base: &str, scoping: &Scoping, used: &FxHashSet<String>) -> String {
    for index in 1..=MAX_UID_ATTEMPTS {
        let candidate = if index == 1 {
            format!("_{base}")
        } else {
            format!("_{base}{index}")
        };
        let taken = used.contains(&candidate)
            || scoping
                .symbol_ids()
                .any(|symbol| scoping.symbol_name(symbol) == candidate);
        if !taken {
            return candidate;
        }
    }
    format!("_{base}_{}", used.len())
}

struct Apply<'a, 'r> {
    free: &'r FxHashMap<&'r str, Ident<'a>>,
    symbols: &'r FxHashMap<SymbolId, Ident<'a>>,
    scoping: &'r Scoping,
    changes: usize,
}

impl<'a> VisitMut<'a> for Apply<'a, '_> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        match symbol {
            Some(symbol) => {
                if let Some(name) = self.symbols.get(&symbol) {
                    it.name = *name;
                    self.changes += 1;
                }
            }
            None => {
                if let Some(name) = self.free.get(it.name.as_str()) {
                    it.name = *name;
                    self.changes += 1;
                }
            }
        }
        walk_mut::walk_identifier_reference(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(name) = it.symbol_id.get().and_then(|id| self.symbols.get(&id)) {
            it.name = *name;
            self.changes += 1;
        }
        walk_mut::walk_binding_identifier(self, it);
    }
}

fn collect_collisions<'a>(
    alloc: &'a Allocator,
    scoping: &Scoping,
    original: &str,
    renamed: &str,
    symbols: &mut FxHashMap<SymbolId, Ident<'a>>,
    used: &mut FxHashSet<String>,
) {
    if original == renamed {
        return;
    }
    let Some(reference_ids) = scoping.root_unresolved_references().get(original) else {
        return;
    };
    let mut ordered: Vec<_> = reference_ids.iter().copied().collect();
    ordered.sort_unstable();
    for reference_id in ordered {
        let scope_id = scoping.get_reference(reference_id).scope_id();
        let Some(symbol) = scoping.find_binding(scope_id, Ident::from(renamed)) else {
            continue;
        };
        if symbols.contains_key(&symbol) {
            continue;
        }
        let uid = generate_uid(renamed, scoping, used);
        used.insert(uid.clone());
        symbols.insert(symbol, Ident::from_str_in(&uid, &alloc));
    }
}

fn uncaptured_target(
    scoping: &Scoping,
    (from, to): (&str, &str),
    params: &[&str],
    used: &mut FxHashSet<String>,
) -> Option<String> {
    if from == to || !scoping.root_unresolved_references().contains_key(to) {
        return None;
    }
    if params.contains(&to) {
        return None;
    }
    let uid = generate_uid(to, scoping, used);
    used.insert(uid.clone());
    Some(uid)
}

pub fn rename_parameters<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    original: &[&str],
    renamed: &[&str],
) -> usize {
    rename_parameters_as(alloc, program, (original, renamed), false).0
}

pub fn rename_factory_parameters<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (original, renamed): (&[&str], &[&str]),
    conservative: bool,
) -> Vec<(usize, String)> {
    rename_parameters_as(alloc, program, (original, renamed), !conservative).1
}

pub fn rename_parameters_uncaptured<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    original: &[&str],
    renamed: &[&str],
) -> usize {
    rename_parameters_as(alloc, program, (original, renamed), true).0
}

fn rename_parameters_as<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (original, renamed): (&[&str], &[&str]),
    uncaptured: bool,
) -> (usize, Vec<(usize, String)>) {
    let scoping = build_scoping(program).scoping;
    let mut symbols: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    let mut used: FxHashSet<String> = FxHashSet::default();
    let mut free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();

    let pairs = original.len().min(renamed.len());
    let mut fresh = Vec::new();
    for index in 0..pairs {
        let from = original[index];
        let uid = uncaptured
            .then(|| uncaptured_target(&scoping, (from, renamed[index]), original, &mut used))
            .flatten();
        let to = uid.as_deref().unwrap_or(renamed[index]);
        collect_collisions(alloc, &scoping, from, to, &mut symbols, &mut used);
        if from != to {
            free.insert(from, Ident::from_str_in(to, &alloc));
        }
        if let Some(uid) = uid {
            fresh.push((index, uid));
        }
    }
    if free.is_empty() && symbols.is_empty() {
        return (0, fresh);
    }

    let mut apply = Apply {
        free: &free,
        symbols: &symbols,
        scoping: &scoping,
        changes: 0,
    };
    apply.visit_program(program);
    (apply.changes, fresh)
}

pub fn rename_binding<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    from: &str,
    to: &str,
) -> bool {
    if from == to {
        return false;
    }
    let scoping = build_scoping(program).scoping;
    let Some(target) = scoping.get_root_binding(Ident::from(from)) else {
        return false;
    };
    let mut symbols: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    let mut used: FxHashSet<String> = FxHashSet::default();
    if let Some(conflict) = scoping.get_root_binding(Ident::from(to)) {
        if conflict != target {
            let uid = generate_uid(to, &scoping, &used);
            used.insert(uid.clone());
            symbols.insert(conflict, Ident::from_str_in(&uid, &alloc));
        }
    }
    symbols.insert(target, Ident::from_str_in(to, &alloc));
    let free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();
    let mut apply = Apply {
        free: &free,
        symbols: &symbols,
        scoping: &scoping,
        changes: 0,
    };
    apply.visit_program(program);
    apply.changes > 0
}
