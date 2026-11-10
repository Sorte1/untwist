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
use super::cookie_minified::minified_cookie_rotation;
use super::for_init::{declare_assigned_forwarders, hoist_for_init_forwarders};
use super::patched::rotator_builtins_patched;
use super::plain_keys::plain_decoded_keys;
use super::prologue::strip_atob_prologues;
use crate::array_rotator::push_shift_target;
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::decoded::syntactic_global_reach;
use crate::legacy_reach;
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

pub(super) fn is_countdown_loop(stmt: &Statement<'_>, counter: &str, array: &str) -> bool {
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

fn declared_count(init: Option<&ForStatementInit<'_>>, counter: &str) -> Option<usize> {
    let Some(ForStatementInit::VariableDeclaration(declaration)) = init else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let Some(Expression::NumericLiteral(count)) = declarator.init.as_ref() else {
        return None;
    };
    let value = count.value;
    let whole = value.fract() == 0.0 && (1.0..=f64::from(u32::MAX)).contains(&value);
    (id.name == counter && declaration.kind == VariableDeclarationKind::Var && whole)
        .then(|| value as usize - 1)
}

fn inline_countdown(
    function: &Function<'_>,
    arguments: &[Argument<'_>],
    array: &str,
) -> Option<usize> {
    let target = only_parameter(function)?;
    let [argument] = arguments else {
        return None;
    };
    if argument.as_expression().and_then(identifier_name) != Some(array) {
        return None;
    }
    let body = function.body.as_ref()?;
    if function.id.is_some() || !body.directives.is_empty() {
        return None;
    }
    let [Statement::ForStatement(node)] = body.statements.as_slice() else {
        return None;
    };
    let Some(Expression::UpdateExpression(update)) = node.test.as_ref() else {
        return None;
    };
    let SimpleAssignmentTarget::AssignmentTargetIdentifier(counter) = &update.argument else {
        return None;
    };
    let counter = counter.name.as_str();
    let shaped = counter != target
        && node.update.is_none()
        && node
            .test
            .as_ref()
            .is_some_and(|test| predecrements(test, counter))
        && pushes_shifted(&node.body, target);
    if !shaped {
        return None;
    }
    declared_count(node.init.as_ref(), counter)
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
    if let Some(count) = inline_countdown(function, arguments, array) {
        return Some(count);
    }
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

pub(super) fn decoder_function<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k Function<'a>> {
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

fn plain_parameters(function: &Function<'_>) -> bool {
    function.params.rest.is_none()
        && function.params.items.iter().all(|param| {
            param.initializer.is_none()
                && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
        })
}

fn decoder_reads(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> Option<Vec<Span>> {
    let function = decoder_function(stmt)?;
    let reassigning = reassigns_itself(stmt);
    if !indexes_by_first_param(function, scoping, symbol) && !reassigning {
        return None;
    }
    if reassigning && !plain_parameters(function) {
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
    shift_expression(&stmt.expression, param)
}

fn shift_expression(expr: &Expression<'_>, param: &str) -> bool {
    let Expression::AssignmentExpression(assignment) = expr.without_parentheses() else {
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
    read_expression(stmt).is_some_and(|read| reads_array_in(read, scoping, symbol, param))
}

fn reads_array_in(expr: &Expression<'_>, scoping: &Scoping, symbol: SymbolId, param: &str) -> bool {
    let read = match expr.without_parentheses() {
        Expression::SequenceExpression(sequence) => match sequence.expressions.as_slice() {
            [shift, read] if shift_expression(shift, param) => read.without_parentheses(),
            _ => return false,
        },
        other => other,
    };
    let Expression::ComputedMemberExpression(member) = read else {
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
            .or_else(|| minified_cookie_rotation(next, name, (scoping, source)))
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
    found: Vec<Legacy>,
}

impl Finder<'_, '_> {
    fn scan(&mut self, body: &[Statement<'_>], top_level: bool) -> bool {
        let found = (0..body.len())
            .find_map(|index| legacy_at(body, index, (self.scoping, self.source, top_level)));
        let Some(found) = found else {
            return false;
        };
        self.found.push(found);
        true
    }
}

impl<'a> VisitJs<'a> for Finder<'_, 'a> {
    fn visit_program(&mut self, it: &Program<'a>) {
        if self.scan(&it.body, true) {
            return;
        }
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        if self.scan(&it.statements, false) {
            return;
        }
        walk_js::walk_function_body(self, it);
    }
}

fn find_legacies(program: &Program<'_>, scoping: &Scoping) -> Vec<Legacy> {
    let mut finder = Finder {
        scoping,
        source: program.source_text,
        found: Vec::new(),
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
    (cx, dynamic): (&Cx<'_, '_>, &DynamicScopes),
    legacy: &Legacy,
    deferred: &mut bool,
) -> Option<(String, Span)> {
    if legacy_array_guards_dropped() {
        return None;
    }
    let scoping = cx.scoping();
    let statement = legacy.statement;
    if let Some(note) = dynamic.refusal(scoping, legacy.symbol) {
        return Some((note.to_string(), statement));
    }
    if let Some(span) = reached_by_name(program, cx, legacy) {
        if cx.opts.unsafe_transforms || !legacy_reach::defers() {
            return Some((notes::LEGACY_ARRAY_REACHED.to_string(), span));
        }
        *deferred = true;
    }
    if legacy.hoisted && legacy.body_code_before {
        return Some((notes::LEGACY_DECODER_BEFORE_ARRAY.to_string(), statement));
    }
    let rotator = legacy.rotator.filter(|(_, count)| *count > 0);
    let (rotator, _) = rotator.filter(|_| !cx.opts.unsafe_transforms && legacy.code_before)?;
    if legacy.body_code_before {
        return Some((notes::LEGACY_ROTATOR_AFTER_CODE.to_string(), statement));
    }
    let count = legacy.rotator.map_or(0, |(_, count)| count);
    let keys = plain_decoded_keys(program, scoping, (statement, count, legacy.decoder));
    let points_to = cx.opts.rule_disabled(crate::proof::BUILTIN_ALIAS_BITS);
    rotator_builtins_patched(program, scoping, (rotator, keys), points_to).map(|(note, span)| {
        let original = span.start < SYNTHETIC_BASE;
        (note, if original { span } else { statement })
    })
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
    names: Vec<(&'n str, Span)>,
    restored: usize,
}

impl<'a> VisitMut<'a> for RestoreName<'_> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.restored == self.names.len() {
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
        let waiting = id.and_then(|id| {
            let at = self
                .names
                .iter()
                .position(|(name, span)| *name == id.name.as_str() && !span.is_unspanned())?;
            Some((id, at))
        });
        if let Some((id, at)) = waiting {
            id.span = self.names[at].1;
            self.names[at].1 = Span::default();
            self.restored += 1;
            return;
        }
        walk_mut::walk_statement(self, it);
    }
}

struct StripDecoder<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    decoders: Vec<Span>,
    stripped: usize,
}

impl<'a> VisitMut<'a> for StripDecoder<'_, 'a, '_> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.decoders.contains(&it.span()) {
            self.stripped += strip_atob_prologues(it, self.cx);
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

struct Planned {
    name: String,
    name_span: Span,
    decoder: Span,
}

struct Batch<'a> {
    statements: StmtEdits<'a>,
    reads: ExprEdits<'a>,
    checksums: usize,
    checksums_wanted: usize,
    planned: Vec<Planned>,
}

fn plan_rewrite<'a>(
    program: &mut Program<'a>,
    cx: &Cx<'a, '_>,
    (legacy, original): (Legacy, Option<Statement<'a>>),
    batch: &mut Batch<'a>,
) -> bool {
    let name = cx.scoping().symbol_name(legacy.symbol).to_string();
    let Some(function) = original else {
        return false;
    };
    let mut planned = batch.statements.replace(legacy.statement, vec![function]);
    if let Some((rotator, _)) = legacy.rotator {
        planned &= batch.statements.remove(rotator);
    }
    let alias = if legacy.reassigning {
        let mut alias = None;
        edit_statement(program, legacy.decoder, |decoder| {
            alias = hoist_array_call(decoder, &name, cx);
        });
        let Some(alias) = alias else {
            return false;
        };
        Some(alias)
    } else {
        None
    };
    for read in &legacy.reads {
        let replacement = match &alias {
            Some(alias) => Expression::new_identifier(*read, Ident::from_str_in(alias, cx), cx),
            None => array_call(*read, &name, cx),
        };
        planned &= batch.reads.insert(*read, replacement);
    }
    if !planned {
        return false;
    }
    if let Some(span) = legacy.checksum {
        batch.checksums_wanted += 1;
        edit_statement(program, span, |rotator| {
            batch.checksums += usize::from(call_the_array_first(rotator, cx));
        });
    }
    batch.planned.push(Planned {
        name,
        name_span: legacy.name_span,
        decoder: legacy.decoder,
    });
    true
}

fn array_functions<'a>(
    program: &Program<'_>,
    cx: &Cx<'a, '_>,
    legacies: &[Legacy],
) -> Vec<Option<Statement<'a>>> {
    let spans: Vec<Span> = legacies.iter().map(|legacy| legacy.statement).collect();
    statements_at(program, &spans)
        .into_iter()
        .zip(legacies)
        .map(|(original, legacy)| {
            let name = cx.scoping().symbol_name(legacy.symbol);
            let count = legacy.rotator.map_or(0, |(_, count)| count);
            original.and_then(|original| array_function(original, name, count, cx))
        })
        .collect()
}

fn rewrite_all<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>, legacies: Vec<Legacy>) -> usize {
    let functions = array_functions(program, &*cx, &legacies);
    let mut batch = Batch {
        statements: StmtEdits::new(),
        reads: ExprEdits::new(),
        checksums: 0,
        checksums_wanted: 0,
        planned: Vec::new(),
    };
    for unit in legacies.into_iter().zip(functions) {
        if !plan_rewrite(program, &*cx, unit, &mut batch) {
            break;
        }
    }
    if batch.planned.is_empty() {
        return 0;
    }
    let wanted = batch.statements.len() + batch.reads.len() + batch.checksums_wanted;
    let applied = batch.checksums
        + batch.reads.apply(program, cx.allocator(), cx.scoping_mut())
        + batch
            .statements
            .apply(program, cx.allocator(), cx.scoping_mut());
    let mut restore = RestoreName {
        names: batch
            .planned
            .iter()
            .map(|planned| (planned.name.as_str(), planned.name_span))
            .collect(),
        restored: 0,
    };
    restore.visit_program(program);
    let restored = restore.restored;
    let stripped = if cx.opts.unsafe_transforms {
        let mut strip = StripDecoder {
            cx: &*cx,
            decoders: batch
                .planned
                .iter()
                .map(|planned| planned.decoder)
                .collect(),
            stripped: 0,
        };
        strip.visit_program(program);
        strip.stripped
    } else {
        0
    };
    cx.changed_by(applied + stripped);
    if applied != wanted || restored != batch.planned.len() {
        cx.note(
            notes::planned_and_applied_edits(super::PASS, wanted, applied),
            None,
        );
    }
    if applied == 0 {
        return 0;
    }
    batch.planned.len()
}

pub(super) fn lift_legacy_array<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
    refused: &mut Option<(String, Span)>,
) -> bool {
    let found = find_legacies(program, cx.scoping());
    let dynamic = DynamicScopes::of(cx.scoping());
    let mut accepted = Vec::new();
    let mut deferred_names = Vec::new();
    for legacy in found {
        let mut deferred = false;
        if let Some((note, span)) = refusal(program, (cx, &dynamic), &legacy, &mut deferred) {
            if accepted.is_empty() {
                cx.note(note.clone(), Some(span));
                *refused = Some((note, span));
            }
            break;
        }
        deferred_names.push(deferred.then(|| cx.scoping().symbol_name(legacy.symbol).to_string()));
        accepted.push(legacy);
    }
    if accepted.is_empty() {
        return false;
    }
    let lifted = rewrite_all(program, cx, accepted);
    if lifted == 0 {
        return false;
    }
    for name in deferred_names.into_iter().take(lifted).flatten() {
        legacy_reach::defer(name);
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
