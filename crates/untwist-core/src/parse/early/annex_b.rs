use oxc_ast::ast::{
    BlockStatement, CatchClause, Function, IfStatement, Program, Statement, SwitchStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

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
}

impl<'a> VisitJs<'a> for Walk<'_> {
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
        walk_js::walk_function(self, it, flags);
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
    pub fn scan(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut walk = Walk {
            scan: Scan::default(),
            scoping,
        };
        walk.visit_program(program);
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
        AnnexB {
            candidates,
            clashing,
            unmodelled,
        }
    }

    pub fn unmodelled(&self) -> &[Span] {
        &self.unmodelled
    }

    pub fn touches_unmodelled(&self, diagnostic: &OxcDiagnostic) -> bool {
        redeclaration_spans(diagnostic)
            .is_some_and(|spans| spans.iter().any(|span| self.unmodelled.contains(span)))
    }

    pub fn explains(&self, diagnostic: &OxcDiagnostic) -> bool {
        let Some(spans) = redeclaration_spans(diagnostic) else {
            return false;
        };
        self.candidates
            .iter()
            .zip(&self.clashing)
            .any(|(candidate, clashing)| {
                !clashing
                    && spans.contains(&candidate.ident)
                    && spans.iter().any(|span| {
                        *span != candidate.ident && !candidate.block.contains_inclusive(*span)
                    })
            })
    }
}
