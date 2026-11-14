use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::{build_scoping, eval_payload_names};

use crate::closed::HOST_NAMES;

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

pub fn reads_free_name(program: &Program<'_>, scoping: &Scoping, name: &str) -> bool {
    scoping.root_unresolved_references().contains_key(name)
        || evaluated_names(program, scoping).contains(name)
}

fn evaluated_names(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<String> {
    if !scoping.root_unresolved_references().contains_key("eval") {
        return FxHashSet::default();
    }
    eval_payload_names(program).into_iter().collect()
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
    (params, evaluated): (&[&str], &FxHashSet<String>),
    used: &mut FxHashSet<String>,
) -> Option<String> {
    let free = scoping.root_unresolved_references().contains_key(to) || evaluated.contains(to);
    if from == to || !free {
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
    let evaluated = evaluated_names(program, &scoping);
    let mut used: FxHashSet<String> = evaluated.clone();
    let mut free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();

    let pairs = original.len().min(renamed.len());
    let mut fresh = Vec::new();
    for index in 0..pairs {
        let from = original[index];
        let uid = uncaptured
            .then(|| {
                let names = (original, &evaluated);
                uncaptured_target(&scoping, (from, renamed[index]), names, &mut used)
            })
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

struct WrapperNames<'n> {
    names: &'n [&'n str],
    found: bool,
}

impl<'a> VisitJs<'a> for WrapperNames<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= self.names.contains(&it.name.as_str());
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.visit_binding_identifier(id);
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(id) = &it.id {
            self.visit_binding_identifier(id);
        }
    }
}

fn fresh_module_name(base: &str, scoping: &Scoping, used: &FxHashSet<String>) -> String {
    let unresolved = scoping.root_unresolved_references();
    (1..=MAX_UID_ATTEMPTS)
        .map(|index| match index {
            1 => format!("_{base}"),
            _ => format!("_{base}{index}"),
        })
        .find(|candidate| {
            !used.contains(candidate)
                && !unresolved.contains_key(candidate.as_str())
                && scoping
                    .get_root_binding(Ident::from(candidate.as_str()))
                    .is_none()
                && !scoping
                    .symbol_ids()
                    .any(|symbol| scoping.symbol_name(symbol) == candidate)
        })
        .unwrap_or_else(|| format!("_{base}_{}", used.len()))
}

pub fn rename_wrapper_bindings<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    params: &[&str],
) -> usize {
    if program.source_type.is_module() {
        return 0;
    }
    let names: Vec<&str> = HOST_NAMES
        .iter()
        .copied()
        .filter(|name| !params.contains(name))
        .collect();
    let mut probe = WrapperNames {
        names: &names,
        found: false,
    };
    probe.visit_program(program);
    if !probe.found {
        return 0;
    }
    let scoping = build_scoping(program).scoping;
    let mut symbols: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    let mut used: FxHashSet<String> = FxHashSet::default();
    for name in names {
        let Some(symbol) = scoping.get_root_binding(Ident::from(name)) else {
            continue;
        };
        let uid = fresh_module_name(name, &scoping, &used);
        symbols.insert(symbol, Ident::from_str_in(&uid, &alloc));
        used.insert(uid);
    }
    if symbols.is_empty() {
        return 0;
    }
    let free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();
    let mut apply = Apply {
        free: &free,
        symbols: &symbols,
        scoping: &scoping,
        changes: 0,
    };
    apply.visit_program(program);
    apply.changes
}
