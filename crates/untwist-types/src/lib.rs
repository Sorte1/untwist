mod builtins;
mod decide;
mod emit;
mod erase;
mod facts;
mod infer;
mod instrument;
mod stats;
mod ty;
mod view;

use oxc_allocator::Allocator;
use oxc_semantic::SemanticBuilder;
use untwist_core::semantic::builtins::{scan, Builtins};

pub use builtins::RELIANCE;
pub use erase::erases;
pub use stats::{Counts, TypeStats};

pub const EMITTED_TYPE_NAMES: &[&str] = &["Promise"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Ts,
    Jsdoc,
}

impl Mode {
    pub fn parse(text: &str) -> Option<Mode> {
        match text {
            "ts" => Some(Mode::Ts),
            "jsdoc" => Some(Mode::Jsdoc),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Ts => "ts",
            Mode::Jsdoc => "jsdoc",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Parse,
    TypeSyntax,
    Hazard(&'static str),
    Erasure,
}

impl Refusal {
    pub fn message(self) -> &'static str {
        match self {
            Refusal::Parse => "types: the output does not parse again, so nothing was annotated",
            Refusal::TypeSyntax => "types: the output already holds TypeScript syntax, so nothing was annotated",
            Refusal::Hazard("direct eval") => "types: a direct eval can reach every binding, so nothing was annotated",
            Refusal::Hazard(_) => "types: a with statement can reach every binding, so nothing was annotated",
            Refusal::Erasure => "types: the annotations did not strip back to the untyped output, so nothing was annotated",
        }
    }
}

pub struct Annotated {
    pub code: String,
    pub stats: TypeStats,
    pub jsx: bool,
    pub refused: Option<Refusal>,
}

impl Annotated {
    fn unchanged(code: &str, refused: Option<Refusal>, jsx: bool) -> Annotated {
        Annotated {
            code: code.to_string(),
            stats: TypeStats {
                files: 1,
                files_refused: usize::from(refused.is_some()),
                ..TypeStats::default()
            },
            jsx,
            refused,
        }
    }
}

struct Analysis {
    typed: Option<String>,
    stats: TypeStats,
    jsx: bool,
}

enum Purpose {
    Annotate,
    Instrument,
}

fn analyse(code: &str, mode: Mode, purpose: Purpose) -> Result<Analysis, (Refusal, bool)> {
    let alloc = Allocator::default();
    let Some((program, source_type)) = erase::parse_js(&alloc, code) else {
        return Err((Refusal::Parse, false));
    };
    let program = alloc.alloc(program);
    let jsx = emit::has_jsx(program);
    if untwist_core::type_space::holds_type_space(program) {
        return Err((Refusal::TypeSyntax, jsx));
    }
    let mut semantic = SemanticBuilder::new().with_build_nodes(true).build(program).semantic;
    untwist_core::semantic::repair_blocked_hoists(program, semantic.scoping_mut());
    let (scoping, nodes) = semantic.into_scoping_and_nodes();
    if let Some(hazard) = untwist_core::semantic::scope_hazard(&scoping) {
        return Err((Refusal::Hazard(hazard), jsx));
    }
    let module = untwist_core::semantic::has_module_syntax(program) || source_type.is_module();
    let scanned = scan(program, &scoping, module, false, &RELIANCE);
    let shadowed = scoping.symbol_names().any(|name| EMITTED_TYPE_NAMES.contains(&name));
    let cx = facts::Ctx {
        program,
        scoping: &scoping,
        nodes: &nodes,
        module,
    };
    let collected = facts::collect(&cx);
    let mut env = infer::Env::new(
        &cx,
        &collected,
        scanned.verdict == Builtins::Intact,
        !scanned.promise_written && !shadowed,
    );
    env.solve();
    let decided = decide::decide(&env, mode);
    let mut stats = decided.stats;
    stats.files = 1;
    let nothing = decided.notes.symbols.is_empty() && decided.notes.returns.is_empty();
    let typed = match (purpose, nothing) {
        (Purpose::Instrument, _) => Some(instrument::instrument(&env, &decided.notes, code)),
        (Purpose::Annotate, true) => None,
        (Purpose::Annotate, false) => {
            let typed = emit::apply(code, emit::edits(&env, &decided.notes, mode, code));
            if !erase::same_after_strip(program, &typed, source_type, mode, jsx) {
                return Err((Refusal::Erasure, jsx));
            }
            Some(typed)
        }
    };
    Ok(Analysis { typed, stats, jsx })
}

pub fn annotate(code: &str, mode: Mode) -> Annotated {
    if code.trim().is_empty() {
        return Annotated::unchanged(code, None, false);
    }
    match analyse(code, mode, Purpose::Annotate) {
        Ok(analysis) => Annotated {
            code: analysis.typed.unwrap_or_else(|| code.to_string()),
            stats: analysis.stats,
            jsx: analysis.jsx,
            refused: None,
        },
        Err((refusal, jsx)) => Annotated::unchanged(code, Some(refusal), jsx),
    }
}

#[doc(hidden)]
pub fn instrumented(code: &str) -> Option<String> {
    analyse(code, Mode::Ts, Purpose::Instrument)
        .ok()
        .and_then(|analysis| analysis.typed)
}
