use oxc_allocator::UnstableAddress;
use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{ContentEq, Span, SPAN};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::Ancestor;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::global::{is_declared_reference, is_global_undefined};
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::pass::Cx;
use untwist_core::semantic::{read_reference_count, write_reference_count};

use crate::settled::is_settled_binding;
pub use crate::settled::script_root;

pub fn reference_symbol(identifier: &IdentifierReference<'_>, cx: &Cx<'_, '_>) -> Option<SymbolId> {
    let reference_id = identifier.reference_id.get()?;
    cx.scoping().get_reference(reference_id).symbol_id()
}

pub fn identifier_symbol_is(expr: &Expression<'_>, symbol: SymbolId, cx: &Cx<'_, '_>) -> bool {
    let Expression::Identifier(identifier) = expr else {
        return false;
    };
    reference_symbol(identifier, cx) == Some(symbol)
}

pub fn has_reference_counts(
    symbol: SymbolId,
    reads: usize,
    writes: usize,
    cx: &Cx<'_, '_>,
) -> bool {
    read_reference_count(cx.scoping(), symbol) == reads
        && write_reference_count(cx.scoping(), symbol) == writes
}

#[cold]
#[inline(never)]
fn pin_unless_at_function_top(declaration: &VariableDeclaration<'_>, cx: &mut Cx<'_, '_>) {
    if !declared_at_function_top(cx) {
        cx.pin(declaration.unstable_address());
    }
}

fn declared_at_function_top(cx: &Cx<'_, '_>) -> bool {
    let list = cx.tcx.ancestors().find(|ancestor| {
        matches!(
            ancestor,
            Ancestor::ProgramBody(_)
                | Ancestor::FunctionBodyStatements(_)
                | Ancestor::StaticBlockBody(_)
                | Ancestor::BlockStatementBody(_)
                | Ancestor::SwitchCaseConsequent(_)
                | Ancestor::TSModuleBlockBody(_)
        )
    });
    matches!(
        list,
        Some(
            Ancestor::ProgramBody(_)
                | Ancestor::FunctionBodyStatements(_)
                | Ancestor::StaticBlockBody(_)
        )
    )
}

pub fn is_undefined_identifier(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    is_global_undefined(expr, cx.scoping())
}

#[derive(Default)]
pub struct TempVars {
    declared: FxHashSet<SymbolId>,
    consumed: FxHashSet<SymbolId>,
}

impl TempVars {
    pub fn reset(&mut self) {
        self.declared.clear();
        self.consumed.clear();
    }

    #[inline]
    pub fn record_declaration(
        &mut self,
        declaration: &VariableDeclaration<'_>,
        cx: &mut Cx<'_, '_>,
    ) {
        if declaration.kind != VariableDeclarationKind::Var {
            return;
        }
        let mut recorded = false;
        for declarator in &declaration.declarations {
            if declarator.init.is_some() {
                continue;
            }
            if let Some(symbol) = binding_symbol(&declarator.id) {
                self.declared.insert(symbol);
                recorded = true;
            }
        }
        if recorded && cx.pins() {
            pin_unless_at_function_top(declaration, cx);
        }
    }

    pub fn is_temp(&self, symbol: SymbolId, reads: usize, cx: &Cx<'_, '_>) -> bool {
        self.declared.contains(&symbol) && has_reference_counts(symbol, reads, 1, cx)
    }

    pub fn consume(&mut self, symbol: SymbolId) {
        self.consumed.insert(symbol);
    }

    pub fn strip_declarations<'a>(&self, stmts: &mut AVec<'a, Statement<'a>>) {
        if self.consumed.is_empty() {
            return;
        }
        stmts.retain_mut(|stmt| match stmt {
            Statement::VariableDeclaration(declaration) => {
                strip_var_declarators(declaration, &self.consumed)
            }
            Statement::ForStatement(node) => {
                strip_for_statement_init(node, &self.consumed);
                true
            }
            _ => true,
        });
    }
}

fn should_keep_declarator(
    declarator: &VariableDeclarator<'_>,
    consumed: &FxHashSet<SymbolId>,
) -> bool {
    declarator.init.is_some()
        || !binding_symbol(&declarator.id).is_some_and(|symbol| consumed.contains(&symbol))
}

fn strip_var_declarators(
    declaration: &mut VariableDeclaration<'_>,
    consumed: &FxHashSet<SymbolId>,
) -> bool {
    if declaration.kind != VariableDeclarationKind::Var {
        return true;
    }
    declaration
        .declarations
        .retain(|declarator| should_keep_declarator(declarator, consumed));
    !declaration.declarations.is_empty()
}

fn strip_for_statement_init(node: &mut ForStatement<'_>, consumed: &FxHashSet<SymbolId>) {
    let Some(ForStatementInit::VariableDeclaration(declaration)) = &mut node.init else {
        return;
    };
    if !strip_var_declarators(declaration, consumed) {
        node.init = None;
    }
}

pub enum MemberKey<'k, 'a> {
    Static(&'k IdentifierName<'a>),
    Computed(&'k Expression<'a>),
}

pub fn member_parts<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, MemberKey<'k, 'a>)> {
    match expr {
        Expression::StaticMemberExpression(member) if !member.optional => {
            Some((&member.object, MemberKey::Static(&member.property)))
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            Some((&member.object, MemberKey::Computed(&member.expression)))
        }
        _ => None,
    }
}

pub fn target_parts<'k, 'a>(
    target: &'k AssignmentTarget<'a>,
) -> Option<(&'k Expression<'a>, MemberKey<'k, 'a>)> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) if !member.optional => {
            Some((&member.object, MemberKey::Static(&member.property)))
        }
        AssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
            Some((&member.object, MemberKey::Computed(&member.expression)))
        }
        _ => None,
    }
}

pub fn keys_match(
    left: &MemberKey<'_, '_>,
    right: &MemberKey<'_, '_>,
    script_root: Option<ScopeId>,
    cx: &Cx<'_, '_>,
) -> bool {
    match (left, right) {
        (MemberKey::Static(left), MemberKey::Static(right)) => {
            left.name.as_str() == right.name.as_str()
        }
        (MemberKey::Computed(left), MemberKey::Computed(right)) => {
            ContentEq::content_eq(*left, *right) && is_repeatable_key(left, script_root, cx)
        }
        _ => false,
    }
}

fn is_plain_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::StringLiteral(_)
    )
}

fn reads_the_same_twice(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    match expr {
        Expression::ThisExpression(_) => true,
        Expression::Identifier(identifier) => is_declared_reference(cx.scoping(), identifier),
        _ => is_plain_literal(expr),
    }
}

#[derive(Default)]
struct RunsCode {
    found: bool,
}

impl<'a> VisitJs<'a> for RunsCode {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        self.found |= match it {
            Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::Delete,
            other => matches!(
                other,
                Expression::CallExpression(_)
                    | Expression::NewExpression(_)
                    | Expression::UpdateExpression(_)
                    | Expression::AssignmentExpression(_)
                    | Expression::AwaitExpression(_)
                    | Expression::YieldExpression(_)
                    | Expression::TaggedTemplateExpression(_)
                    | Expression::ImportExpression(_)
                    | Expression::ChainExpression(_)
            ),
        };
        if !self.found {
            walk_js::walk_expression(self, it);
        }
    }
}

fn is_free_of_calls(expr: &Expression<'_>) -> bool {
    let mut scan = RunsCode::default();
    scan.visit_expression(expr);
    !scan.found
}

fn is_call_free_chain(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier(_) | Expression::ThisExpression(_) => true,
        Expression::StaticMemberExpression(member) => is_call_free_chain(&member.object),
        Expression::ComputedMemberExpression(member) => {
            is_call_free_chain(&member.object) && is_free_of_calls(&member.expression)
        }
        _ => is_plain_literal(expr),
    }
}

pub fn is_repeatable(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    if cx.opts.unsafe_transforms {
        is_call_free_chain(expr)
    } else {
        reads_the_same_twice(expr, cx)
    }
}

fn holds_still(expr: &Expression<'_>, script_root: Option<ScopeId>, cx: &Cx<'_, '_>) -> bool {
    match expr {
        Expression::Identifier(identifier) => {
            reads_the_same_twice(expr, cx)
                && reference_symbol(identifier, cx)
                    .is_some_and(|symbol| is_settled_binding(cx.scoping(), symbol, script_root))
        }
        _ => reads_the_same_twice(expr, cx),
    }
}

pub fn is_repeatable_object(
    object: &Expression<'_>,
    script_root: Option<ScopeId>,
    cx: &Cx<'_, '_>,
) -> bool {
    if cx.opts.unsafe_transforms {
        is_call_free_chain(object)
    } else {
        holds_still(object, script_root, cx)
    }
}

fn is_repeatable_key(key: &Expression<'_>, script_root: Option<ScopeId>, cx: &Cx<'_, '_>) -> bool {
    if cx.opts.unsafe_transforms {
        is_free_of_calls(key)
    } else {
        holds_still(key, script_root, cx)
    }
}

pub fn is_repeatable_target(
    expr: &Expression<'_>,
    script_root: Option<ScopeId>,
    cx: &Cx<'_, '_>,
) -> bool {
    match member_parts(expr) {
        Some((object, MemberKey::Static(_))) => is_repeatable_object(object, script_root, cx),
        Some((object, MemberKey::Computed(key))) => {
            is_repeatable_object(object, script_root, cx) && is_repeatable_key(key, script_root, cx)
        }
        None => matches!(expr, Expression::Identifier(_)) && is_repeatable(expr, cx),
    }
}

pub fn assignment_temp_symbol(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<SymbolId> {
    let Expression::AssignmentExpression(assignment) = expr else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(identifier) = &assignment.left else {
        return None;
    };
    reference_symbol(identifier, cx)
}

pub fn restore_logical_assignment<'a>(
    span: Span,
    left: Expression<'a>,
    operator: LogicalOperator,
    assignment: AssignmentExpression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Expression<'a> {
    let right = Expression::new_assignment_expression(
        assignment.span,
        assignment.operator,
        assignment.left,
        assignment.right,
        cx,
    );
    Expression::new_logical_expression(span, left, operator, right, cx)
}

pub fn take_temp_member_target<'a>(
    left: Expression<'a>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Result<AssignmentTarget<'a>, Expression<'a>> {
    match left {
        Expression::StaticMemberExpression(member) => {
            take_temp_static_member_target(member.unbox(), span, cx)
        }
        Expression::ComputedMemberExpression(member) => {
            take_temp_computed_member_target(member.unbox(), span, cx)
        }
        other => Err(other),
    }
}

fn take_temp_static_member_target<'a>(
    member: StaticMemberExpression<'a>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Result<AssignmentTarget<'a>, Expression<'a>> {
    let member_span = member.span;
    let property = member.property;
    let optional = member.optional;
    match member.object {
        Expression::AssignmentExpression(inner) => {
            Ok(AssignmentTarget::new_static_member_expression(
                span,
                inner.unbox().right,
                property,
                false,
                cx,
            ))
        }
        other => Err(Expression::new_static_member_expression(
            member_span,
            other,
            property,
            optional,
            cx,
        )),
    }
}

fn take_temp_computed_member_target<'a>(
    member: ComputedMemberExpression<'a>,
    span: Span,
    cx: &mut Cx<'a, '_>,
) -> Result<AssignmentTarget<'a>, Expression<'a>> {
    let member_span = member.span;
    let optional = member.optional;
    match member.object {
        Expression::AssignmentExpression(inner) => {
            Ok(AssignmentTarget::new_computed_member_expression(
                span,
                inner.unbox().right,
                member.expression,
                false,
                cx,
            ))
        }
        other => Err(Expression::new_computed_member_expression(
            member_span,
            other,
            member.expression,
            optional,
            cx,
        )),
    }
}

pub fn take_logical_assignment<'a>(
    node: &mut Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<(
    Span,
    LogicalOperator,
    Expression<'a>,
    AssignmentExpression<'a>,
)> {
    let logical = match std::mem::replace(node, Expression::new_null_literal(SPAN, cx)) {
        Expression::LogicalExpression(logical) => logical.unbox(),
        other => {
            *node = other;
            return None;
        }
    };
    let span = logical.span;
    let operator = logical.operator;
    match logical.right {
        Expression::AssignmentExpression(assignment) => {
            Some((span, operator, logical.left, assignment.unbox()))
        }
        other => {
            *node = Expression::new_logical_expression(span, logical.left, operator, other, cx);
            None
        }
    }
}

pub fn take_logical_left_assignment_value<'a>(
    test: Expression<'a>,
    cx: &mut Cx<'a, '_>,
) -> Result<Expression<'a>, Expression<'a>> {
    let logical = match test {
        Expression::LogicalExpression(logical) => logical.unbox(),
        other => return Err(other),
    };
    let logical_span = logical.span;
    let logical_operator = logical.operator;
    let binary = match logical.left {
        Expression::BinaryExpression(binary) => binary.unbox(),
        other => {
            return Err(Expression::new_logical_expression(
                logical_span,
                other,
                logical_operator,
                logical.right,
                cx,
            ))
        }
    };
    let binary_span = binary.span;
    let binary_operator = binary.operator;
    match binary.left {
        Expression::AssignmentExpression(assignment) => Ok(assignment.unbox().right),
        other => {
            let rebuilt_binary = Expression::new_binary_expression(
                binary_span,
                other,
                binary_operator,
                binary.right,
                cx,
            );
            Err(Expression::new_logical_expression(
                logical_span,
                rebuilt_binary,
                logical_operator,
                logical.right,
                cx,
            ))
        }
    }
}
