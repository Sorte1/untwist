use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default)]
pub struct Renames<'a> {
    map: FxHashMap<SymbolId, Ident<'a>>,
}

struct Applier<'a, 'r> {
    map: &'r FxHashMap<SymbolId, Ident<'a>>,
    scoping: &'r Scoping,
    count: usize,
}

struct BindingApplier<'a, 'r> {
    map: &'r FxHashMap<SymbolId, Ident<'a>>,
    count: usize,
}

impl<'a> Renames<'a> {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn insert(&mut self, symbol: SymbolId, name: Ident<'a>) {
        self.map.insert(symbol, name);
    }

    pub fn apply(&self, program: &mut Program<'a>, scoping: &Scoping) -> usize {
        let count = self.run(scoping, |applier| applier.visit_program(program));
        if count > 0 {
            let renames = self
                .map
                .iter()
                .map(|(symbol, name)| (*symbol, name.as_str()));
            crate::semantic::note_renames(program, scoping, renames);
        }
        count
    }

    pub fn apply_bindings(&self, program: &mut Program<'a>) -> usize {
        if self.map.is_empty() {
            return 0;
        }
        let mut applier = BindingApplier {
            map: &self.map,
            count: 0,
        };
        applier.visit_program(program);
        applier.count
    }

    fn run(&self, scoping: &Scoping, visit: impl FnOnce(&mut Applier<'a, '_>)) -> usize {
        if self.map.is_empty() {
            return 0;
        }
        let mut applier = Applier {
            map: &self.map,
            scoping,
            count: 0,
        };
        visit(&mut applier);
        applier.count
    }
}

impl<'a> VisitMut<'a> for Applier<'a, '_> {
    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            if let Some(name) = self.map.get(&symbol) {
                it.name = *name;
                self.count += 1;
            }
        }
        walk_mut::walk_binding_identifier(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if let Some(reference_id) = it.reference_id.get() {
            if let Some(symbol) = self.scoping.get_reference(reference_id).symbol_id() {
                if let Some(name) = self.map.get(&symbol) {
                    it.name = *name;
                    self.count += 1;
                }
            }
        }
        walk_mut::walk_identifier_reference(self, it);
    }
}

impl<'a> VisitMut<'a> for BindingApplier<'a, '_> {
    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            if let Some(name) = self.map.get(&symbol) {
                it.name = *name;
                self.count += 1;
            }
        }
        walk_mut::walk_binding_identifier(self, it);
    }
}

pub fn is_name_taken(scoping: &Scoping, name: &str) -> bool {
    if scoping.root_unresolved_references().contains_key(name) {
        return true;
    }
    scoping
        .symbol_ids()
        .any(|symbol| scoping.symbol_name(symbol) == name)
}

pub fn taken_names(scoping: &Scoping) -> FxHashSet<&str> {
    scoping
        .symbol_ids()
        .map(|symbol| scoping.symbol_name(symbol))
        .chain(
            scoping
                .root_unresolved_references()
                .keys()
                .map(|name| name.as_str()),
        )
        .collect()
}

pub const UNIQUE_NAME_SUFFIX_CAP: u32 = 1000;

pub fn unique_name<'a>(
    base: &str,
    scoping: &Scoping,
    alloc: &impl GetAllocator<'a>,
) -> Option<Ident<'a>> {
    unique_name_avoiding(base, scoping, &FxHashSet::default(), alloc)
}

pub fn unique_name_avoiding<'a>(
    base: &str,
    scoping: &Scoping,
    reserved: &FxHashSet<String>,
    alloc: &impl GetAllocator<'a>,
) -> Option<Ident<'a>> {
    if !is_taken(base, scoping, reserved) {
        return Some(Ident::from_str_in(base, alloc));
    }
    let stem = base.strip_suffix("__").unwrap_or(base);
    let taken = taken_names(scoping);
    for suffix in 2..=UNIQUE_NAME_SUFFIX_CAP {
        let candidate = format!("{stem}_{suffix}__");
        if !taken.contains(candidate.as_str()) && !reserved.contains(&candidate) {
            return Some(Ident::from_str_in(&candidate, alloc));
        }
    }
    None
}

fn is_taken(name: &str, scoping: &Scoping, reserved: &FxHashSet<String>) -> bool {
    is_name_taken(scoping, name) || reserved.contains(name)
}
