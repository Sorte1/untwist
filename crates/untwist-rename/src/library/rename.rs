use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::{BindingInfo, Collected};
use crate::generate::to_identifier;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::simple::is_renameable;
use crate::text_reads::{text_read_bindings, TextReads};
use crate::RenameConfig;

pub const NAME: &str = "library-rename";

const NUMBERED: u32 = 64;

pub const TEXT_READ_UNKNOWN: &str =
    "a function reads its own source through this, callee or caller, so no library name was given";

pub struct LibraryRename {
    config: RenameConfig,
    done: bool,
}

pub fn boxed<'a>(config: RenameConfig) -> Box<dyn Pass<'a> + 'a> {
    Box::new(LibraryRename {
        config,
        done: false,
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
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Suppress,
            OuterRule::UsedEnclosing,
            move |program, collected, scoping| decide(program, collected, scoping, config),
        );
    }
}

fn candidates(base: &str) -> impl Iterator<Item = String> + '_ {
    std::iter::once(base.to_string())
        .chain((2..=NUMBERED).map(move |number| format!("{base}{number}")))
}

pub fn decide(
    program: &Program<'_>,
    collected: &Collected,
    scoping: &Scoping,
    config: RenameConfig,
) -> RenamePlan {
    let found = &collected.library;
    if found.names.is_empty() {
        return RenamePlan::new();
    }
    let text_read = match text_read_bindings(program, scoping) {
        TextReads::Bindings(pinned) => pinned,
        TextReads::Unknown => {
            let mut plan = RenamePlan::new();
            plan.notes.push(TEXT_READ_UNKNOWN.to_string());
            return plan;
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
    for (info, base) in &planned {
        let _ = oracle.take(info.symbol, candidates(base));
    }
    oracle.into_plan()
}
