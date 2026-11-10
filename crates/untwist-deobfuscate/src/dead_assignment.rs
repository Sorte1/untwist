use oxc_allocator::{GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::ast_util::literal::is_safe_literal;
use untwist_core::ast_util::search::statement_at;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{bindings_possibly_used_before, scope_hazard, write_reference_count};
use untwist_eval::fold::{fold_to_const, FoldOptions};

use crate::bindings::refuse_annex_b_writes;
use crate::notes;
use crate::var_transformation::var_with_init;

pub const PASS: &str = "dead-assignment-elimination";

struct Candidate<'p> {
    symbol: SymbolId,
    name: Ident<'p>,
    declaration: Span,
    assignment_statement: Span,
}

fn empty_var_declaration<'k, 'p>(stmt: &'k Statement<'p>) -> Option<&'k BindingIdentifier<'p>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var || declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    if declarator.init.is_some() {
        return None;
    }
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some(id)
}

fn assignment_to<'k, 'p>(
    stmt: &'k Statement<'p>,
    symbol: SymbolId,
    scoping: &Scoping,
) -> Option<&'k Expression<'p>> {
    let Statement::ExpressionStatement(inner) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &inner.expression else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    let reference_id = target.reference_id.get()?;
    if scoping.get_reference(reference_id).symbol_id() != Some(symbol) {
        return None;
    }
    Some(&assignment.right)
}

fn is_constant_value(expr: &Expression<'_>) -> bool {
    fold_to_const(expr, &FoldOptions::default()).is_some() || is_safe_literal(expr)
}

fn assignment_in<'k, 'p>(
    later: &'k [Statement<'p>],
    symbol: SymbolId,
    scoping: &Scoping,
) -> Option<(usize, &'k Expression<'p>)> {
    if scoping.symbol_scope_id(symbol) == scoping.root_scope_id() {
        return assignment_to(later.first()?, symbol, scoping).map(|value| (0, value));
    }
    later.iter().enumerate().find_map(|(offset, statement)| {
        assignment_to(statement, symbol, scoping).map(|value| (offset, value))
    })
}

fn candidate_of<'p>(
    list: &AVec<'p, Statement<'p>>,
    index: usize,
    owner: ScopeId,
    scoping: &Scoping,
) -> Option<Candidate<'p>> {
    let statement = list.get(index)?;
    let id = empty_var_declaration(statement)?;
    let symbol = id.symbol_id.get()?;
    if scoping.symbol_scope_id(symbol) != owner {
        return None;
    }
    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }
    if write_reference_count(scoping, symbol) != 1 {
        return None;
    }
    let (offset, value) = assignment_in(list.get(index + 1..)?, symbol, scoping)?;
    if !is_constant_value(value) {
        return None;
    }
    if offset != 0 && !is_safe_literal(value) {
        return None;
    }
    Some(Candidate {
        symbol,
        name: id.name,
        declaration: statement.span(),
        assignment_statement: list.get(index + 1 + offset)?.span(),
    })
}

fn block_body<'k, 'p>(body: &'k ArrowFunctionBody<'p>) -> Option<&'k FunctionBody<'p>> {
    match body {
        ArrowFunctionBody::FunctionBody(inner) => Some(inner),
        _ => None,
    }
}

struct Search<'s, 'p> {
    scoping: &'s Scoping,
    found: Vec<Candidate<'p>>,
}

impl<'p> Search<'_, 'p> {
    fn scan(&mut self, list: &AVec<'p, Statement<'p>>, owner: ScopeId) {
        for index in 0..list.len() {
            if let Some(candidate) = candidate_of(list, index, owner, self.scoping) {
                self.found.push(candidate);
            }
        }
    }

    fn enter(&mut self, scope_id: Option<ScopeId>, body: Option<&FunctionBody<'p>>) {
        let (Some(scope_id), Some(body)) = (scope_id, body) else {
            return;
        };
        self.scan(&body.statements, scope_id);
    }
}

impl<'p> VisitJs<'p> for Search<'_, 'p> {
    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        self.enter(it.scope_id.get(), it.body.as_deref());
        walk_js::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'p>) {
        self.enter(it.scope_id.get(), block_body(&it.body));
        walk_js::walk_arrow_function_expression(self, it);
    }
}

fn collect<'p>(program: &Program<'p>, scoping: &Scoping) -> Vec<Candidate<'p>> {
    let mut search = Search {
        scoping,
        found: Vec::new(),
    };
    search.scan(&program.body, scoping.root_scope_id());
    search.visit_program(program);
    search.found
}

fn declaration_for<'a>(
    program: &Program<'_>,
    candidate: &Candidate<'_>,
    cx: &Cx<'a, '_>,
) -> Option<Statement<'a>> {
    let statement = statement_at(program, candidate.assignment_statement)?;
    let value = assignment_to(statement, candidate.symbol, cx.scoping())?;
    Some(var_with_init(candidate.name.as_str(), value, cx))
}

fn plan<'a>(
    program: &Program<'_>,
    candidate: &Candidate<'_>,
    used_before: &FxHashSet<SymbolId>,
    edits: &mut StmtEdits<'a>,
    cx: &mut Cx<'a, '_>,
) {
    if used_before.contains(&candidate.symbol) {
        cx.note(
            notes::DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE,
            Some(candidate.assignment_statement),
        );
        return;
    }
    let Some(declaration) = declaration_for(program, candidate, cx) else {
        return;
    };
    if !edits.replace(candidate.declaration, vec![declaration]) {
        return;
    }
    edits.remove(candidate.assignment_statement);
}

#[derive(Default)]
pub struct DeadAssignmentElimination;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DeadAssignmentElimination)
}

impl<'a> Pass<'a> for DeadAssignmentElimination {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if scope_hazard(cx.scoping()).is_some() {
            return;
        }
        let mut candidates = collect(program, cx.scoping());
        refuse_annex_b_writes(
            &mut candidates,
            |candidate| (candidate.assignment_statement, candidate.symbol),
            program,
            cx,
        );
        let boundaries: Vec<(SymbolId, Span)> = candidates
            .iter()
            .map(|candidate| (candidate.symbol, candidate.assignment_statement))
            .collect();
        let used_before = bindings_possibly_used_before(program, cx.scoping(), &boundaries);
        let mut edits = StmtEdits::new();
        for candidate in &candidates {
            plan(program, candidate, &used_before, &mut edits, cx);
        }
        let applied = edits.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
    }
}
