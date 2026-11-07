use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::operator::{UnaryOperator, UpdateOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::matcher::{call_parts, iife, statement_iife};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::parse::{has_error, parse_with, SourceType};
use untwist_core::pass::Cx;

use super::array_declaration;
use super::prologue::strip_atob_prologues;
use crate::array_rotator::push_shift_target;
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::decoded::syntactic_global_reach;
use crate::mutants::legacy_array_guards_dropped;
use crate::notes;
use crate::stamp::{declaration_symbol, string_array_unit, DynamicScopes};

const INNER_NAMES: [&str; 2] = ["a", "b"];

struct Legacy {
    array: usize,
    rotator: Option<(Span, usize)>,
    symbol: SymbolId,
    name_span: Span,
    decoder: Span,
    reads: Vec<Span>,
}

fn is_var_array<'k>(stmt: &'k Statement<'_>) -> Option<(&'k str, usize)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var {
        return None;
    }
    array_declaration(stmt)
        .filter(|(name, length)| *length > 0 && string_array_unit(name).is_none())
}

fn only_parameter<'a>(function: &'a Function<'a>) -> Option<&'a str> {
    let [param] = function.params.items.as_slice() else {
        return None;
    };
    if function.params.rest.is_some() {
        return None;
    }
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn parameters<'a>(function: &'a Function<'a>) -> Option<(&'a str, &'a str)> {
    let [first, second] = function.params.items.as_slice() else {
        return None;
    };
    if function.params.rest.is_some() {
        return None;
    }
    match (&first.pattern, &second.pattern) {
        (BindingPattern::BindingIdentifier(a), BindingPattern::BindingIdentifier(b))
            if a.name != b.name =>
        {
            Some((a.name.as_str(), b.name.as_str()))
        }
        _ => None,
    }
}

fn single_statement<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Statement<'a>> {
    match stmt {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [inner] => single_statement(inner),
            _ => None,
        },
        other => Some(other),
    }
}

fn pushes_shifted(stmt: &Statement<'_>, array: &str) -> bool {
    let Some(Statement::ExpressionStatement(stmt)) = single_statement(stmt) else {
        return false;
    };
    push_shift_target(stmt.expression.without_parentheses()) == Some(array)
}

fn predecrements(expr: &Expression<'_>, counter: &str) -> bool {
    let Expression::UpdateExpression(update) = expr.without_parentheses() else {
        return false;
    };
    let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &update.argument else {
        return false;
    };
    update.prefix && update.operator == UpdateOperator::Decrement && target.name == counter
}

fn is_countdown_loop(stmt: &Statement<'_>, counter: &str, array: &str) -> bool {
    match single_statement(stmt) {
        Some(Statement::ForStatement(node)) => {
            node.init.is_none()
                && node.update.is_none()
                && node
                    .test
                    .as_ref()
                    .is_some_and(|test| predecrements(test, counter))
                && pushes_shifted(&node.body, array)
        }
        Some(Statement::WhileStatement(node)) => {
            predecrements(&node.test, counter) && pushes_shifted(&node.body, array)
        }
        _ => false,
    }
}

fn is_countdown(function: &Function<'_>, array: &str) -> bool {
    let Some(counter) = only_parameter(function) else {
        return false;
    };
    if counter == array || function.id.is_some() {
        return false;
    }
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    body.directives.is_empty()
        && matches!(body.statements.as_slice(), [stmt] if is_countdown_loop(stmt, counter, array))
}

fn incremented(arguments: &[Argument<'_>], count: &str) -> bool {
    let [argument] = arguments else {
        return false;
    };
    let Some(Expression::UpdateExpression(update)) = argument
        .as_expression()
        .map(Expression::without_parentheses)
    else {
        return false;
    };
    let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = &update.argument else {
        return false;
    };
    update.prefix && update.operator == UpdateOperator::Increment && target.name == count
}

fn nested_countdown(stmt: &Statement<'_>, array: &str, count: &str) -> bool {
    let Some((function, arguments)) = statement_iife(stmt) else {
        return false;
    };
    is_countdown(function, array) && incremented(arguments, count)
}

fn named_countdown(statements: &[Statement<'_>], array: &str, count: &str) -> bool {
    let [Statement::VariableDeclaration(declaration), Statement::ExpressionStatement(call)] =
        statements
    else {
        return false;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return false;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return false;
    };
    let Some(Expression::FunctionExpression(function)) = declarator.init.as_ref() else {
        return false;
    };
    let Some((callee, arguments)) = call_parts(call.expression.without_parentheses()) else {
        return false;
    };
    id.name != array
        && id.name != count
        && identifier_name(callee) == Some(id.name.as_str())
        && is_countdown(function, array)
        && incremented(arguments, count)
}

fn rotation_count(arguments: &[Argument<'_>], array: &str) -> Option<usize> {
    let [first, second] = arguments else {
        return None;
    };
    if first.as_expression().and_then(identifier_name) != Some(array) {
        return None;
    }
    let Some(Expression::NumericLiteral(count)) = second.as_expression() else {
        return None;
    };
    let value = count.value;
    let whole = value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value);
    whole.then_some(value as usize)
}

fn legacy_rotation(stmt: &Statement<'_>, array: &str) -> Option<usize> {
    let Statement::ExpressionStatement(outer) = stmt else {
        return None;
    };
    let call = match outer.expression.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    let (function, arguments) = iife(call)?;
    let (target, count) = parameters(function)?;
    let body = function.body.as_ref()?;
    if function.id.is_some() || !body.directives.is_empty() {
        return None;
    }
    let shaped = match body.statements.as_slice() {
        [stmt] => nested_countdown(stmt, target, count),
        statements => named_countdown(statements, target, count),
    };
    if !shaped {
        return None;
    }
    rotation_count(arguments, array)
}

#[derive(Default)]
struct Reads {
    symbol: Option<SymbolId>,
    spans: Vec<Span>,
    deleted: bool,
}

impl Reads {
    fn names(&self, scoping: &Scoping, expr: &Expression<'_>) -> Option<Span> {
        let Expression::Identifier(id) = expr else {
            return None;
        };
        let reference = id.reference_id.get()?;
        let symbol = scoping.get_reference(reference).symbol_id();
        (symbol == self.symbol).then_some(id.span)
    }
}

struct ReadScan<'s> {
    scoping: &'s Scoping,
    reads: Reads,
}

impl<'a> VisitJs<'a> for ReadScan<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::ComputedMemberExpression(member) = it {
            if let Some(span) = self.reads.names(self.scoping, &member.object) {
                self.reads.spans.push(span);
            }
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Expression::ComputedMemberExpression(member) = it.argument.without_parentheses()
            {
                self.reads.deleted |= self.reads.names(self.scoping, &member.object).is_some();
            }
        }
        walk_js::walk_unary_expression(self, it);
    }
}

fn decoder_reads(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> Option<Vec<Span>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var
        || !matches!(declarator.init, Some(Expression::FunctionExpression(_)))
    {
        return None;
    }
    let mut scan = ReadScan {
        scoping,
        reads: Reads {
            symbol: Some(symbol),
            ..Reads::default()
        },
    };
    scan.visit_statement(stmt);
    (!scan.reads.deleted && !scan.reads.spans.is_empty()).then_some(scan.reads.spans)
}

fn array_name_span(stmt: &Statement<'_>) -> Option<Span> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    match &declaration.declarations.first()?.id {
        BindingPattern::BindingIdentifier(id) => Some(id.span),
        _ => None,
    }
}

fn legacy_at(body: &[Statement<'_>], index: usize, scoping: &Scoping) -> Option<Legacy> {
    let stmt = body.get(index)?;
    let (name, _) = is_var_array(stmt)?;
    let symbol = declaration_symbol(stmt)?;
    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }
    let rotator = body
        .get(index + 1)
        .and_then(|next| legacy_rotation(next, name).map(|count| (next.span(), count)));
    let after = index + 1 + usize::from(rotator.is_some());
    let references = scoping.get_resolved_reference_ids(symbol).len();
    let expected = references.checked_sub(usize::from(rotator.is_some()))?;
    let (decoder, reads) = body
        .get(after..)?
        .iter()
        .find_map(|stmt| decoder_reads(stmt, scoping, symbol).map(|reads| (stmt.span(), reads)))?;
    if reads.len() != expected {
        return None;
    }
    Some(Legacy {
        array: index,
        rotator,
        symbol,
        name_span: array_name_span(stmt)?,
        decoder,
        reads,
    })
}

fn find_legacy(program: &Program<'_>, scoping: &Scoping) -> Option<Legacy> {
    (0..program.body.len()).find_map(|index| legacy_at(&program.body, index, scoping))
}

fn runs_code_before(body: &[Statement<'_>], index: usize) -> bool {
    body[..index].iter().any(|stmt| {
        !matches!(
            stmt,
            Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_)
        )
    })
}

fn reached_by_name(program: &Program<'_>, cx: &Cx<'_, '_>, legacy: &Legacy) -> Option<Span> {
    let scoping = cx.scoping();
    let script = !cx.opts.nested_root && !untwist_core::semantic::has_module_syntax(program);
    if !script || scoping.symbol_scope_id(legacy.symbol) != scoping.root_scope_id() {
        return None;
    }
    let names = [scoping.symbol_name(legacy.symbol)];
    let machinery: Vec<Span> = std::iter::once(program.body[legacy.array].span())
        .chain(legacy.rotator.map(|(span, _)| span))
        .chain(std::iter::once(legacy.decoder))
        .collect();
    if cx.opts.unsafe_transforms {
        let strings = rustc_hash::FxHashMap::default();
        return syntactic_global_reach((program, scoping), &strings, &names, &machinery);
    }
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &[],
        observe: false,
    };
    global_reach(program, scoping, (None, &machinery), watch).reach
}

fn refusal(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    legacy: &Legacy,
) -> Option<(&'static str, Span)> {
    if legacy_array_guards_dropped() {
        return None;
    }
    let scoping = cx.scoping();
    let statement = program.body[legacy.array].span();
    if let Some(note) = DynamicScopes::of(scoping).refusal(scoping, legacy.symbol) {
        return Some((note, statement));
    }
    if let Some(span) = reached_by_name(program, cx, legacy) {
        return Some((notes::LEGACY_ARRAY_REACHED, span));
    }
    let rotated = legacy.rotator.is_some_and(|(_, count)| count > 0);
    if rotated && !cx.opts.unsafe_transforms && runs_code_before(&program.body, legacy.array) {
        return Some((notes::LEGACY_ROTATOR_AFTER_CODE, statement));
    }
    None
}

fn array_expression<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut ArrayExpression<'a>> {
    let Statement::FunctionDeclaration(function) = stmt else {
        return None;
    };
    let body = function.body.as_mut()?;
    let Statement::VariableDeclaration(declaration) = body.statements.first_mut()? else {
        return None;
    };
    match declaration.declarations.first_mut()?.init.as_mut()? {
        Expression::ArrayExpression(array) => Some(array),
        _ => None,
    }
}

fn array_function<'a>(
    original: &Statement<'a>,
    name: &str,
    count: usize,
    cx: &Cx<'a, '_>,
) -> Option<Statement<'a>> {
    let inner = INNER_NAMES.into_iter().find(|inner| *inner != name)?;
    let text = format!(
        "function {name}() {{ var {inner} = []; {name} = function () {{ return {inner}; }}; return {name}(); }}"
    );
    let source = cx.allocator().alloc_str(&text);
    let parsed = parse_with(cx.allocator(), source, SourceType::cjs());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let mut program = parsed.program;
    let mut statement = program.body.pop()?;
    let Statement::VariableDeclaration(declaration) = original else {
        return None;
    };
    let Some(Expression::ArrayExpression(elements)) =
        declaration.declarations.first()?.init.as_ref()
    else {
        return None;
    };
    let length = elements.elements.len();
    let target = array_expression(&mut statement)?;
    for offset in 0..length {
        let element = elements.elements.get((offset + count) % length)?;
        target.elements.push(element.clone_in(cx.allocator()));
    }
    Some(statement)
}

fn restore_name(program: &mut Program<'_>, index: usize, name_span: Span) -> bool {
    let Some(Statement::FunctionDeclaration(function)) = program.body.get_mut(index) else {
        return false;
    };
    let Some(id) = function.id.as_mut() else {
        return false;
    };
    id.span = name_span;
    true
}

fn array_call<'a>(span: Span, name: &str, cx: &Cx<'a, '_>) -> Expression<'a> {
    let callee = Expression::new_identifier(span, Ident::from_str_in(name, cx), cx);
    let arguments = oxc_allocator::Vec::new_in(cx);
    Expression::new_call_expression(span, callee, None, arguments, false, cx)
}

fn rewrite<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>, legacy: Legacy) -> bool {
    let original = &program.body[legacy.array];
    let span = original.span();
    let name = cx.scoping().symbol_name(legacy.symbol).to_string();
    let count = legacy.rotator.map_or(0, |(_, count)| count);
    let Some(function) = array_function(original, &name, count, &*cx) else {
        return false;
    };
    let mut statements = StmtEdits::new();
    let mut planned = statements.replace(span, vec![function]);
    if let Some((rotator, _)) = legacy.rotator {
        planned &= statements.remove(rotator);
    }
    let mut reads = ExprEdits::new();
    for read in &legacy.reads {
        planned &= reads.insert(*read, array_call(*read, &name, &*cx));
    }
    if !planned {
        return false;
    }
    let wanted = statements.len() + reads.len();
    let applied = reads.apply(program, cx.allocator(), cx.scoping_mut())
        + statements.apply(program, cx.allocator(), cx.scoping_mut());
    let restored = restore_name(program, legacy.array, legacy.name_span);
    let stripped = if cx.opts.unsafe_transforms {
        program
            .body
            .iter_mut()
            .find(|stmt| stmt.span() == legacy.decoder)
            .map_or(0, strip_atob_prologues)
    } else {
        0
    };
    cx.changed_by(applied + stripped);
    if applied != wanted || !restored {
        cx.note(
            notes::planned_and_applied_edits(super::PASS, wanted, applied),
            None,
        );
    }
    applied > 0
}

pub(super) fn lift_legacy_array<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) -> bool {
    let Some(legacy) = find_legacy(program, cx.scoping()) else {
        return false;
    };
    if let Some((note, span)) = refusal(program, cx, &legacy) {
        cx.note(note, Some(span));
        return false;
    }
    rewrite(program, cx, legacy)
}
