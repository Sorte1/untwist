use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use oxc_traverse::Ancestor;
use untwist_core::ast_util::literal::is_safe_literal;
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass, StatementKind,
    StatementVariants,
};

use crate::sequence_hazard::is_hoist_order_safe;

#[derive(Default)]
pub struct Sequence;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Sequence)
}

fn is_sequence(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::SequenceExpression(seq) if seq.expressions.len() >= 2)
}

fn take_sequence_parts<'a>(
    expr: &mut Expression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<(AVec<'a, Expression<'a>>, Expression<'a>)> {
    if !is_sequence(expr) {
        return None;
    }
    let span = expr.span();
    let placeholder = Expression::new_null_literal(span, cx);
    let old = std::mem::replace(expr, placeholder);
    let seq = match old {
        Expression::SequenceExpression(seq) => seq,
        other => {
            *expr = other;
            return None;
        }
    };
    let mut expressions = seq.unbox().expressions;
    let Some(last) = expressions.pop() else {
        *expr = Expression::new_sequence_expression(span, expressions, cx);
        return None;
    };
    Some((expressions, last))
}

fn push_expression_statement<'a>(
    out: &mut AVec<'a, Statement<'a>>,
    expr: Expression<'a>,
    cx: &Cx<'a, '_>,
) {
    let span = expr.span();
    out.push(Statement::new_expression_statement(span, expr, cx));
}

fn hoist_head<'a>(expr: &mut Expression<'a>, out: &mut AVec<'a, Statement<'a>>, cx: &Cx<'a, '_>) {
    let Some((leading, last)) = take_sequence_parts(expr, cx) else {
        return;
    };
    for expression in leading {
        push_expression_statement(out, expression, cx);
    }
    *expr = last;
}

fn declarator_has_sequence_init(declaration: &VariableDeclaration<'_>) -> bool {
    declaration.declarations.len() == 1
        && declaration
            .declarations
            .first()
            .is_some_and(|declarator| declarator.init.as_ref().is_some_and(is_sequence))
}

fn hoist_declarator<'a>(
    declaration: &mut VariableDeclaration<'a>,
    out: &mut AVec<'a, Statement<'a>>,
    cx: &Cx<'a, '_>,
) {
    if declaration.declarations.len() != 1 {
        return;
    }
    let Some(declarator) = declaration.declarations.first_mut() else {
        return;
    };
    let Some(init) = &mut declarator.init else {
        return;
    };
    hoist_head(init, out, cx);
}

fn is_fast_path(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ExpressionStatement(node) => is_sequence(&node.expression),
        Statement::ReturnStatement(node) => node.argument.as_ref().is_some_and(is_sequence),
        _ => false,
    }
}

fn is_for_init_expandable(init: &Option<ForStatementInit<'_>>) -> bool {
    match init {
        Some(ForStatementInit::SequenceExpression(seq)) => seq.expressions.len() >= 2,
        Some(ForStatementInit::VariableDeclaration(declaration)) => {
            declarator_has_sequence_init(declaration)
        }
        _ => false,
    }
}

fn is_lexical_declaration(declaration: &VariableDeclaration<'_>) -> bool {
    declaration.kind != VariableDeclarationKind::Var
}

fn has_closure(expr: &Expression<'_>) -> bool {
    let mut finder = ClosureFinder::default();
    finder.visit_expression(expr);
    finder.found
}

fn declared_names<'a>(declaration: &VariableDeclaration<'a>) -> Vec<&'a str> {
    let mut bound = BoundNames::default();
    for declarator in &declaration.declarations {
        bound.visit_binding_pattern(&declarator.id);
    }
    bound.names
}

fn reads_any(expr: &Expression<'_>, names: &[&str]) -> bool {
    let mut reader = NameReader {
        names,
        found: false,
    };
    reader.visit_expression(expr);
    reader.found
}

fn leading_disturbs_head(expr: &Expression<'_>, names: &[&str]) -> bool {
    let Expression::SequenceExpression(seq) = expr else {
        return false;
    };
    let leading = seq.expressions.len().saturating_sub(1);
    seq.expressions
        .iter()
        .take(leading)
        .any(|part| has_closure(part) || reads_any(part, names))
}

#[inline(never)]
fn lexical_init_disturbs_head(init: &Option<ForStatementInit<'_>>) -> bool {
    let Some(ForStatementInit::VariableDeclaration(declaration)) = init else {
        return false;
    };
    if !is_lexical_declaration(declaration) {
        return false;
    }
    let names = declared_names(declaration);
    declaration
        .declarations
        .iter()
        .filter_map(|declarator| declarator.init.as_ref())
        .any(|init| leading_disturbs_head(init, &names))
}

#[inline(never)]
fn is_loop_head_movable(left: &ForStatementLeft<'_>, right: &Expression<'_>) -> bool {
    if !is_sequence(right) {
        return false;
    }
    match left {
        ForStatementLeft::VariableDeclaration(declaration)
            if is_lexical_declaration(declaration) =>
        {
            !leading_disturbs_head(right, &declared_names(declaration))
        }
        _ => true,
    }
}

fn is_expandable(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ExpressionStatement(node) => is_sequence(&node.expression),
        Statement::ReturnStatement(node) => node.argument.as_ref().is_some_and(is_sequence),
        Statement::IfStatement(node) => is_sequence(&node.test),
        Statement::SwitchStatement(node) => is_sequence(&node.discriminant),
        Statement::ThrowStatement(node) => is_sequence(&node.argument),
        Statement::ForInStatement(node) => is_loop_head_movable(&node.left, &node.right),
        Statement::ForOfStatement(node) => is_loop_head_movable(&node.left, &node.right),
        Statement::ForStatement(node) => {
            is_for_init_expandable(&node.init) && !lexical_init_disturbs_head(&node.init)
        }
        Statement::VariableDeclaration(node) => declarator_has_sequence_init(node),
        _ => false,
    }
}

fn expand_for_statement_init<'a>(
    node: &mut ForStatement<'a>,
    out: &mut AVec<'a, Statement<'a>>,
    cx: &Cx<'a, '_>,
) {
    if matches!(node.init, Some(ForStatementInit::SequenceExpression(_))) {
        let Some(ForStatementInit::SequenceExpression(seq)) = node.init.take() else {
            unreachable!()
        };
        for expression in seq.unbox().expressions {
            push_expression_statement(out, expression, cx);
        }
        return;
    }
    let Some(ForStatementInit::VariableDeclaration(declaration)) = &mut node.init else {
        return;
    };
    hoist_declarator(declaration, out, cx);
}

fn expand<'a>(mut stmt: Statement<'a>, cx: &Cx<'a, '_>) -> AVec<'a, Statement<'a>> {
    let mut out = AVec::new_in(cx);
    let mut consumed = false;
    match &mut stmt {
        Statement::ExpressionStatement(node) => {
            if let Some((leading, last)) = take_sequence_parts(&mut node.expression, cx) {
                for expression in leading {
                    push_expression_statement(&mut out, expression, cx);
                }
                push_expression_statement(&mut out, last, cx);
                consumed = true;
            }
        }
        Statement::ReturnStatement(node) => {
            if let Some(argument) = &mut node.argument {
                hoist_head(argument, &mut out, cx);
            }
        }
        Statement::IfStatement(node) => hoist_head(&mut node.test, &mut out, cx),
        Statement::SwitchStatement(node) => hoist_head(&mut node.discriminant, &mut out, cx),
        Statement::ThrowStatement(node) => hoist_head(&mut node.argument, &mut out, cx),
        Statement::ForInStatement(node) => hoist_head(&mut node.right, &mut out, cx),
        Statement::ForOfStatement(node) => hoist_head(&mut node.right, &mut out, cx),
        Statement::ForStatement(node) => expand_for_statement_init(node, &mut out, cx),
        Statement::VariableDeclaration(node) => hoist_declarator(node, &mut out, cx),
        _ => {}
    }
    if !consumed {
        out.push(stmt);
    }
    out
}

#[derive(Default)]
struct BoundNames<'a> {
    names: Vec<&'a str>,
}

impl<'a> VisitJs<'a> for BoundNames<'a> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.push(it.name.as_str());
    }
}

struct NameReader<'n, 'a> {
    names: &'n [&'a str],
    found: bool,
}

impl<'a> VisitJs<'a> for NameReader<'_, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= self.names.contains(&it.name.as_str());
    }
}

#[derive(Default)]
struct ClosureFinder {
    found: bool,
}

impl<'a> VisitJs<'a> for ClosureFinder {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {
        self.found = true;
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {
        self.found = true;
    }

    fn visit_class(&mut self, _it: &Class<'a>) {
        self.found = true;
    }
}

fn captures_per_iteration_bindings(node: &ForStatement<'_>) -> bool {
    let lexical = matches!(
        &node.init,
        Some(ForStatementInit::VariableDeclaration(declaration))
            if declaration.kind != VariableDeclarationKind::Var
    );
    if !lexical {
        return false;
    }
    node.update.as_ref().is_some_and(has_closure) || node.test.as_ref().is_some_and(has_closure)
}

fn move_update_into_body<'a>(node: &mut ForStatement<'a>, cx: &mut Cx<'a, '_>) -> bool {
    if !matches!(node.body, Statement::EmptyStatement(_)) {
        return false;
    }
    if !node.update.as_ref().is_some_and(is_sequence) {
        return false;
    }
    if captures_per_iteration_bindings(node) {
        return false;
    }
    let mut body = AVec::new_in(&*cx);
    if let Some(update) = &mut node.update {
        hoist_head(update, &mut body, &*cx);
    }
    let span = node.body.span();
    node.body = cx.block(span, body);
    true
}

fn in_single_statement_slot(cx: &Cx<'_, '_>) -> bool {
    matches!(
        cx.tcx.parent(),
        Ancestor::IfStatementConsequent(_)
            | Ancestor::IfStatementAlternate(_)
            | Ancestor::DoWhileStatementBody(_)
            | Ancestor::WhileStatementBody(_)
            | Ancestor::ForStatementBody(_)
            | Ancestor::ForInStatementBody(_)
            | Ancestor::ForOfStatementBody(_)
            | Ancestor::WithStatementBody(_)
            | Ancestor::LabeledStatementBody(_)
    )
}

fn is_hoistable_target(target: &AssignmentTarget<'_>) -> bool {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(_) => true,
        AssignmentTarget::StaticMemberExpression(member) => {
            matches!(member.object, Expression::Identifier(_))
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            matches!(member.object, Expression::Identifier(_))
                && (matches!(member.expression, Expression::Identifier(_))
                    || is_safe_literal(&member.expression))
        }
        _ => false,
    }
}

fn collapse_literal_sequence<'a>(expr: &mut Expression<'a>, cx: &Cx<'a, '_>) -> bool {
    let Expression::SequenceExpression(seq) = &*expr else {
        return false;
    };
    if seq.expressions.len() < 2 || !seq.expressions.iter().all(is_safe_literal) {
        return false;
    }
    let Some((_, last)) = take_sequence_parts(expr, cx) else {
        return false;
    };
    *expr = last;
    true
}

fn hoist_assignment_sequence<'a>(expr: &mut Expression<'a>, cx: &Cx<'a, '_>) -> bool {
    let leading = {
        let Expression::AssignmentExpression(assign) = &mut *expr else {
            return false;
        };
        if matches!(
            assign.operator,
            AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalNullish
        ) {
            return false;
        }
        if !is_sequence(&assign.right)
            || !is_hoistable_target(&assign.left)
            || !is_hoist_order_safe(assign, cx)
        {
            return false;
        }
        let Some((leading, last)) = take_sequence_parts(&mut assign.right, cx) else {
            return false;
        };
        assign.right = last;
        leading
    };
    let span = expr.span();
    let placeholder = Expression::new_null_literal(span, cx);
    let assignment = std::mem::replace(expr, placeholder);
    let mut expressions = leading;
    expressions.push(assignment);
    *expr = Expression::new_sequence_expression(span, expressions, cx);
    true
}

fn expand_matching_statements<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
    matches_site: fn(&Statement<'_>) -> bool,
) {
    if !stmts.iter().any(matches_site) {
        return;
    }
    let capacity = stmts.len() + 4;
    let old = std::mem::replace(stmts, AVec::with_capacity_in(capacity, &*cx));
    for stmt in old {
        if !matches_site(&stmt) {
            stmts.push(stmt);
            continue;
        }
        for expanded in expand(stmt, &*cx) {
            stmts.push(expanded);
        }
        cx.changed();
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::SequenceExpression,
    ExpressionKind::AssignmentExpression,
]);

const ACTED_ON_STATEMENTS: StatementVariants = StatementVariants::of(&[
    StatementKind::ExpressionStatement,
    StatementKind::ReturnStatement,
    StatementKind::IfStatement,
    StatementKind::SwitchStatement,
    StatementKind::ThrowStatement,
    StatementKind::ForInStatement,
    StatementKind::ForOfStatement,
    StatementKind::ForStatement,
    StatementKind::VariableDeclaration,
]);

impl<'a> Pass<'a> for Sequence {
    fn name(&self) -> &'static str {
        "sequence"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_STATEMENTS
            | Interest::EXIT_STATEMENTS
            | Interest::EXIT_STATEMENT
            | Interest::EXIT_EXPRESSION)
            .on_expressions(ACTED_ON_EXPRESSIONS)
            .on_statements(ACTED_ON_STATEMENTS)
    }

    fn enter_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        expand_matching_statements(stmts, cx, is_fast_path);
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        expand_matching_statements(stmts, cx, is_expandable);
    }

    fn exit_statement(&mut self, stmt: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if let Statement::ForStatement(node) = stmt {
            if move_update_into_body(node, cx) {
                cx.changed();
            }
        }
        if !in_single_statement_slot(cx) || !is_expandable(stmt) {
            return;
        }
        if matches!(stmt, Statement::VariableDeclaration(node) if node.kind.is_lexical()) {
            cx.binding_moved();
        }
        let span = stmt.span();
        let placeholder = Statement::new_empty_statement(span, &*cx);
        let taken = std::mem::replace(stmt, placeholder);
        let body = expand(taken, &*cx);
        *stmt = cx.block(span, body);
        cx.changed();
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if collapse_literal_sequence(expr, &*cx) {
            cx.changed();
            return;
        }
        if hoist_assignment_sequence(expr, &*cx) {
            cx.changed();
        }
    }
}
