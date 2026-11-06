use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::cell::RefCell;

use super::assembled::{broken_tables, drop_tables};
use super::bind::Bindings;
use super::filtered::Filters;
use super::flow::Flow;
use super::fresh::Fresh;
use super::native_map;
use super::sites::*;
use super::{AKey, ASite, Agnostic};
use crate::proof::ProtoRoot;
use untwist_eval::value::JsString;

#[derive(Clone, Copy, Default)]
pub(super) struct Watch<'k> {
    pub(super) names: &'k [&'k str],
    pub(super) props: &'k [&'k str],
    pub(super) fns: &'k [SymbolId],
    pub(super) depth: u8,
    pub(super) observe: bool,
    pub(super) defer: bool,
}

const LINEAR_LOOKUP: usize = 16;

#[inline(always)]
fn listed<T: Ord>(list: &[T], item: &T) -> bool {
    if list.len() <= LINEAR_LOOKUP {
        return list.contains(item);
    }
    searched(list, item)
}

#[inline(never)]
fn searched<T: Ord>(list: &[T], item: &T) -> bool {
    list.binary_search(item).is_ok()
}

impl Watch<'_> {
    #[inline(always)]
    pub(super) fn has_name(&self, name: &str) -> bool {
        listed(self.names, &name)
    }

    #[inline(always)]
    pub(super) fn has_prop(&self, name: &str) -> bool {
        listed(self.props, &name)
    }

    #[inline(always)]
    pub(super) fn has_fn(&self, symbol: SymbolId) -> bool {
        listed(self.fns, &symbol)
    }
}

fn ordered<T: Ord + Copy>(list: &[T]) -> Cow<'_, [T]> {
    if list.len() <= LINEAR_LOOKUP || list.is_sorted_by(|left, right| left < right) {
        return Cow::Borrowed(list);
    }
    let mut owned = list.to_vec();
    owned.sort_unstable();
    owned.dedup();
    Cow::Owned(owned)
}

pub(super) type Inputs<'k> = (
    Option<&'k FxHashMap<Span, &'k JsString>>,
    &'k [Span],
    Watch<'k>,
);

pub(super) fn run<'s, 'a>(
    program: &'a Program<'a>,
    scoping: &'s Scoping,
    bindings: &'s Bindings<'s, 'a>,
    (resolve, machinery, watch): Inputs<'s>,
    fresh_calls: bool,
    pristine: Pristine,
    flow: &'s Flow<'s, 'a>,
) -> (Agnostic<'a>, bool, bool) {
    let fresh = Fresh {
        scoping,
        bindings,
        sealed: RefCell::default(),
    };
    let mut sites = Sites::new(
        fresh,
        (resolve, machinery, watch),
        fresh_calls,
        pristine,
        flow,
    );
    sites.inline_budget = inline_budget(program);
    sites.visit_program(program);
    sites.judge_collected()
}

const INLINE_BYTES_PER_INPUT_BYTE: usize = 4;
const INLINE_BYTES_FLOOR: usize = 1 << 16;

fn inline_budget(program: &Program<'_>) -> usize {
    (program.span.size() as usize)
        .saturating_mul(INLINE_BYTES_PER_INPUT_BYTE)
        .max(INLINE_BYTES_FLOOR)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Pristine {
    pub(super) symbol: bool,
    pub(super) map: bool,
}

impl Pristine {
    fn of(bindings: &Bindings<'_, '_>) -> Self {
        Self {
            symbol: bindings.symbol_safe(),
            map: bindings.map_safe(),
        }
    }

    fn after(self, written: &Written<'_>) -> Self {
        Self {
            symbol: self.symbol && !written.may_be("Symbol"),
            map: self.map && !native_map::may_be_patched(written),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Judged {
    source: bool,
    pristine: Pristine,
}

pub(super) fn settle<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    (resolve, machinery, watch): Inputs<'_>,
) -> (Agnostic<'a>, Vec<SymbolId>) {
    let (names, props, fns) = (
        ordered(watch.names),
        ordered(watch.props),
        ordered(watch.fns),
    );
    let watch = Watch {
        names: &names,
        props: &props,
        fns: &fns,
        ..watch
    };
    settle_ordered(program, scoping, (resolve, machinery, watch))
}

fn settle_ordered<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    (resolve, machinery, watch): Inputs<'_>,
) -> (Agnostic<'a>, Vec<SymbolId>) {
    let mut bindings = Bindings::collect(program, scoping, !watch.names.is_empty(), resolve);
    let flow = Flow::new(program, scoping, resolve);
    let filters = std::rc::Rc::clone(&bindings.filters);
    let globals = script_globals(program, scoping);
    let mut judged = Judged {
        source: bindings.global_source(),
        pristine: Pristine::of(&bindings),
    };
    if judged.source {
        bindings.hide_calls(&globals);
    }
    let mut settled = Vec::new();
    loop {
        let mut out = analyse(
            program,
            scoping,
            &bindings,
            (resolve, machinery, watch),
            judged.pristine,
            &flow,
        );
        let written = Written::of(&out.sites, machinery, (&filters, scoping));
        let next = Judged {
            source: judged.source || out.global_source,
            pristine: judged.pristine.after(&written),
        };
        if next.source && !judged.source {
            bindings.hide_calls(&globals);
        }
        let rebound = rebound_globals(&out, &globals, &bindings, (next.source, &written));
        let broken = broken_tables(&out, &bindings.assembled, machinery);
        drop_tables(&mut bindings.assembled, &broken);
        if rebound.is_empty() && broken.is_empty() && next == judged {
            let (reach, observed) = rebound_ambient(&out, scoping, &written);
            out.realm_rebound |= roots_rebound(scoping, &out.sites, machinery, &filters);
            out.global_source = judged.source;
            #[cfg(feature = "refusal-spans")]
            {
                out.source_at = bindings.first_source().or(out.source_at);
            }
            out.reach = out.reach.or(reach);
            out.observed = final_observed(&out, &written, observed);
            out.bootstrap = bindings.bootstrap_refusal;
            return (out, settled);
        }
        judged = next;
        bindings.make_opaque(&rebound);
        settled.extend(rebound);
    }
}

pub(super) fn analyse<'s, 'a>(
    program: &'a Program<'a>,
    scoping: &'s Scoping,
    bindings: &'s Bindings<'s, 'a>,
    inputs: Inputs<'s>,
    pristine: Pristine,
    flow: &'s Flow<'s, 'a>,
) -> Agnostic<'a> {
    let (mut out, mut chains, natives) =
        run(program, scoping, bindings, inputs, true, pristine, flow);
    if !natives {
        let source = out.global_source;
        #[cfg(feature = "refusal-spans")]
        let source_at = out.source_at;
        drop(out);
        let rerun = run(program, scoping, bindings, inputs, false, pristine, flow);
        out = rerun.0;
        out.global_source |= source;
        #[cfg(feature = "refusal-spans")]
        {
            out.source_at = out.source_at.or(source_at);
        }
        chains = rerun.1;
    }
    if !chains {
        let span = out.sites.first().map_or(Span::default(), |site| site.span);
        #[cfg(feature = "refusal-spans")]
        if crate::refusal_spans::active() {
            out.trail.push(crate::refusal_spans::RefusalSpan {
                what: "a folded chain's assumed key may be written".to_string(),
                span,
            });
        }
        out.sites.push(ASite {
            span,
            key: AKey::Unknown,
            identity: false,
            root: ProtoRoot::Either,
            outside: !super::inside(inputs.1, span),
        });
    }
    out
}

pub(super) fn script_globals(program: &Program<'_>, scoping: &Scoping) -> Vec<SymbolId> {
    if untwist_core::semantic::has_module_syntax(program) {
        return Vec::new();
    }
    let declared = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
    scoping
        .get_bindings(scoping.root_scope_id())
        .iter()
        .filter(|(_, symbol)| scoping.symbol_flags(**symbol).intersects(declared))
        .map(|(_, symbol)| *symbol)
        .collect()
}

fn is_index_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| byte.is_ascii_digit())
        && (name == "0" || !name.starts_with('0'))
}

#[derive(Default)]
pub(super) struct Written<'k> {
    named: Vec<&'k str>,
    spelled: Vec<&'k str>,
    index: bool,
    any: bool,
    filtered: Vec<Span>,
    filters: Option<(&'k Filters<'k>, &'k Scoping)>,
}

impl<'k> Written<'k> {
    fn of(
        sites: &'k [ASite<'_>],
        machinery: &[Span],
        filters: (&'k Filters<'k>, &'k Scoping),
    ) -> Self {
        let outside = |site: &&ASite<'_>| {
            !machinery
                .iter()
                .any(|outer| outer.start <= site.span.start && site.span.end <= outer.end)
        };
        let keys = || sites.iter().filter(outside).map(|site| &site.key);
        let any = keys().any(|key| matches!(key, AKey::Pending(_) | AKey::Unknown));
        if any {
            return Self {
                any,
                ..Self::default()
            };
        }
        let mut named =
            Vec::with_capacity(keys().filter(|key| matches!(key, AKey::Named(_))).count());
        let mut spelled = Vec::new();
        let mut index = false;
        let mut filtered = Vec::new();
        for key in keys() {
            match key {
                AKey::Named(name) => {
                    index |= is_index_name(name);
                    named.push(name.as_ref());
                    if let Cow::Borrowed(name) = name {
                        spelled.push(*name);
                    }
                }
                AKey::Index => index = true,
                AKey::Filtered(filter) => filtered.push(*filter),
                AKey::Pending(_) | AKey::Unknown => {}
            }
        }
        named.sort_unstable();
        named.dedup();
        spelled.sort_unstable();
        spelled.dedup();
        filtered.sort_unstable();
        filtered.dedup();
        Self {
            named,
            spelled,
            index,
            any,
            filters: (!filtered.is_empty()).then_some(filters),
            filtered,
        }
    }

    fn filter_accepts(&self, name: &str) -> bool {
        self.filters.is_some_and(|(filters, scoping)| {
            self.filtered
                .iter()
                .any(|filter| filters.accepts(scoping, *filter, name))
        })
    }

    pub(super) fn may_be_spelled(&self, name: &str) -> bool {
        self.any || self.spelled.binary_search(&name).is_ok() || self.filter_accepts(name)
    }

    fn names(&self, name: &str) -> bool {
        self.named.binary_search(&name).is_ok()
    }

    pub(super) fn may_be(&self, name: &str) -> bool {
        self.any
            || self.names(name)
            || (self.index && is_index_name(name))
            || self.filter_accepts(name)
    }

    pub(super) fn may_be_element(&self) -> bool {
        self.any || self.index || self.names("length") || !self.filtered.is_empty()
    }
}

fn rebound_globals(
    out: &Agnostic<'_>,
    globals: &[SymbolId],
    bindings: &Bindings<'_, '_>,
    (source, written): (bool, &Written<'_>),
) -> Vec<SymbolId> {
    if globals.is_empty() || out.saturated || out.code_text.is_some() {
        return Vec::new();
    }
    globals
        .iter()
        .filter(|symbol| {
            !bindings.is_opaque(**symbol)
                && (written.may_be(bindings.symbol_name(**symbol))
                    || (source && bindings.state_written(**symbol, written)))
        })
        .copied()
        .collect()
}

fn final_observed(
    out: &Agnostic<'_>,
    written: &Written<'_>,
    observed: Option<Span>,
) -> Option<Span> {
    out.observed
        .or(observed)
        .or_else(|| keyed_observed(out, written))
        .or_else(|| out.observe_stored.filter(|_| out.stores_global))
}

fn keyed_observed(out: &Agnostic<'_>, written: &Written<'_>) -> Option<Span> {
    out.observe_keyed
        .iter()
        .find(|(_, key)| out.code_text.is_some() || written.may_be(key))
        .map(|(span, _)| *span)
}

fn rebound_ambient(
    out: &Agnostic<'_>,
    scoping: &Scoping,
    written: &Written<'_>,
) -> (Option<Span>, Option<Span>) {
    let unresolved = scoping.root_unresolved_references();
    let assigned = |name: &str| {
        unresolved.get(name).is_some_and(|references| {
            references
                .iter()
                .any(|reference| scoping.get_reference(*reference).is_write())
        })
    };
    let rebound = |name: &str| out.saturated || written.may_be(name) || assigned(name);
    let first = |observe: bool| {
        out.ambient
            .iter()
            .find(|(_, name, kind)| *kind == observe && rebound(name))
            .map(|(span, _, _)| *span)
    };
    (first(false), first(true))
}

pub(super) const PRECISE_ROOTS: [&str; 7] = [
    "String", "Array", "Object", "Function", "Number", "RegExp", "Symbol",
];

pub(super) fn roots_rebound(
    scoping: &Scoping,
    sites: &[ASite<'_>],
    machinery: &[Span],
    filters: &Filters<'_>,
) -> bool {
    let unresolved = scoping.root_unresolved_references();
    let rewritten = PRECISE_ROOTS.iter().any(|name| {
        unresolved.get(*name).is_some_and(|references| {
            references
                .iter()
                .any(|reference| scoping.get_reference(*reference).is_write())
        })
    });
    let inside = |span: Span| {
        machinery
            .iter()
            .any(|outer| outer.start <= span.start && span.end <= outer.end)
    };
    rewritten
        || sites.iter().any(|site| {
            site.root == ProtoRoot::Either
                && !inside(site.span)
                && match &site.key {
                    AKey::Named(name) => PRECISE_ROOTS.contains(&name.as_ref()),
                    AKey::Index => false,
                    AKey::Filtered(filter) => PRECISE_ROOTS
                        .iter()
                        .any(|name| filters.accepts(scoping, *filter, name)),
                    AKey::Pending(_) | AKey::Unknown => true,
                }
        })
}
