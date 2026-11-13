use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, Function, IdentifierReference, Program,
    WithStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

const EVERY_NAME: &str = "eval";

pub(super) struct Watch<'w> {
    pub function: Span,
    pub site: Span,
    pub names: &'w [String],
    pub clashed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hit {
    Within(Option<usize>),
    Spread,
}

#[derive(Default)]
struct Watched<'w> {
    sites: Vec<(Span, usize)>,
    named: FxHashSet<&'w str>,
    hits: FxHashMap<&'w str, Hit>,
}

impl<'w> Watched<'w> {
    fn container(&self, span: Span) -> Option<usize> {
        let after = self
            .sites
            .partition_point(|(site, _)| site.start <= span.start);
        let (site, index) = self.sites.get(after.checked_sub(1)?)?;
        site.contains_inclusive(span).then_some(*index)
    }

    fn note(&mut self, span: Span, name: &str) {
        let Some(&key) = self.named.get(name) else {
            return;
        };
        let within = Hit::Within(self.container(span));
        self.hits
            .entry(key)
            .and_modify(|hit| {
                if *hit != within {
                    *hit = Hit::Spread;
                }
            })
            .or_insert(within);
    }

    fn clashes(&self, index: usize, names: &[String]) -> bool {
        std::iter::once(EVERY_NAME)
            .chain(names.iter().map(String::as_str))
            .filter_map(|name| self.hits.get(name))
            .any(|hit| *hit != Hit::Within(Some(index)))
    }
}

struct Walk<'w> {
    by_function: FxHashMap<Span, Watched<'w>>,
    functions: Vec<Span>,
}

pub(super) fn clashes(program: &Program<'_>, watches: &mut [Watch<'_>]) {
    if watches.is_empty() {
        return;
    }
    let mut by_function: FxHashMap<Span, Watched<'_>> = FxHashMap::default();
    for (index, watch) in watches.iter().enumerate() {
        let watched = by_function.entry(watch.function).or_default();
        watched.sites.push((watch.site, index));
        watched.named.insert(EVERY_NAME);
        watched.named.extend(watch.names.iter().map(String::as_str));
    }
    for watched in by_function.values_mut() {
        watched.sites.sort_unstable_by_key(|(site, _)| site.start);
    }
    let mut walk = Walk {
        by_function,
        functions: Vec::new(),
    };
    walk.visit_program(program);
    for (index, watch) in watches.iter_mut().enumerate() {
        watch.clashed = walk
            .by_function
            .get(&watch.function)
            .is_some_and(|watched| watched.clashes(index, watch.names));
    }
}

impl Walk<'_> {
    fn seen(&mut self, span: Span, name: &str) {
        for function in &self.functions {
            if let Some(watched) = self.by_function.get_mut(function) {
                watched.note(span, name);
            }
        }
    }

    fn enter(&mut self, span: Span) {
        if self.by_function.contains_key(&span) {
            self.functions.push(span);
        }
    }

    fn leave(&mut self, span: Span) {
        if self.functions.last() == Some(&span) {
            self.functions.pop();
        }
    }
}

impl<'a> Visit<'a> for Walk<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.enter(it.span);
        walk::walk_function(self, it, flags);
        self.leave(it.span);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.enter(it.span);
        walk::walk_arrow_function_expression(self, it);
        self.leave(it.span);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.seen(it.span, it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.seen(it.span, it.name.as_str());
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.seen(it.span, EVERY_NAME);
        walk::walk_with_statement(self, it);
    }
}
