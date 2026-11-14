use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

struct Watched {
    declared_at: usize,
    seen: usize,
    late_only: bool,
    first: Option<usize>,
}

impl Watched {
    fn at(declared_at: usize) -> Self {
        Self {
            declared_at,
            seen: 0,
            late_only: true,
            first: None,
        }
    }
}

struct CaptureScan<'s> {
    scoping: &'s Scoping,
    watched: FxHashMap<SymbolId, Watched>,
    unseen: usize,
    statement: usize,
    closures: usize,
    expression_closure: bool,
}

impl CaptureScan<'_> {
    fn closure(&mut self, expression: bool, walk: impl FnOnce(&mut Self)) {
        let outer = self.expression_closure;
        if self.closures == 0 {
            self.expression_closure = expression;
        }
        self.closures += 1;
        walk(self);
        self.closures -= 1;
        self.expression_closure = outer;
    }
}

impl<'a> VisitJs<'a> for CaptureScan<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let Some(symbol) = self.scoping.get_reference(reference).symbol_id() else {
            return;
        };
        let (statement, captured, expression) =
            (self.statement, self.closures > 0, self.expression_closure);
        if let Some(watched) = self.watched.get_mut(&symbol) {
            watched.seen += 1;
            if watched.first.is_none() {
                watched.first = Some(statement);
                self.unseen = self.unseen.saturating_sub(1);
            }
            if captured && (!expression || statement <= watched.declared_at) {
                watched.late_only = false;
            }
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.closure(!it.is_declaration(), |scan| {
            walk_js::walk_function(scan, it, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.closure(true, |scan| {
            walk_js::walk_arrow_function_expression(scan, it);
        });
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.closure(false, |scan| walk_js::walk_class(scan, it));
    }
}

fn scan<'s>(scoping: &'s Scoping, watched: &[(SymbolId, usize)]) -> CaptureScan<'s> {
    CaptureScan {
        scoping,
        watched: watched
            .iter()
            .map(|&(symbol, declared_at)| (symbol, Watched::at(declared_at)))
            .collect(),
        unseen: watched.len(),
        statement: 0,
        closures: 0,
        expression_closure: false,
    }
}

pub(crate) fn captures_follow<'a>(
    scoping: &Scoping,
    stmts: &AVec<'a, Statement<'a>>,
    watched: &[(SymbolId, usize)],
) -> FxHashMap<SymbolId, bool> {
    let mut scan = scan(scoping, watched);
    for (index, stmt) in stmts.iter().enumerate() {
        scan.statement = index;
        scan.visit_statement(stmt);
    }
    scan.watched
        .into_iter()
        .map(|(symbol, watched)| {
            let complete = watched.seen == scoping.get_resolved_reference_ids(symbol).len();
            (symbol, watched.late_only && complete)
        })
        .collect()
}

pub(crate) fn first_uses<'a>(
    scoping: &Scoping,
    stmts: &AVec<'a, Statement<'a>>,
    watched: &[SymbolId],
) -> FxHashMap<SymbolId, usize> {
    let pairs: Vec<(SymbolId, usize)> = watched.iter().map(|symbol| (*symbol, 0)).collect();
    let mut scan = scan(scoping, &pairs);
    for (index, stmt) in stmts.iter().enumerate() {
        if scan.unseen == 0 {
            break;
        }
        scan.statement = index;
        scan.visit_statement(stmt);
    }
    scan.watched
        .into_iter()
        .filter_map(|(symbol, watched)| Some((symbol, watched.first?)))
        .collect()
}
