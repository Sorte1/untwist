use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;
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
    let Expression::ObjectExpression(object) = &assignment.right else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign
        || object.properties.is_empty()
        || !pure_object(object)
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
    let mut empties: Vec<(usize, SymbolId)> = Vec::new();
    let mut assigned: Vec<usize> = Vec::new();
    for (at, stmt) in stmts.iter().enumerate() {
        match stmt {
            Statement::VariableDeclaration(_) => {
                if let Some(symbol) = empty_var(stmt) {
                    empties.push((at, symbol));
                }
            }
            Statement::ExpressionStatement(node)
                if matches!(&node.expression, Expression::AssignmentExpression(assignment)
                    if matches!(assignment.right, Expression::ObjectExpression(_))) =>
            {
                assigned.push(at);
            }
            _ => {}
        }
    }
    if empties.is_empty() || assigned.is_empty() {
        return 0;
    }
    let mut pairs = Vec::new();
    for at in assigned {
        let Some(symbol) = object_assignment(&stmts[at], cx) else {
            continue;
        };
        if !sole_declaration(symbol, cx) {
            continue;
        }
        if let Some(&(declared, _)) = empties.iter().find(|(_, empty)| *empty == symbol) {
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

fn pure_object(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().all(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        property.kind == PropertyKind::Init
            && !property.computed
            && !property.method
            && matches!(
                property.value,
                Expression::FunctionExpression(_)
                    | Expression::ArrowFunctionExpression(_)
                    | Expression::StringLiteral(_)
                    | Expression::NumericLiteral(_)
            )
    })
}

fn assigned_callee<'k, 'a>(
    call: &'k CallExpression<'a>,
) -> Option<(&'k IdentifierReference<'a>, &'k ObjectExpression<'a>)> {
    let callee = call.callee.as_member_expression()?;
    let Expression::AssignmentExpression(assignment) = callee.object().without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let Expression::ObjectExpression(object) = &assignment.right else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign).then_some((target, object))
}

pub(super) fn assigns_its_callee_object(call: &CallExpression<'_>) -> bool {
    assigned_callee(call).is_some_and(|(_, object)| pure_object(object))
}

struct Finder<'s> {
    scoping: &'s oxc_semantic::Scoping,
    symbol: Option<SymbolId>,
    nested: usize,
    call: Option<oxc_span::Span>,
    arguments: Vec<oxc_span::Span>,
    reads: Vec<oxc_span::Span>,
    candidates: Vec<SymbolId>,
}

impl<'a> oxc_ast_visit::VisitJs<'a> for Finder<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some((target, object)) = assigned_callee(it) {
            let symbol = target
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
            if let Some(found) = symbol.filter(|_| self.nested == 0 && pure_object(object)) {
                self.candidates.push(found);
                if Some(found) == self.symbol && self.call.is_none() {
                    self.call = Some(it.span);
                    self.arguments = it
                        .arguments
                        .iter()
                        .map(|argument| argument.span())
                        .collect();
                }
            }
        }
        oxc_ast_visit::walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol.is_some() && symbol == self.symbol {
            self.reads.push(it.span);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {
        let _ = it;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let _ = it;
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.nested += 1;
        oxc_ast_visit::walk_js::walk_for_statement(self, it);
        self.nested -= 1;
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.nested += 1;
        oxc_ast_visit::walk_js::walk_while_statement(self, it);
        self.nested -= 1;
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.nested += 1;
        oxc_ast_visit::walk_js::walk_do_while_statement(self, it);
        self.nested -= 1;
    }
}

fn empty_lexical_or_var(stmt: &Statement<'_>) -> Option<SymbolId> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    if declaration.kind == VariableDeclarationKind::Const || declarator.init.is_some() {
        return None;
    }
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    id.symbol_id.get()
}

struct Hoist<'c, 'a, 'b> {
    call: oxc_span::Span,
    object: Option<Expression<'a>>,
    cx: &'c Cx<'a, 'b>,
}

impl<'a> oxc_ast_visit::VisitMut<'a> for Hoist<'_, 'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if it.span == self.call && self.object.is_none() {
            if let Some(member) = it.callee.as_member_expression_mut() {
                if let Expression::AssignmentExpression(assignment) =
                    member.object_mut().without_parentheses_mut()
                {
                    if let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left {
                        let name = target.name;
                        let span = target.span;
                        self.object = Some(assignment.right.take_in(self.cx));
                        *member.object_mut() = Expression::new_identifier(span, name, self.cx);
                    }
                }
            }
        }
        oxc_ast_visit::walk_mut::walk_call_expression(self, it);
    }
}

fn finder<'s>(scoping: &'s oxc_semantic::Scoping, symbol: Option<SymbolId>) -> Finder<'s> {
    Finder {
        scoping,
        symbol,
        nested: 0,
        call: None,
        arguments: Vec::new(),
        reads: Vec::new(),
        candidates: Vec::new(),
    }
}

pub(super) fn declare_callee_objects<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let has_declaration = stmts
        .iter()
        .any(|stmt| empty_lexical_or_var(stmt).is_some());
    if !has_declaration {
        return 0;
    }
    let mut candidates: Vec<(usize, SymbolId)> = Vec::new();
    for (at, stmt) in stmts.iter().enumerate() {
        if !matches!(stmt, Statement::ExpressionStatement(_)) {
            continue;
        }
        let mut search = finder(cx.scoping(), None);
        oxc_ast_visit::VisitJs::visit_statement(&mut search, stmt);
        candidates.extend(search.candidates.into_iter().map(|symbol| (at, symbol)));
    }
    if candidates.is_empty() {
        return 0;
    }
    let mut changed = 0;
    for (at, symbol) in candidates {
        let Some(declared) =
            (0..at).find(|index| empty_lexical_or_var(&stmts[*index]) == Some(symbol))
        else {
            continue;
        };
        if write_reference_count(cx.scoping(), symbol) != 1
            || !cx.scoping().symbol_redeclarations(symbol).is_empty()
        {
            continue;
        }
        let total = cx.scoping().get_resolved_reference_ids(symbol).len();
        let mut search = finder(cx.scoping(), Some(symbol));
        oxc_ast_visit::VisitJs::visit_statement(&mut search, &stmts[at]);
        let Some(call) = search.call else {
            continue;
        };
        let inside = |span: &oxc_span::Span| {
            search
                .arguments
                .iter()
                .any(|argument| argument.start <= span.start && span.end <= argument.end)
        };
        let outside = search.reads.iter().filter(|span| !inside(span)).count();
        let sealed = search.reads.len() == total
            && (outside == 1 || crate::mutants::callee_objects_trusted());
        if !sealed {
            continue;
        }
        let mut hoist = Hoist {
            call,
            object: None,
            cx: &*cx,
        };
        oxc_ast_visit::VisitMut::visit_statement(&mut hoist, &mut stmts[at]);
        let Some(object) = hoist.object else {
            continue;
        };
        if let Statement::VariableDeclaration(declaration) = &mut stmts[declared] {
            if let Some(declarator) = declaration.declarations.first_mut() {
                declarator.init = Some(object);
                changed += 1;
            }
        }
        return changed;
    }
    changed
}
