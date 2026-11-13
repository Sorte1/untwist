use oxc_allocator::Allocator;
use std::collections::HashSet;
use untwist_core::depth::{limits_for, scan};
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::{build_scoping, SharedGlobals};
use untwist_deobfuscate::calls::agnostic::{names_reachable_from, NamesReached};
use untwist_eval::stack;

struct Seen {
    script: bool,
    bound: HashSet<String>,
    free: HashSet<String>,
    readable: bool,
}

impl Seen {
    fn names(&self, name: &str) -> bool {
        self.free.contains(name) || (self.script && self.bound.contains(name))
    }

    fn named(&self) -> impl Iterator<Item = &str> {
        let bound = self.bound.iter().filter(|_| self.script);
        self.free.iter().chain(bound).map(String::as_str)
    }
}

fn fits_this_stack(source: &str) -> bool {
    scan(source, &limits_for(stack::room(), source.len())).is_ok()
}

fn unreadable() -> Seen {
    Seen {
        script: true,
        bound: HashSet::new(),
        free: HashSet::new(),
        readable: false,
    }
}

fn seen(source: &str) -> Seen {
    if !fits_this_stack(source) {
        return unreadable();
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let scoping = build_scoping(&parsed.program).scoping;
    Seen {
        script: !parsed.program.source_type.is_module(),
        bound: scoping
            .get_bindings(scoping.root_scope_id())
            .keys()
            .map(|name| name.as_str().to_string())
            .collect(),
        free: scoping
            .root_unresolved_references()
            .keys()
            .map(|name| name.as_str().to_string())
            .collect(),
        readable: !has_error(&parsed.diagnostics),
    }
}

fn reaches_others(source: &str, others: &[&str]) -> NamesReached {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let scoping = build_scoping(&parsed.program).scoping;
    names_reachable_from(&parsed.program, &scoping, others)
}

fn other_script_names(all: &[Seen], own: usize) -> Vec<&str> {
    let mut names: Vec<&str> = all
        .iter()
        .enumerate()
        .filter(|(index, other)| *index != own && other.script)
        .flat_map(|(_, other)| other.bound.iter().map(String::as_str))
        .filter(|name| !all[own].bound.contains(*name))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

pub(super) fn shared_globals(sources: &[&str]) -> Vec<Option<SharedGlobals>> {
    if sources.len() < 2 {
        return vec![None; sources.len()];
    }
    let all: Vec<Seen> = sources.iter().map(|source| seen(source)).collect();
    let reached: Vec<NamesReached> = sources
        .iter()
        .enumerate()
        .map(|(index, source)| match all[index].readable {
            true => reaches_others(source, &other_script_names(&all, index)),
            false => NamesReached {
                reached: true,
                observed: true,
            },
        })
        .collect();
    all.iter()
        .enumerate()
        .map(|(index, own)| own.script.then(|| shared_for(&all, &reached, index)))
        .collect()
}

fn shared_for(all: &[Seen], reached: &[NamesReached], own: usize) -> SharedGlobals {
    let others = || {
        all.iter()
            .enumerate()
            .filter(move |(index, _)| *index != own)
    };
    let every_name = !all[own].readable || others().any(|(index, _)| reached[index].reached);
    let names = all[own]
        .bound
        .iter()
        .filter(|name| others().any(|(_, other)| other.names(name)))
        .cloned()
        .collect();
    let other_scripts = others().any(|(_, other)| other.script);
    let observed_elsewhere = others().any(|(index, _)| reached[index].observed);
    let named_elsewhere = others()
        .flat_map(|(_, other)| other.named())
        .map(str::to_string)
        .collect();
    SharedGlobals {
        every_name,
        names,
        other_scripts,
        observed_elsewhere,
        named_elsewhere,
    }
}
