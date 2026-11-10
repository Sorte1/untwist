use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program};
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_span::SPAN;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::PassOptions;
use untwist_core::semantic::{has_module_syntax, is_constant_binding};
use untwist_eval::fold::{const_to_expression, Const};
use untwist_eval::value::JsString;

use super::eval::literal_argument;
use super::eval::{Machine, Shape};
use super::scan::{declared, Declared, Func};
use super::uses::{uses, Arg, Role, Site, Uses};
use crate::bindings::block_function_names;
use crate::calls::agnostic::{global_reach, ReachWatch};

pub(super) struct Plan {
    pub(super) folds: Vec<(Span, Const)>,
    pub(super) removals: Vec<Span>,
}

struct Candidate<'d, 'p> {
    func: &'d Func<'p>,
    consulted: Vec<SymbolId>,
}

pub(super) fn plan<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    opts: &PassOptions,
) -> Option<Plan> {
    let declared = declared(program, scoping);
    if !declared
        .funcs
        .iter()
        .any(|func| declared.called.contains(&func.symbol))
    {
        return None;
    }
    let candidates = unshared(candidates(&declared, scoping), scoping);
    if candidates.is_empty() {
        return None;
    }
    let found = uses(program, scoping, &tracked(&candidates, &declared));
    let candidates = contained(candidates, &found, opts);
    let (folds, parents) = folds(&candidates, &declared, &found, scoping);
    if folds.is_empty() {
        return None;
    }
    let all: Vec<Span> = folds.iter().map(|(span, _, _)| *span).collect();
    let mut screened = unreached(candidates, folds, (program, scoping), opts);
    let kept = kept_nests(&all, &mut screened, &parents);
    if screened.folds.is_empty() {
        return None;
    }
    let removals = removals(&screened, (&declared, &found), scoping);
    let folds = screened.folds;
    Some(Plan {
        folds: folds
            .into_iter()
            .filter(|(span, _, _)| {
                parents
                    .get(span)
                    .is_none_or(|parent| !kept.contains(parent))
            })
            .map(|(span, value, _)| (span, value))
            .collect(),
        removals,
    })
}

fn kept_nests(
    all: &[Span],
    screened: &mut Screened<'_, '_>,
    parents: &FxHashMap<Span, Span>,
) -> FxHashSet<Span> {
    let mut kept: FxHashSet<Span> = screened.folds.iter().map(|(span, _, _)| *span).collect();
    let mut lost: Vec<Span> = all
        .iter()
        .filter(|span| !kept.contains(span))
        .copied()
        .collect();
    while let Some(span) = lost.pop() {
        if let Some(parent) = parents.get(&span).filter(|parent| kept.remove(parent)) {
            lost.push(*parent);
        }
    }
    screened.folds.retain(|(span, _, _)| kept.contains(span));
    kept
}

fn candidates<'d, 'p>(declared: &'d Declared<'p>, scoping: &Scoping) -> Vec<Candidate<'d, 'p>> {
    let constant = |symbol: SymbolId| {
        declared.constants.contains_key(&symbol) && is_constant_binding(scoping, symbol)
    };
    declared
        .funcs
        .iter()
        .filter(|func| declared.called.contains(&func.symbol))
        .filter(|func| is_constant_binding(scoping, func.symbol))
        .filter_map(|func| {
            let mut shape = Shape {
                scoping,
                params: &func.params,
                constants: &constant,
                consulted: Vec::new(),
            };
            let pure = shape.pure(func.body);
            pure.then_some(Candidate {
                func,
                consulted: shape.consulted,
            })
        })
        .collect()
}

fn unshared<'d, 'p>(
    candidates: Vec<Candidate<'d, 'p>>,
    scoping: &Scoping,
) -> Vec<Candidate<'d, 'p>> {
    if candidates.is_empty() {
        return candidates;
    }
    let shared = block_function_names(scoping);
    if shared.is_empty() {
        return candidates;
    }
    let free = |symbol: &SymbolId| !shared.contains(scoping.symbol_name(*symbol));
    candidates
        .into_iter()
        .filter(|candidate| free(&candidate.func.symbol) && candidate.consulted.iter().all(free))
        .collect()
}

fn tracked(candidates: &[Candidate<'_, '_>], declared: &Declared<'_>) -> FxHashMap<SymbolId, Role> {
    let mut tracked = FxHashMap::default();
    for candidate in candidates {
        tracked.insert(candidate.func.symbol, Role::Func);
        for symbol in &candidate.consulted {
            let data = declared.constants.get(symbol).is_some_and(|constant| {
                matches!(
                    constant.init,
                    Expression::ArrayExpression(_) | Expression::ObjectExpression(_)
                )
            });
            let role = if data { Role::Data } else { Role::Primitive };
            tracked.insert(*symbol, role);
        }
    }
    tracked
}

type Fold = (Span, Const, SymbolId);

fn contained<'d, 'p>(
    candidates: Vec<Candidate<'d, 'p>>,
    found: &Uses,
    opts: &PassOptions,
) -> Vec<Candidate<'d, 'p>> {
    let strict = !opts.unsafe_transforms;
    candidates
        .into_iter()
        .filter(|candidate| {
            !candidate
                .consulted
                .iter()
                .any(|symbol| found.escaped.contains(symbol))
        })
        .filter(|candidate| !strict || !found.escaped.contains(&candidate.func.symbol))
        .collect()
}

struct Screened<'d, 'p> {
    candidates: Vec<Candidate<'d, 'p>>,
    folds: Vec<Fold>,
    declared_kept: FxHashSet<SymbolId>,
}

fn unscreened<'d, 'p>(candidates: Vec<Candidate<'d, 'p>>, folds: Vec<Fold>) -> Screened<'d, 'p> {
    Screened {
        candidates,
        folds,
        declared_kept: FxHashSet::default(),
    }
}

fn sorted_names(scoping: &Scoping, symbols: impl Iterator<Item = SymbolId>) -> Vec<&str> {
    let mut names: Vec<&str> = symbols.map(|symbol| scoping.symbol_name(symbol)).collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn unreached<'d, 'p>(
    candidates: Vec<Candidate<'d, 'p>>,
    folds: Vec<Fold>,
    (program, scoping): (&'p Program<'p>, &Scoping),
    opts: &PassOptions,
) -> Screened<'d, 'p> {
    if !reach_checked(program, opts) {
        return unscreened(candidates, folds);
    }
    let root = scoping.root_scope_id();
    let at_root = |symbol: &SymbolId| scoping.symbol_scope_id(*symbol) == root;
    let (touching, apart): (Vec<_>, Vec<_>) = candidates.into_iter().partition(|candidate| {
        at_root(&candidate.func.symbol) || candidate.consulted.iter().any(at_root)
    });
    let folded: FxHashSet<SymbolId> = folds.iter().map(|(_, _, symbol)| *symbol).collect();
    let watched: Vec<SymbolId> = touching
        .iter()
        .filter(|candidate| folded.contains(&candidate.func.symbol))
        .flat_map(|candidate| std::iter::once(&candidate.func.symbol).chain(&candidate.consulted))
        .filter(|symbol| at_root(symbol))
        .copied()
        .collect();
    let mut candidates = apart;
    if watched.is_empty() {
        candidates.extend(touching);
        return unscreened(candidates, folds);
    }
    match root_reached(&watched, &folds, (program, scoping)) {
        Reached::Rebound => {
            let dropped: FxHashSet<SymbolId> = touching
                .iter()
                .map(|candidate| candidate.func.symbol)
                .collect();
            let folds = folds
                .into_iter()
                .filter(|(_, _, symbol)| !dropped.contains(symbol))
                .collect();
            unscreened(candidates, folds)
        }
        Reached::Observed(properties) => {
            candidates.extend(touching);
            Screened {
                candidates,
                folds,
                declared_kept: properties,
            }
        }
        Reached::Neither => {
            candidates.extend(touching);
            unscreened(candidates, folds)
        }
    }
}

fn reach_checked(program: &Program<'_>, opts: &PassOptions) -> bool {
    !opts.nested_root && !has_module_syntax(program)
}

enum Reached {
    Rebound,
    Observed(FxHashSet<SymbolId>),
    Neither,
}

fn root_reached<'p>(
    watched: &[SymbolId],
    folds: &[Fold],
    (program, scoping): (&'p Program<'p>, &Scoping),
) -> Reached {
    let global_property = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
    let properties: FxHashSet<SymbolId> = watched
        .iter()
        .copied()
        .filter(|symbol| scoping.symbol_flags(*symbol).intersects(global_property))
        .collect();
    let names = sorted_names(scoping, watched.iter().copied());
    let props = sorted_names(scoping, properties.iter().copied());
    let texts: Vec<(Span, JsString)> = folds
        .iter()
        .filter_map(|(span, value, _)| match value {
            Const::Str(text) => Some((*span, JsString::from_str(text))),
            _ => None,
        })
        .collect();
    let resolve: FxHashMap<Span, &JsString> =
        texts.iter().map(|(span, text)| (*span, text)).collect();
    let watch = ReachWatch {
        names: &names,
        props: &props,
        fns: &[],
        observe: true,
    };
    let found = global_reach(program, scoping, (Some(&resolve), &[]), watch);
    if found.reach.is_some() {
        return Reached::Rebound;
    }
    if found.observed.is_some() && !properties.is_empty() {
        return Reached::Observed(properties);
    }
    Reached::Neither
}

fn runs_after(site: &Site, symbol: SymbolId, found: &Uses, scoping: &Scoping) -> bool {
    let scope = scoping.symbol_scope_id(symbol);
    let declared_first = found
        .declared_at
        .get(&symbol)
        .is_some_and(|declared| site.position > *declared);
    declared_first
        && !site.hoisted.iter().any(|function| {
            scoping
                .scope_ancestors(*function)
                .skip(1)
                .any(|ancestor| ancestor == scope)
        })
}

fn ordered(
    site: &Site,
    candidate: &Candidate<'_, '_>,
    (declared, found): (&Declared<'_>, &Uses),
    scoping: &Scoping,
) -> bool {
    let func = candidate.func;
    let own = func.hoisted || runs_after(site, func.symbol, found, scoping);
    own && candidate.consulted.iter().all(|symbol| {
        declared.constants.contains_key(symbol) && runs_after(site, *symbol, found, scoping)
    })
}

fn evaluate<'p>(
    args: &[Const],
    func: &Func<'p>,
    declared: &Declared<'p>,
    scoping: &Scoping,
) -> Option<Const> {
    let bound: FxHashMap<SymbolId, Const> = func
        .params
        .iter()
        .copied()
        .zip(args.iter().cloned())
        .collect();
    let lookup = |symbol: SymbolId| {
        declared
            .constants
            .get(&symbol)
            .map(|constant| constant.init)
    };
    Machine::new(scoping, &func.params, bound, &lookup).result(func.body)
}

fn folded_literal(value: &Const, builder: &AstBuilder<'_>) -> Option<Const> {
    literal_argument(&const_to_expression(value, SPAN, builder)?)
}

fn nested_values(
    nested: &[Arg],
    index: &FxHashMap<Span, usize>,
    values: &[Option<Const>],
    builder: &AstBuilder<'_>,
) -> Option<Vec<Const>> {
    nested
        .iter()
        .map(|arg| match arg {
            Arg::Literal(value) => Some(value.clone()),
            Arg::Call(span) if span.start < span.end => {
                folded_literal(values.get(*index.get(span)?)?.as_ref()?, builder)
            }
            Arg::Call(_) => None,
        })
        .collect()
}

fn folds(
    candidates: &[Candidate<'_, '_>],
    declared: &Declared<'_>,
    found: &Uses,
    scoping: &Scoping,
) -> (Vec<Fold>, FxHashMap<Span, Span>) {
    let by_symbol: FxHashMap<SymbolId, &Candidate<'_, '_>> = candidates
        .iter()
        .map(|candidate| (candidate.func.symbol, candidate))
        .collect();
    let index: FxHashMap<Span, usize> = found
        .sites
        .iter()
        .enumerate()
        .map(|(at, site)| (site.span, at))
        .collect();
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut values: Vec<Option<Const>> = vec![None; found.sites.len()];
    let mut parents = FxHashMap::default();
    for at in (0..found.sites.len()).rev() {
        let site = &found.sites[at];
        let Some(candidate) = by_symbol.get(&site.symbol) else {
            continue;
        };
        if !ordered(site, candidate, (declared, found), scoping) {
            continue;
        }
        let args = match (&site.args, &site.nested) {
            (Some(args), _) => args.clone(),
            (None, Some(nested)) => match nested_values(nested, &index, &values, &builder) {
                Some(args) => args,
                None => continue,
            },
            (None, None) => continue,
        };
        values[at] = evaluate(&args, candidate.func, declared, scoping);
        if values[at].is_some() {
            record_parents(site, &mut parents);
        }
    }
    let folds = found
        .sites
        .iter()
        .zip(values)
        .filter_map(|(site, value)| Some((site.span, value?, site.symbol)))
        .collect();
    (folds, parents)
}

fn record_parents(site: &Site, parents: &mut FxHashMap<Span, Span>) {
    for arg in site.nested.iter().flatten() {
        if let Arg::Call(span) = arg {
            parents.insert(*span, site.span);
        }
    }
}

fn inside(spans: &[Span], span: Span) -> bool {
    spans
        .iter()
        .any(|outer| outer.start <= span.start && span.end <= outer.end)
}

fn removals(
    screened: &Screened<'_, '_>,
    (declared, found): (&Declared<'_>, &Uses),
    scoping: &Scoping,
) -> Vec<Span> {
    let kept = &screened.declared_kept;
    let mut folded: FxHashMap<SymbolId, usize> = FxHashMap::default();
    for (_, _, symbol) in &screened.folds {
        *folded.entry(*symbol).or_default() += 1;
    }
    let only_folded = |symbol: SymbolId| {
        let seen = found.references.get(&symbol).map_or(0, Vec::len);
        let resolved = scoping.get_resolved_references(symbol).count();
        folded
            .get(&symbol)
            .is_some_and(|count| *count == seen && *count == resolved)
    };
    let gone: Vec<&Candidate<'_, '_>> = screened
        .candidates
        .iter()
        .filter(|candidate| !kept.contains(&candidate.func.symbol))
        .filter(|candidate| only_folded(candidate.func.symbol))
        .collect();
    let mut removed: Vec<Span> = gone
        .iter()
        .map(|candidate| candidate.func.statement)
        .collect();
    let mut consulted: Vec<SymbolId> = gone
        .iter()
        .flat_map(|candidate| candidate.consulted.iter().copied())
        .collect();
    consulted.sort_unstable();
    consulted.dedup();
    let unused: Vec<Span> = consulted
        .iter()
        .filter(|symbol| !kept.contains(symbol))
        .filter(|symbol| {
            let resolved = scoping.get_resolved_references(**symbol).count();
            found.references.get(symbol).is_some_and(|spans| {
                spans.len() == resolved && spans.iter().all(|span| inside(&removed, *span))
            })
        })
        .filter_map(|symbol| {
            declared
                .constants
                .get(symbol)
                .map(|constant| constant.statement)
        })
        .collect();
    removed.extend(unused);
    removed
}
