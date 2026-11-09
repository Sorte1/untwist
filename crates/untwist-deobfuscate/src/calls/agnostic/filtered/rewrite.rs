use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;

use super::{identifier_symbol, Candidate};

#[derive(Clone, Copy)]
pub(super) struct Rewrite {
    pub(super) span: Span,
    pub(super) ordered: bool,
}

impl Rewrite {
    pub(super) fn after(&self, at: Span) -> bool {
        at.start >= self.span.end
    }
}

fn redeclares(statement: &Statement<'_>, found: SymbolId) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    let Some(first) = declaration.declarations.first() else {
        return false;
    };
    declaration.kind == VariableDeclarationKind::Var
        && first.init.is_some()
        && matches!(&first.id, BindingPattern::BindingIdentifier(id) if id.symbol_id.get() == Some(found))
}

fn rewrites_first(it: &ForStatement<'_>, found: SymbolId, ordered: bool) -> bool {
    let Statement::BlockStatement(block) = &it.body else {
        return false;
    };
    if ordered {
        block
            .body
            .first()
            .is_some_and(|first| redeclares(first, found))
    } else {
        block.body.iter().any(|stmt| redeclares(stmt, found))
    }
}

pub(super) fn rewrite_loop(candidate: &Candidate<'_>, ordered: bool) -> Option<Rewrite> {
    let block = candidate.within.block?;
    block.statements.iter().find_map(|statement| {
        let Statement::ForStatement(it) = statement else {
            return None;
        };
        (it.span.start >= candidate.span.end && rewrites_first(it, candidate.found, ordered))
            .then_some(Rewrite {
                span: it.span,
                ordered,
            })
    })
}

struct Mentions<'s> {
    scoping: &'s Scoping,
    symbols: [SymbolId; 2],
    reads: usize,
    writes: usize,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = identifier_symbol(self.scoping, it) else {
            return;
        };
        if !self.symbols.contains(&symbol) {
            return;
        }
        let write = it
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_write());
        if write {
            self.writes += 1;
        } else {
            self.reads += 1;
        }
    }
}

fn mentions<'a>(
    scoping: &Scoping,
    candidate: &Candidate<'a>,
    visit: impl FnOnce(&mut Mentions<'_>),
) -> (usize, usize) {
    let mut found = Mentions {
        scoping,
        symbols: [candidate.found, candidate.key],
        reads: 0,
        writes: 0,
    };
    visit(&mut found);
    (found.reads, found.writes)
}

pub(super) struct Checked {
    pub(super) bad: bool,
    pub(super) writes: usize,
}

pub(super) fn check_loop<'a>(
    scoping: &Scoping,
    candidate: &Candidate<'a>,
    it: &ForStatement<'a>,
    ordered: bool,
) -> Checked {
    let (head_reads, head_writes) = mentions(scoping, candidate, |found| {
        if let Some(init) = &it.init {
            found.visit_for_statement_init(init);
        }
        if let Some(test) = &it.test {
            found.visit_expression(test);
        }
        if let Some(update) = &it.update {
            found.visit_expression(update);
        }
    });
    let first_init = match &it.body {
        Statement::BlockStatement(block) => match block.body.first() {
            Some(Statement::VariableDeclaration(declaration)) => declaration
                .declarations
                .first()
                .and_then(|first| first.init.as_ref()),
            _ => None,
        },
        _ => None,
    };
    let (init_reads, init_writes) = mentions(scoping, candidate, |found| {
        if let Some(init) = first_init {
            found.visit_expression(init);
        }
    });
    let (_, body_writes) = mentions(scoping, candidate, |found| {
        found.visit_statement(&it.body);
    });
    let early = head_reads + head_writes + init_reads + init_writes;
    Checked {
        bad: ordered && early > 0,
        writes: head_writes + body_writes,
    }
}

pub(super) fn is_rewrite_loop(rewrite: Option<Rewrite>, it: &ForStatement<'_>) -> bool {
    rewrite.is_some_and(|rewrite| rewrite.span == it.span())
}
