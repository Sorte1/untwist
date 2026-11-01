use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::pattern::array_pattern;
use untwist_core::pass::{Cx, Safety};

use super::identify::{resolved, Table};
use super::shapes::Kind;
use super::spread::Gate;

const MAX_SLOTS: f64 = 32.0;

#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub span: Span,
    pub temp: SymbolId,
}

struct Plan {
    kind: Kind,
    slots: usize,
    reads: Vec<(usize, usize)>,
}

fn small_index(expression: &Expression<'_>, below: f64) -> Option<usize> {
    let Expression::NumericLiteral(literal) = expression.without_parentheses() else {
        return None;
    };
    let value = literal.value;
    let whole = value.fract() == 0.0 && (0.0..below).contains(&value);
    whole.then_some(value as usize)
}

fn destructuring_call<'k, 'a>(
    init: &'k Expression<'a>,
    table: &Table,
    scoping: &Scoping,
) -> Option<(Kind, &'k Expression<'a>, usize)> {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return None;
    };
    if call.optional || call.arguments.len() != 2 {
        return None;
    }
    let kind = match &call.callee {
        Expression::Identifier(_) => table.kind_of_callee(&call.callee, scoping)?,
        _ => return None,
    };
    if !matches!(kind, Kind::SlicedToArray | Kind::Read) {
        return None;
    }
    let source = call.arguments.first()?.as_expression()?;
    let slots = small_index(call.arguments.get(1)?.as_expression()?, MAX_SLOTS + 1.0)?;
    (slots > 0).then_some((kind, source, slots))
}

pub fn safety_of(source: &Expression<'_>) -> Safety {
    match source.without_parentheses() {
        Expression::ArrayExpression(_) | Expression::StringLiteral(_) => Safety::Safe,
        _ => Safety::Unsafe,
    }
}

pub fn candidate(
    declarator: &VariableDeclarator<'_>,
    table: &Table,
    scoping: &Scoping,
) -> Option<(SymbolId, Safety)> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let (_, source, _) = destructuring_call(declarator.init.as_ref()?, table, scoping)?;
    Some((id.symbol_id.get()?, safety_of(source)))
}

fn single<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(
    &'k VariableDeclaration<'a>,
    &'k VariableDeclarator<'a>,
    SymbolId,
)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    (!declaration.declare).then_some((declaration, declarator, id.symbol_id.get()?))
}

fn read_index(
    statement: &Statement<'_>,
    kind: VariableDeclarationKind,
    temp: SymbolId,
    scoping: &Scoping,
) -> Option<usize> {
    let (declaration, declarator, target) = single(statement)?;
    if declaration.kind != kind || target == temp {
        return None;
    }
    let Some(Expression::ComputedMemberExpression(member)) = &declarator.init else {
        return None;
    };
    if member.optional || resolved(&member.object, scoping) != Some(temp) {
        return None;
    }
    small_index(&member.expression, MAX_SLOTS)
}

fn plan(
    statements: &[Statement<'_>],
    index: usize,
    temp: SymbolId,
    table: &Table,
    scoping: &Scoping,
) -> Option<(Plan, Safety)> {
    let (declaration, declarator, symbol) = single(statements.get(index)?)?;
    if symbol != temp {
        return None;
    }
    let (kind, source, slots) = destructuring_call(declarator.init.as_ref()?, table, scoping)?;
    let mut reads: Vec<(usize, usize)> = Vec::new();
    for (at, statement) in statements.iter().enumerate().skip(index + 1) {
        let Some(slot) = read_index(statement, declaration.kind, temp, scoping) else {
            break;
        };
        if slot >= slots || reads.last().is_some_and(|(last, _)| *last >= slot) {
            break;
        }
        reads.push((slot, at));
    }
    let complete =
        !reads.is_empty() && scoping.get_resolved_reference_ids(temp).len() == reads.len();
    complete.then(|| (Plan { kind, slots, reads }, safety_of(source)))
}

fn take_target<'a>(statement: &mut Statement<'a>, cx: &Cx<'a, '_>) -> Option<BindingPattern<'a>> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let declarator = declaration.declarations.first_mut()?;
    let dummy = BindingPattern::new_binding_identifier(SPAN, "", cx);
    Some(std::mem::replace(&mut declarator.id, dummy))
}

fn take_source<'a>(
    statement: &mut Statement<'a>,
    cx: &Cx<'a, '_>,
) -> Option<(Span, VariableDeclarationKind, Expression<'a>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let span = declaration.span;
    let kind = declaration.kind;
    let declarator = declaration.declarations.first_mut()?;
    let Some(Expression::CallExpression(call)) = declarator.init.as_mut() else {
        return None;
    };
    let first = call.arguments.first_mut()?;
    let placeholder = Argument::from(Expression::new_null_literal(SPAN, cx));
    let source = Expression::try_from(std::mem::replace(first, placeholder)).ok()?;
    Some((span, kind, source))
}

pub fn shape(kind: Kind, safety: Safety) -> &'static str {
    match (kind, safety) {
        (Kind::SlicedToArray, Safety::Safe) => "destructure/sliced-to-array/safe",
        (Kind::SlicedToArray, Safety::Unsafe) => "destructure/sliced-to-array/unsafe",
        (_, Safety::Safe) => "destructure/read/safe",
        (_, Safety::Unsafe) => "destructure/read/unsafe",
    }
}

fn rewrite<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    index: usize,
    (plan, safety): (Plan, Safety),
    cx: &Cx<'a, '_>,
) -> Option<&'static str> {
    let mut slots: Vec<Option<BindingPattern<'a>>> = (0..plan.slots).map(|_| None).collect();
    for (slot, at) in &plan.reads {
        let target = take_target(statements.get_mut(*at)?, cx)?;
        *slots.get_mut(*slot)? = Some(target);
    }
    let (span, kind, source) = take_source(statements.get_mut(index)?, cx)?;
    let pattern = array_pattern(slots, cx);
    let declarator = VariableDeclarator::new(span, pattern, None, Some(source), false, cx);
    let declarations = AVec::from_iter_in([declarator], cx);
    *statements.get_mut(index)? =
        Statement::new_variable_declaration(span, kind, declarations, false, cx);
    let last = plan.reads.last().map_or(index, |(_, at)| *at);
    statements.drain(index + 1..=last);
    Some(shape(plan.kind, safety))
}

pub fn covers(outer: Span, inner: Span) -> bool {
    outer.is_empty() || (outer.start <= inner.start && inner.end <= outer.end)
}

pub fn covered_suffix(sites: &[Site], outer: Span) -> usize {
    sites
        .iter()
        .rposition(|site| !covers(outer, site.span))
        .map_or(0, |at| at + 1)
}

struct Finder<'f, 'a, 'c> {
    site: Site,
    table: &'f Table,
    gate: &'f Gate<'f>,
    cx: &'f mut Cx<'a, 'c>,
    done: Option<Option<&'static str>>,
}

impl<'f, 'a, 'c> Finder<'f, 'a, 'c> {
    fn at(&self, statements: &[Statement<'_>]) -> Option<usize> {
        statements.iter().position(|statement| {
            covers(statement.span(), self.site.span)
                && single(statement).is_some_and(|(_, _, symbol)| symbol == self.site.temp)
        })
    }
}

impl<'f, 'a, 'c> VisitMut<'a> for Finder<'f, 'a, 'c> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        if self.done.is_some() {
            return;
        }
        let Some(index) = self.at(it) else {
            walk_mut::walk_statements(self, it);
            return;
        };
        let planned = plan(it, index, self.site.temp, self.table, self.cx.scoping());
        let allowed = planned.filter(|(_, safety)| self.gate.allows(*safety));
        self.done = Some(allowed.and_then(|planned| rewrite(it, index, planned, self.cx)));
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.done.is_none() && covers(it.span(), self.site.span) {
            walk_mut::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.done.is_none() && covers(it.span(), self.site.span) {
            walk_mut::walk_expression(self, it);
        }
    }
}

pub fn rewrite_site<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    site: Site,
    table: &Table,
    gate: &Gate<'_>,
    cx: &mut Cx<'a, '_>,
) -> Option<&'static str> {
    let mut finder = Finder {
        site,
        table,
        gate,
        cx,
        done: None,
    };
    finder.visit_statements(statements);
    finder.done.flatten()
}
