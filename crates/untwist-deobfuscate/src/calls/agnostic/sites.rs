use hashbrown::hash_table::HashTable;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::hash::BuildHasher;

use super::flow::Flow;
use super::fresh::Fresh;
use super::roles::*;
use super::source::SOURCE_KEYS;
use super::{root_folds, AKey, ASite, Agnostic};
use crate::proof::{prototype_root_ref, ProtoRoot, GLOBAL_OBJECTS};
use untwist_core::semantic::{observers, Observers};
use untwist_eval::value::JsString;

fn site_hash(site: &ASite<'_>) -> u64 {
    FxBuildHasher.hash_one((&site.key, site.root, site.identity, site.outside))
}

fn same_site(left: &ASite<'_>, right: &ASite<'_>) -> bool {
    left.root == right.root
        && left.identity == right.identity
        && left.outside == right.outside
        && left.key == right.key
}

pub(super) fn span_key(span: Span) -> u64 {
    (u64::from(span.start) << 32) | u64::from(span.end)
}

pub(super) fn literal_root(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
        Expression::TemplateLiteral(t) => t.expressions.is_empty(),
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
            element.as_expression().is_some_and(|inner| {
                matches!(
                    inner.without_parentheses(),
                    Expression::StringLiteral(_) | Expression::NumericLiteral(_)
                )
            })
        }),
        _ => false,
    }
}

pub(super) fn plain_argument(argument: &Argument<'_>) -> bool {
    argument
        .as_expression()
        .is_some_and(|inner| match inner.without_parentheses() {
            Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
            Expression::TemplateLiteral(t) => t.expressions.is_empty(),
            _ => false,
        })
}

pub(super) const SPREAD_WIDTH: usize = 4;

pub(super) fn arg_elems<'k, 'a>(arguments: &'k [Argument<'a>]) -> Vec<Elem<'k, 'a>> {
    let mut elems = Vec::with_capacity(arguments.len());
    for argument in arguments {
        match argument.as_expression() {
            Some(expr) => elems.push(Elem::Value(expr)),
            None => {
                elems.extend([Elem::Hidden; SPREAD_WIDTH]);
                break;
            }
        }
    }
    elems
}

pub(super) fn expr_member<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k MemberExpression<'a>> {
    expr.without_parentheses().as_member_expression()
}

#[derive(Clone, Copy, Default)]
pub(super) struct Forward {
    pub(super) call: bool,
    pub(super) apply: bool,
    pub(super) reflect: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Elem<'k, 'a> {
    Value(&'k Expression<'a>),
    Hidden,
}

pub(super) struct Sites<'s, 'a> {
    pub(super) fresh: Fresh<'s, 'a>,
    pub(super) resolve: Option<&'s FxHashMap<Span, &'s JsString>>,
    pub(super) machinery: &'s [Span],
    pub(super) watch: super::settle::Watch<'s>,
    pub(super) out: Agnostic<'a>,
    site_index: HashTable<u32>,
    pub(super) direct: FxHashSet<u64>,
    pub(super) tests: FxHashSet<u64>,
    pub(super) calls: Vec<&'a CallExpression<'a>>,
    pub(super) news: Vec<&'a NewExpression<'a>>,
    pub(super) tagged: Vec<&'a TaggedTemplateExpression<'a>>,
    pub(super) identity: Option<u64>,
    pub(super) assumed: RefCell<Vec<Cow<'a, str>>>,
    pub(super) defined: Vec<AKey<'a>>,
    pub(super) fresh_calls: bool,
    pub(super) fresh_used: Cell<bool>,
    pub(super) bind_unbounded: bool,
    pub(super) apply_unbounded: bool,
    pub(super) subst: RefCell<Vec<(oxc_syntax::symbol::SymbolId, &'a Expression<'a>)>>,
    pub(super) creation_only: FxHashSet<u64>,
    pub(super) creation_keys: Vec<AKey<'a>>,
    pub(super) refute_fresh: bool,
    pub(super) saturated: bool,
    pub(super) inlining: Vec<oxc_syntax::symbol::SymbolId>,
    pub(super) walked_unbound: FxHashSet<oxc_syntax::symbol::SymbolId>,
    pub(super) inline_budget: usize,
    pub(super) symbol_safe: bool,
    pub(super) param_values: RefCell<FxHashMap<oxc_syntax::symbol::SymbolId, bool>>,
    pub(super) param_keys: RefCell<FxHashMap<oxc_syntax::symbol::SymbolId, bool>>,
    pub(super) text_judged: bool,
    pub(super) forward_calls: FxHashSet<Span>,
    pub(super) pending_texts: RefCell<Vec<String>>,
    pub(super) flow: &'s Flow<'s, 'a>,
    pub(super) known_rests: FxHashMap<Span, Vec<AKey<'a>>>,
}

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn new(
        fresh: Fresh<'s, 'a>,
        (resolve, machinery, watch): super::settle::Inputs<'s>,
        fresh_calls: bool,
        symbol_safe: bool,
        flow: &'s Flow<'s, 'a>,
    ) -> Self {
        Self {
            fresh,
            resolve,
            machinery,
            watch,
            out: Agnostic::default(),
            site_index: HashTable::new(),
            direct: FxHashSet::default(),
            tests: FxHashSet::default(),
            calls: Vec::new(),
            news: Vec::new(),
            tagged: Vec::new(),
            identity: None,
            assumed: RefCell::default(),
            defined: Vec::new(),
            fresh_calls,
            fresh_used: Cell::new(false),
            bind_unbounded: false,
            apply_unbounded: false,
            subst: RefCell::default(),
            creation_only: FxHashSet::default(),
            creation_keys: Vec::new(),
            refute_fresh: false,
            saturated: false,
            inlining: Vec::new(),
            walked_unbound: FxHashSet::default(),
            inline_budget: 0,
            symbol_safe,
            param_values: RefCell::default(),
            param_keys: RefCell::default(),
            text_judged: false,
            forward_calls: FxHashSet::default(),
            pending_texts: RefCell::default(),
            flow,
            known_rests: FxHashMap::default(),
        }
    }

    pub(super) fn push(&mut self, span: Span, key: AKey<'a>, kind: &'static str) {
        self.push_rooted(span, key, kind, ProtoRoot::Either);
    }

    pub(super) fn push_rooted(
        &mut self,
        span: Span,
        key: AKey<'a>,
        kind: &'static str,
        root: ProtoRoot,
    ) {
        let source = match &key {
            AKey::Named(name) => SOURCE_KEYS.contains(&name.as_ref()),
            AKey::Index => false,
            AKey::Pending(_) | AKey::Unknown => true,
        };
        if source {
            self.mark_source(span, "a write by a key that may be prepareStackTrace");
        }
        let outside = !self.in_machinery(span);
        if outside {
            let open = matches!(key, AKey::Unknown)
                || (self.resolve.is_none() && matches!(key, AKey::Pending(_)));
            self.saturated |= open && root_folds(root);
        }
        self.record_site(ASite {
            span,
            key,
            identity: kind == IDENTITY,
            root,
            outside,
        });
    }

    fn record_site(&mut self, site: ASite<'a>) {
        let hash = site_hash(&site);
        let sites = &self.out.sites;
        let same = |stored: &u32| {
            sites
                .get(*stored as usize)
                .is_some_and(|seen| same_site(seen, &site))
        };
        if self.site_index.find(hash, same).is_some() {
            return;
        }
        let ordinal = u32::try_from(sites.len()).ok();
        self.out.sites.push(site);
        if let Some(ordinal) = ordinal {
            let sites = &self.out.sites;
            self.site_index.insert_unique(hash, ordinal, |stored| {
                sites.get(*stored as usize).map_or(0, site_hash)
            });
        }
    }

    pub(super) fn target_root(&self, obj: &Expression<'a>) -> ProtoRoot {
        match prototype_root_ref(obj.without_parentheses()) {
            Some((id, root)) if self.fresh.symbol(id).is_none() => root,
            _ => ProtoRoot::Either,
        }
    }

    pub(super) fn is_global_object(&self, expr: &Expression<'a>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if GLOBAL_OBJECTS.contains(&id.name.as_str()) && self.fresh.symbol(id).is_none())
    }

    pub(super) fn note_rebind(&mut self, target: &Expression<'a>) {
        if self.is_global_object(target) {
            self.out.realm_rebound = true;
        }
    }

    pub(super) fn in_machinery(&self, span: Span) -> bool {
        self.machinery
            .iter()
            .any(|outer| outer.start <= span.start && span.end <= outer.end)
    }

    pub(super) fn mark_origin(&mut self, origin: Origin, span: Span, reason: &'static str) {
        self.mark_origin_sink(origin, span, reason, true);
    }

    pub(super) fn mark_origin_sink(
        &mut self,
        origin: Origin,
        span: Span,
        reason: &'static str,
        has_sink: bool,
    ) {
        if origin.runner {
            self.mark_source(span, "code from text");
        }
        if self.in_machinery(span) {
            return;
        }
        #[cfg(feature = "refusal-spans")]
        self.trail(span, || format!("{reason}{}", roles_of(origin)));
        if has_sink {
            if origin.definer && self.out.unbounded.is_none() {
                self.out.unbounded = Some((span, reason));
            }
            if origin.runner && self.out.runner_unbounded.is_none() {
                self.out.runner_unbounded = Some((span, reason));
            }
        }
        self.bind_unbounded |= origin.bind;
        self.apply_unbounded |= origin.apply;
        self.refute_fresh |= origin.construct;
    }

    pub(super) fn mark_text_code(&mut self, span: Span) {
        let judged = self.text_judged;
        self.text_judged |= observers() == Observers::None;
        self.mark_code(span);
        self.text_judged = judged;
    }

    pub(super) fn mark_code(&mut self, span: Span) {
        if self.in_machinery(span) {
            return;
        }
        #[cfg(feature = "refusal-spans")]
        self.trail(span, || "code from text".to_string());
        if self.out.code_text.is_none() {
            self.out.code_text = Some(span);
        }
        if !self.text_judged {
            self.out.reach_code.get_or_insert(span);
        }
        self.saturated = true;
    }

    pub(super) fn mark_source(&mut self, span: Span, what: &'static str) {
        #[cfg(feature = "refusal-spans")]
        if !self.out.global_source {
            self.out.source_at = Some((span, what));
        }
        #[cfg(not(feature = "refusal-spans"))]
        let _ = (span, what);
        self.out.global_source = true;
    }

    #[cfg(feature = "refusal-spans")]
    pub(super) fn trail(&mut self, span: Span, what: impl FnOnce() -> String) {
        if crate::refusal_spans::active() && !self.in_machinery(span) {
            self.out
                .trail
                .push(crate::refusal_spans::RefusalSpan { what: what(), span });
        }
    }

    pub(super) fn walks_on(&self) -> bool {
        !self.saturated || !self.watch.names.is_empty()
    }

    pub(super) fn mark_test(&mut self, expr: &Expression<'a>) {
        let inner = expr.without_parentheses();
        self.tests.insert(span_key(inner.span()));
        match inner {
            Expression::LogicalExpression(logical) => {
                self.mark_test(&logical.left);
                self.mark_test(&logical.right);
            }
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::LogicalNot | UnaryOperator::Typeof
                ) =>
            {
                self.mark_test(&unary.argument);
            }
            Expression::BinaryExpression(binary)
                if binary.operator.is_equality() || binary.operator.is_compare() =>
            {
                self.mark_test(&binary.left);
                self.mark_test(&binary.right);
            }
            _ => {}
        }
    }
}

#[cfg(feature = "refusal-spans")]
fn roles_of(origin: Origin) -> String {
    let roles = [
        (origin.definer, "definer"),
        (origin.runner, "runner"),
        (origin.bind, "bind"),
        (origin.apply, "apply"),
        (origin.construct, "construct"),
    ];
    let named: Vec<&str> = roles
        .iter()
        .filter(|(held, _)| *held)
        .map(|(_, name)| *name)
        .collect();
    format!(" ({})", named.join(" "))
}
