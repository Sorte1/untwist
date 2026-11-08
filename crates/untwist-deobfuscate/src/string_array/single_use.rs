use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, GetSpanMut, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::ast_util::respan::{fresh_spans_statement, synthetic_spans_exhausted};
use untwist_core::pass::Cx;

use super::array_declaration;
use super::checksum::parsed_statement;
use crate::mutants::single_use_index_trusted;
use crate::stamp::{declaration_symbol, DynamicScopes};

const DECODER_SUFFIX: &str = "_decode";
const MAX_ARRAY_REFERENCES: usize = 3;

fn symbol_of(scoping: &Scoping, expr: &Expression<'_>) -> Option<SymbolId> {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

struct Free<'s> {
    scoping: &'s Scoping,
    allowed: &'s [SymbolId],
    other: bool,
}

impl<'a> VisitJs<'a> for Free<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.other |= !symbol.is_some_and(|symbol| self.allowed.contains(&symbol));
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.other = true;
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {
        self.other = true;
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {
        self.other = true;
    }
}

fn only_reads(expr: &Expression<'_>, scoping: &Scoping, allowed: &[SymbolId]) -> bool {
    let mut free = Free {
        scoping,
        allowed,
        other: false,
    };
    free.visit_expression(expr);
    !free.other
}

fn array_read<'k, 'a>(
    expr: &'k Expression<'a>,
    scoping: &Scoping,
    arrays: &[SymbolId],
) -> Option<(SymbolId, &'k ComputedMemberExpression<'a>)> {
    let Expression::ComputedMemberExpression(member) = expr.without_parentheses() else {
        return None;
    };
    let array = symbol_of(scoping, &member.object).filter(|symbol| arrays.contains(symbol))?;
    Some((array, &**member))
}

fn parameter_symbols(function: &Function<'_>) -> Option<Vec<SymbolId>> {
    if function.params.rest.is_some() {
        return None;
    }
    function
        .params
        .items
        .iter()
        .map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) if param.initializer.is_none() => {
                id.symbol_id.get()
            }
            _ => None,
        })
        .collect()
}

fn inline_decoder(function: &Function<'_>, scoping: &Scoping, arrays: &[SymbolId]) -> Option<SymbolId> {
    if function.id.is_some() || function.r#async || function.generator {
        return None;
    }
    let body = function.body.as_ref()?;
    let params = parameter_symbols(function)?;
    if params.is_empty() || !body.directives.is_empty() {
        return None;
    }
    let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return None;
    };
    let (array, member) = array_read(ret.argument.as_ref()?, scoping, arrays)?;
    only_reads(&member.expression, scoping, &params[..1]).then_some(array)
}

fn sequence_decoder(
    expr: &Expression<'_>,
    scoping: &Scoping,
    arrays: &[SymbolId],
) -> Option<(SymbolId, SymbolId)> {
    let Expression::SequenceExpression(sequence) = expr.without_parentheses() else {
        return None;
    };
    let [Expression::AssignmentExpression(assignment), read] = sequence.expressions.as_slice() else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let index = scoping
        .get_reference(target.reference_id.get()?)
        .symbol_id()?;
    let literal = matches!(
        assignment.right.without_parentheses(),
        Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    );
    let (array, member) = array_read(read, scoping, arrays)?;
    let indexed = only_reads(&member.expression, scoping, &[index]);
    (literal && indexed).then_some((array, index))
}

#[derive(Clone, Copy)]
enum Site {
    Called(Span),
    Sequenced(Span, SymbolId),
}

struct Sites<'s> {
    scoping: &'s Scoping,
    arrays: &'s [SymbolId],
    found: Vec<(SymbolId, Site)>,
}

impl<'a> VisitJs<'a> for Sites<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::CallExpression(call) = it.without_parentheses() {
            if let Expression::FunctionExpression(function) = call.callee.without_parentheses() {
                if let Some(array) = inline_decoder(function, self.scoping, self.arrays) {
                    self.found.push((array, Site::Called(call.callee.span())));
                }
            }
        }
        if let Some((array, index)) = sequence_decoder(it, self.scoping, self.arrays) {
            self.found.push((array, Site::Sequenced(it.span(), index)));
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
}

fn sites_of(
    statements: &[Statement<'_>],
    scoping: &Scoping,
    arrays: &[SymbolId],
) -> Vec<(SymbolId, Site)> {
    let mut sites = Sites {
        scoping,
        arrays,
        found: Vec::new(),
    };
    for stmt in statements {
        sites.visit_statement(stmt);
    }
    sites.found
}

fn only_site(sites: &[(SymbolId, Site)], array: SymbolId) -> Option<Site> {
    let mut of_array = sites.iter().filter(|(symbol, _)| *symbol == array);
    match (of_array.next(), of_array.next()) {
        (Some((_, site)), None) => Some(*site),
        _ => None,
    }
}

fn index_is_private(scoping: &Scoping, index: SymbolId, array: SymbolId) -> bool {
    if single_use_index_trusted() {
        return true;
    }
    let declared_beside = scoping.symbol_scope_id(index) == scoping.symbol_scope_id(array)
        && scoping
            .symbol_flags(index)
            .contains(SymbolFlags::FunctionScopedVariable);
    let references = scoping.get_resolved_reference_ids(index).len();
    declared_beside
        && references == 2
        && scoping.symbol_redeclarations(index).is_empty()
        && DynamicScopes::of(scoping).refusal(scoping, index).is_none()
}

fn fresh_name(scoping: &Scoping, base: &str) -> Option<String> {
    let taken = |name: &str| {
        scoping.symbol_names().any(|taken| taken == name)
            || scoping.root_unresolved_references().contains_key(name)
    };
    (0..64)
        .map(|n| format!("{base}{DECODER_SUFFIX}{n}"))
        .find(|candidate| !taken(candidate))
}

struct Plan {
    array: SymbolId,
    site: Site,
    name: String,
}

fn candidate(stmt: &Statement<'_>, scoping: &Scoping) -> Option<SymbolId> {
    array_declaration(stmt)?;
    let array = declaration_symbol(stmt)?;
    let references = scoping.get_resolved_reference_ids(array).len();
    let candidate = references <= MAX_ARRAY_REFERENCES
        && scoping.symbol_scope_id(array) != scoping.root_scope_id()
        && scoping.symbol_redeclarations(array).is_empty();
    candidate.then_some(array)
}

fn plan(statements: &[Statement<'_>], scoping: &Scoping) -> Option<Plan> {
    let arrays: Vec<SymbolId> = statements
        .iter()
        .filter_map(|stmt| candidate(stmt, scoping))
        .collect();
    if arrays.is_empty() {
        return None;
    }
    let sites = sites_of(statements, scoping, &arrays);
    arrays.iter().find_map(|array| {
        let site = only_site(&sites, *array)?;
        if DynamicScopes::of(scoping).refusal(scoping, *array).is_some() {
            return None;
        }
        if let Site::Sequenced(_, index) = site {
            if !index_is_private(scoping, index, *array) {
                return None;
            }
        }
        Some(Plan {
            array: *array,
            site,
            name: fresh_name(scoping, scoping.symbol_name(*array))?,
        })
    })
}

struct Replace<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    plan: &'c Plan,
    declaration: Option<Statement<'a>>,
}

fn returned<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    match function.body.as_mut()?.statements.first_mut()? {
        Statement::ReturnStatement(ret) => ret.argument.as_mut(),
        _ => None,
    }
}

fn first_argument<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    let Statement::ExpressionStatement(stmt) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = &mut stmt.expression else {
        return None;
    };
    call.arguments.first_mut()?.as_expression_mut()
}

fn parameter_inside(statement: &mut Statement<'_>) {
    let Statement::FunctionDeclaration(function) = statement else {
        return;
    };
    let start = function.span.start;
    if let Some(BindingPattern::BindingIdentifier(id)) =
        function.params.items.first_mut().map(|param| &mut param.pattern)
    {
        id.span = Span::new(start, start + 1);
    }
}

fn called_expression<'a>(statement: Statement<'a>, cx: &Cx<'a, '_>) -> Option<Expression<'a>> {
    match statement {
        Statement::ExpressionStatement(mut stmt) => Some(stmt.expression.take_in(cx)),
        _ => None,
    }
}

impl<'a> Replace<'_, 'a, '_> {
    fn called(&mut self, callee: &mut Expression<'a>) {
        let target = callee.without_parentheses_mut();
        if !matches!(target, Expression::FunctionExpression(_)) {
            return;
        }
        let (Some(Statement::ExpressionStatement(mut name)), Some(Statement::FunctionDeclaration(mut declared))) = (
            parsed_statement(&format!("{};", self.plan.name), self.cx),
            parsed_statement(&format!("function {}() {{}}", self.plan.name), self.cx),
        ) else {
            return;
        };
        let Expression::FunctionExpression(mut function) =
            std::mem::replace(target, name.expression.take_in(self.cx))
        else {
            return;
        };
        function.r#type = FunctionType::FunctionDeclaration;
        function.id = declared.id.take();
        self.declaration = Some(Statement::FunctionDeclaration(function));
    }

    fn sequenced(&mut self, expr: &mut Expression<'a>) {
        let scoping = self.cx.scoping();
        let Some((_, index)) = sequence_decoder(expr, scoping, &[self.plan.array]) else {
            return;
        };
        let parameter = scoping.symbol_name(index).to_string();
        let name = &self.plan.name;
        let (Some(mut declaration), Some(mut call)) = (
            parsed_statement(&format!("function {name}({parameter}) {{ return 0; }}"), self.cx),
            parsed_statement(&format!("{name}(0);"), self.cx),
        ) else {
            return;
        };
        let (Some(body), Some(argument)) = (returned(&mut declaration), first_argument(&mut call))
        else {
            return;
        };
        if synthetic_spans_exhausted() {
            return;
        }
        let Expression::SequenceExpression(sequence) = expr.without_parentheses_mut() else {
            return;
        };
        let read = sequence.expressions.pop();
        let assignment = sequence.expressions.pop();
        let (Some(read), Some(Expression::AssignmentExpression(mut assignment))) = (read, assignment)
        else {
            return;
        };
        *body = read;
        *argument = assignment.right.take_in(self.cx);
        if fresh_spans_statement(&mut declaration) {
            parameter_inside(&mut declaration);
        }
        let span = expr.span();
        let Some(mut call) = called_expression(call, self.cx) else {
            return;
        };
        *call.span_mut() = span;
        *expr = call;
        self.declaration = Some(declaration);
    }
}

impl<'a> VisitMut<'a> for Replace<'_, 'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.declaration.is_some() {
            return;
        }
        match self.plan.site {
            Site::Sequenced(span, _) if it.span() == span => {
                self.sequenced(it);
                return;
            }
            Site::Called(span) => {
                if let Expression::CallExpression(call) = it.without_parentheses_mut() {
                    if call.callee.span() == span {
                        self.called(&mut call.callee);
                        return;
                    }
                }
            }
            Site::Sequenced(..) => {}
        }
        walk_mut::walk_expression(self, it);
    }
}

pub(super) fn declare_single_use_decoder<'a>(
    statements: &mut AVec<'a, Statement<'a>>,
    cx: &Cx<'a, '_>,
) -> usize {
    let Some(plan) = plan(statements, cx.scoping()) else {
        return 0;
    };
    let mut replace = Replace {
        cx,
        plan: &plan,
        declaration: None,
    };
    for stmt in statements.iter_mut() {
        replace.visit_statement(stmt);
    }
    let Some(declaration) = replace.declaration else {
        return 0;
    };
    if let Site::Sequenced(_, index) = plan.site {
        drop_bare_declarator(statements, index);
    }
    statements.push(declaration);
    1
}

fn drop_bare_declarator(statements: &mut AVec<'_, Statement<'_>>, index: SymbolId) {
    let bare = |declarator: &VariableDeclarator<'_>| {
        declarator.init.is_none()
            && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                if id.symbol_id.get() == Some(index))
    };
    for stmt in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = stmt {
            if declaration.kind == VariableDeclarationKind::Var {
                declaration.declarations.retain(|declarator| !bare(declarator));
            }
        }
    }
    statements.retain(|stmt| {
        !matches!(stmt, Statement::VariableDeclaration(declaration)
            if declaration.declarations.is_empty())
    });
}
