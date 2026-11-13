use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, Function, IdentifierReference, Program,
    WithStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;

pub(super) struct Watch<'w> {
    pub function: Span,
    pub site: Span,
    pub names: &'w [String],
    pub clashed: bool,
}

struct Walk<'s, 'w> {
    watches: &'s mut [Watch<'w>],
    by_function: FxHashMap<Span, Vec<usize>>,
    functions: Vec<Span>,
}

pub(super) fn clashes(program: &Program<'_>, watches: &mut [Watch<'_>]) {
    if watches.is_empty() {
        return;
    }
    let mut by_function: FxHashMap<Span, Vec<usize>> = FxHashMap::default();
    for (index, watch) in watches.iter().enumerate() {
        by_function.entry(watch.function).or_default().push(index);
    }
    let mut walk = Walk {
        watches,
        by_function,
        functions: Vec::new(),
    };
    walk.visit_program(program);
}

impl Walk<'_, '_> {
    fn seen(&mut self, span: Span, name: Option<&str>) {
        for function in &self.functions {
            let Some(indices) = self.by_function.get(function) else {
                continue;
            };
            for &index in indices {
                let watch = &mut self.watches[index];
                if watch.site.contains_inclusive(span) {
                    continue;
                }
                watch.clashed |= match name {
                    None => true,
                    Some(name) => name == "eval" || watch.names.iter().any(|own| own == name),
                };
            }
        }
    }
}

impl<'a> Visit<'a> for Walk<'_, '_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions.push(it.span);
        walk::walk_function(self, it, flags);
        self.functions.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.functions.push(it.span);
        walk::walk_arrow_function_expression(self, it);
        self.functions.pop();
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.seen(it.span, Some(it.name.as_str()));
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.seen(it.span, Some(it.name.as_str()));
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.seen(it.span, None);
        walk::walk_with_statement(self, it);
    }
}
