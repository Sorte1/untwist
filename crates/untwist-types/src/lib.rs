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
use untwist_core::parse::{has_error, parse_program, SourceType};
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
    notes: view::Annotations,
    stats: TypeStats,
    edits: Vec<emit::Edit>,
    instrumented: Option<String>,
}

fn analyse(code: &str, mode: Mode, instrument: bool) -> Result<(Analysis, SourceType, bool), (Refusal, bool)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, code);
    if has_error(&parsed.diagnostics) {
        return Err((Refusal::Parse, false));
    }
    let program = alloc.alloc(parsed.program);
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
    let module = untwist_core::semantic::has_module_syntax(program)
        || parsed.source_type.is_module();
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
    let edits = emit::edits(&env, &decided.notes, mode, code);
    let instrumented = instrument.then(|| instrument::instrument(&env, &decided.notes, code));
    let mut stats = decided.stats;
    stats.files = 1;
    Ok((
        Analysis {
            notes: decided.notes,
            stats,
            edits,
            instrumented,
        },
        parsed.source_type,
        jsx,
    ))
}

pub fn annotate(code: &str, mode: Mode) -> Annotated {
    if code.trim().is_empty() {
        return Annotated::unchanged(code, None, false);
    }
    let (analysis, source_type, jsx) = match analyse(code, mode, false) {
        Ok(done) => done,
        Err((refusal, jsx)) => return Annotated::unchanged(code, Some(refusal), jsx),
    };
    if analysis.notes.symbols.is_empty() && analysis.notes.returns.is_empty() {
        return Annotated {
            code: code.to_string(),
            stats: analysis.stats,
            jsx,
            refused: None,
        };
    }
    let typed = emit::apply(code, analysis.edits);
    if erase::erases_as(code, &typed, source_type, mode, jsx).is_err() {
        let mut stats = analysis.stats;
        stats.files_refused = 1;
        return Annotated {
            code: code.to_string(),
            stats,
            jsx,
            refused: Some(Refusal::Erasure),
        };
    }
    Annotated {
        code: typed,
        stats: analysis.stats,
        jsx,
        refused: None,
    }
}

#[doc(hidden)]
pub fn instrumented(code: &str) -> Option<String> {
    analyse(code, Mode::Ts, true)
        .ok()
        .and_then(|(analysis, _, _)| analysis.instrumented)
}
