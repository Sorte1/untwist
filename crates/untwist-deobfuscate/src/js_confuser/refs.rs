use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::fold::{const_from_literal, Const};

#[derive(Clone, Copy)]
pub(super) struct Ref {
    pub(super) span: Span,
    pub(super) symbol: Option<SymbolId>,
    pub(super) write: bool,
}

pub(super) struct Call {
    pub(super) span: Span,
    pub(super) args: Vec<Option<Const>>,
    pub(super) spread: bool,
}

#[derive(Default)]
pub(super) struct Refs<'p> {
    pub(super) all: Vec<(Ref, &'p str)>,
    pub(super) by_symbol: FxHashMap<SymbolId, Vec<Ref>>,
    pub(super) calls: FxHashMap<SymbolId, Vec<Call>>,
    pub(super) in_tests: FxHashMap<SymbolId, Vec<(Span, String)>>,
    pub(super) index_reads: FxHashMap<SymbolId, Vec<(Span, f64)>>,
    pub(super) has_with: bool,
    excluded: FxHashSet<Span>,
}

impl Refs<'_> {
    pub(super) fn of(&self, symbol: SymbolId) -> &[Ref] {
        self.by_symbol.get(&symbol).map_or(&[], Vec::as_slice)
    }

    pub(super) fn calls_of(&self, symbol: SymbolId) -> &[Call] {
        self.calls.get(&symbol).map_or(&[], Vec::as_slice)
    }

    pub(super) fn stable(&self, symbol: SymbolId, scoping: &Scoping) -> bool {
        scoping.symbol_redeclarations(symbol).is_empty() && self.of(symbol).iter().all(|r| !r.write)
    }
}

pub(super) fn literal_value(expr: &Expression<'_>) -> Option<Const> {
    match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
            match const_from_literal(&unary.argument)? {
                Const::Num(n) => Some(Const::Num(-n)),
                _ => None,
            }
        }
        other => const_from_literal(other),
    }
}

fn symbol_of(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

pub(super) struct RefScan<'s, 'p> {
    scoping: &'s Scoping,
    refs: Refs<'p>,
}

pub(super) fn scan<'p>(program: &Program<'p>, scoping: &Scoping) -> Refs<'p> {
    let mut scan = RefScan {
        scoping,
        refs: Refs::default(),
    };
    scan.visit_program(program);
    let mut refs = scan.refs;
    let excluded = std::mem::take(&mut refs.excluded);
    for reads in refs.index_reads.values_mut() {
        reads.retain(|(span, _)| !excluded.contains(span));
    }
    refs
}

impl<'p> VisitJs<'p> for RefScan<'_, 'p> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'p>) {
        let Some(id) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(id);
        let found = Ref {
            span: it.span,
            symbol: reference.symbol_id(),
            write: reference.is_write(),
        };
        self.refs.all.push((found, it.name.as_str()));
        if let Some(symbol) = found.symbol {
            self.refs.by_symbol.entry(symbol).or_default().push(found);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'p>) {
        self.refs.excluded.insert(it.callee.span());
        if let Some(symbol) = symbol_of(&it.callee, self.scoping) {
            let spread = it
                .arguments
                .iter()
                .any(|a| matches!(a, Argument::SpreadElement(_)));
            let args = it
                .arguments
                .iter()
                .map(|a| a.as_expression().and_then(literal_value))
                .collect();
            self.refs.calls.entry(symbol).or_default().push(Call {
                span: it.span,
                args,
                spread,
            });
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'p>) {
        self.refs.excluded.insert(it.callee.span());
        walk_js::walk_new_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'p>) {
        self.refs.excluded.insert(it.tag.span());
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'p>) {
        if it.operator == UnaryOperator::Delete {
            self.refs.excluded.insert(it.argument.span());
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'p>) {
        match it {
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::In => {
                if let (Expression::StringLiteral(key), Some(symbol)) =
                    (&binary.left, symbol_of(&binary.right, self.scoping))
                {
                    if let Some(key) = key.value.as_str() {
                        self.refs
                            .in_tests
                            .entry(symbol)
                            .or_default()
                            .push((binary.span, key.to_string()));
                    }
                }
            }
            Expression::ComputedMemberExpression(member) => {
                if let (Some(symbol), Expression::NumericLiteral(index)) =
                    (symbol_of(&member.object, self.scoping), &member.expression)
                {
                    self.refs
                        .index_reads
                        .entry(symbol)
                        .or_default()
                        .push((member.span, index.value));
                }
            }
            _ => {}
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'p>) {
        self.refs.has_with = true;
        walk_js::walk_with_statement(self, it);
    }
}

pub(super) fn inside(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

pub(super) fn unpatched_outside(
    program: &Program<'_>,
    scoping: &Scoping,
    reliance: &untwist_core::semantic::builtins::Reliance,
    skipped: &[Span],
) -> bool {
    use untwist_core::semantic::builtins::{patched_by_another_input, scan_outside, Builtins};
    let skipped: FxHashSet<Span> = skipped.iter().copied().collect();
    let module = untwist_core::semantic::has_module_syntax(program);
    !patched_by_another_input(reliance)
        && scan_outside(program, scoping, module, reliance, &skipped) == Builtins::Intact
}

pub(super) fn names_observed(reach: &crate::calls::agnostic::GlobalReach) -> bool {
    use untwist_core::semantic::{observers, Observers};
    observers() != Observers::None && reach.observed.is_some()
}

pub(super) struct Judged {
    pub(super) intact: bool,
    pub(super) reach: crate::calls::agnostic::GlobalReach,
}

pub(super) fn judge_replaced<'a>(
    program: &Program<'a>,
    cx: &untwist_core::pass::Cx<'a, '_>,
    replacements: Vec<(Span, Expression<'a>)>,
    (reliance, skipped): (&untwist_core::semantic::builtins::Reliance, &[Span]),
    symbols: &[SymbolId],
) -> Option<Judged> {
    use crate::calls::agnostic::{global_reach, ReachWatch};
    use oxc_allocator::{CloneIn, GetAllocator};
    use oxc_semantic::SemanticBuilder;
    use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
    let original = cx.scoping();
    let alloc = cx.allocator();
    let mut clone = program.clone_in(alloc);
    let mut scoping = SemanticBuilder::new().build(&clone).semantic.into_scoping();
    let mut edits = ExprEdits::new();
    for (span, replacement) in replacements {
        if !edits.insert(span, replacement) {
            return None;
        }
    }
    let wanted = edits.len();
    if edits.apply(&mut clone, alloc, &mut scoping) != wanted {
        return None;
    }
    let mut stripped = StmtEdits::new();
    for span in skipped {
        stripped.remove(*span);
    }
    stripped.apply(&mut clone, alloc, &mut scoping);
    let scoping = SemanticBuilder::new().build(&clone).semantic.into_scoping();
    let names: Vec<&str> = symbols
        .iter()
        .map(|symbol| original.symbol_name(*symbol))
        .collect();
    let intact = unpatched_outside(&clone, &scoping, reliance, &[]);
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &[],
        observe: true,
    };
    let reach = global_reach(&clone, &scoping, (None, &[]), watch);
    Some(Judged { intact, reach })
}
