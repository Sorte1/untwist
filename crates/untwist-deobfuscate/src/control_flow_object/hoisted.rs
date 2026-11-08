use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;
use untwist_core::semantic::write_reference_count;

fn empty_var(stmt: &Statement<'_>) -> Option<SymbolId> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var || declarator.init.is_some() {
        return None;
    }
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    id.symbol_id.get()
}

fn object_assignment(stmt: &Statement<'_>, cx: &Cx<'_, '_>) -> Option<SymbolId> {
    let Statement::ExpressionStatement(node) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &node.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign
        || !matches!(assignment.right, Expression::ObjectExpression(_))
    {
        return None;
    }
    cx.scoping()
        .get_reference(target.reference_id.get()?)
        .symbol_id()
}

fn sole_declaration(symbol: SymbolId, cx: &Cx<'_, '_>) -> bool {
    cx.scoping().symbol_redeclarations(symbol).is_empty()
        && write_reference_count(cx.scoping(), symbol) == 1
}

pub(super) fn declare_assigned_objects<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let mut pairs = Vec::new();
    for (at, stmt) in stmts.iter().enumerate() {
        let Some(symbol) = object_assignment(stmt, cx) else {
            continue;
        };
        if !sole_declaration(symbol, cx) {
            continue;
        }
        let declared = stmts
            .iter()
            .position(|other| empty_var(other) == Some(symbol));
        if let Some(declared) = declared {
            pairs.push((at, declared));
        }
    }
    if pairs.is_empty() {
        return 0;
    }
    let mut removed = Vec::new();
    for &(at, declared) in &pairs {
        let mut declaration = stmts[declared].take_in(cx);
        let Statement::ExpressionStatement(node) = &mut stmts[at] else {
            continue;
        };
        let Expression::AssignmentExpression(assignment) = &mut node.expression else {
            continue;
        };
        let value = assignment.right.take_in(cx);
        if let Statement::VariableDeclaration(found) = &mut declaration {
            if let Some(declarator) = found.declarations.first_mut() {
                declarator.init = Some(value);
            }
        }
        stmts[at] = declaration;
        removed.push(declared);
    }
    removed.sort_unstable();
    for index in removed.into_iter().rev() {
        stmts.remove(index);
    }
    pairs.len()
}
