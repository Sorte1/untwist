use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::ast_util::prop::{literal_member_parts, prop_name};
use untwist_core::ast_util::readonly::{is_readonly_object, ObjectUses, PropRead};
use untwist_core::ast_util::respan::fresh_spans_expression;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Pass};
use untwist_core::semantic::scope_hazard;

use crate::bindings::unexposed_globals;
use crate::init_order::{
    lend_order_and_readonly_uses, pass_done, reads_are_initialised, refuse_read_before_declaration,
    InitOrder, LentOrder,
};
use crate::notes;

mod keyed;
mod spans;

pub const PASS: &str = "inline-object-props";

pub const KEYED_RULE: &str = "inline-object-props/keyed";

const PROTO_KEY: &str = "__proto__";

pub fn ambiguous_key(expr: &Expression<'_>) -> Option<String> {
    let Expression::ObjectExpression(object) = expr else {
        return None;
    };
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let Some(name) = prop_name(&property.key) else {
            continue;
        };
        let name = name.into_owned();
        if name == PROTO_KEY || !seen.insert(name.clone()) {
            return Some(name);
        }
    }
    None
}

fn is_word_key(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub fn literal_props<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<Vec<(String, &'k Expression<'a>)>> {
    let Expression::ObjectExpression(object) = expr else {
        return None;
    };
    if ambiguous_key(expr).is_some() {
        return None;
    }
    let mut props = Vec::with_capacity(object.properties.len());
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.method || property.kind != PropertyKind::Init {
            return None;
        }
        let name = prop_name(&property.key)?;
        if !is_word_key(&name) {
            return None;
        }
        if !matches!(
            property.value,
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        ) {
            return None;
        }
        props.push((name.into_owned(), &property.value));
    }
    if props.is_empty() {
        return None;
    }
    Some(props)
}

struct Declared {
    statement: Span,
    symbol: SymbolId,
    value_spans: Vec<(String, Span)>,
}

#[derive(Default)]
struct Search {
    found: Vec<Declared>,
    refused: Vec<(Span, String)>,
}

fn declared_initialiser<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    declaration.declarations.first()?.init.as_ref()
}

impl<'a> VisitJs<'a> for Search {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(declared) = declared_object(it) {
            self.found.push(declared);
        } else if let Some(name) = declared_initialiser(it).and_then(ambiguous_key) {
            self.refused.push((it.span(), name));
        }
        walk_js::walk_statement(self, it);
    }
}

fn declared_object(stmt: &Statement<'_>) -> Option<Declared> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let props = literal_props(declarator.init.as_ref()?)?;
    Some(Declared {
        statement: stmt.span(),
        symbol: id.symbol_id.get()?,
        value_spans: props
            .into_iter()
            .map(|(name, value)| (name, value.span()))
            .collect(),
    })
}

fn undefined_property<'u>(candidate: &Declared, reads: &'u [PropRead]) -> Option<&'u PropRead> {
    reads.iter().find(|read| {
        !candidate
            .value_spans
            .iter()
            .any(|(name, _)| *name == read.name)
    })
}

fn value_span(candidate: &Declared, name: &str) -> Option<Span> {
    candidate
        .value_spans
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, span)| *span)
}

type Values<'p> = FxHashMap<Span, &'p Expression<'p>>;

fn substitute_reads<'a>(
    values: &Values<'_>,
    candidate: &Declared,
    reads: &[PropRead],
    cx: &mut Cx<'a, '_>,
    edits: &mut spans::Edits<'a>,
) -> usize {
    let mut substituted = 0usize;
    for read in reads {
        let value = value_span(candidate, &read.name).and_then(|span| values.get(&span));
        let Some(value) = value else {
            cx.note(
                notes::object_prop_value_missing(&read.name),
                Some(read.member_span),
            );
            continue;
        };
        if edits.insert(read.member_span, value.clone_in(cx.allocator())) {
            substituted += 1;
        }
    }
    substituted
}

struct Reads<'r> {
    reads: &'r [PropRead],
    unknown: u32,
}

fn admits(
    candidate: &Declared,
    reads: &[PropRead],
    order: &InitOrder,
    cx: &mut Cx<'_, '_>,
) -> bool {
    if let Some(read) = undefined_property(candidate, reads) {
        cx.note(
            notes::object_missing_prop(&read.name),
            Some(read.member_span),
        );
        return false;
    }
    if let Err(refused) = reads_are_initialised(reads, candidate.statement, order) {
        refuse_read_before_declaration(
            refused,
            candidate.symbol,
            candidate.statement,
            notes::OBJECT_READ_BEFORE_DECLARATION,
            cx,
        );
        return false;
    }
    true
}

fn inline_declared_object<'a>(
    values: &Values<'_>,
    candidate: &Declared,
    Reads { reads, unknown }: Reads<'_>,
    cx: &mut Cx<'a, '_>,
    edits: &mut spans::Edits<'a>,
    removals: &mut StmtEdits<'a>,
) {
    let substituted = substitute_reads(values, candidate, reads, cx, edits);
    if substituted == reads.len() {
        if unknown == 0 {
            removals.remove(candidate.statement);
        }
        return;
    }
    cx.note(
        notes::object_props_partial(substituted, reads.len()),
        Some(candidate.statement),
    );
}

type UsesBySymbol = FxHashMap<SymbolId, ObjectUses>;

#[cfg(debug_assertions)]
fn record_strict(order: &InitOrder, uses: &UsesBySymbol) {
    let strict: UsesBySymbol = uses
        .iter()
        .filter(|(_, uses)| is_readonly_object(uses))
        .map(|(symbol, uses)| (*symbol, uses.clone()))
        .collect();
    crate::fused::check::record_order_and_uses("object-props", order, &strict);
}

fn order_and_uses(
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
    symbols: &FxHashSet<SymbolId>,
    wants_keyed: bool,
) -> (LentOrder, UsesBySymbol) {
    let shares = cx.opts.shares_init_order;
    let (order, mut uses) = lend_order_and_readonly_uses(shares, program, cx, symbols);
    #[cfg(debug_assertions)]
    record_strict(&order, &uses);
    if uses.values().all(is_readonly_object) {
        return (order, uses);
    }
    if wants_keyed {
        keyed::resolve_keyed(
            program,
            cx.scoping(),
            &mut uses,
            &order,
            cx.opts.unsafe_transforms,
        );
    } else {
        uses.retain(|_, uses| is_readonly_object(uses));
    }
    uses.shrink_to_fit();
    (order, uses)
}

fn candidate_reads(uses: &ObjectUses) -> Reads<'_> {
    let unknown = if is_readonly_object(uses) {
        0
    } else {
        uses.keyed
    };
    Reads {
        reads: &uses.reads,
        unknown,
    }
}

#[derive(Default)]
pub struct InlineObjectProps {
    superset_only: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineObjectProps::default())
}

pub fn boxed_superset<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineObjectProps {
        superset_only: true,
    })
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::StaticMemberExpression,
    ExpressionKind::ComputedMemberExpression,
]);

impl<'a> Pass<'a> for InlineObjectProps {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM | Interest::EXIT_EXPRESSION).on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn requires_extended(&self) -> bool {
        self.superset_only
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if scope_hazard(cx.scoping()).is_some() {
            return;
        }
        let search = {
            let mut search = Search::default();
            search.visit_program(program);
            search
        };
        for (span, name) in &search.refused {
            cx.note(notes::object_ambiguous_key(name), Some(*span));
        }
        let mut candidates = unexposed_globals(
            search.found,
            |declared| declared.symbol,
            program,
            cx.scoping(),
        );
        if candidates.is_empty() {
            return;
        }
        let symbols: FxHashSet<SymbolId> =
            candidates.iter().map(|declared| declared.symbol).collect();
        let shares = cx.opts.shares_init_order;
        let wants_keyed = self.superset_only && !cx.opts.rule_disabled(KEYED_RULE);
        let (order, uses) = order_and_uses(program, cx, &symbols, wants_keyed);
        candidates.retain(|declared| uses.contains_key(&declared.symbol));
        crate::proof::refuse_annex_b_bindings(
            &mut candidates,
            |declared| (declared.statement, [declared.symbol]),
            program,
            cx,
        );
        let mut admitted = Vec::new();
        for candidate in &candidates {
            let Some(reads) = uses.get(&candidate.symbol).map(candidate_reads) else {
                continue;
            };
            if admits(candidate, reads.reads, &order, cx) {
                admitted.push((candidate, reads));
            }
        }
        let values = spans::expressions_at(
            program,
            admitted.iter().flat_map(|(candidate, reads)| {
                reads
                    .reads
                    .iter()
                    .filter_map(|read| value_span(candidate, &read.name))
            }),
        );
        let mut edits = spans::Edits::default();
        let mut removals = StmtEdits::new();
        for (candidate, reads) in admitted {
            inline_declared_object(&values, candidate, reads, cx, &mut edits, &mut removals);
        }
        if !self.superset_only {
            order.give_back_any(program, cx);
        }
        let wanted = edits.len() + removals.len();
        let applied = edits.apply(program, cx.allocator())
            + removals.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
        pass_done(shares, applied > 0);
        if applied != wanted {
            cx.note(
                notes::planned_and_applied_edits(PASS, wanted, applied),
                None,
            );
        }
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let mut replacement = {
            let Some((object, name)) = literal_member_parts(expr) else {
                return;
            };
            if let Some(duplicate) = ambiguous_key(object) {
                cx.note(notes::object_ambiguous_key(&duplicate), Some(object.span()));
                return;
            }
            let Some(props) = literal_props(object) else {
                return;
            };
            let Some((_, value)) = props.into_iter().find(|(key, _)| *key == name) else {
                return;
            };
            value.clone_in(cx.allocator())
        };
        if !fresh_spans_expression(&mut replacement) {
            cx.note(notes::OBJECT_PROPS_SPANS_EXHAUSTED, Some(expr.span()));
            return;
        }
        *expr = replacement;
        cx.changed();
        pass_done(cx.opts.shares_init_order, true);
    }
}
