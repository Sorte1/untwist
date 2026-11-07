use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::{Ident, Str};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::global::{is_global_reference, is_global_reference_id};
use untwist_core::ast_util::literal::{is_undefined_like, undefined_expression};
use untwist_core::ast_util::matcher::{call_parts, var_function_or_declaration};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::readonly::{
    readonly_bindings_uses, scan_object_uses, ObjectUses, PropRead,
};
use untwist_core::ast_util::rename::Renames;
use untwist_core::ast_util::search::statements_at;
use untwist_core::pass::{Cx, Interest, NoteKind, Pass, RepeatedSites};
use untwist_core::semantic::is_constant_binding;

use crate::init_order::{
    early_or_written, lend_order, pass_done, read_before_declaration, reads_are_initialised,
    refuse_read_before_declaration, InitOrder, LentOrder,
};
use crate::notes;
use crate::proof::{
    declared_text, reachable_by_name, redeclared, refuse_annex_b_bindings, watch_texts, TextWatch,
};
use crate::stamp::{
    apply_stamps, declaration_symbol, reached_outside_its_block, string_array_name,
    string_array_unit, DynamicScopes, IndexAllocator, RefusedByDecode,
};

pub const PASS: &str = "find-string-array";

pub struct ArrayShape {
    pub statement: Span,
    pub symbol: SymbolId,
    pub length: usize,
    pub undefined_read: Option<(Span, Option<ReferenceId>)>,
    pub bound: bool,
}

mod aliases;
mod legacy;
mod positions;
mod prologue;
mod reach;

use aliases::{resolve_array_aliases, search_aliases};

pub fn bare_template_text<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    let Expression::TemplateLiteral(template) = expr else {
        return None;
    };
    if !template.expressions.is_empty() || template.quasis.len() != 1 {
        return None;
    }
    let element = template.quasis.first()?;
    element.value.cooked?.as_str()
}

fn array_literal<'k, 'a>(expr: &'k Expression<'a>) -> Option<usize> {
    let Expression::ArrayExpression(array) = expr else {
        return None;
    };
    for element in &array.elements {
        let expr = element.as_expression()?;
        let is_entry = matches!(expr, Expression::StringLiteral(_))
            || is_undefined_like(expr)
            || bare_template_text(expr).is_some();
        if !is_entry {
            return None;
        }
    }
    Some(array.elements.len())
}

fn array_declaration<'k, 'a>(stmt: &'k Statement<'a>) -> Option<(&'k str, usize)> {
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
    let length = array_literal(declarator.init.as_ref()?)?;
    Some((id.name.as_str(), length))
}

fn returns_array(expr: &Expression<'_>, array: &str) -> bool {
    let Expression::FunctionExpression(function) = expr else {
        return false;
    };
    if function.id.is_some() || !function.params.items.is_empty() {
        return false;
    }
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    if body.statements.len() != 1 {
        return false;
    }
    let Some(Statement::ReturnStatement(statement)) = body.statements.first() else {
        return false;
    };
    statement
        .argument
        .as_ref()
        .and_then(identifier_name)
        .is_some_and(|name| name == array)
}

fn self_assignment(expr: &Expression<'_>, function: &str, array: &str) -> bool {
    let Expression::AssignmentExpression(assignment) = expr else {
        return false;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return false;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return false;
    };
    target.name.as_str() == function && returns_array(&assignment.right, array)
}

fn calls(expr: &Expression<'_>, name: &str) -> bool {
    let Some((callee, arguments)) = call_parts(expr) else {
        return false;
    };
    arguments.is_empty() && identifier_name(callee) == Some(name)
}

fn body_matches(statements: &[Statement<'_>], function: &str, array: &str) -> bool {
    match statements {
        [_, Statement::ReturnStatement(ret)] => {
            let Some(argument) = ret.argument.as_ref() else {
                return false;
            };
            let Some((callee, arguments)) = call_parts(argument) else {
                return false;
            };
            arguments.is_empty() && self_assignment(callee, function, array)
        }
        [_, Statement::ExpressionStatement(stmt), Statement::ReturnStatement(ret)] => {
            self_assignment(&stmt.expression, function, array)
                && ret.argument.as_ref().is_some_and(|a| calls(a, function))
        }
        _ => false,
    }
}

pub(crate) fn is_string_array_function(function: &Function<'_>, name: &str) -> bool {
    if !function.params.items.is_empty() || function.params.rest.is_some() {
        return false;
    }
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let Some((array, _)) = body.statements.first().and_then(array_declaration) else {
        return false;
    };
    body_matches(&body.statements, name, array)
}

pub fn match_string_array(stmt: &Statement<'_>) -> Option<ArrayShape> {
    let declaration = var_function_or_declaration(stmt)?;
    if !declaration.function.params.items.is_empty() || declaration.function.params.rest.is_some() {
        return None;
    }
    let body = declaration.function.body.as_ref()?;
    let first = body.statements.first()?;
    let (array, length) = array_declaration(first)?;
    if !body_matches(&body.statements, declaration.name, array) {
        return None;
    }
    Some(ArrayShape {
        statement: stmt.span(),
        symbol: declaration_symbol(stmt)?,
        length,
        undefined_read: first_undefined_read(first),
        bound: matches!(stmt, Statement::VariableDeclaration(_)),
    })
}

struct Search<'a, const WATCHING: bool> {
    found: Vec<(ArrayShape, &'a str)>,
    watch: TextWatch<'a>,
}

impl<'a, const WATCHING: bool> VisitJs<'a> for Search<'a, WATCHING> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(shape) = match_string_array(it) {
            self.found.push((shape, declared_text(it)));
        }
        walk_js::walk_statement(self, it);
    }

    watch_texts!();
}

fn searched<'a, const WATCHING: bool>(
    program: &Program<'a>,
    watching: Vec<&'a str>,
) -> (Vec<(ArrayShape, &'a str)>, TextWatch<'a>) {
    let mut search = Search::<WATCHING> {
        found: Vec::new(),
        watch: TextWatch::over(watching),
    };
    search.visit_program(program);
    (search.found, search.watch)
}

fn top_level_arrays<'a>(program: &Program<'a>) -> Vec<&'a str> {
    program
        .body
        .iter()
        .filter(|statement| match_string_array(statement).is_some())
        .map(declared_text)
        .filter(|name| string_array_unit(name).is_none())
        .collect()
}

#[derive(Default)]
pub struct FindStringArray {
    simple_arrays_seen: Option<FxHashSet<Span>>,
    refused_by_decode: RefusedByDecode,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(FindStringArray::default())
}

pub fn boxed_with<'a>(refused_by_decode: RefusedByDecode) -> Box<dyn Pass<'a> + 'a> {
    Box::new(FindStringArray {
        refused_by_decode,
        ..FindStringArray::default()
    })
}

fn unstamped_arrays(
    program: &Program<'_>,
    scoping: &Scoping,
) -> (Vec<ArrayShape>, Vec<Option<Span>>) {
    let watching = top_level_arrays(program);
    let (found, watch) = if watching.is_empty() {
        searched::<false>(program, watching)
    } else {
        searched::<true>(program, watching)
    };
    let (unstamped, names): (Vec<ArrayShape>, Vec<&str>) = found
        .into_iter()
        .filter(|(_, name)| string_array_unit(name).is_none())
        .unzip();
    if unstamped.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let reachable: Vec<bool> = unstamped
        .iter()
        .map(|shape| reachable_by_name(scoping, shape.symbol))
        .collect();
    let watched = watch.resolve(program, &names, &reachable);
    let mut paired: Vec<(ArrayShape, Option<Span>)> =
        unstamped.into_iter().zip(watched.spelled).collect();
    paired.sort_by_key(|(shape, _)| (shape.statement.start, shape.statement.end));
    paired.into_iter().unzip()
}

fn array_refusal(
    scoping: &Scoping,
    dynamic: &DynamicScopes,
    shape: &ArrayShape,
    spelled: Option<Span>,
) -> Option<(&'static str, Span)> {
    let seen = dynamic
        .refusal(scoping, shape.symbol)
        .or_else(|| reached_outside_its_block(scoping, shape.symbol));
    let spelled = spelled.map(|span| (notes::MACHINERY_NAME_SPELLED, span));
    spelled
        .or(seen.map(|note| (note, shape.statement)))
        .or_else(|| {
            redeclared(scoping, shape.symbol).map(|span| (notes::MACHINERY_REDECLARED, span))
        })
}

fn first_undefined_read(declaration: &Statement<'_>) -> Option<(Span, Option<ReferenceId>)> {
    array_elements(declaration)?
        .iter()
        .filter_map(ArrayExpressionElement::as_expression)
        .find_map(|element| match element.without_parentheses() {
            Expression::Identifier(id) if id.name == "undefined" => {
                Some((id.span, id.reference_id.get()))
            }
            _ => None,
        })
}

fn declared_undefined(scoping: &Scoping, shape: &ArrayShape) -> Option<Span> {
    let (span, reference) = shape.undefined_read?;
    (!is_global_reference_id(scoping, reference)).then_some(span)
}

fn stamp_wrapped_array<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    refused_by_decode: &RefusedByDecode,
) {
    let (found, spelled) = unstamped_arrays(program, cx.scoping());
    if found.is_empty() {
        return;
    }
    let refused = refused_by_decode.borrow();
    let bound: Vec<(Span, SymbolId)> = found
        .iter()
        .filter(|shape| shape.bound && !refused.contains(&shape.statement))
        .map(|shape| (shape.statement, shape.symbol))
        .collect();
    let early = read_before_declaration(program, cx.scoping(), &bound);
    let dynamic = DynamicScopes::of(cx.scoping());
    let mut indices = IndexAllocator::string_arrays(cx.scoping());
    let mut renames = Renames::new();
    let mut stamped_now = Vec::new();
    for (shape, spelled) in found.iter().zip(spelled) {
        if refused.contains(&shape.statement) {
            continue;
        }
        if let Some(note) = early_or_written(cx.scoping(), &early, shape.symbol) {
            cx.note(note, Some(shape.statement));
            continue;
        }
        if let Some((note, span)) = array_refusal(cx.scoping(), &dynamic, shape, spelled) {
            cx.note(note, Some(span));
            continue;
        }
        if let Some(span) = declared_undefined(cx.scoping(), shape) {
            cx.note(notes::STRING_ARRAY_DECLARED_UNDEFINED, Some(span));
            continue;
        }
        let name = Ident::from_str_in(&string_array_name(indices.allocate()), &*cx);
        renames.insert(shape.symbol, name);
        stamped_now.push((shape.length, shape.statement));
    }
    drop(refused);
    apply_stamps(program, cx, &renames, stamped_now);
}

struct SimpleArray {
    statement: Span,
    symbol: SymbolId,
    length: usize,
}

fn simple_array(stmt: &Statement<'_>) -> Option<SimpleArray> {
    let (_, length) = array_declaration(stmt).filter(|(_, length)| *length > 0)?;
    let symbol = declaration_symbol(stmt)?;
    Some(SimpleArray {
        statement: stmt.span(),
        symbol,
        length,
    })
}

#[derive(Default)]
struct SimpleSearch {
    found: Vec<SimpleArray>,
}

impl<'a> VisitJs<'a> for SimpleSearch {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(simple) = simple_array(it) {
            self.found.push(simple);
        }
        walk_js::walk_statement(self, it);
    }
}

fn simple_arrays(program: &Program<'_>) -> Vec<SimpleArray> {
    let mut search = SimpleSearch::default();
    search.visit_program(program);
    search.found
}

fn array_elements<'p>(
    stmt: &'p Statement<'p>,
) -> Option<&'p oxc_allocator::Vec<'p, ArrayExpressionElement<'p>>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let Some(Expression::ArrayExpression(array)) = declaration
        .declarations
        .first()
        .and_then(|declarator| declarator.init.as_ref())
    else {
        return None;
    };
    Some(&array.elements)
}

fn collect_indices(
    reads: &[PropRead],
    length: usize,
    unexpected: &mut RepeatedSites,
) -> Option<Vec<(Span, usize)>> {
    if !reads.iter().any(|read| read.name.parse::<usize>().is_ok()) {
        return None;
    }
    let mut indices = Vec::with_capacity(reads.len());
    for read in reads {
        match read.name.parse::<usize>() {
            Ok(index) if index < length => indices.push((read.member_span, index)),
            _ => {
                unexpected.record(read.member_span, &read.name);
                return None;
            }
        }
    }
    Some(indices)
}

fn undefined_not_global(
    elements: &oxc_allocator::Vec<'_, ArrayExpressionElement<'_>>,
    scoping: &Scoping,
) -> Option<Span> {
    elements
        .iter()
        .filter_map(ArrayExpressionElement::as_expression)
        .map(Expression::without_parentheses)
        .find_map(|element| match element {
            Expression::Identifier(id)
                if id.name == "undefined" && !is_global_reference(scoping, id) =>
            {
                Some(id.span)
            }
            _ => None,
        })
}

fn element_expression<'a>(element: &Expression<'_>, span: Span, cx: &Cx<'a, '_>) -> Expression<'a> {
    if is_undefined_like(element) {
        return undefined_expression(span, cx);
    }
    match bare_template_text(element) {
        Some(text) => {
            let value = Str::from_str_in(text, cx);
            Expression::new_string_literal(span, value, None, cx)
        }
        None => element.clone_in(cx.allocator()),
    }
}

struct SimplePlan<'a> {
    edits: ExprEdits<'a>,
    removals: StmtEdits<'a>,
    unexpected: RepeatedSites,
    declared: FxHashSet<SymbolId>,
}

fn insert_element_edits<'a>(
    indices: Vec<(Span, usize)>,
    elements: Option<&oxc_allocator::Vec<'_, ArrayExpressionElement<'_>>>,
    cx: &mut Cx<'a, '_>,
    plan: &mut SimplePlan<'a>,
) {
    for (span, index) in indices {
        let element = elements
            .and_then(|elements| elements.get(index))
            .and_then(ArrayExpressionElement::as_expression);
        let Some(element) = element else {
            cx.note(notes::array_element_not_found(index), Some(span));
            continue;
        };
        plan.edits
            .insert(span, element_expression(element, span, &*cx));
    }
}

fn initialised_indices(
    candidate: &SimpleArray,
    uses: &ObjectUses,
    order: &InitOrder,
    cx: &mut Cx<'_, '_>,
    indices: Vec<(Span, usize)>,
) -> Option<Vec<(Span, usize)>> {
    if let Err(refused) = reads_are_initialised(&uses.reads, candidate.statement, order) {
        refuse_read_before_declaration(
            refused,
            candidate.symbol,
            candidate.statement,
            notes::ARRAY_READ_BEFORE_DECLARATION,
            cx,
        );
        return None;
    }
    Some(indices)
}

fn plan_simple_array<'a>(
    (candidate, statement): (&SimpleArray, Option<&Statement<'_>>),
    indices: Vec<(Span, usize)>,
    uses: &ObjectUses,
    cx: &mut Cx<'a, '_>,
    plan: &mut SimplePlan<'a>,
) {
    let elements = statement
        .filter(|_| !indices.is_empty())
        .and_then(array_elements);
    if let Some(span) = elements.and_then(|elements| undefined_not_global(elements, cx.scoping())) {
        cx.note(notes::SIMPLE_ARRAY_UNDEFINED_NOT_GLOBAL, Some(span));
        return;
    }
    let before = plan.edits.len();
    insert_element_edits(indices, elements, cx, plan);
    if plan.edits.len() - before != uses.reads.len() {
        cx.note(
            notes::ARRAY_READS_NOT_ALL_SUBSTITUTED,
            Some(candidate.statement),
        );
        return;
    }
    if !plan.declared.contains(&candidate.symbol) {
        plan.removals.remove(candidate.statement);
    }
}

fn eligible_at_first_scan(candidates: &mut Vec<SimpleArray>, seen: &mut Option<FxHashSet<Span>>) {
    let seen = seen.get_or_insert_with(|| candidates.iter().map(|found| found.statement).collect());
    candidates.retain(|found| seen.contains(&found.statement));
}

type Checked<'k> = Vec<(&'k SimpleArray, Vec<(Span, usize)>)>;

fn keep_value_reads(program: &Program<'_>, checked: &mut Checked<'_>, cx: &mut Cx<'_, '_>) {
    let reads: Vec<Span> = checked
        .iter()
        .flat_map(|(_, indices)| indices.iter().map(|(span, _)| *span))
        .collect();
    let consumed = positions::reads_not_values(program, &reads);
    if consumed.is_empty() {
        return;
    }
    checked.retain(|(_, indices)| {
        let Some((span, _)) = indices.iter().find(|(span, _)| consumed.contains(span)) else {
            return true;
        };
        cx.note(notes::SIMPLE_ARRAY_READ_NOT_A_VALUE, Some(*span));
        false
    });
}

fn keep_unreached(
    program: &Program<'_>,
    checked: &mut Checked<'_>,
    cx: &mut Cx<'_, '_>,
) -> FxHashSet<SymbolId> {
    let candidates: Vec<(SymbolId, Span)> = checked
        .iter()
        .map(|(candidate, _)| (candidate.symbol, candidate.statement))
        .collect();
    let kept = reach::reached(program, cx, &candidates);
    checked.retain(|(candidate, _)| !kept.symbols.contains(&candidate.symbol));
    for (note, span) in kept.notes {
        cx.note(note, Some(span));
    }
    if let Some(span) = kept.observed {
        cx.progress(notes::SIMPLE_ARRAY_NAME_OBSERVED, Some(span));
    }
    kept.declared
}

fn simple_candidates(
    mut candidates: Vec<SimpleArray>,
    seen: &mut Option<FxHashSet<Span>>,
    renamed_into: &FxHashSet<SymbolId>,
    extended: bool,
) -> Vec<SimpleArray> {
    if !extended {
        eligible_at_first_scan(&mut candidates, seen);
    }
    candidates.retain(|candidate| !renamed_into.contains(&candidate.symbol));
    candidates
}

fn simple_uses(
    program: &Program<'_>,
    candidates: &[SimpleArray],
    scoping: &Scoping,
) -> FxHashMap<SymbolId, ObjectUses> {
    let symbols: FxHashSet<SymbolId> = candidates.iter().map(|found| found.symbol).collect();
    readonly_bindings_uses(program, scoping, &symbols)
}

#[cfg(debug_assertions)]
fn record_simple_order(
    program: &Program<'_>,
    scoping: &Scoping,
    order: Option<&LentOrder>,
    uses: &FxHashMap<SymbolId, ObjectUses>,
) {
    if !crate::fused::check::active() {
        return;
    }
    let scanned = || crate::init_order::try_scan_init_order(program, scoping).unwrap_or_default();
    match order {
        Some(order) => crate::fused::check::record_order_and_uses("simple-arrays", order, uses),
        None => crate::fused::check::record_order_and_uses("simple-arrays", &scanned(), uses),
    }
}

fn checked_simple_arrays<'k>(
    program: &Program<'_>,
    candidates: &'k [SimpleArray],
    uses: &FxHashMap<SymbolId, ObjectUses>,
    (cx, unexpected): (&mut Cx<'_, '_>, &mut RepeatedSites),
    shares: bool,
) -> (Checked<'k>, Option<LentOrder>) {
    let mut order: Option<LentOrder> = None;
    let mut checked: Checked<'k> = Vec::new();
    for candidate in candidates {
        let Some(candidate_uses) = uses.get(&candidate.symbol) else {
            continue;
        };
        let Some(indices) = collect_indices(&candidate_uses.reads, candidate.length, unexpected)
        else {
            continue;
        };
        let order = match &mut order {
            Some(order) => order,
            empty => empty.insert(lend_order(shares, program, cx)),
        };
        if let Some(indices) = initialised_indices(candidate, candidate_uses, order, cx, indices) {
            checked.push((candidate, indices));
        }
    }
    (checked, order)
}

fn inline_simple_arrays<'a>(
    program: &mut Program<'a>,
    (found, seen): (Vec<SimpleArray>, &mut Option<FxHashSet<Span>>),
    renamed_into: &FxHashSet<SymbolId>,
    cx: &mut Cx<'a, '_>,
    shares: bool,
) {
    let candidates = simple_candidates(found, seen, renamed_into, cx.opts.extended);
    if candidates.is_empty() {
        return;
    }
    let uses = simple_uses(program, &candidates, cx.scoping());
    let mut plan = SimplePlan {
        edits: ExprEdits::new(),
        removals: StmtEdits::new(),
        unexpected: RepeatedSites::default(),
        declared: FxHashSet::default(),
    };
    let found = (&mut *cx, &mut plan.unexpected);
    let (mut checked, order) = checked_simple_arrays(program, &candidates, &uses, found, shares);
    #[cfg(debug_assertions)]
    record_simple_order(program, cx.scoping(), order.as_ref(), &uses);
    refuse_annex_b_bindings(
        &mut checked,
        |(found, _)| (found.statement, [found.symbol]),
        program,
        cx,
    );
    keep_value_reads(program, &mut checked, cx);
    if !checked.is_empty() {
        plan.declared = keep_unreached(program, &mut checked, cx);
    }
    let wanted: Vec<Span> = checked
        .iter()
        .map(|(candidate, _)| candidate.statement)
        .collect();
    let statements = statements_at(program, &wanted);
    for ((candidate, indices), statement) in checked.into_iter().zip(statements) {
        let Some(candidate_uses) = uses.get(&candidate.symbol) else {
            continue;
        };
        let found = (candidate, statement);
        plan_simple_array(found, indices, candidate_uses, cx, &mut plan);
    }
    plan.unexpected.emit(
        NoteKind::Refusal,
        notes::simple_array_access,
        notes::simple_array_accesses,
        cx,
    );
    if let Some(order) = order {
        order.give_back_any(program, cx);
    }
    apply_and_account(PASS, program, plan.edits, plan.removals, cx);
}

fn apply_and_account<'a>(
    pass: &'static str,
    program: &mut Program<'a>,
    edits: ExprEdits<'a>,
    removals: StmtEdits<'a>,
    cx: &mut Cx<'a, '_>,
) {
    let wanted = edits.len() + removals.len();
    let applied = edits.apply(program, cx.allocator(), cx.scoping_mut())
        + removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(applied);
    if applied != wanted {
        cx.note(
            notes::planned_and_applied_edits(pass, wanted, applied),
            None,
        );
    }
}

impl<'a> Pass<'a> for FindStringArray {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if legacy::lift_legacy_array(program, cx) {
            return;
        }
        stamp_wrapped_array(program, cx, &self.refused_by_decode);
        let (sites, simple) = search_aliases(program, cx.scoping());
        let unchanged = *cx.changes;
        let renamed_into = resolve_array_aliases(program, cx, sites);
        let simple = if *cx.changes == unchanged {
            simple
        } else {
            simple_arrays(program)
        };
        let found = (simple, &mut self.simple_arrays_seen);
        let shares = cx.opts.shares_init_order;
        inline_simple_arrays(program, found, &renamed_into, cx, shares);
        pass_done(shares, *cx.changes > 0);
    }
}
