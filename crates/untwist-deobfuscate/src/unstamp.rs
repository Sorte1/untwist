use oxc_ast::ast::{IdentifierReference, Program};
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::notes;
use crate::stamp::{
    decoder_name, is_machinery_name, may_name_machinery, original_name, unstamp_names,
};

pub const UNSTAMP_REMAINING_PASS: &str = "unstamp-remaining-machinery";

#[derive(Default)]
pub struct UnstampRemainingMachinery;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UnstampRemainingMachinery)
}

struct Survivor<'s> {
    symbol: SymbolId,
    scope: ScopeId,
    original: &'s str,
}

enum Found<'s> {
    Restorable(Survivor<'s>),
    Unreadable,
}

fn survivor<'s>(source_text: &'s str, scoping: &Scoping, symbol: SymbolId) -> Option<Found<'s>> {
    let name = scoping.symbol_name(symbol);
    if !may_name_machinery(name) || !is_machinery_name(name) {
        return None;
    }
    let Some(original) = original_name(source_text, scoping, symbol) else {
        return Some(Found::Unreadable);
    };
    if original == name {
        return None;
    }
    if !is_identifier_name(original) {
        return Some(Found::Unreadable);
    }
    Some(Found::Restorable(Survivor {
        symbol,
        scope: scoping.symbol_scope_id(symbol),
        original,
    }))
}

fn survivors<'s>(source_text: &'s str, scoping: &Scoping) -> (Vec<Survivor<'s>>, bool) {
    let mut found = Vec::new();
    let mut unreadable = false;
    for symbol in scoping.symbol_ids() {
        match survivor(source_text, scoping, symbol) {
            Some(Found::Restorable(survivor)) => found.push(survivor),
            Some(Found::Unreadable) => unreadable = true,
            None => {}
        }
    }
    (found, unreadable)
}

pub fn has_survivor(source_text: &str, scoping: &Scoping) -> bool {
    let shortest = decoder_name(0).len();
    let candidate = scoping
        .symbol_names()
        .any(|name| name.len() >= shortest && may_name_machinery(name));
    candidate
        && scoping
            .symbol_ids()
            .any(|symbol| survivor(source_text, scoping, symbol).is_some())
}

fn within(scoping: &Scoping, scope: ScopeId, ancestor: ScopeId) -> bool {
    scope == ancestor || scoping.scope_is_descendant_of(scope, ancestor)
}

fn reaches_through(scoping: &Scoping, references: &[ReferenceId], scope: ScopeId) -> bool {
    references
        .iter()
        .any(|reference| within(scoping, scoping.get_reference(*reference).scope_id(), scope))
}

fn is_input_binding(source_text: &str, scoping: &Scoping, symbol: SymbolId) -> bool {
    original_name(source_text, scoping, symbol) == Some(scoping.symbol_name(symbol))
}

struct ReferenceSpans<'n> {
    names: Vec<&'n str>,
    spans: FxHashMap<ReferenceId, Span>,
}

impl<'a> VisitJs<'a> for ReferenceSpans<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.names.contains(&it.name.as_str()) {
            if let Some(reference) = it.reference_id.get() {
                self.spans.insert(reference, it.span);
            }
        }
    }
}

fn reference_spans(
    program: &Program<'_>,
    scoping: &Scoping,
    found: &[Survivor<'_>],
) -> FxHashMap<ReferenceId, Span> {
    let names = found
        .iter()
        .map(|survivor| scoping.symbol_name(survivor.symbol))
        .collect();
    let mut walk = ReferenceSpans {
        names,
        spans: FxHashMap::default(),
    };
    walk.visit_program(program);
    walk.spans
}

struct Context<'c> {
    source_text: &'c str,
    scoping: &'c Scoping,
    spans: &'c FxHashMap<ReferenceId, Span>,
}

impl Context<'_> {
    fn spelled_in_input(&self, reference: ReferenceId, name: &str) -> bool {
        self.spans.get(&reference).is_some_and(|span| {
            self.source_text
                .get(span.start as usize..span.end as usize)
                .is_some_and(|text| text == name)
        })
    }
}

fn captures_own_reference(cx: &Context<'_>, survivor: &Survivor<'_>) -> bool {
    let scoping = cx.scoping;
    let name = Ident::from(survivor.original);
    scoping
        .get_resolved_reference_ids(survivor.symbol)
        .iter()
        .any(|reference| {
            let from = scoping.get_reference(*reference).scope_id();
            scoping.find_binding(from, name).is_some_and(|other| {
                scoping.scope_is_descendant_of(scoping.symbol_scope_id(other), survivor.scope)
                    && !(is_input_binding(cx.source_text, scoping, other)
                        && cx.spelled_in_input(*reference, survivor.original))
            })
        })
}

fn rebinds_in_scope(source_text: &str, scoping: &Scoping, survivor: &Survivor<'_>) -> bool {
    scoping
        .get_binding(survivor.scope, Ident::from(survivor.original))
        .is_some_and(|other| !is_input_binding(source_text, scoping, other))
}

fn reaches_a_free_reference(scoping: &Scoping, survivor: &Survivor<'_>) -> bool {
    scoping
        .root_unresolved_references()
        .get(survivor.original)
        .is_some_and(|references| reaches_through(scoping, references, survivor.scope))
}

fn captures_foreign_reference(scoping: &Scoping, survivor: &Survivor<'_>) -> bool {
    scoping.symbol_ids().any(|other| {
        other != survivor.symbol
            && scoping.symbol_name(other) == survivor.original
            && scoping.scope_is_descendant_of(survivor.scope, scoping.symbol_scope_id(other))
            && reaches_through(
                scoping,
                scoping.get_resolved_reference_ids(other),
                survivor.scope,
            )
    })
}

fn shares_a_scope_chain(scoping: &Scoping, survivor: &Survivor<'_>, all: &[Survivor<'_>]) -> bool {
    all.iter().any(|other| {
        other.symbol != survivor.symbol
            && other.original == survivor.original
            && (within(scoping, other.scope, survivor.scope)
                || within(scoping, survivor.scope, other.scope))
    })
}

fn block_reason(
    cx: &Context<'_>,
    survivor: &Survivor<'_>,
    all: &[Survivor<'_>],
) -> Option<&'static str> {
    let scoping = cx.scoping;
    if reaches_a_free_reference(scoping, survivor) {
        return Some(notes::UNSTAMP_FREE_REFERENCE);
    }
    let bound = rebinds_in_scope(cx.source_text, scoping, survivor)
        || captures_own_reference(cx, survivor)
        || captures_foreign_reference(scoping, survivor)
        || shares_a_scope_chain(scoping, survivor, all);
    bound.then_some(notes::UNSTAMP_BLOCKED)
}

type Decision = (Vec<(SymbolId, String)>, Vec<&'static str>);

fn decide(program: &Program<'_>, scoping: &Scoping) -> Decision {
    let source_text = program.source_text;
    let (found, unreadable) = survivors(source_text, scoping);
    let spans = reference_spans(program, scoping, &found);
    let context = Context {
        source_text,
        scoping,
        spans: &spans,
    };
    let mut restored = Vec::with_capacity(found.len());
    let mut reasons = Vec::new();
    if unreadable {
        reasons.push(notes::MACHINERY_NAME_UNREADABLE);
    }
    for survivor in &found {
        match block_reason(&context, survivor, &found) {
            Some(reason) if !reasons.contains(&reason) => reasons.push(reason),
            Some(_) => {}
            None => restored.push((survivor.symbol, survivor.original.to_string())),
        }
    }
    (restored, reasons)
}

impl<'a> Pass<'a> for UnstampRemainingMachinery {
    fn name(&self) -> &'static str {
        UNSTAMP_REMAINING_PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let (restored, reasons) = decide(program, cx.scoping());
        for reason in reasons {
            cx.note(reason, None);
        }
        if !restored.is_empty() {
            unstamp_names(program, cx, &restored);
        }
    }
}
