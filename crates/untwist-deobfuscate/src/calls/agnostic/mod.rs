mod assembled;
mod bind;
mod bootstrap;
mod consume;
mod defs;
mod flow;
mod forward;
mod fresh;
mod holds;
mod hosts;
mod judge;
mod keys;
mod live;
mod observe;
mod reach;
mod reach_text;
mod roles;
mod settle;
mod sites;
mod source;
mod subst;
mod text;
mod visit;
mod writes;

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;

use crate::proof::{PatchKey, ProtoRoot, ProtoWrite};
use settle::settle;
use untwist_eval::value::JsString;

#[cfg(feature = "test-hooks")]
pub use reach_text::comparing_text_scopings;
pub use roles::{key_roles, KeyRoles};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AKey<'a> {
    Named(Cow<'a, str>),
    Index,
    Pending(Span),
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ARoot {
    Any,
    Str,
    Arr,
    Obj,
    Other,
}

pub fn aroot_of(root: ProtoRoot) -> ARoot {
    match root {
        ProtoRoot::String => ARoot::Str,
        ProtoRoot::Array => ARoot::Arr,
        ProtoRoot::Object => ARoot::Obj,
        ProtoRoot::Number | ProtoRoot::Function | ProtoRoot::RegExp => ARoot::Other,
        ProtoRoot::Either => ARoot::Any,
    }
}

pub(super) fn root_folds(root: ProtoRoot) -> bool {
    matches!(
        root,
        ProtoRoot::Either | ProtoRoot::String | ProtoRoot::Array | ProtoRoot::Object
    )
}

#[derive(Clone, Debug)]
pub struct ASite<'a> {
    pub span: Span,
    pub key: AKey<'a>,
    pub identity: bool,
    pub root: ProtoRoot,
    pub(super) outside: bool,
}

#[derive(Default)]
pub struct Agnostic<'a> {
    pub sites: Vec<ASite<'a>>,
    pub code_text: Option<Span>,
    pub unbounded: Option<(Span, &'static str)>,
    pub runner_unbounded: Option<(Span, &'static str)>,
    pub realm_rebound: bool,
    pub reach: Option<Span>,
    pub reach_code: Option<Span>,
    pub observed: Option<Span>,
    pub(super) observe_keyed: Vec<(Span, Cow<'a, str>)>,
    pub(super) ambient: Vec<(Span, &'a str, bool)>,
    pub(super) saturated: bool,
    pub(super) global_source: bool,
    pub(super) observe_stored: Option<Span>,
    pub(super) stores_global: bool,
    pub(super) pending_texts: Vec<String>,
    pub bootstrap: Option<&'static str>,
    #[cfg(feature = "refusal-spans")]
    pub(super) trail: Vec<crate::refusal_spans::RefusalSpan>,
    #[cfg(feature = "refusal-spans")]
    pub(super) source_at: Option<(Span, &'static str)>,
}

pub fn agnostic<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    resolve: Option<&FxHashMap<Span, &JsString>>,
    machinery: &[Span],
) -> Agnostic<'a> {
    settle(
        program,
        scoping,
        (resolve, machinery, settle::Watch::default()),
    )
    .0
}

#[derive(Clone, Copy)]
pub struct ReachWatch<'k> {
    pub names: &'k [&'k str],
    pub props: &'k [&'k str],
    pub fns: &'k [SymbolId],
    pub observe: bool,
}

#[derive(Default)]
pub struct GlobalReach {
    pub reach: Option<Span>,
    pub observed: Option<Span>,
    pub bootstrap: Option<&'static str>,
    #[cfg(feature = "refusal-spans")]
    pub trail: Vec<crate::refusal_spans::RefusalSpan>,
    #[cfg(feature = "refusal-spans")]
    pub source: Option<crate::refusal_spans::RefusalSpan>,
}

fn reach_watch<'k>(reach: ReachWatch<'k>) -> settle::Watch<'k> {
    settle::Watch {
        names: reach.names,
        props: reach.props,
        fns: reach.fns,
        depth: reach_text::TEXT_DEPTH,
        observe: reach.observe,
        defer: false,
    }
}

fn reach_of(found: &Agnostic<'_>) -> GlobalReach {
    if !found.global_source {
        return GlobalReach::default();
    }
    GlobalReach {
        reach: found.reach.or(found.reach_code),
        observed: found.observed,
        bootstrap: found.bootstrap,
        #[cfg(feature = "refusal-spans")]
        trail: found.trail.clone(),
        #[cfg(feature = "refusal-spans")]
        source: found
            .source_at
            .map(|(span, what)| crate::refusal_spans::RefusalSpan {
                what: format!("global source: {what}"),
                span,
            }),
    }
}

pub fn global_reach<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    (resolve, machinery): (Option<&FxHashMap<Span, &JsString>>, &[Span]),
    reach: ReachWatch<'_>,
) -> GlobalReach {
    let found = settle(program, scoping, (resolve, machinery, reach_watch(reach))).0;
    reach_of(&found)
}

pub fn globals_reachable_by_key<'a>(program: &'a Program<'a>, scoping: &Scoping) -> bool {
    let globals = settle::script_globals(program, scoping);
    if globals.is_empty() {
        return false;
    }
    let mut names: Vec<&str> = globals
        .iter()
        .map(|symbol| scoping.symbol_name(*symbol))
        .collect();
    names.sort_unstable();
    names.dedup();
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &[],
        observe: false,
    };
    global_reach(program, scoping, (None, &[]), watch)
        .reach
        .is_some()
}

fn text_reach<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    (names, props): (&[&str], &[&str]),
    depth: u8,
) -> (bool, Vec<String>) {
    let watch = settle::Watch {
        names,
        props,
        fns: &[],
        depth,
        observe: true,
        defer: true,
    };
    let found = settle(program, scoping, (None, &[], watch)).0;
    let reached = found
        .reach
        .or(found.reach_code)
        .or(found.observed)
        .is_some();
    (reached, found.pending_texts)
}

pub fn exposed_script_globals<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
) -> Option<FxHashSet<SymbolId>> {
    let (out, rebound) = settle(program, scoping, (None, &[], settle::Watch::default()));
    if out.saturated || out.code_text.is_some() {
        return None;
    }
    if out.global_source {
        return Some(
            settle::script_globals(program, scoping)
                .into_iter()
                .collect(),
        );
    }
    Some(rebound.into_iter().collect())
}

fn written_key<'a>(key: AKey<'a>, extra: &mut Vec<(Span, String)>) -> PatchKey<'a> {
    match key {
        AKey::Named(Cow::Borrowed(name)) => PatchKey::Named(name),
        AKey::Named(Cow::Owned(name)) => {
            let marker = u32::MAX - u32::try_from(extra.len()).unwrap_or(0);
            let span = Span::new(marker, marker);
            extra.push((span, name));
            PatchKey::Pending(span)
        }
        AKey::Index => PatchKey::Named("@@index"),
        AKey::Pending(span) => PatchKey::Pending(span),
        AKey::Unknown => PatchKey::Dynamic,
    }
}

pub struct GuardWrites<'a> {
    pub writes: Vec<ProtoWrite<'a>>,
    pub extra: Vec<(Span, String)>,
    pub reach: Option<Span>,
    pub bootstrap: Option<&'static str>,
    #[cfg(feature = "refusal-spans")]
    pub trail: Vec<crate::refusal_spans::RefusalSpan>,
}

pub fn guard_writes<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    (resolve, machinery): (Option<&FxHashMap<Span, &JsString>>, &[Span]),
    reach: Option<ReachWatch<'_>>,
) -> GuardWrites<'a> {
    let watch = reach.map_or_else(settle::Watch::default, reach_watch);
    let found = settle(program, scoping, (resolve, machinery, watch)).0;
    let reached = reach_of(&found).reach;
    let bootstrap = found.bootstrap;
    #[cfg(feature = "refusal-spans")]
    let trail = guard_trail(&found, machinery);
    let mut extra = Vec::new();
    let mut writes: Vec<ProtoWrite<'a>> = guard_bail(&found, machinery)
        .map(|span| ProtoWrite {
            span,
            root: ProtoRoot::Either,
            root_key: None,
            key: PatchKey::Whole,
        })
        .into_iter()
        .collect();
    for (root, site) in outside_sites(found, machinery) {
        writes.push(ProtoWrite {
            span: site.span,
            root,
            root_key: None,
            key: written_key(site.key, &mut extra),
        });
    }
    GuardWrites {
        writes,
        extra,
        reach: reached,
        bootstrap,
        #[cfg(feature = "refusal-spans")]
        trail,
    }
}

#[cfg(feature = "refusal-spans")]
fn guard_trail(found: &Agnostic<'_>, machinery: &[Span]) -> Vec<crate::refusal_spans::RefusalSpan> {
    if !crate::refusal_spans::active() {
        return Vec::new();
    }
    let keyed = found
        .sites
        .iter()
        .filter(|site| !inside(machinery, site.span))
        .filter(|site| matches!(site.key, AKey::Pending(_) | AKey::Unknown))
        .map(|site| crate::refusal_spans::RefusalSpan {
            what: "write by an unknown key".to_string(),
            span: site.span,
        });
    found.trail.iter().cloned().chain(keyed).collect()
}

fn inside(machinery: &[Span], span: Span) -> bool {
    machinery
        .iter()
        .any(|outer| outer.start <= span.start && span.end <= outer.end)
}

fn guard_bail(found: &Agnostic<'_>, machinery: &[Span]) -> Option<Span> {
    let outside = |span: &Span| !inside(machinery, *span);
    found.code_text.filter(outside).or_else(|| {
        found
            .saturated
            .then(|| found.sites.first().map(|site| site.span))
            .flatten()
            .filter(outside)
    })
}

fn outside_sites<'a>(found: Agnostic<'a>, machinery: &[Span]) -> Vec<(ProtoRoot, ASite<'a>)> {
    let rebound = found.realm_rebound;
    let mut sites: Vec<(ProtoRoot, ASite<'a>)> = found
        .sites
        .into_iter()
        .filter(|site| !inside(machinery, site.span) && (rebound || !site.identity))
        .map(|site| {
            let root = if rebound {
                ProtoRoot::Either
            } else {
                site.root
            };
            (root, site)
        })
        .collect();
    sites.sort_by(|(a, x), (b, y)| (a, &x.key).cmp(&(b, &y.key)));
    sites.dedup_by(|(a, x), (b, y)| a == b && x.key == y.key);
    sites
}

#[derive(Default, Clone, Debug)]
pub struct FoldRecord {
    pub keys: Vec<(Option<String>, ProtoRoot)>,
    pub code_text: bool,
}

pub fn fold_record(program: &Program<'_>, scoping: &Scoping) -> FoldRecord {
    let found = agnostic(program, scoping, None, &[]);
    let rebound = found.realm_rebound;
    let mut keys: Vec<(Option<String>, ProtoRoot)> = found
        .sites
        .iter()
        .filter(|site| rebound || !site.identity)
        .filter_map(|site| {
            let root = if rebound {
                ProtoRoot::Either
            } else {
                site.root
            };
            match &site.key {
                AKey::Named(name) => Some((Some(name.to_string()), root)),
                AKey::Pending(_) | AKey::Unknown => Some((None, root)),
                AKey::Index => None,
            }
        })
        .collect();
    keys.sort();
    keys.dedup();
    FoldRecord {
        keys,
        code_text: found.code_text.is_some() || found.saturated,
    }
}
