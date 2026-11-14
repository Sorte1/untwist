mod builtins;
mod decide;
mod dts;
mod emit;
mod erase;
mod facts;
mod infer;
mod instrument;
mod realm;
mod specifiers;
mod stats;
mod ty;
mod view;

use oxc_allocator::Allocator;
use oxc_semantic::SemanticBuilder;

pub use builtins::RELIANCE;
pub use erase::erases;
pub use realm::{realm, Realm};
pub use specifiers::{specifiers, Specifier};
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
            Refusal::Parse => "a file did not parse again, so it was left untyped",
            Refusal::TypeSyntax => "a file already holds TypeScript syntax, so it was left untyped",
            Refusal::Hazard(_) => "a file has a with statement, which can reach every binding, so it was left untyped",
            Refusal::Erasure => "a file reads differently as TypeScript, so it was left untyped and keeps its .js name",
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

struct Prepared<'a> {
    program: &'a oxc_ast::ast::Program<'a>,
    source_type: oxc_span::SourceType,
    scoping: oxc_semantic::Scoping,
    nodes: oxc_semantic::AstNodes<'a>,
    jsx: bool,
    module: bool,
}

fn prepare<'a>(alloc: &'a Allocator, code: &'a str) -> Result<Prepared<'a>, (Refusal, bool)> {
    let Some((program, source_type)) = erase::parse_js(alloc, code) else {
        return Err((Refusal::Parse, false));
    };
    let program = alloc.alloc(program);
    let jsx = emit::has_jsx(program);
    if untwist_core::type_space::holds_type_space(program) {
        return Err((Refusal::TypeSyntax, jsx));
    }
    let mut semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    untwist_core::semantic::repair_blocked_hoists(program, semantic.scoping_mut());
    let (scoping, nodes) = semantic.into_scoping_and_nodes();
    let module = untwist_core::semantic::has_module_syntax(program) || source_type.is_module();
    Ok(Prepared {
        program,
        source_type,
        scoping,
        nodes,
        jsx,
        module,
    })
}

fn has_with(scoping: &oxc_semantic::Scoping) -> bool {
    scoping.scope_descendants_from_root().any(|scope| {
        scoping
            .scope_flags(scope)
            .contains(oxc_syntax::scope::ScopeFlags::With)
    })
}

fn solved<R>(
    prepared: &Prepared<'_>,
    given: Option<Realm>,
    then: impl for<'f, 'b> FnOnce(&infer::Env<'f, 'b>) -> R,
) -> R {
    let program = prepared.program;
    let scoping = &prepared.scoping;
    let local = realm::of_program(program, scoping, prepared.module);
    let realm = local.meet(given.unwrap_or(local));
    let shadowed = scoping
        .symbol_names()
        .any(|name| EMITTED_TYPE_NAMES.contains(&name));
    let cx = facts::Ctx {
        exports: facts::export_spans(program),
        frames_reachable: facts::frames_reachable(program, scoping),
        annexed: facts::annexed_symbols(scoping, &prepared.nodes),
        tainted: facts::eval_tainted(scoping, &prepared.nodes),
        program,
        scoping,
        nodes: &prepared.nodes,
        module: prepared.module,
    };
    let collected = facts::collect(&cx);
    let mut env = infer::Env::new(&cx, &collected, realm.builtins, realm.promises && !shadowed);
    env.solve();
    then(&env)
}

const ERASURE_CHECKS: usize = 32;

fn kept_edits(
    code: &str,
    mut edits: Vec<emit::Edit>,
    check: &dyn Fn(&str) -> bool,
    stats: &mut TypeStats,
) -> Option<Option<String>> {
    edits.sort_by_key(|edit| (edit.at, edit.rank, edit.tie));
    let mut budget = ERASURE_CHECKS;
    loop {
        let typed = emit::apply(code, edits.clone());
        if check(&typed) {
            return Some(Some(typed));
        }
        let (mut from, mut to) = (0, edits.len());
        while to - from > 1 {
            if budget == 0 {
                return None;
            }
            budget -= 1;
            let middle = from + (to - from) / 2;
            let without: Vec<emit::Edit> = edits[..from]
                .iter()
                .chain(&edits[middle..])
                .cloned()
                .collect();
            match check(&emit::apply(code, without)) {
                true => to = middle,
                false => from = middle,
            }
        }
        if edits.is_empty() || budget == 0 {
            return None;
        }
        edits.remove(from);
        stats.erasure_dropped += 1;
        budget -= 1;
    }
}

fn analyse(
    code: &str,
    mode: Mode,
    purpose: Purpose,
    given: Option<Realm>,
) -> Result<Analysis, (Refusal, bool)> {
    let alloc = Allocator::default();
    let prepared = prepare(&alloc, code)?;
    let jsx = prepared.jsx;
    let parses_alike = || {
        mode == Mode::Jsdoc
            || erase::same_after_strip(prepared.program, code, prepared.source_type, mode, jsx)
    };
    if has_with(&prepared.scoping) {
        return Err((Refusal::Hazard("with statement"), jsx));
    }
    let (typed, stats) = solved(&prepared, given, |env| {
        let decided = decide::decide(env, mode);
        let mut stats = decided.stats;
        stats.files = 1;
        stats.near_eval = env
            .facts
            .slots
            .keys()
            .filter(|symbol| env.cx.tainted.contains(symbol))
            .count();
        let nothing = decided.notes.symbols.is_empty() && decided.notes.returns.is_empty();
        let typed = match (purpose, nothing) {
            (Purpose::Instrument, _) => {
                Some(Some(instrument::instrument(env, &decided.notes, code)))
            }
            (Purpose::Annotate, true) => Some(None),
            (Purpose::Annotate, false) => {
                let edits = emit::edits(env, &decided.notes, mode, code);
                let check = |typed: &str| {
                    erase::same_after_strip(
                        prepared.program,
                        typed,
                        prepared.source_type,
                        mode,
                        jsx,
                    )
                };
                match parses_alike() {
                    true => kept_edits(code, edits, &check, &mut stats),
                    false => None,
                }
            }
        };
        (typed, stats)
    });
    match typed {
        Some(None) if !parses_alike() => Err((Refusal::Erasure, jsx)),
        Some(typed) => Ok(Analysis { typed, stats, jsx }),
        None => Err((Refusal::Erasure, jsx)),
    }
}

pub fn annotate(code: &str, mode: Mode) -> Annotated {
    annotate_in(code, mode, None)
}

pub fn annotate_in(code: &str, mode: Mode, realm: Option<Realm>) -> Annotated {
    if code.trim().is_empty() {
        return Annotated::unchanged(code, None, false);
    }
    match analyse(code, mode, Purpose::Annotate, realm) {
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
    analyse(code, Mode::Ts, Purpose::Instrument, None)
        .ok()
        .and_then(|analysis| analysis.typed)
}

pub fn declarations(code: &str) -> Option<String> {
    declarations_in(code, None)
}

pub fn declarations_in(code: &str, realm: Option<Realm>) -> Option<String> {
    let alloc = Allocator::default();
    let prepared = prepare(&alloc, code).ok()?;
    if has_with(&prepared.scoping) {
        return dts::render(prepared.program, &prepared.scoping, code, &|_| None);
    }
    let types = solved(&prepared, realm, |env| {
        env.vals
            .iter()
            .filter_map(|(symbol, val)| val.ty().map(|ty| (*symbol, ty.clone())))
            .collect::<rustc_hash::FxHashMap<_, _>>()
    });
    let lookup = |symbol| types.get(&symbol).cloned();
    dts::render(prepared.program, &prepared.scoping, code, &lookup)
}
