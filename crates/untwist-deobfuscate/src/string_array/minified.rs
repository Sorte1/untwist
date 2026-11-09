use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::{GetSpan, SPAN};
use oxc_str::Ident;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use untwist_core::ast_util::matcher::{infinite_loop_body, is_plain_declaration};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::Cx;

use super::checksum::{sees_its_parameters, unused_name};
use super::{array_literal, match_string_array};
use crate::array_rotator::is_infinite_rotator_loop;
use crate::mutants::minified_rotators_trusted;
use crate::probe::contains_expression;
use crate::stamp::string_array_unit;
use untwist_core::ast_util::matcher::var_function_or_declaration;

fn minified_array_name<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if !is_plain_declaration(declaration.kind) {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let length = array_literal(declarator.init.as_ref()?)?;
    let name = id.name.as_str();
    (length > 0 && string_array_unit(name).is_none()).then_some(name)
}

fn function_array_name<'k>(stmt: &'k Statement<'_>) -> Option<&'k str> {
    match_string_array(stmt)?;
    let name = var_function_or_declaration(stmt)?.name;
    string_array_unit(name).is_none().then_some(name)
}

fn iife_body<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k [Statement<'a>]> {
    let called = match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    let Expression::CallExpression(call) = called else {
        return None;
    };
    match call.callee.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => Some(body.statements.as_slice()),
            _ => None,
        },
        Expression::FunctionExpression(function) => function
            .body
            .as_ref()
            .map(|body| body.statements.as_slice()),
        _ => None,
    }
}

fn is_function_array_rotator(expr: &Expression<'_>, array: &str) -> bool {
    let Some(body) = iife_body(expr) else {
        return false;
    };
    body.iter().any(is_infinite_rotator_loop)
        && body
            .iter()
            .any(|stmt| contains_expression(stmt, |found| identifier_name(found) == Some(array)))
}

fn sequence_head<'k, 'a>(expr: Option<&'k Expression<'a>>) -> Option<&'k Expression<'a>> {
    match expr.map(Expression::without_parentheses) {
        Some(Expression::SequenceExpression(sequence)) => sequence.expressions.first(),
        _ => None,
    }
}

fn declaration_led_by_rotator(stmt: &Statement<'_>, array: &str) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    declaration.declarations.iter().any(|declarator| {
        sequence_head(declarator.init.as_ref())
            .is_some_and(|head| is_function_array_rotator(head, array))
    })
}

fn statement_led_by_rotator(stmt: &Statement<'_>, array: &str) -> bool {
    let Statement::ExpressionStatement(node) = stmt else {
        return false;
    };
    sequence_head(Some(&node.expression)).is_some_and(|head| is_function_array_rotator(head, array))
}

fn is_sequence(expr: Option<&Expression<'_>>) -> bool {
    matches!(
        expr.map(Expression::without_parentheses),
        Some(Expression::SequenceExpression(_))
    )
}

fn joined_declaration(stmt: &Statement<'_>, array: &str) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    minified_array_name(stmt) == Some(array)
        && (declaration.declarations.len() > 1
            || declaration
                .declarations
                .iter()
                .any(|declarator| is_sequence(declarator.init.as_ref())))
}

fn passes_array(expr: &Expression<'_>, array: &str) -> bool {
    let callee_call = match expr.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    let Expression::CallExpression(call) = callee_call else {
        return false;
    };
    call.arguments
        .iter()
        .any(|argument| argument.as_expression().and_then(identifier_name) == Some(array))
}

fn sequence_led_by_array_call(stmt: &Statement<'_>, array: &str) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let Expression::SequenceExpression(sequence) = stmt.expression.without_parentheses() else {
        return false;
    };
    sequence
        .expressions
        .first()
        .is_some_and(|head| passes_array(head, array))
}

#[derive(Default)]
struct LexicalUses {
    found: bool,
}

impl<'a> VisitJs<'a> for LexicalUses {
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.found {
            walk_js::walk_expression(self, it);
        }
    }
}

fn convertible_arrow(arrow: &ArrowFunctionExpression<'_>) -> bool {
    if arrow.r#async || arrow.type_parameters.is_some() || arrow.return_type.is_some() {
        return false;
    }
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        return false;
    };
    let mut uses = LexicalUses::default();
    uses.visit_formal_parameters(&arrow.params);
    uses.visit_function_body(body);
    !uses.found || minified_rotators_trusted()
}

fn without_parentheses_mut<'k, 'a>(expr: &'k mut Expression<'a>) -> &'k mut Expression<'a> {
    match expr {
        Expression::ParenthesizedExpression(inner) => {
            without_parentheses_mut(&mut inner.expression)
        }
        other => other,
    }
}

fn called_mut<'k, 'a>(expr: &'k mut Expression<'a>) -> Option<&'k mut CallExpression<'a>> {
    match without_parentheses_mut(expr) {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            called_mut(&mut unary.argument)
        }
        Expression::CallExpression(call) => Some(call),
        _ => None,
    }
}

fn arrow_as_function<'a>(callee: &mut Expression<'a>, cx: &Cx<'a, '_>) -> bool {
    let target = without_parentheses_mut(callee);
    let Expression::ArrowFunctionExpression(arrow) = target else {
        return false;
    };
    if !convertible_arrow(arrow) {
        return false;
    }
    let Expression::ArrowFunctionExpression(arrow) = target.take_in(cx) else {
        return false;
    };
    let arrow = arrow.unbox();
    let ArrowFunctionBody::FunctionBody(body) = arrow.body else {
        return false;
    };
    *target = Expression::new_function_expression(
        arrow.span,
        FunctionType::FunctionExpression,
        None,
        false,
        false,
        false,
        None,
        None,
        arrow.params,
        None,
        Some(body),
        cx,
    );
    true
}

fn breaks(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::BreakStatement(node) => node.label.is_none(),
        Statement::BlockStatement(block) => matches!(block.body.as_slice(), [only] if breaks(only)),
        _ => false,
    }
}

fn checksum_test<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    let Statement::IfStatement(node) = stmt else {
        return None;
    };
    if node.alternate.is_some() || !breaks(&node.consequent) {
        return None;
    }
    let Expression::BinaryExpression(binary) = without_parentheses_mut(&mut node.test) else {
        return None;
    };
    if !matches!(
        binary.operator,
        BinaryOperator::StrictEquality | BinaryOperator::Equality
    ) {
        return None;
    }
    let numeric = |expr: &Expression<'_>| matches!(expr, Expression::NumericLiteral(_));
    match (numeric(&binary.left), numeric(&binary.right)) {
        (true, false) => Some(&mut binary.left),
        (false, true) => Some(&mut binary.right),
        _ => None,
    }
}

fn guarded_try<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    let target = match stmt {
        Statement::BlockStatement(block) => match block.body.as_mut_slice() {
            [only] => only,
            _ => return None,
        },
        other => other,
    };
    let Statement::TryStatement(node) = target else {
        return None;
    };
    node.handler.as_ref()?;
    checksum_test(node.block.body.first_mut()?)
}

fn loop_body_mut<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<&'k mut Statement<'a>> {
    infinite_loop_body(stmt)?;
    match stmt {
        Statement::WhileStatement(node) => Some(&mut node.body),
        Statement::ForStatement(node) => Some(&mut node.body),
        _ => None,
    }
}

fn checksum_literal<'k, 'a>(function: &'k mut Function<'a>) -> Option<&'k mut Expression<'a>> {
    let body = function.body.as_mut()?;
    let mut loops = body.statements.iter_mut().filter_map(loop_body_mut);
    let found = loops.next()?;
    if loops.next().is_some() {
        return None;
    }
    guarded_try(found)
}

fn one_parameter(function: &Function<'_>) -> Option<String> {
    let [param] = function.params.items.as_slice() else {
        return None;
    };
    if function.params.rest.is_some() || param.initializer.is_some() {
        return None;
    }
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

fn checksum_as_parameter<'a>(call: &mut CallExpression<'a>, array: &str, cx: &Cx<'a, '_>) -> bool {
    let [only] = call.arguments.as_slice() else {
        return false;
    };
    if only.as_expression().and_then(identifier_name) != Some(array) {
        return false;
    }
    let Expression::FunctionExpression(function) = without_parentheses_mut(&mut call.callee) else {
        return false;
    };
    if function.id.is_some() || function.generator || function.r#async {
        return false;
    }
    let Some(param) = one_parameter(function) else {
        return false;
    };
    if sees_its_parameters(function) && !minified_rotators_trusted() {
        return false;
    }
    let Some(fresh) = unused_name(function, &param) else {
        return false;
    };
    let fresh = Ident::from_str_in(&fresh, cx);
    let Some(literal) = checksum_literal(function) else {
        return false;
    };
    let span = literal.span();
    let value = std::mem::replace(literal, Expression::new_identifier(span, fresh, cx));
    let pattern = BindingPattern::new_binding_identifier(SPAN, fresh, cx);
    let decorators: AVec<'a, Decorator<'a>> = AVec::new_in(cx);
    function.params.items.push(FormalParameter::new(
        SPAN, decorators, pattern, None, None, false, None, false, false, cx,
    ));
    call.arguments.push(Argument::from(value));
    true
}

struct Normalise<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    changed: usize,
}

impl<'a> Normalise<'_, 'a, '_> {
    fn rotator(&mut self, expr: &mut Expression<'a>, array: &str) {
        if !passes_array(expr, array) {
            return;
        }
        let Some(call) = called_mut(expr) else {
            return;
        };
        if arrow_as_function(&mut call.callee, self.cx) {
            self.changed += 1;
        }
        if checksum_as_parameter(call, array, self.cx) {
            self.changed += 1;
        }
    }

    fn split_declaration(&mut self, stmt: Statement<'a>, out: &mut AVec<'a, Statement<'a>>) {
        let Statement::VariableDeclaration(mut declaration) = stmt else {
            return;
        };
        let kind = declaration.kind;
        let declarators = declaration.declarations.take_in(self.cx);
        for mut declarator in declarators {
            if let Some(init) = declarator.init.as_mut() {
                if let Expression::SequenceExpression(sequence) = without_parentheses_mut(init) {
                    let mut expressions = sequence.expressions.take_in(self.cx);
                    let last = expressions.pop();
                    for expression in expressions {
                        let span = expression.span();
                        out.push(Statement::new_expression_statement(
                            span, expression, self.cx,
                        ));
                    }
                    declarator.init = last;
                }
            }
            let span = declarator.span();
            out.push(Statement::new_variable_declaration(
                span,
                kind,
                AVec::from_iter_in([declarator], self.cx),
                false,
                self.cx,
            ));
        }
        self.changed += 1;
    }

    fn split_sequence(&mut self, stmt: Statement<'a>, out: &mut AVec<'a, Statement<'a>>) {
        let Statement::ExpressionStatement(mut node) = stmt else {
            return;
        };
        let Expression::SequenceExpression(sequence) =
            without_parentheses_mut(&mut node.expression)
        else {
            return;
        };
        let span = sequence.span;
        let mut expressions = sequence.expressions.take_in(self.cx);
        let head = expressions.remove(0);
        out.push(Statement::new_expression_statement(
            head.span(),
            head,
            self.cx,
        ));
        let rest = if expressions.len() == 1 {
            expressions.pop()
        } else {
            let start = expressions
                .first()
                .map_or(span.start, |first| first.span().start);
            let rest_span = oxc_span::Span::new(start, span.end);
            Some(Expression::new_sequence_expression(
                rest_span,
                expressions,
                self.cx,
            ))
        };
        if let Some(rest) = rest {
            out.push(Statement::new_expression_statement(
                rest.span(),
                rest,
                self.cx,
            ));
        }
        self.changed += 1;
    }

    fn place_function_array(
        &mut self,
        stmt: Statement<'a>,
        array: &str,
        out: &mut AVec<'a, Statement<'a>>,
    ) {
        if declaration_led_by_rotator(&stmt, array) {
            self.split_declaration(stmt, out);
            return;
        }
        if statement_led_by_rotator(&stmt, array) {
            self.split_sequence(stmt, out);
            return;
        }
        out.push(stmt);
    }

    fn function_array_rotator(&mut self, expr: &mut Expression<'a>, array: &str) {
        if !is_function_array_rotator(expr, array) {
            return;
        }
        let Some(call) = called_mut(expr) else {
            return;
        };
        if arrow_as_function(&mut call.callee, self.cx) {
            self.changed += 1;
        }
    }

    fn function_array_statements(&mut self, it: &mut AVec<'a, Statement<'a>>, array: &str) {
        let taken = it.take_in(self.cx);
        for stmt in taken {
            self.place_function_array(stmt, array, it);
        }
        for stmt in it.iter_mut() {
            if let Statement::ExpressionStatement(node) = stmt {
                self.function_array_rotator(&mut node.expression, array);
            }
        }
    }

    fn place(&mut self, stmt: Statement<'a>, array: &str, out: &mut AVec<'a, Statement<'a>>) {
        if joined_declaration(&stmt, array) {
            self.split_declaration(stmt, out);
            return;
        }
        if sequence_led_by_array_call(&stmt, array) {
            self.split_sequence(stmt, out);
            return;
        }
        out.push(stmt);
    }
}

impl<'a> VisitMut<'a> for Normalise<'_, 'a, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        if let Some(array) = it.iter().find_map(minified_array_name).map(str::to_string) {
            let taken = it.take_in(self.cx);
            for stmt in taken {
                self.place(stmt, &array, it);
            }
            let again = it.take_in(self.cx);
            for mut stmt in again {
                if let Statement::ExpressionStatement(node) = &mut stmt {
                    self.rotator(&mut node.expression, &array);
                }
                it.push(stmt);
            }
        } else if let Some(array) = it.iter().find_map(function_array_name).map(str::to_string) {
            self.function_array_statements(it, &array);
        }
        walk_mut::walk_statements(self, it);
    }
}

pub(super) fn normalise_minified<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    let mut normalise = Normalise { cx, changed: 0 };
    normalise.visit_program(program);
    normalise.changed
}
