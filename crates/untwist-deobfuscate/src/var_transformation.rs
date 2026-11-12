use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_str::Ident;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::StmtEdits;
use untwist_core::ast_util::frame_reads::{reads_arguments_property, reads_function_frame};
use untwist_core::ast_util::rename::{unique_name, Renames};
use untwist_core::ast_util::search::statement_at;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{
    bindings_possibly_used_before, parameter_symbols, scope_hazard, write_reference_count,
};

use crate::notes;

pub const PASS: &str = "var-transformation";

const ARGUMENTS: &str = "arguments";

struct Candidate<'p> {
    symbol: SymbolId,
    name: Ident<'p>,
    owner: ScopeId,
    statement: Span,
    assignment: Span,
}

#[derive(Default)]
struct Facts<'p> {
    found: Vec<Candidate<'p>>,
    reads_arguments: FxHashSet<ScopeId>,
    shadowed: FxHashSet<(ScopeId, Ident<'p>)>,
}

struct Frame<'p> {
    scope_id: ScopeId,
    wanted: FxHashSet<Ident<'p>>,
}

struct Search<'s, 'p> {
    scoping: &'s Scoping,
    facts: Facts<'p>,
    open: Vec<Frame<'p>>,
}

impl<'p> Search<'_, 'p> {
    fn enter(&mut self, scope_id: Option<ScopeId>, body: Option<&FunctionBody<'p>>) {
        let Some(scope_id) = scope_id else {
            return;
        };
        let mut wanted = FxHashSet::default();
        for statement in body.iter().flat_map(|body| body.statements.iter()) {
            let Some(candidate) = candidate_of(statement, scope_id, self.scoping) else {
                continue;
            };
            wanted.insert(candidate.name);
            self.facts.found.push(candidate);
        }
        self.open.push(Frame { scope_id, wanted });
    }

    fn leave(&mut self, scope_id: Option<ScopeId>) {
        if scope_id.is_some() {
            self.open.pop();
        }
    }

    fn record_hoisted_function(&mut self, it: &Function<'p>) {
        if it.r#type != FunctionType::FunctionDeclaration {
            return;
        }
        let Some(id) = it.id.as_ref() else {
            return;
        };
        for frame in &self.open {
            if frame.wanted.contains(&id.name) {
                self.facts.shadowed.insert((frame.scope_id, id.name));
            }
        }
    }
}

impl<'p> VisitJs<'p> for Search<'_, 'p> {
    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        self.record_hoisted_function(it);
        let scope_id = it.scope_id.get();
        self.enter(scope_id, it.body.as_deref());
        walk_js::walk_function(self, it, flags);
        self.leave(scope_id);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'p>) {
        let scope_id = it.scope_id.get();
        self.enter(scope_id, block_body(&it.body));
        walk_js::walk_arrow_function_expression(self, it);
        self.leave(scope_id);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'p>) {
        if it.name.as_str() == ARGUMENTS {
            self.facts
                .reads_arguments
                .extend(self.open.iter().map(|frame| frame.scope_id));
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn block_body<'a, 'b>(body: &'b ArrowFunctionBody<'a>) -> Option<&'b FunctionBody<'a>> {
    match body {
        ArrowFunctionBody::FunctionBody(inner) => Some(inner),
        _ => None,
    }
}

struct SelfRead<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    found: bool,
}

impl<'a> VisitJs<'a> for SelfRead<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.found {
            return;
        }
        self.found = match it.reference_id.get() {
            Some(_) => reference_symbol(it, self.scoping) == Some(self.symbol),
            None => self.scoping.symbol_name(self.symbol) == it.name.as_str(),
        };
    }
}

fn reference_symbol(it: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(it.reference_id.get()?).symbol_id()
}

fn reads_itself(init: &Expression<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    let mut scan = SelfRead {
        scoping,
        symbol,
        found: false,
    };
    scan.visit_expression(init);
    scan.found
}

fn candidate_of<'p>(
    stmt: &Statement<'p>,
    owner: ScopeId,
    scoping: &Scoping,
) -> Option<Candidate<'p>> {
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
    let symbol = reference_symbol(target, scoping)?;
    if scoping.symbol_scope_id(symbol) != owner {
        return None;
    }
    if reads_itself(&assignment.right, symbol, scoping) {
        return None;
    }
    Some(Candidate {
        symbol,
        name: target.name,
        owner,
        statement: stmt.span(),
        assignment: inner.expression.span(),
    })
}

fn collect<'p>(program: &Program<'p>, scoping: &Scoping) -> Facts<'p> {
    let mut search = Search {
        scoping,
        facts: Facts::default(),
        open: Vec::new(),
    };
    search.visit_program(program);
    search.facts
}

pub fn var_with_init<'a>(name: &str, init: &Expression<'_>, cx: &Cx<'a, '_>) -> Statement<'a> {
    let pattern = BindingPattern::new_binding_identifier(SPAN, Ident::from_str_in(name, cx), cx);
    let declarator = VariableDeclarator::new(
        SPAN,
        pattern,
        None,
        Some(init.clone_in(cx.allocator())),
        false,
        cx,
    );
    let declarators = AVec::from_iter_in([declarator], cx);
    Statement::new_variable_declaration(SPAN, VariableDeclarationKind::Var, declarators, false, cx)
}

fn assignment_parts<'a, 'p>(stmt: &'p Statement<'a>) -> Option<(&'p str, &'p Expression<'a>)> {
    let Statement::ExpressionStatement(inner) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &inner.expression else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    Some((target.name.as_str(), &assignment.right))
}

fn declaration_for<'a>(
    program: &Program<'_>,
    statement: Span,
    cx: &Cx<'a, '_>,
) -> Option<Statement<'a>> {
    let (name, init) = assignment_parts(statement_at(program, statement)?)?;
    Some(var_with_init(name, init, cx))
}

fn eligible<'p>(
    facts: &'p Facts<'p>,
    parameters: &FxHashSet<SymbolId>,
    cx: &mut Cx<'_, '_>,
) -> Vec<&'p Candidate<'p>> {
    let mut kept = Vec::new();
    for candidate in &facts.found {
        if !parameters.contains(&candidate.symbol) {
            continue;
        }
        if facts.shadowed.contains(&(candidate.owner, candidate.name)) {
            continue;
        }
        let at = Some(candidate.statement);
        if facts.reads_arguments.contains(&candidate.owner) {
            cx.note(notes::VAR_FUNCTION_READS_ARGUMENTS, at);
            continue;
        }
        if write_reference_count(cx.scoping(), candidate.symbol) != 1 {
            cx.note(notes::VAR_PARAMETER_WRITTEN_AGAIN, at);
            continue;
        }
        kept.push(candidate);
    }
    kept
}

fn plan<'a>(
    program: &Program<'a>,
    candidate: &Candidate<'_>,
    renames: &mut Renames<'a>,
    edits: &mut StmtEdits<'a>,
    cx: &mut Cx<'a, '_>,
) {
    let base = format!("{}2", cx.scoping().symbol_name(candidate.symbol));
    let at = Some(candidate.statement);
    let Some(replacement) = unique_name(&base, cx.scoping(), &*cx) else {
        cx.note(notes::VAR_DECLARATION_NOT_REBUILT, at);
        return;
    };
    let Some(declaration) = declaration_for(program, candidate.statement, cx) else {
        cx.note(notes::VAR_DECLARATION_NOT_REBUILT, at);
        return;
    };
    if !edits.replace(candidate.statement, vec![declaration]) {
        cx.note(notes::VAR_DECLARATION_NOT_REBUILT, at);
        return;
    }
    renames.insert(candidate.symbol, replacement);
}

#[derive(Default)]
pub struct VarTransformation;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(VarTransformation)
}

impl<'a> Pass<'a> for VarTransformation {
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
        let facts = collect(program, cx.scoping());
        let frame_read = if cx.opts.unsafe_transforms {
            reads_arguments_property(program, cx.scoping())
        } else {
            reads_function_frame(program, cx.scoping())
        };
        if frame_read {
            cx.note(notes::VAR_ARGUMENTS_PROPERTY_ALIAS, None);
            return;
        }
        let parameters = parameter_symbols(program);
        let kept = eligible(&facts, &parameters, cx);
        let boundaries: Vec<(SymbolId, Span)> = kept
            .iter()
            .map(|candidate| (candidate.symbol, candidate.assignment))
            .collect();
        let used_before = bindings_possibly_used_before(program, cx.scoping(), &boundaries);
        let mut renames = Renames::new();
        let mut edits = StmtEdits::new();
        for candidate in kept {
            if used_before.contains(&candidate.symbol) {
                let at = Some(candidate.statement);
                cx.note(notes::VAR_PARAMETER_USED_BEFORE_THE_ASSIGNMENT, at);
                continue;
            }
            plan(program, candidate, &mut renames, &mut edits, cx);
        }
        let applied = edits.apply(program, cx.allocator(), cx.scoping_mut());
        let renamed = renames.apply_bindings(program);
        cx.changed_by(applied + renamed);
    }
}
