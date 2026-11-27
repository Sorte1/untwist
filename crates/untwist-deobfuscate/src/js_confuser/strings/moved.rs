use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;
use untwist_eval::fold::Const;

use super::super::names::Names;
use super::super::refs::{inside, literal_value, Refs};
use super::super::templates::{matches_buffer_to_string, matches_moved_get_global};
use super::pairs::{function_symbol, text_of};
use super::sandbox::{outside_symbols, Sandbox};
use crate::pure_calls::literal_data;

const GLOBAL_MARK: &str = "return globalThis";

pub(super) struct LiteralArray {
    pub(super) symbol: SymbolId,
    pub(super) statement: Span,
    pub(super) init: Span,
    pub(super) values: Vec<Const>,
}

fn runs_no_code(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => declaration.declarations.iter().all(|d| {
            matches!(d.id, BindingPattern::BindingIdentifier(_))
                && d.init.as_ref().is_none_or(|init| literal_data(init, true))
        }),
        _ => false,
    }
}

pub(super) fn literal_array(
    program: &Program<'_>,
    refs: &Refs<'_>,
    scoping: &Scoping,
) -> Option<LiteralArray> {
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            if runs_no_code(statement) {
                continue;
            }
            return None;
        };
        let found = match declaration.declarations.as_slice() {
            [declarator] => match (&declarator.id, &declarator.init) {
                (
                    BindingPattern::BindingIdentifier(id),
                    Some(Expression::ArrayExpression(array)),
                ) => id.symbol_id.get().map(|s| (s, array.span)).zip(
                    array
                        .elements
                        .iter()
                        .map(|element| element.as_expression().and_then(literal_value))
                        .collect::<Option<Vec<Const>>>(),
                ),
                _ => None,
            },
            _ => None,
        };
        let Some(((symbol, init), values)) = found else {
            if runs_no_code(statement) {
                continue;
            }
            return None;
        };
        let reads = refs.index_reads.get(&symbol).map_or(0, Vec::len);
        let usable = refs.stable(symbol, scoping) && refs.of(symbol).len() == reads;
        return usable.then_some(LiteralArray {
            symbol,
            statement: statement.span(),
            init,
            values,
        });
    }
    None
}

pub(super) fn written_once(symbol: SymbolId, refs: &Refs<'_>, scoping: &Scoping) -> Option<Span> {
    let root = scoping.root_scope_id();
    let flags = scoping.symbol_flags(symbol);
    if scoping.symbol_scope_id(symbol) != root
        || !flags.contains(SymbolFlags::FunctionScopedVariable)
        || !scoping.symbol_redeclarations(symbol).is_empty()
    {
        return None;
    }
    match refs
        .of(symbol)
        .iter()
        .filter(|r| r.write)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [only] => Some(only.span),
        _ => None,
    }
}

fn identifier_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

fn assignments<'k, 'a>(
    statement: &'k Statement<'a>,
    helpers: &rustc_hash::FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> Option<Vec<&'k AssignmentExpression<'a>>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let parts: Vec<&Expression<'a>> = match &statement.expression {
        Expression::CallExpression(call)
            if identifier_symbol(&call.callee, scoping).is_some_and(|s| helpers.contains(&s)) =>
        {
            call.arguments
                .iter()
                .map(Argument::as_expression)
                .collect::<Option<_>>()?
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
        other => vec![other],
    };
    parts
        .into_iter()
        .map(|part| match part {
            Expression::AssignmentExpression(assign)
                if assign.operator == AssignmentOperator::Assign =>
            {
                Some(&**assign)
            }
            _ => None,
        })
        .collect()
}

pub(super) struct Moved {
    pub(super) statements: Vec<Span>,
    pub(super) span: Span,
    pub(super) buffer_to_string: SymbolId,
    pub(super) last_index: usize,
    pub(super) vars: Vec<SymbolId>,
}

struct Prelude<'k, 'a> {
    getter: (SymbolId, Span),
    assigned: FxHashMap<SymbolId, &'k Expression<'a>>,
    buffer_to_string: (SymbolId, Span),
}

fn member_of(expr: &Expression<'_>, object: SymbolId, property: &str, scoping: &Scoping) -> bool {
    matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
        if member.property.name == property && identifier_symbol(&member.object, scoping) == Some(object))
}

fn global_object(expr: &Expression<'_>, getter: SymbolId, scoping: &Scoping) -> bool {
    let Expression::LogicalExpression(logical) = expr.without_parentheses() else {
        return false;
    };
    let call = match logical.left.without_parentheses() {
        Expression::CallExpression(call) => call,
        _ => return false,
    };
    let empty = matches!(logical.right.without_parentheses(), Expression::ObjectExpression(o) if o.properties.is_empty());
    logical.operator == LogicalOperator::Or
        && empty
        && call.arguments.is_empty()
        && identifier_symbol(&call.callee, scoping) == Some(getter)
}

fn text_decoder_parts(function: &Function<'_>, scoping: &Scoping) -> Option<(SymbolId, SymbolId)> {
    let [Statement::ReturnStatement(ret)] = function.body.as_ref()?.statements.as_slice() else {
        return None;
    };
    let Some(Expression::ConditionalExpression(choice)) = &ret.argument else {
        return None;
    };
    let Expression::CallExpression(call) = &choice.consequent else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    let Expression::NewExpression(decoder) = member.object.without_parentheses() else {
        return None;
    };
    let [Argument::NewExpression(bytes)] = call.arguments.as_slice() else {
        return None;
    };
    Some((
        identifier_symbol(&decoder.callee, scoping)?,
        identifier_symbol(&bytes.callee, scoping)?,
    ))
}

fn decoded_through_ok(
    assigned: &FxHashMap<SymbolId, &Expression<'_>>,
    getter: SymbolId,
    function: &Function<'_>,
    scoping: &Scoping,
) -> bool {
    let Some((decoder, bytes)) = text_decoder_parts(function, scoping) else {
        return false;
    };
    let global = assigned
        .iter()
        .find(|(_, value)| global_object(value, getter, scoping))
        .map(|(symbol, _)| *symbol);
    let Some(global) = global else {
        return false;
    };
    let is = |symbol: SymbolId, property: &str| {
        assigned
            .get(&symbol)
            .is_some_and(|value| member_of(value, global, property, scoping))
    };
    is(decoder, "TextDecoder") && is(bytes, "Uint8Array")
}

pub(super) fn declared_bare(program: &Program<'_>, symbol: SymbolId) -> bool {
    program.body.iter().any(|statement| {
        let Statement::VariableDeclaration(declaration) = statement else {
            return false;
        };
        declaration.kind == VariableDeclarationKind::Var
            && declaration.declarations.iter().any(|d| {
                d.init.is_none()
                    && matches!(&d.id, BindingPattern::BindingIdentifier(id) if id.symbol_id.get() == Some(symbol))
            })
    })
}

fn closed_to(
    refs: &Refs<'_>,
    scoping: &Scoping,
    region: Span,
    allowed: &dyn Fn(SymbolId) -> bool,
) -> bool {
    refs.all
        .iter()
        .filter(|(r, _)| inside(region, r.span))
        .all(|(r, name)| match r.symbol {
            None => {
                super::super::tokens::FIXED.contains(name)
                    && !untwist_core::semantic::is_foreign_global(name)
            }
            Some(symbol) => inside(region, scoping.symbol_span(symbol)) || allowed(symbol),
        })
}

fn not_equal_undefined(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::BinaryExpression(b) if b.operator == BinaryOperator::StrictInequality)
}

pub(super) fn find_moved_prelude(
    program: &Program<'_>,
    (refs, scoping, names): (&Refs<'_>, &Scoping, &Names),
    sandbox: &Sandbox<'_>,
) -> Option<Moved> {
    let body = &program.body;
    for index in 0..body.len().saturating_sub(2) {
        let Some((getter, _)) = function_symbol(&body[index]) else {
            continue;
        };
        if !text_of(program.source_text, body[index].span()).contains(GLOBAL_MARK) {
            continue;
        }
        let Some((buffer, function)) = function_symbol(&body[index + 2]) else {
            continue;
        };
        let checked = function.body.as_ref().is_some_and(|b| match b.statements.as_slice() {
            [Statement::ReturnStatement(ret)] => matches!(&ret.argument, Some(Expression::ConditionalExpression(c))
                if matches!(&c.test, Expression::LogicalExpression(l) if not_equal_undefined(&l.left))),
            _ => false,
        });
        if !checked || !matches_buffer_to_string(program.source_text, body[index + 2].span(), names)
        {
            continue;
        }
        let Some(parts) = assignments(&body[index + 1], sandbox.helpers, scoping) else {
            continue;
        };
        let mut assigned = FxHashMap::default();
        for part in parts {
            let AssignmentTarget::AssignmentTargetIdentifier(target) = &part.left else {
                return None;
            };
            let symbol = scoping
                .get_reference(target.reference_id.get()?)
                .symbol_id()?;
            if written_once(symbol, refs, scoping) != Some(target.span) {
                return None;
            }
            if assigned.insert(symbol, &part.right).is_some() {
                return None;
            }
        }
        if !assigned
            .keys()
            .all(|symbol| declared_bare(program, *symbol))
            || !decoded_through_ok(&assigned, getter, function, scoping)
        {
            return None;
        }
        let prelude = Prelude {
            getter: (getter, body[index].span()),
            assigned,
            buffer_to_string: (buffer, body[index + 2].span()),
        };
        return proven(program, (refs, scoping, names), sandbox, &prelude, index);
    }
    None
}

fn proven(
    program: &Program<'_>,
    (refs, scoping, names): (&Refs<'_>, &Scoping, &Names),
    sandbox: &Sandbox<'_>,
    prelude: &Prelude<'_, '_>,
    index: usize,
) -> Option<Moved> {
    let body = &program.body;
    let span = Span::new(body[index].span().start, body[index + 2].span().end);
    let vars: Vec<SymbolId> = prelude.assigned.keys().copied().collect();
    let array = sandbox.array.map(|(symbol, _)| symbol);
    let allowed = |symbol: SymbolId| {
        vars.contains(&symbol)
            || Some(symbol) == array
            || sandbox.helpers.contains(&symbol)
            || symbol == prelude.getter.0
            || symbol == prelude.buffer_to_string.0
    };
    let only_inside = [prelude.getter.0]
        .iter()
        .chain(vars.iter())
        .all(|symbol| refs.of(*symbol).iter().all(|r| inside(span, r.span)));
    if !only_inside || !closed_to(refs, scoping, span, &allowed) {
        return None;
    }
    let (_, getter_span) = prelude.getter;
    let outside = outside_symbols(refs, scoping, getter_span);
    let reads_only = outside
        .iter()
        .all(|s| Some(*s) == array || sandbox.helpers.contains(s));
    let template = matches_moved_get_global(program.source_text, getter_span, names);
    if !reads_only || !template {
        return None;
    }
    Some(Moved {
        statements: body[index..=index + 2].iter().map(GetSpan::span).collect(),
        span,
        buffer_to_string: prelude.buffer_to_string.0,
        last_index: index + 2,
        vars,
    })
}
