use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;

fn forwards_a_call(function: &Function<'_>) -> bool {
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    if function.params.items.len() < 2 || function.params.rest.is_some() {
        return false;
    }
    let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return false;
    };
    matches!(
        ret.argument.as_ref().map(Expression::without_parentheses),
        Some(Expression::CallExpression(call))
            if matches!(call.callee.without_parentheses(), Expression::Identifier(_))
                && call.arguments.len() >= 2
    )
}

fn is_forwarder(declarator: &VariableDeclarator<'_>) -> bool {
    matches!(declarator.id, BindingPattern::BindingIdentifier(_))
        && matches!(
            declarator.init.as_ref(),
            Some(Expression::FunctionExpression(function)) if forwards_a_call(function)
        )
}

fn is_bare(declarator: &VariableDeclarator<'_>) -> bool {
    matches!(declarator.id, BindingPattern::BindingIdentifier(_)) && declarator.init.is_none()
}

fn leading_forwarders(stmt: &Statement<'_>) -> usize {
    let Statement::ForStatement(node) = stmt else {
        return 0;
    };
    let Some(ForStatementInit::VariableDeclaration(declaration)) = node.init.as_ref() else {
        return 0;
    };
    if declaration.kind != VariableDeclarationKind::Var {
        return 0;
    }
    let leading = declaration
        .declarations
        .iter()
        .take_while(|declarator| is_forwarder(declarator) || is_bare(declarator))
        .count();
    let forwards = declaration
        .declarations
        .iter()
        .take(leading)
        .any(|declarator| is_forwarder(declarator));
    if forwards {
        leading
    } else {
        0
    }
}

struct Hoist<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    moved: usize,
}

impl<'a> Hoist<'_, 'a, '_> {
    fn split(&mut self, stmt: &mut Statement<'a>, count: usize, out: &mut AVec<'a, Statement<'a>>) {
        let Statement::ForStatement(node) = stmt else {
            return;
        };
        let Some(ForStatementInit::VariableDeclaration(declaration)) = node.init.as_mut() else {
            return;
        };
        let all = declaration.declarations.take_in(self.cx);
        for (index, declarator) in all.into_iter().enumerate() {
            if index >= count {
                declaration.declarations.push(declarator);
                continue;
            }
            let span = declarator.span();
            let declarations = AVec::from_iter_in([declarator], self.cx);
            out.push(Statement::new_variable_declaration(
                span,
                VariableDeclarationKind::Var,
                declarations,
                false,
                self.cx,
            ));
            self.moved += 1;
        }
        if declaration.declarations.is_empty() {
            node.init = None;
        }
    }
}

impl<'a> VisitMut<'a> for Hoist<'_, 'a, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        if it.iter().any(|stmt| leading_forwarders(stmt) > 0) {
            let taken = it.take_in(self.cx);
            for mut stmt in taken {
                let count = leading_forwarders(&stmt);
                if count > 0 {
                    self.split(&mut stmt, count, it);
                }
                it.push(stmt);
            }
        }
        walk_mut::walk_statements(self, it);
    }
}

pub(super) fn hoist_for_init_forwarders<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    let mut hoist = Hoist { cx, moved: 0 };
    hoist.visit_program(program);
    hoist.moved
}

fn assigned_forwarder<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k IdentifierReference<'a>> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &stmt.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let forwards = matches!(
        &assignment.right,
        Expression::FunctionExpression(function) if forwards_a_call(function)
    );
    (assignment.operator == AssignmentOperator::Assign && forwards).then_some(&**target)
}

struct Assigned<'s> {
    scoping: &'s Scoping,
    scopes: Vec<ScopeId>,
    symbols: FxHashSet<SymbolId>,
}

impl Assigned<'_> {
    fn movable(&self, target: &IdentifierReference<'_>) -> Option<SymbolId> {
        let symbol = self
            .scoping
            .get_reference(target.reference_id.get()?)
            .symbol_id()?;
        let flags = self.scoping.symbol_flags(symbol);
        let writes = self
            .scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .filter(|reference| self.scoping.get_reference(**reference).is_write())
            .count();
        let here = self.scopes.last().copied();
        let single = flags.contains(SymbolFlags::FunctionScopedVariable)
            && here == Some(self.scoping.symbol_scope_id(symbol))
            && self.scoping.symbol_redeclarations(symbol).is_empty()
            && writes == 1;
        single.then_some(symbol)
    }
}

impl<'a> VisitJs<'a> for Assigned<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.scopes.push(self.scoping.root_scope_id());
        walk_js::walk_program(self, it);
        self.scopes.pop();
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let Some(scope) = it.scope_id.get() else {
            return;
        };
        self.scopes.push(scope);
        walk_js::walk_function(self, it, flags);
        self.scopes.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let Some(scope) = it.scope_id.get() else {
            return;
        };
        self.scopes.push(scope);
        walk_js::walk_arrow_function_expression(self, it);
        self.scopes.pop();
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(symbol) = assigned_forwarder(it).and_then(|target| self.movable(target)) {
            self.symbols.insert(symbol);
        }
        walk_js::walk_statement(self, it);
    }
}

struct BareDeclarations<'s> {
    symbols: &'s FxHashSet<SymbolId>,
    bare: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for BareDeclarations<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), None) = (&it.id, &it.init) {
            if let Some(symbol) = id.symbol_id.get().filter(|s| self.symbols.contains(s)) {
                self.bare.insert(symbol);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

struct Declare<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    symbols: FxHashSet<SymbolId>,
    moved: usize,
}

impl<'a> Declare<'_, 'a, '_> {
    fn symbol_of(&self, target: &IdentifierReference<'_>) -> Option<SymbolId> {
        let symbol = self
            .cx
            .scoping()
            .get_reference(target.reference_id.get()?)
            .symbol_id()?;
        self.symbols.contains(&symbol).then_some(symbol)
    }

    fn declared(&mut self, stmt: Statement<'a>) -> Statement<'a> {
        let Some(symbol) = assigned_forwarder(&stmt).and_then(|target| self.symbol_of(target))
        else {
            return stmt;
        };
        let Statement::ExpressionStatement(mut node) = stmt else {
            return stmt;
        };
        let span = node.span;
        let Expression::AssignmentExpression(assignment) = &mut node.expression else {
            return Statement::ExpressionStatement(node);
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
            return Statement::ExpressionStatement(node);
        };
        let name = target.name;
        let function = assignment.right.take_in(self.cx);
        let pattern =
            BindingPattern::new_binding_identifier_with_symbol_id(span, name, symbol, self.cx);
        let declarator =
            VariableDeclarator::new(span, pattern, None, Some(function), false, self.cx);
        self.moved += 1;
        Statement::new_variable_declaration(
            span,
            VariableDeclarationKind::Var,
            AVec::from_iter_in([declarator], self.cx),
            false,
            self.cx,
        )
    }

    fn bare(&self, stmt: &mut Statement<'a>) -> bool {
        let Statement::VariableDeclaration(declaration) = stmt else {
            return false;
        };
        declaration.declarations.retain(|declarator| {
            !matches!((&declarator.id, &declarator.init),
                (BindingPattern::BindingIdentifier(id), None)
                    if id.symbol_id.get().is_some_and(|s| self.symbols.contains(&s)))
        });
        declaration.declarations.is_empty()
    }
}

impl<'a> VisitMut<'a> for Declare<'_, 'a, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        let taken = it.take_in(self.cx);
        for mut stmt in taken {
            if self.bare(&mut stmt) {
                continue;
            }
            let stmt = self.declared(stmt);
            it.push(stmt);
        }
        walk_mut::walk_statements(self, it);
    }
}

pub(super) fn declare_assigned_forwarders<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    let mut assigned = Assigned {
        scoping: cx.scoping(),
        scopes: Vec::new(),
        symbols: FxHashSet::default(),
    };
    assigned.visit_program(program);
    if assigned.symbols.is_empty() {
        return 0;
    }
    let mut bare = BareDeclarations {
        symbols: &assigned.symbols,
        bare: FxHashSet::default(),
    };
    bare.visit_program(program);
    let mut declare = Declare {
        cx,
        symbols: bare.bare,
        moved: 0,
    };
    if declare.symbols.is_empty() {
        return 0;
    }
    declare.visit_program(program);
    declare.moved
}
