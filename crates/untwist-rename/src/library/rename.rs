use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::{BindingInfo, Collected};
use crate::generate::to_identifier;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::simple::is_renameable;
use crate::smart::lane_self_reads;
use crate::text_reads::{text_read_bindings_with, SelfReads, TextReads};
use crate::unique::{await_turn, is_taken, next_number, take_names, SharedUniqueState};
use crate::RenameConfig;
use std::cell::RefCell;

pub const NAME: &str = "library-rename";

const NUMBERED: u32 = 64;

pub const TEXT_READ_UNKNOWN: &str =
    "a function reads its own source through this, callee or caller, so no library name was given";

pub struct LibraryRename {
    config: RenameConfig,
    done: bool,
    unique: Option<(SharedUniqueState, usize)>,
}

pub fn boxed<'a>(
    config: RenameConfig,
    unique: Option<(SharedUniqueState, usize)>,
) -> Box<dyn Pass<'a> + 'a> {
    Box::new(LibraryRename {
        config,
        done: false,
        unique,
    })
}

impl<'a> Pass<'a> for LibraryRename {
    fn name(&self) -> &'static str {
        NAME
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let config = RenameConfig {
            rename_all: true,
            ..self.config
        };
        if self.done {
            return;
        }
        let shared = self.unique.as_ref().map(|(state, index)| {
            await_turn(state, *index);
            state
        });
        let chosen: RefCell<Vec<(String, String, u32)>> = RefCell::new(Vec::new());
        let self_reads = lane_self_reads(cx.opts.unsafe_transforms);
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Suppress,
            OuterRule::UsedEnclosing,
            |program, collected, scoping| {
                let (plan, names) =
                    decide_names(program, collected, scoping, (config, self_reads), shared);
                *chosen.borrow_mut() = names;
                plan
            },
        );
        if let Some(state) = shared {
            take_names(state, &chosen.into_inner());
        }
    }
}

fn numbered(base: &str, number: u32) -> String {
    match number {
        1 => base.to_string(),
        _ => format!("{base}{number}"),
    }
}

fn candidates<'s>(
    base: &'s str,
    shared: Option<&'s SharedUniqueState>,
) -> impl Iterator<Item = (u32, String)> + 's {
    let first = shared
        .and_then(|state| next_number(state, base))
        .unwrap_or(1)
        .max(1);
    let numbers = std::iter::once(first)
        .filter(|&number| number == 1)
        .chain((first.max(2)..).take(NUMBERED as usize));
    numbers
        .map(move |number| (number, numbered(base, number)))
        .filter(move |(_, name)| !shared.is_some_and(|state| is_taken(state, name)))
}

pub fn decide(
    program: &Program<'_>,
    collected: &Collected,
    scoping: &Scoping,
    config: RenameConfig,
) -> RenamePlan {
    decide_names(
        program,
        collected,
        scoping,
        (config, SelfReads::Unknown),
        None,
    )
    .0
}

fn decide_names(
    program: &Program<'_>,
    collected: &Collected,
    scoping: &Scoping,
    (config, self_reads): (RenameConfig, SelfReads),
    shared: Option<&SharedUniqueState>,
) -> (RenamePlan, Vec<(String, String, u32)>) {
    let found = &collected.library;
    if found.names.is_empty() {
        return (RenamePlan::new(), Vec::new());
    }
    let text_read = match text_read_bindings_with(program, scoping, self_reads) {
        TextReads::Bindings(pinned) => pinned,
        TextReads::Unknown => {
            let mut plan = RenamePlan::new();
            plan.notes.push(TEXT_READ_UNKNOWN.to_string());
            return (plan, Vec::new());
        }
    };
    let planned: Vec<(&BindingInfo, String)> = collected
        .bindings
        .iter()
        .filter(|info| !text_read.contains(&info.symbol))
        .filter(|info| is_renameable(info, collected, config))
        .filter_map(|info| {
            let name = to_identifier(found.name(info.symbol)?);
            (!name.is_empty() && name != "_" && name != info.name).then_some((info, name))
        })
        .collect();
    let mut oracle = NameOracle::new(scoping, collected);
    let mut chosen = Vec::new();
    for (info, base) in &planned {
        let numbers: Vec<(u32, String)> = candidates(base, shared).collect();
        let taken = oracle.take(info.symbol, numbers.iter().map(|(_, name)| name.clone()));
        let Some(name) = taken else {
            continue;
        };
        let number = numbers
            .iter()
            .find(|(_, known)| *known == name)
            .map_or(1, |(number, _)| *number);
        chosen.push((base.clone(), name, number));
    }
    (oracle.into_plan(), chosen)
}
