use oxc_allocator::Allocator;
use std::collections::HashSet;
use untwist_core::ast_util::key_names::{key_names, text_names};
use untwist_core::ast_util::naming_slot::reads_names;
use untwist_core::depth::{limits_for, scan};
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::{assume_shared_globals, build_scoping, SharedGlobals};
use untwist_deobfuscate::calls::agnostic::{names_reached_from, NamesReached};
use untwist_eval::stack;

use super::global_writes::{global_writes, GlobalWrites, EVERY_ROOT};
use untwist_core::semantic::builtins::{self, Builtins};

struct Seen {
    script: bool,
    bound: HashSet<String>,
    free: HashSet<String>,
    written: HashSet<String>,
    global_written: GlobalWrites,
    keys: HashSet<String>,
    readable: bool,
    names_read: bool,
    builtins: Builtins,
}

impl Seen {
    fn names(&self, name: &str) -> bool {
        self.free.contains(name) || (self.script && self.bound.contains(name))
    }

    fn named(&self) -> impl Iterator<Item = &str> {
        let bound = self.bound.iter().filter(|_| self.script);
        self.free
            .iter()
            .chain(bound)
            .chain(self.global_written.names.iter())
            .chain(self.keys.iter())
            .map(String::as_str)
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
        written: HashSet::new(),
        global_written: GlobalWrites::default(),
        keys: HashSet::new(),
        readable: false,
        names_read: true,
        builtins: Builtins::Unknown,
    }
}

fn seen(source: &str, unsafe_transforms: bool) -> Seen {
    if !fits_this_stack(source) {
        return unreadable();
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let scoping = build_scoping(&parsed.program).scoping;
    let module = parsed.program.source_type.is_module();
    let builtins =
        builtins::scan_with_unknown_keys_plain(&parsed.program, &scoping, module, &EVERY_ROOT);
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
        written: scoping
            .root_unresolved_references()
            .iter()
            .filter(|(_, ids)| ids.iter().any(|id| scoping.get_reference(*id).is_write()))
            .map(|(name, _)| name.as_str().to_string())
            .collect(),
        global_written: {
            let mut written = global_writes(&parsed.program);
            written.every |= !builtins.allows_safe(unsafe_transforms);
            written
        },
        keys: key_names(&parsed.program, &scoping)
            .into_iter()
            .chain(text_names(&parsed.program, &scoping))
            .collect(),
        readable: !has_error(&parsed.diagnostics),
        names_read: reads_names(&parsed.program, &scoping),
        builtins,
    }
}

fn reaches_others(source: &str, others: &[&str], foreign: SharedGlobals) -> NamesReached {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let scoping = build_scoping(&parsed.program).scoping;
    let _foreign = assume_shared_globals(Some(foreign));
    names_reached_from(&parsed.program, &scoping, others)
}

fn other_script_names(all: &[Seen], own: usize) -> Vec<&str> {
    let mut names: Vec<&str> = all
        .iter()
        .enumerate()
        .filter(|(index, other)| *index != own && other.script)
        .flat_map(|(_, other)| other.bound.iter().chain(other.global_written.names.iter()))
        .map(String::as_str)
        .filter(|name| !all[own].script || !all[own].bound.contains(*name))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

#[derive(Default)]
pub(super) struct Shared {
    pub globals: Vec<Option<SharedGlobals>>,
    pub causes: Vec<Option<String>>,
}

pub(super) fn shared_globals(sources: &[&str], unsafe_transforms: bool) -> Shared {
    if sources.len() < 2 {
        return Shared {
            globals: vec![None; sources.len()],
            causes: vec![None; sources.len()],
        };
    }
    let all: Vec<Seen> = sources
        .iter()
        .map(|source| seen(source, unsafe_transforms))
        .collect();
    let reached: Vec<NamesReached> = sources
        .iter()
        .enumerate()
        .map(
            |(index, source)| match all[index].readable && !others_unknown(&all, index) {
                true => {
                    let foreign = SharedGlobals {
                        names: rebound_elsewhere(&all, index, unsafe_transforms)
                            .into_iter()
                            .collect(),
                        ..foreign_of(&all, index)
                    };
                    reaches_others(source, &other_script_names(&all, index), foreign)
                }
                false => NamesReached {
                    reached: true,
                    observed: true,
                    at: None,
                },
            },
        )
        .collect();
    let globals: Vec<Option<SharedGlobals>> = (0..all.len())
        .map(|index| Some(shared_for(&all, &reached, index, unsafe_transforms)))
        .collect();
    let causes = globals
        .iter()
        .enumerate()
        .map(|(index, shared)| {
            let every_name = shared.as_ref().is_some_and(|shared| shared.every_name);
            every_name
                .then(|| cause_of(sources, (&all, &reached), index, unsafe_transforms))
                .flatten()
        })
        .collect();
    Shared { globals, causes }
}

fn line_of(source: &str, at: u32) -> usize {
    let end = source.len().min(at as usize);
    source.as_bytes()[..end]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn cause_of(
    sources: &[&str],
    (all, reached): (&[Seen], &[NamesReached]),
    own: usize,
    unsafe_transforms: bool,
) -> Option<String> {
    if !all[own].readable {
        return None;
    }
    if builtins_unclean(all, unsafe_transforms) {
        let index = all
            .iter()
            .position(|seen| seen.builtins != Builtins::Intact)?;
        return Some(format!(
            "input {}, which may write or hand on a built-in or one of its prototypes",
            index + 1
        ));
    }
    let (index, found) = reached
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != own)
        .find(|(_, found)| found.reached || (!unsafe_transforms && found.observed))?;
    Some(match found.at {
        Some(span) => format!(
            "input {}, line {}",
            index + 1,
            line_of(sources[index], span.start)
        ),
        None => format!("input {}, which could not be read in full", index + 1),
    })
}

fn shared_for(
    all: &[Seen],
    reached: &[NamesReached],
    own: usize,
    unsafe_transforms: bool,
) -> SharedGlobals {
    let others = || {
        all.iter()
            .enumerate()
            .filter(move |(index, _)| *index != own)
    };
    let observed_elsewhere = others().any(|(index, _)| reached[index].observed);
    let every_name = !all[own].readable
        || builtins_unclean(all, unsafe_transforms)
        || others().any(|(index, _)| reached[index].reached)
        || (!unsafe_transforms && observed_elsewhere);
    let names = all[own]
        .bound
        .iter()
        .chain(all[own].global_written.names.iter())
        .filter(|name| others().any(|(_, other)| other.names(name)))
        .cloned()
        .collect();
    let SharedGlobals {
        every_foreign,
        foreign,
        ..
    } = foreign_of(all, own);
    let other_scripts = others().any(|(_, other)| other.script);
    let named_elsewhere = others()
        .flat_map(|(_, other)| other.named())
        .map(str::to_string)
        .collect();
    let names_read_elsewhere = others().any(|(_, other)| other.names_read);
    SharedGlobals {
        names_read_elsewhere,
        every_name,
        names,
        every_foreign,
        foreign,
        other_scripts,
        observed_elsewhere,
        named_elsewhere,
    }
}

fn rebound_elsewhere(all: &[Seen], own: usize, unsafe_transforms: bool) -> Vec<String> {
    if unsafe_transforms || !all[own].script {
        return Vec::new();
    }
    let named: HashSet<&str> = all
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != own)
        .flat_map(|(_, other)| other.named())
        .collect();
    all[own]
        .bound
        .iter()
        .filter(|name| named.contains(name.as_str()))
        .cloned()
        .collect()
}

fn builtins_unclean(all: &[Seen], unsafe_transforms: bool) -> bool {
    !unsafe_transforms && all.iter().any(|seen| seen.builtins != Builtins::Intact)
}

fn others_unknown(all: &[Seen], own: usize) -> bool {
    all.iter()
        .enumerate()
        .any(|(index, other)| index != own && !other.readable)
}

fn foreign_of(all: &[Seen], own: usize) -> SharedGlobals {
    let others = all
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != own)
        .map(|(_, other)| other);
    let mut shared = SharedGlobals::default();
    for other in others {
        shared.every_foreign |= !other.readable || other.global_written.every;
        let bound = other.bound.iter().filter(|_| other.script);
        shared.foreign.extend(
            bound
                .chain(other.written.iter())
                .chain(other.global_written.names.iter())
                .cloned(),
        );
    }
    shared
}

pub(super) fn names_read_in_any(programs: &[oxc_ast::ast::Program<'_>]) -> Option<SharedGlobals> {
    if programs.len() < 2 {
        return None;
    }
    let read = programs
        .iter()
        .any(|program| reads_names(program, &build_scoping(program).scoping));
    read.then(|| SharedGlobals {
        names_read_elsewhere: true,
        ..SharedGlobals::default()
    })
}
