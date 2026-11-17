use std::cell::Cell;

use oxc_ast::ast::{
    ArrowFunctionExpression, BlockStatement, CatchClause, Class, Function, IfStatement,
    ImportDeclaration, Program, Statement, StaticBlock, SwitchStatement, VariableDeclaration,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

use super::hoisting::{Collect, Hoisting};

struct Candidate {
    symbol: Option<SymbolId>,
    name: String,
    ident: Span,
    block: Span,
    declared_in: ScopeId,
    clause: bool,
}

pub struct AnnexB {
    candidates: Vec<Candidate>,
    clashing: Vec<bool>,
    unmodelled: Vec<Span>,
}

#[derive(Default)]
struct Scan {
    blocks: Vec<Span>,
    catch_bodies: Vec<(Span, Span)>,
    clauses: Vec<Span>,
    candidates: Vec<Candidate>,
}

impl Scan {
    fn declared(&mut self, function: &Function<'_>, scoping: &Scoping) {
        let (Some(id), Some(own)) = (&function.id, function.scope_id.get()) else {
            return;
        };
        if function.r#async || function.generator || !function.is_declaration() {
            return;
        }
        let Some(declared_in) = scoping.scope_parent_id(own) else {
            return;
        };
        let flags = scoping.scope_flags(declared_in);
        if flags.is_strict_mode() {
            return;
        }
        let clause = self.clauses.last() == Some(&function.span);
        let block = match (clause, flags.is_var()) {
            (true, _) => function.span,
            (false, true) => return,
            (false, false) => match self.blocks.last() {
                Some(block) => *block,
                None => return,
            },
        };
        self.candidates.push(Candidate {
            symbol: id.symbol_id.get(),
            name: id.name.to_string(),
            ident: id.span,
            block,
            declared_in,
            clause,
        });
    }
}

struct Walk<'s> {
    scan: Scan,
    scoping: &'s Scoping,
    hoisting: Collect<'s>,
}

impl<'a> VisitJs<'a> for Walk<'_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.hoisting.enter_scope(scope_id);
    }

    fn leave_scope(&mut self) {
        self.hoisting.leave_scope();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.hoisting.arrow();
        walk_js::walk_arrow_function_expression(self, it);
        self.hoisting.leave_frame();
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.hoisting.static_block(it);
        walk_js::walk_static_block(self, it);
        self.hoisting.leave_frame();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.hoisting.class(it);
        walk_js::walk_class(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        for specifier in it.specifiers.iter().flatten() {
            self.hoisting.import(specifier.local());
        }
        walk_js::walk_import_declaration(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        self.hoisting.variable_declaration(it);
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        let block = match self.scan.catch_bodies.last() {
            Some((body, clause)) if *body == it.span => *clause,
            _ => it.span,
        };
        self.scan.blocks.push(block);
        walk_js::walk_block_statement(self, it);
        self.scan.blocks.pop();
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        self.hoisting.catch_clause(it);
        self.scan.catch_bodies.push((it.body.span, it.span));
        walk_js::walk_catch_clause(self, it);
        self.scan.catch_bodies.pop();
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.scan.blocks.push(it.span);
        walk_js::walk_switch_statement(self, it);
        self.scan.blocks.pop();
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        let clauses = [Some(&it.consequent), it.alternate.as_ref()];
        let functions: Vec<Span> = clauses
            .into_iter()
            .flatten()
            .filter_map(|clause| match clause {
                Statement::FunctionDeclaration(function) => Some(function.span),
                _ => None,
            })
            .collect();
        let pushed = functions.len();
        self.scan.clauses.extend(functions);
        walk_js::walk_if_statement(self, it);
        self.scan
            .clauses
            .truncate(self.scan.clauses.len().saturating_sub(pushed));
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.scan.declared(it, self.scoping);
        let clause = self.scan.clauses.last() == Some(&it.span);
        self.hoisting.function(it, clause);
        walk_js::walk_function(self, it, flags);
        self.hoisting.leave_frame();
    }
}

const LEXICAL: SymbolFlags = SymbolFlags::BlockScopedVariable
    .union(SymbolFlags::ConstVariable)
    .union(SymbolFlags::Class);

fn lexical_beside(candidate: &Candidate, scoping: &Scoping) -> bool {
    let name = Ident::from(candidate.name.as_str());
    for scope in scoping.scope_ancestors(candidate.declared_in) {
        let lexical = scoping
            .get_binding(scope, name)
            .is_some_and(|bound| scoping.symbol_flags(bound).intersects(LEXICAL));
        if lexical {
            return true;
        }
        if scoping.scope_flags(scope).is_var() {
            return false;
        }
    }
    false
}

fn clashes_outside(candidate: &Candidate, scoping: &Scoping) -> bool {
    let Some(symbol) = candidate.symbol else {
        return true;
    };
    let outside: Vec<SymbolFlags> = scoping
        .symbol_redeclarations(symbol)
        .iter()
        .filter(|declared| {
            declared.span != candidate.ident && !candidate.block.contains_inclusive(declared.span)
        })
        .map(|declared| declared.flags)
        .collect();
    outside.len() > 1 && outside.iter().any(|flags| flags.intersects(LEXICAL))
}

fn redeclaration_spans(diagnostic: &OxcDiagnostic) -> Option<Vec<Span>> {
    let redeclared = diagnostic.message.ends_with("has already been declared")
        && diagnostic.message.starts_with("Identifier `");
    redeclared.then(|| {
        diagnostic
            .labels
            .iter()
            .map(|label| Span::sized(label.offset(), label.len()))
            .collect()
    })
}

impl AnnexB {
    pub fn scan(program: &Program<'_>, scoping: &Scoping) -> (Self, Hoisting) {
        let mut walk = Walk {
            scan: Scan::default(),
            scoping,
            hoisting: Collect::new(scoping, program.source_type.is_module()),
        };
        walk.visit_program(program);
        let hoisting = Hoisting::collected(walk.hoisting);
        let candidates = walk.scan.candidates;
        let unmodelled = candidates
            .iter()
            .filter(|candidate| candidate.clause && lexical_beside(candidate, scoping))
            .map(|candidate| candidate.ident)
            .collect();
        let clashing = candidates
            .iter()
            .map(|candidate| clashes_outside(candidate, scoping))
            .collect();
        let annex_b = AnnexB {
            candidates,
            clashing,
            unmodelled,
        };
        (annex_b, hoisting)
    }

    pub fn unmodelled(&self) -> &[Span] {
        &self.unmodelled
    }

    pub fn unmodelled_functions(&self) -> Vec<Span> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.clause && self.unmodelled.contains(&candidate.ident))
            .map(|candidate| candidate.block)
            .collect()
    }

    pub fn is_redeclaration(diagnostic: &OxcDiagnostic) -> bool {
        redeclaration_spans(diagnostic).is_some()
    }

    pub fn explains(&self, diagnostic: &OxcDiagnostic) -> bool {
        self.explaining(diagnostic).is_some()
    }

    pub fn explains_by_clause(&self, diagnostic: &OxcDiagnostic) -> bool {
        self.explaining(diagnostic)
            .is_some_and(|candidate| candidate.clause)
    }

    pub fn clause_shares_name(&self, scoping: &Scoping) -> bool {
        self.candidates.iter().any(|candidate| {
            candidate.clause
                && candidate
                    .symbol
                    .is_some_and(|symbol| !scoping.symbol_redeclarations(symbol).is_empty())
        })
    }

    pub fn clause_functions(&self) -> Vec<Span> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.clause)
            .map(|candidate| candidate.block)
            .collect()
    }

    fn explaining(&self, diagnostic: &OxcDiagnostic) -> Option<&Candidate> {
        let spans = redeclaration_spans(diagnostic)?;
        self.candidates
            .iter()
            .zip(&self.clashing)
            .find(|(candidate, clashing)| {
                !**clashing
                    && spans.contains(&candidate.ident)
                    && spans.iter().any(|span| {
                        *span != candidate.ident && !candidate.block.contains_inclusive(*span)
                    })
            })
            .map(|(candidate, _)| candidate)
    }
}
