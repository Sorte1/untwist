use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::Cx;

struct Candidate {
    at: usize,
    symbol: SymbolId,
    span: Span,
    target: Option<usize>,
    alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Mover {
    at: usize,
    target: usize,
    merged: Option<(SymbolId, Span)>,
}

#[derive(Default)]
pub(crate) struct Plan {
    movers: Vec<Mover>,
    pub(crate) moved: usize,
    pub(crate) merged: usize,
}

impl Plan {
    pub(crate) fn is_empty(&self) -> bool {
        self.moved + self.merged == 0
    }
}

fn lone_declarator<'s, 'a>(stmt: &'s Statement<'a>) -> Option<&'s VariableDeclarator<'a>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var || declaration.declare {
        return None;
    }
    match declaration.declarations.as_slice() {
        [declarator] => Some(declarator),
        _ => None,
    }
}

fn bare_binding<'s, 'a>(stmt: &'s Statement<'a>) -> Option<&'s BindingIdentifier<'a>> {
    let declarator = lone_declarator(stmt)?;
    if declarator.init.is_some() {
        return None;
    }
    match &declarator.id {
        BindingPattern::BindingIdentifier(id) => Some(id),
        _ => None,
    }
}

fn initialises(stmt: &Statement<'_>, symbol: SymbolId) -> bool {
    lone_declarator(stmt).is_some_and(|declarator| {
        declarator.init.is_some()
            && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                if id.symbol_id.get() == Some(symbol))
    })
}

fn candidates(
    scoping: &Scoping,
    stmts: &AVec<'_, Statement<'_>>,
    var_scope: ScopeId,
    refused: &impl Fn(&str) -> bool,
) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = Vec::new();
    let mut seen: FxHashSet<SymbolId> = FxHashSet::default();
    let mut doubled: FxHashSet<SymbolId> = FxHashSet::default();
    let body = body_span(stmts);
    for (at, stmt) in stmts.iter().enumerate() {
        let Some(id) = bare_binding(stmt) else {
            continue;
        };
        let Some(symbol) = id.symbol_id.get() else {
            continue;
        };
        let plain = scoping.symbol_scope_id(symbol) == var_scope
            && scoping.symbol_flags(symbol) == SymbolFlags::FunctionScopedVariable
            && !refused(id.name.as_str())
            && declared_in_body(scoping, symbol, body);
        if !plain {
            continue;
        }
        if !seen.insert(symbol) {
            doubled.insert(symbol);
            continue;
        }
        found.push(Candidate {
            at,
            symbol,
            span: stmt.span(),
            target: None,
            alive: true,
        });
    }
    found.retain(|candidate| !doubled.contains(&candidate.symbol));
    found
}

fn statement_holding(stmts: &AVec<'_, Statement<'_>>, span: Span) -> Option<usize> {
    stmts.iter().position(|stmt| within(stmt.span(), span))
}

fn first_declaration(
    scoping: &Scoping,
    stmts: &AVec<'_, Statement<'_>>,
    candidate: &Candidate,
) -> Option<Option<usize>> {
    let mut first = None;
    for declaration in scoping.symbol_redeclarations(candidate.symbol) {
        if within(candidate.span, declaration.span) {
            continue;
        }
        let at = statement_holding(stmts, declaration.span)?;
        first = Some(first.map_or(at, |seen: usize| seen.min(at)));
    }
    Some(first)
}

fn find_targets(scoping: &Scoping, stmts: &AVec<'_, Statement<'_>>, candidates: &mut [Candidate]) {
    let symbols: Vec<SymbolId> = candidates.iter().map(|c| c.symbol).collect();
    let uses = crate::var_capture::first_uses(scoping, stmts, &symbols);
    for candidate in candidates.iter_mut() {
        let Some(declared) = first_declaration(scoping, stmts, candidate) else {
            candidate.alive = false;
            continue;
        };
        let used = uses.get(&candidate.symbol).copied();
        let first = match (used, declared) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        match first {
            Some(at) if at < candidate.at => candidate.alive = false,
            Some(at) => candidate.target = Some(at),
            None => {}
        }
    }
}

fn within(outer: Span, inner: Span) -> bool {
    !inner.is_empty() && outer.start <= inner.start && inner.end <= outer.end
}

fn declared_in_body(scoping: &Scoping, symbol: SymbolId, body: Span) -> bool {
    scoping
        .symbol_redeclarations(symbol)
        .iter()
        .all(|declaration| within(body, declaration.span))
}

fn merge_span(scoping: &Scoping, symbol: SymbolId, bare: Span) -> Option<(SymbolId, Span)> {
    let declarations = scoping.symbol_redeclarations(symbol);
    let mut inside = declarations.iter().filter(|d| within(bare, d.span));
    let span = inside.next()?.span;
    let lone = inside.next().is_none()
        && declarations.len() == 2
        && declarations.iter().filter(|d| d.span == span).count() == 1;
    lone.then_some((symbol, span))
}

fn body_span(stmts: &AVec<'_, Statement<'_>>) -> Span {
    match (stmts.first(), stmts.last()) {
        (Some(first), Some(last)) => Span::new(first.span().start, last.span().end),
        _ => Span::default(),
    }
}

fn next_exits(stmts: &AVec<'_, Statement<'_>>) -> Vec<usize> {
    let mut exits = vec![usize::MAX; stmts.len() + 1];
    for (index, stmt) in stmts.iter().enumerate().rev() {
        let exit = matches!(
            stmt,
            Statement::ReturnStatement(_) | Statement::ThrowStatement(_)
        );
        exits[index] = if exit { index } else { exits[index + 1] };
    }
    exits
}

pub(crate) fn plan(
    scoping: &Scoping,
    stmts: &AVec<'_, Statement<'_>>,
    var_scope: ScopeId,
    refused: impl Fn(&str) -> bool,
) -> Plan {
    let mut found = candidates(scoping, stmts, var_scope, &refused);
    if found.is_empty() {
        return Plan::default();
    }
    find_targets(scoping, stmts, &mut found);
    let exits = next_exits(stmts);
    let moving: Vec<(&Candidate, usize)> = found
        .iter()
        .filter(|candidate| candidate.alive)
        .filter_map(|candidate| {
            let target = candidate.target?;
            Some((candidate, target.min(exits[candidate.at + 1])))
        })
        .collect();
    let mut mover_at = vec![false; stmts.len()];
    for (candidate, _) in &moving {
        mover_at[candidate.at] = true;
    }
    let mut still = Vec::with_capacity(stmts.len() + 1);
    still.push(0usize);
    for is_mover in &mover_at {
        let last = still.last().copied().unwrap_or(0);
        still.push(last + usize::from(!is_mover));
    }
    let between = |from: usize, to: usize| still[to] - still[from + 1];
    let mut plan = Plan::default();
    for (candidate, target) in moving {
        let merged = stmts
            .get(target)
            .filter(|stmt| initialises(stmt, candidate.symbol))
            .and_then(|_| merge_span(scoping, candidate.symbol, candidate.span));
        if merged.is_some() {
            plan.merged += 1;
        } else if between(candidate.at, target) > 0 {
            plan.moved += 1;
        }
        plan.movers.push(Mover {
            at: candidate.at,
            target,
            merged,
        });
    }
    if plan.is_empty() {
        return Plan::default();
    }
    plan
}

pub(crate) fn apply<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    plan: &Plan,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let movers: FxHashMap<usize, Mover> = plan.movers.iter().map(|m| (m.at, *m)).collect();
    let old = stmts.take_in(&*cx);
    let mut waiting: FxHashMap<usize, Vec<Statement<'a>>> = FxHashMap::default();
    let mut out = AVec::with_capacity_in(old.len(), &*cx);
    for (index, stmt) in old.into_iter().enumerate() {
        if let Some(parked) = waiting.remove(&index) {
            out.extend(parked);
        }
        match movers.get(&index) {
            Some(Mover {
                merged: Some((symbol, span)),
                ..
            }) => cx.scoping_mut().remove_symbol_declaration(*symbol, *span),
            Some(mover) => waiting.entry(mover.target).or_default().push(stmt),
            None => out.push(stmt),
        }
    }
    *stmts = out;
    plan.moved + plan.merged
}
