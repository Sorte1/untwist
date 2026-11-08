use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator, UpdateOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::ast_util::matcher::{
    call_parts, iife, is_literal_argument, is_plain_declaration, statement_iife,
};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::ast_util::search::statements_at;
use untwist_core::parse::{has_error, parse_with, SourceType};
use untwist_core::pass::Cx;

use super::array_declaration;
use super::checksum::{
    call_the_array_first, checksum_rotator, edit_statement, hoist_array_call, reassigns_itself,
};
use super::cookie::cookie_rotation;
use super::for_init::{declare_assigned_forwarders, hoist_for_init_forwarders};
use super::patched::rotator_builtins_patched;
use super::prologue::strip_atob_prologues;
use crate::array_rotator::push_shift_target;
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::decoded::syntactic_global_reach;
use crate::mutants::legacy_array_guards_dropped;
use crate::notes;
use crate::stamp::{declaration_symbol, string_array_unit, DynamicScopes};

const INNER_NAMES: [&str; 2] = ["a", "b"];

struct Legacy {
    statement: Span,
    code_before: bool,
    body_code_before: bool,
    rotator: Option<(Span, usize)>,
    checksum: Option<Span>,
    symbol: SymbolId,
    name_span: Span,
    decoder: Span,
    hoisted: bool,
    reassigning: bool,
    reads: Vec<Span>,
}

fn is_var_array<'k>(stmt: &'k Statement<'_>) -> Option<(&'k str, usize)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if !is_plain_declaration(declaration.kind) {
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

pub(super) fn parameters<'a>(function: &'a Function<'a>) -> Option<(&'a str, &'a str)> {
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

pub(super) fn pushes_shifted(stmt: &Statement<'_>, array: &str) -> bool {
    let Some(Statement::ExpressionStatement(stmt)) = single_statement(stmt) else {
        return false;
    };
    push_shift_target(stmt.expression.without_parentheses()) == Some(array)
}

pub(super) fn predecrements(expr: &Expression<'_>, counter: &str) -> bool {
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

pub(super) fn is_countdown(function: &Function<'_>, array: &str) -> bool {
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

pub(super) fn incremented(arguments: &[Argument<'_>], count: &str) -> bool {
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

pub(super) fn literal_arithmetic(expr: &Expression<'_>) -> Option<f64> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(number) => Some(number.value),
        Expression::UnaryExpression(unary) => {
            let value = literal_arithmetic(&unary.argument)?;
            match unary.operator {
                UnaryOperator::UnaryNegation => Some(-value),
                UnaryOperator::UnaryPlus => Some(value),
                _ => None,
            }
        }
        Expression::BinaryExpression(binary) => {
            let left = literal_arithmetic(&binary.left)?;
            let right = literal_arithmetic(&binary.right)?;
            match binary.operator {
                BinaryOperator::Addition => Some(left + right),
                BinaryOperator::Subtraction => Some(left - right),
                BinaryOperator::Multiplication => Some(left * right),
                _ => None,
            }
        }
        _ => None,
    }
}

fn rotation_count(arguments: &[Argument<'_>], array: &str) -> Option<usize> {
    let [first, second] = arguments else {
        return None;
    };
    if first.as_expression().and_then(identifier_name) != Some(array) {
        return None;
    }
    let value = literal_arithmetic(second.as_expression()?)?;
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

fn decoder_function<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Function<'a>> {
    match stmt {
        Statement::FunctionDeclaration(function)
            if function.id.is_some() && !function.generator && !function.r#async =>
        {
            Some(function)
        }
        Statement::VariableDeclaration(declaration) if is_plain_declaration(declaration.kind) => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            match declarator.init.as_ref() {
                Some(Expression::FunctionExpression(function)) => Some(function),
                _ => None,
            }
        }
        _ => None,
    }
}

fn decoder_reads(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> Option<Vec<Span>> {
    let function = decoder_function(stmt)?;
    if !indexes_by_first_param(function, scoping, symbol) && !reassigns_itself(stmt) {
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

fn shifts(expr: &Expression<'_>, param: &str) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => id.name == param,
        Expression::AssignmentExpression(assignment) => {
            let target = matches!(&assignment.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == param);
            let shifted = match assignment.operator {
                AssignmentOperator::Subtraction => {
                    is_literal_argument(assignment.right.without_parentheses())
                }
                AssignmentOperator::Assign => coerces(&assignment.right, param),
                _ => false,
            };
            target && shifted
        }
        _ => false,
    }
}

fn coerces(expr: &Expression<'_>, param: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::UnaryPlus && identifier_name(&unary.argument) == Some(param))
}

fn shift_statement(stmt: &Statement<'_>, param: &str) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses() else {
        return false;
    };
    if assignment.operator == AssignmentOperator::Subtraction {
        return matches!(&assignment.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == param)
            && is_literal_argument(assignment.right.without_parentheses());
    }
    let shifted = match assignment.right.without_parentheses() {
        Expression::BinaryExpression(binary) => {
            binary.operator == BinaryOperator::Subtraction
                && identifier_name(&binary.left) == Some(param)
                && is_literal_argument(binary.right.without_parentheses())
        }
        other => coerces(other, param),
    };
    assignment.operator == AssignmentOperator::Assign
        && matches!(&assignment.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == param)
        && shifted
}

fn read_expression<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match stmt {
        Statement::VariableDeclaration(declaration) => match declaration.declarations.as_slice() {
            [declarator] => declarator.init.as_ref(),
            _ => None,
        },
        Statement::ReturnStatement(ret) => ret.argument.as_ref(),
        _ => None,
    }
}

fn reads_array_at(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId, param: &str) -> bool {
    let Some(Expression::ComputedMemberExpression(member)) =
        read_expression(stmt).map(Expression::without_parentheses)
    else {
        return false;
    };
    let reads = Reads {
        symbol: Some(symbol),
        ..Reads::default()
    };
    reads.names(scoping, &member.object).is_some() && shifts(&member.expression, param)
}

fn bare_declaration(stmt: &Statement<'_>) -> bool {
    matches!(stmt, Statement::VariableDeclaration(declaration)
        if declaration.declarations.iter().all(|declarator| declarator.init.is_none()))
}

fn indexes_by_first_param(function: &Function<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let Some(param) = function
        .params
        .items
        .first()
        .and_then(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
    else {
        return false;
    };
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let leading = body
        .statements
        .iter()
        .take_while(|stmt| bare_declaration(stmt))
        .count();
    match &body.statements[leading..] {
        [first, ..] if reads_array_at(first, scoping, symbol, param) => true,
        [first, second, ..] => {
            shift_statement(first, param) && reads_array_at(second, scoping, symbol, param)
        }
        _ => false,
    }
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

fn legacy_at(
    body: &[Statement<'_>],
    index: usize,
    (scoping, source, top_level): (&Scoping, &str, bool),
) -> Option<Legacy> {
    let stmt = body.get(index)?;
    let (name, _) = is_var_array(stmt)?;
    let symbol = declaration_symbol(stmt)?;
    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }
    let rotator = body.get(index + 1).and_then(|next| {
        legacy_rotation(next, name)
            .or_else(|| cookie_rotation(next, name, (scoping, source)))
            .map(|count| (next.span(), count))
    });
    let after = index + 1 + usize::from(rotator.is_some());
    let checksum = rotator
        .is_none()
        .then(|| {
            body.get(after..)?
                .iter()
                .find_map(|next| checksum_rotator(next, name, scoping, symbol))
        })
        .flatten();
    let references = scoping.get_resolved_reference_ids(symbol).len();
    let expected =
        references.checked_sub(usize::from(rotator.is_some()) + usize::from(checksum.is_some()))?;
    let declared_before = body[..index]
        .iter()
        .filter(|stmt| matches!(stmt, Statement::FunctionDeclaration(_)));
    let (decoder, hoisted, reassigning, reads) = body
        .get(after..)?
        .iter()
        .chain(declared_before)
        .find_map(|stmt| {
        decoder_reads(stmt, scoping, symbol).map(|reads| {
            let hoisted = matches!(stmt, Statement::FunctionDeclaration(_));
            (stmt.span(), hoisted, reassigns_itself(stmt), reads)
        })
    })?;
    if reads.len() != expected {
        return None;
    }
    Some(Legacy {
        statement: stmt.span(),
        code_before: !top_level || runs_code_before(body, index),
        body_code_before: runs_code_before(body, index),
        rotator,
        checksum,
        symbol,
        name_span: array_name_span(stmt)?,
        decoder,
        hoisted,
        reassigning,
        reads,
    })
}

struct Finder<'s, 'a> {
    scoping: &'s Scoping,
    source: &'a str,
    found: Option<Legacy>,
}

impl Finder<'_, '_> {
    fn scan(&mut self, body: &[Statement<'_>], top_level: bool) {
        if self.found.is_some() {
            return;
        }
        self.found = (0..body.len())
            .find_map(|index| legacy_at(body, index, (self.scoping, self.source, top_level)));
    }
}

impl<'a> VisitJs<'a> for Finder<'_, 'a> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.scan(&it.body, true);
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        if self.found.is_some() {
            return;
        }
        self.scan(&it.statements, false);
        walk_js::walk_function_body(self, it);
    }
}

fn find_legacy(program: &Program<'_>, scoping: &Scoping) -> Option<Legacy> {
    let mut finder = Finder {
        scoping,
        source: program.source_text,
        found: None,
    };
    finder.visit_program(program);
    finder.found
}

fn bare_var(stmt: &Statement<'_>) -> bool {
    matches!(stmt, Statement::VariableDeclaration(declaration)
    if declaration.kind == VariableDeclarationKind::Var
        && declaration.declarations.iter().all(|declarator| {
            declarator.init.is_none()
                && matches!(declarator.id, BindingPattern::BindingIdentifier(_))
        }))
}

fn runs_code_before(body: &[Statement<'_>], index: usize) -> bool {
    body[..index].iter().any(|stmt| {
        !bare_var(stmt)
            && !matches!(
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
    let machinery: Vec<Span> = std::iter::once(legacy.statement)
        .chain(legacy.rotator.map(|(span, _)| span))
        .chain(legacy.checksum)
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
    let statement = legacy.statement;
    if let Some(note) = DynamicScopes::of(scoping).refusal(scoping, legacy.symbol) {
        return Some((note, statement));
    }
    if let Some(span) = reached_by_name(program, cx, legacy) {
        return Some((notes::LEGACY_ARRAY_REACHED, span));
    }
    if legacy.hoisted && legacy.body_code_before {
        return Some((notes::LEGACY_DECODER_BEFORE_ARRAY, statement));
    }
    let rotator = legacy.rotator.filter(|(_, count)| *count > 0);
    let (rotator, _) = rotator.filter(|_| !cx.opts.unsafe_transforms && legacy.code_before)?;
    if legacy.body_code_before {
        return Some((notes::LEGACY_ROTATOR_AFTER_CODE, statement));
    }
    rotator_builtins_patched(program, scoping, rotator).map(|span| (notes::BUILTIN_PATCHED, span))
}

fn declared_function<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut Function<'a>> {
    match stmt {
        Statement::FunctionDeclaration(function) => Some(function),
        Statement::VariableDeclaration(declaration) => {
            match declaration.declarations.first_mut()?.init.as_mut()? {
                Expression::FunctionExpression(function) => Some(function),
                _ => None,
            }
        }
        _ => None,
    }
}

fn array_expression<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut ArrayExpression<'a>> {
    let body = declared_function(stmt)?.body.as_mut()?;
    let Statement::VariableDeclaration(declaration) = body.statements.first_mut()? else {
        return None;
    };
    match declaration.declarations.first_mut()?.init.as_mut()? {
        Expression::ArrayExpression(array) => Some(array),
        _ => None,
    }
}

fn array_function_text(name: &str, inner: &str, kind: VariableDeclarationKind) -> String {
    let body = format!(
        "{{ var {inner} = []; {name} = function () {{ return {inner}; }}; return {name}(); }}"
    );
    if kind == VariableDeclarationKind::Var {
        format!("function {name}() {body}")
    } else {
        format!("let {name} = function () {body};")
    }
}

fn array_function<'a>(
    original: &Statement<'_>,
    name: &str,
    count: usize,
    cx: &Cx<'a, '_>,
) -> Option<Statement<'a>> {
    let Statement::VariableDeclaration(declaration) = original else {
        return None;
    };
    let inner = INNER_NAMES.into_iter().find(|inner| *inner != name)?;
    let text = array_function_text(name, inner, declaration.kind);
    let source = cx.allocator().alloc_str(&text);
    let parsed = parse_with(cx.allocator(), source, SourceType::cjs());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let mut program = parsed.program;
    let mut statement = program.body.pop()?;
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

struct RestoreName<'n> {
    name: &'n str,
    name_span: Span,
    restored: bool,
}

impl<'a> VisitMut<'a> for RestoreName<'_> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.restored {
            return;
        }
        let synthetic = it.span().start >= SYNTHETIC_BASE;
        let id = match it {
            Statement::FunctionDeclaration(function) if synthetic => function.id.as_mut(),
            Statement::VariableDeclaration(declaration) if synthetic => declaration
                .declarations
                .first_mut()
                .and_then(|declarator| match &mut declarator.id {
                    BindingPattern::BindingIdentifier(id) => Some(&mut **id),
                    _ => None,
                }),
            _ => None,
        };
        if let Some(id) = id.filter(|id| id.name == self.name) {
            id.span = self.name_span;
            self.restored = true;
            return;
        }
        walk_mut::walk_statement(self, it);
    }
}

struct StripDecoder {
    decoder: Span,
    stripped: usize,
}

impl<'a> VisitMut<'a> for StripDecoder {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if it.span() == self.decoder {
            self.stripped += strip_atob_prologues(it);
            return;
        }
        walk_mut::walk_statement(self, it);
    }
}

fn array_call<'a>(span: Span, name: &str, cx: &Cx<'a, '_>) -> Expression<'a> {
    let callee = Expression::new_identifier(span, Ident::from_str_in(name, cx), cx);
    let arguments = oxc_allocator::Vec::new_in(cx);
    Expression::new_call_expression(span, callee, None, arguments, false, cx)
}

fn rewrite<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>, legacy: Legacy) -> bool {
    let span = legacy.statement;
    let name = cx.scoping().symbol_name(legacy.symbol).to_string();
    let count = legacy.rotator.map_or(0, |(_, count)| count);
    let function = {
        let view: &Program<'_> = program;
        statements_at(view, &[span])
            .into_iter()
            .next()
            .flatten()
            .and_then(|original| array_function(original, &name, count, &*cx))
    };
    let Some(function) = function else {
        return false;
    };
    let mut statements = StmtEdits::new();
    let mut planned = statements.replace(span, vec![function]);
    if let Some((rotator, _)) = legacy.rotator {
        planned &= statements.remove(rotator);
    }
    let alias = if legacy.reassigning {
        let mut alias = None;
        edit_statement(program, legacy.decoder, |decoder| {
            alias = hoist_array_call(decoder, &name, &*cx);
        });
        let Some(alias) = alias else {
            return false;
        };
        Some(alias)
    } else {
        None
    };
    let mut reads = ExprEdits::new();
    for read in &legacy.reads {
        let replacement = match &alias {
            Some(alias) => Expression::new_identifier(*read, Ident::from_str_in(alias, &*cx), &*cx),
            None => array_call(*read, &name, &*cx),
        };
        planned &= reads.insert(*read, replacement);
    }
    if !planned {
        return false;
    }
    let mut checksum = 0;
    if let Some(span) = legacy.checksum {
        edit_statement(program, span, |rotator| {
            checksum = usize::from(call_the_array_first(rotator, &*cx));
        });
    }
    let wanted = statements.len() + reads.len() + usize::from(legacy.checksum.is_some());
    let applied = checksum
        + reads.apply(program, cx.allocator(), cx.scoping_mut())
        + statements.apply(program, cx.allocator(), cx.scoping_mut());
    let mut restore = RestoreName {
        name: &name,
        name_span: legacy.name_span,
        restored: false,
    };
    restore.visit_program(program);
    let restored = restore.restored;
    let stripped = if cx.opts.unsafe_transforms {
        let mut strip = StripDecoder {
            decoder: legacy.decoder,
            stripped: 0,
        };
        strip.visit_program(program);
        strip.stripped
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
    if !rewrite(program, cx, legacy) {
        return false;
    }
    LIFTED.with(|lifted| lifted.set(true));
    let moved = hoist_for_init_forwarders(program, cx) + declare_assigned_forwarders(program, cx);
    cx.changed_by(moved);
    true
}

thread_local! {
    static LIFTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn forget_lifts() {
    LIFTED.with(|lifted| lifted.set(false));
}

pub(crate) fn lifted_any() -> bool {
    LIFTED.with(std::cell::Cell::get)
}
