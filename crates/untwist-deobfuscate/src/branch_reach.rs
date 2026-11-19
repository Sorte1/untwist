use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

struct InBranch<'e> {
    scoping: &'e Scoping,
    escapes: &'e FxHashSet<Span>,
    dormant: &'e FxHashSet<SymbolId>,
    depth: usize,
    found: FxHashSet<Span>,
    read_outside: FxHashSet<SymbolId>,
    declared: FxHashSet<SymbolId>,
}

impl InBranch<'_> {
    fn note(&mut self, span: Span) {
        if self.depth > 0 && self.escapes.contains(&span) {
            self.found.insert(span);
        }
    }

    fn branch<T>(&mut self, visit: impl FnOnce(&mut Self) -> T) -> T {
        self.depth += 1;
        let out = visit(self);
        self.depth -= 1;
        out
    }
}

fn declared_symbol(function: &Function<'_>) -> Option<SymbolId> {
    match function.is_declaration() {
        true => function.id.as_ref()?.symbol_id.get(),
        false => None,
    }
}

impl<'a> VisitJs<'a> for InBranch<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let symbol = declared_symbol(it);
        if let Some(symbol) = symbol {
            self.declared.insert(symbol);
        }
        match symbol.is_some_and(|symbol| self.dormant.contains(&symbol)) {
            true => self.branch(|scan| walk_js::walk_function(scan, it, flags)),
            false => walk_js::walk_function(self, it, flags),
        }
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.visit_expression(&it.test);
        self.branch(|scan| scan.visit_statement(&it.consequent));
        if let Some(alternate) = &it.alternate {
            self.branch(|scan| scan.visit_statement(alternate));
        }
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.visit_expression(&it.test);
        self.branch(|scan| scan.visit_expression(&it.consequent));
        self.branch(|scan| scan.visit_expression(&it.alternate));
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.visit_expression(&it.left);
        self.branch(|scan| scan.visit_expression(&it.right));
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        if let Some(test) = &it.test {
            self.visit_expression(test);
        }
        self.branch(|scan| scan.visit_statements(&it.consequent));
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.note(it.span);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.note(it.span);
        if self.depth > 0 {
            return;
        }
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        if let Some(symbol) = reference.symbol_id().filter(|_| reference.is_read()) {
            self.read_outside.insert(symbol);
        }
    }
}

fn scan<'e>(
    (program, scoping): (&Program<'_>, &'e Scoping),
    escapes: &'e FxHashSet<Span>,
    dormant: &'e FxHashSet<SymbolId>,
) -> InBranch<'e> {
    let mut scan = InBranch {
        scoping,
        escapes,
        dormant,
        depth: 0,
        found: FxHashSet::default(),
        read_outside: FxHashSet::default(),
        declared: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan
}

pub fn in_branches(
    program: &Program<'_>,
    scoping: &Scoping,
    escapes: &FxHashSet<Span>,
) -> FxHashSet<Span> {
    let none = FxHashSet::default();
    let first = scan((program, scoping), escapes, &none);
    let dormant: FxHashSet<SymbolId> = first
        .declared
        .difference(&first.read_outside)
        .copied()
        .collect();
    if dormant.is_empty() {
        return first.found;
    }
    scan((program, scoping), escapes, &dormant).found
}
