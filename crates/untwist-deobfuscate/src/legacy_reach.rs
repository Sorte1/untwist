use std::cell::RefCell;

use oxc_ast::ast::{BindingIdentifier, CallExpression, Expression, IdentifierReference, Program};
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashSet;

use crate::mutants::deferred_legacy_reach_trusted;

#[derive(Clone, Debug, Default)]
pub(crate) struct Layered {
    pub(crate) names: Vec<String>,
    pub(crate) used: Option<Vec<String>>,
    pub(crate) points_to: bool,
    pub(crate) both_sites: bool,
    pub(crate) late_reads: Option<Vec<untwist_eval::object::ProtoRead>>,
    pub(crate) roots: Vec<String>,
}

#[derive(Default)]
struct Deferral {
    rerunnable: bool,
    forbidden: bool,
    open: bool,
    pending: Vec<String>,
    layered: Vec<(String, Layered)>,
    observations: Vec<Vec<String>>,
    unverified: bool,
    escapes_open: bool,
    escapes: Vec<Span>,
    escape_names: Vec<Vec<String>>,
    escape_names_opened: Vec<Vec<String>>,
    carries_names: bool,
    reached: bool,
}

thread_local! {
    static DEFERRAL: RefCell<Deferral> = RefCell::new(Deferral::default());
}

pub(crate) struct Stage {
    open: bool,
    pending: Vec<String>,
    layered: Vec<(String, Layered)>,
}

pub(crate) fn open_stage(on: bool) -> Stage {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        let open = on && deferral.rerunnable && !deferral.forbidden;
        Stage {
            open: std::mem::replace(&mut deferral.open, open),
            pending: std::mem::take(&mut deferral.pending),
            layered: std::mem::take(&mut deferral.layered),
        }
    })
}

impl Drop for Stage {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| {
            let mut deferral = deferral.borrow_mut();
            deferral.unverified |= !deferral.pending.is_empty();
            deferral.open = self.open;
            deferral.pending = std::mem::take(&mut self.pending);
            deferral.layered = std::mem::take(&mut self.layered);
        });
    }
}

pub(crate) struct EscapeStage {
    open: bool,
    escapes: Vec<Span>,
}

pub(crate) fn open_escape_stage(on: bool) -> EscapeStage {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        let open = on && deferral.rerunnable && !deferral.forbidden;
        EscapeStage {
            open: std::mem::replace(&mut deferral.escapes_open, open),
            escapes: std::mem::take(&mut deferral.escapes),
        }
    })
}

impl Drop for EscapeStage {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| {
            let mut deferral = deferral.borrow_mut();
            deferral.unverified |= !deferral.escapes.is_empty();
            deferral.escapes_open = self.open;
            deferral.escapes = std::mem::take(&mut self.escapes);
        });
    }
}

pub(crate) fn speculates_escapes() -> bool {
    DEFERRAL.with(|deferral| deferral.borrow().escapes_open)
}

pub(crate) fn defer_escapes(decoders: &[Span]) {
    DEFERRAL.with(|deferral| deferral.borrow_mut().escapes.extend_from_slice(decoders));
}

struct Surviving<'s> {
    spans: &'s [Span],
    found: Vec<Vec<String>>,
    unnamed: bool,
}

impl Surviving<'_> {
    fn note(&mut self, span: Span, name: &str) {
        if self.spans.contains(&span) {
            self.found.push(vec![name.to_string()]);
        }
    }
}

impl<'a> VisitJs<'a> for Surviving<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.note(it.span, it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.note(it.span, it.name.as_str());
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.spans.contains(&it.span) {
            match &it.callee {
                Expression::Identifier(callee) => self.found.push(vec![callee.name.to_string()]),
                _ => self.unnamed = true,
            }
        }
        oxc_ast_visit::walk_js::walk_call_expression(self, it);
    }
}

#[derive(Default)]
struct Names {
    present: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for Names {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.present.insert(it.name.to_string());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.present.insert(it.name.to_string());
    }
}

fn surviving_names(
    (program, scoping): (&Program<'_>, &Scoping),
    escapes: &[Span],
) -> Option<Vec<Vec<String>>> {
    let mut surviving = Surviving {
        spans: escapes,
        found: Vec::new(),
        unnamed: false,
    };
    surviving.visit_program(program);
    for symbol in scoping.symbol_ids() {
        let span = scoping.symbol_span(symbol);
        if !escapes.contains(&span) {
            continue;
        }
        let mut group = vec![scoping.symbol_name(symbol).to_string()];
        if let Some(text) = program
            .source_text
            .get(span.start as usize..span.end as usize)
        {
            group.push(text.to_string());
        }
        surviving.found.push(group);
    }
    (!surviving.unnamed).then_some(surviving.found)
}

pub(crate) fn settle_escapes(program: &Program<'_>, scoping: &Scoping) {
    let (escapes, carried, carries) = DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        (
            std::mem::take(&mut deferral.escapes),
            std::mem::take(&mut deferral.escape_names),
            deferral.carries_names,
        )
    });
    if escapes.is_empty() && carried.is_empty() {
        return;
    }
    let Some(mut pending) = surviving_names((program, scoping), &escapes) else {
        DEFERRAL.with(|deferral| deferral.borrow_mut().unverified = true);
        return;
    };
    pending.extend(carried);
    if pending.is_empty() {
        return;
    }
    let mut names = Names::default();
    names.visit_program(program);
    pending.retain(|group| group.iter().any(|name| names.present.contains(name)));
    if pending.is_empty() {
        return;
    }
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        match carries {
            true => deferral.escape_names = pending,
            false => deferral.unverified = true,
        }
    });
}

pub fn round_opened() {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        deferral.escape_names_opened = deferral.escape_names.clone();
    });
}

pub fn round_discarded() {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        deferral.escape_names = std::mem::take(&mut deferral.escape_names_opened);
    });
}

pub struct NameCarry(bool);

impl NameCarry {
    pub fn enter(on: bool) -> Self {
        Self(
            DEFERRAL
                .with(|deferral| std::mem::replace(&mut deferral.borrow_mut().carries_names, on)),
        )
    }
}

impl Drop for NameCarry {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| deferral.borrow_mut().carries_names = self.0);
    }
}

pub(crate) fn defers() -> bool {
    DEFERRAL.with(|deferral| deferral.borrow().open)
}

pub(crate) fn defer(name: String) {
    if deferred_legacy_reach_trusted() {
        return;
    }
    DEFERRAL.with(|deferral| deferral.borrow_mut().pending.push(name));
}

pub(crate) fn defer_layered(token: String, layered: Layered) {
    defer(token.clone());
    DEFERRAL.with(|deferral| deferral.borrow_mut().layered.push((token, layered)));
}

pub(crate) fn defer_names(token: String, names: Vec<String>) {
    let held = DEFERRAL.with(|deferral| {
        deferral
            .borrow()
            .layered
            .iter()
            .any(|(pending, _)| *pending == token)
    });
    if held {
        return;
    }
    let layered = Layered {
        names,
        ..Layered::default()
    };
    defer_layered(token, layered);
}

fn widen_newest(token: &str, widen: impl FnOnce(&mut Layered)) {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        if let Some((_, layered)) = deferral
            .layered
            .iter_mut()
            .rev()
            .find(|(pending, _)| pending == token)
        {
            widen(layered);
        }
    });
}

fn merge<T: PartialEq + Clone>(held: &mut Option<Vec<T>>, more: &[T]) {
    let held = held.get_or_insert_with(Vec::new);
    for item in more {
        if !held.contains(item) {
            held.push(item.clone());
        }
    }
}

pub(crate) fn widen_layered(token: &str, used: Vec<String>, points_to: bool) {
    widen_newest(token, |layered| {
        match layered.used.is_some() {
            true => layered.both_sites |= layered.points_to != points_to,
            false => layered.points_to = points_to,
        }
        merge(&mut layered.used, &used);
    });
}

pub(crate) fn drop_layered(token: &str) {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        if let Some(last) = deferral
            .pending
            .iter()
            .rposition(|pending| pending == token)
        {
            deferral.pending.remove(last);
        }
        if let Some(last) = deferral
            .layered
            .iter()
            .rposition(|(pending, _)| pending == token)
        {
            deferral.layered.remove(last);
        }
    });
}

pub(crate) fn widen_late(token: &str, reads: &[untwist_eval::object::ProtoRead]) {
    widen_newest(token, |layered| merge(&mut layered.late_reads, reads));
}

pub(crate) fn widen_root(token: &str, root: &str) {
    widen_newest(token, |layered| {
        if !layered.roots.iter().any(|held| held == root) {
            layered.roots.push(root.to_string());
        }
    });
}

pub(crate) fn waive_observation(names: &[&str]) -> bool {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        let pending = |name: &&str| {
            deferral
                .layered
                .iter()
                .any(|(_, layered)| layered.names.iter().any(|held| held == name))
        };
        if !deferral.open || names.is_empty() || !names.iter().all(pending) {
            return false;
        }
        let names: Vec<String> = names.iter().map(|name| (*name).to_string()).collect();
        if !deferral.observations.contains(&names) {
            deferral.observations.push(names);
        }
        true
    })
}

pub(crate) fn observe_at_stage_end(names: &[&str]) -> bool {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        if !deferral.open || names.is_empty() {
            return false;
        }
        let names: Vec<String> = names.iter().map(|name| (*name).to_string()).collect();
        if !deferral.observations.contains(&names) {
            deferral.observations.push(names);
        }
        true
    })
}

pub fn settle_observations(mut observed: impl FnMut(&[&str]) -> bool) {
    let observations =
        DEFERRAL.with(|deferral| std::mem::take(&mut deferral.borrow_mut().observations));
    let failed = observations.iter().any(|names| {
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        observed(&names)
    });
    if failed {
        DEFERRAL.with(|deferral| {
            let mut deferral = deferral.borrow_mut();
            deferral.unverified = true;
            deferral.reached = true;
        });
    }
}

pub(crate) fn settle_layered(mut holds: impl FnMut(&Layered) -> bool) {
    let layered = DEFERRAL.with(|deferral| std::mem::take(&mut deferral.borrow_mut().layered));
    for (token, obligation) in layered {
        if holds(&obligation) {
            verify_one(&token);
        } else {
            DEFERRAL.with(|deferral| deferral.borrow_mut().reached = true);
        }
    }
}

pub(crate) fn verify(name: &str) {
    DEFERRAL.with(|deferral| {
        deferral
            .borrow_mut()
            .pending
            .retain(|pending| pending != name)
    });
}

fn verify_one(name: &str) {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        if let Some(last) = deferral.pending.iter().rposition(|pending| pending == name) {
            deferral.pending.remove(last);
        }
    });
}

pub fn unverified() -> bool {
    DEFERRAL.with(|deferral| {
        let deferral = deferral.borrow();
        deferral.unverified
            || !deferral.observations.is_empty()
            || !deferral.escape_names.is_empty()
    })
}

pub fn reach_failed() -> bool {
    DEFERRAL.with(|deferral| deferral.borrow().reached)
}

pub(crate) fn forget() {
    DEFERRAL.with(|deferral| {
        let mut deferral = deferral.borrow_mut();
        deferral.unverified = false;
        deferral.reached = false;
        deferral.pending.clear();
        deferral.layered.clear();
        deferral.observations.clear();
        deferral.escapes.clear();
        deferral.escape_names.clear();
        deferral.escape_names_opened.clear();
    });
}

pub struct Forbidden(bool);

impl Forbidden {
    pub fn enter() -> Self {
        Self(
            DEFERRAL.with(|deferral| std::mem::replace(&mut deferral.borrow_mut().forbidden, true)),
        )
    }
}

impl Drop for Forbidden {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| deferral.borrow_mut().forbidden = self.0);
    }
}

pub struct Rerunnable(bool);

impl Rerunnable {
    pub fn enter() -> Self {
        Self(
            DEFERRAL
                .with(|deferral| std::mem::replace(&mut deferral.borrow_mut().rerunnable, true)),
        )
    }
}

impl Drop for Rerunnable {
    fn drop(&mut self) {
        DEFERRAL.with(|deferral| deferral.borrow_mut().rerunnable = self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(name: &str) -> Layered {
        Layered {
            names: vec![name.to_string()],
            ..Layered::default()
        }
    }

    fn counts() -> (usize, usize) {
        DEFERRAL.with(|deferral| {
            let deferral = deferral.borrow();
            (deferral.pending.len(), deferral.layered.len())
        })
    }

    #[test]
    fn dropping_a_later_speculation_keeps_the_obligation_an_earlier_round_kept() {
        forget();
        defer_layered("layered:a".to_string(), held("a"));
        defer_layered("layered:a".to_string(), held("a"));
        drop_layered("layered:a");
        let kept = counts();
        drop_layered("layered:a");
        let dropped = counts();
        forget();
        assert_eq!(kept, (1, 1));
        assert_eq!(dropped, (0, 0));
    }

    fn pending() -> usize {
        DEFERRAL.with(|deferral| deferral.borrow().pending.len())
    }

    fn widened() -> Vec<Layered> {
        DEFERRAL.with(|deferral| {
            deferral
                .borrow()
                .layered
                .iter()
                .map(|(_, layered)| layered.clone())
                .collect()
        })
    }

    #[test]
    fn widening_a_later_speculation_leaves_the_earlier_obligation_as_it_was() {
        forget();
        defer_layered("layered:a".to_string(), held("a"));
        widen_layered("layered:a", vec!["x".to_string()], false);
        defer_layered("layered:a".to_string(), held("a"));
        widen_layered("layered:a", vec!["y".to_string()], true);
        widen_root("layered:a", "Object");
        let entries = widened();
        forget();
        assert_eq!(entries[0].used, Some(vec!["x".to_string()]));
        assert!(!entries[0].points_to && entries[0].roots.is_empty());
        assert_eq!(entries[1].used, Some(vec!["y".to_string()]));
        assert!(entries[1].points_to);
    }

    #[test]
    fn widening_one_obligation_twice_keeps_both_checks() {
        forget();
        defer_layered("layered:a".to_string(), held("a"));
        widen_layered("layered:a", vec!["x".to_string()], false);
        widen_layered("layered:a", vec!["y".to_string()], true);
        let entries = widened();
        forget();
        assert_eq!(
            entries[0].used,
            Some(vec!["x".to_string(), "y".to_string()])
        );
        assert!(entries[0].both_sites);
    }

    #[test]
    fn one_holding_obligation_verifies_only_its_own_pending_entry() {
        forget();
        defer_layered("layered:a".to_string(), held("a"));
        defer_layered("layered:a".to_string(), held("b"));
        settle_layered(|layered| layered.names == ["a"]);
        let left = pending();
        forget();
        assert_eq!(left, 1);
    }
}
