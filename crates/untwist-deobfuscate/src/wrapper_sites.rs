use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::var_function_or_declaration;

use crate::stamp::declaration_symbol;
use crate::units::MachineryDepth;
use crate::wrapper_calls::WrapperShape;

pub(crate) fn called_expression<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<&'k CallExpression<'a>> {
    match expr {
        Expression::CallExpression(call) => Some(call),
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => Some(call),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) struct CallSite<'p> {
    pub(crate) wrapper: SymbolId,
    pub(crate) span: Span,
    pub(crate) arguments: &'p AVec<'p, Argument<'p>>,
    pub(crate) ancestors: Vec<usize>,
    pub(crate) body_of: Option<SymbolId>,
    pub(crate) in_parameters: bool,
}

struct SiteSearch<'p, 's> {
    scoping: &'s Scoping,
    wrappers: FxHashSet<SymbolId>,
    units: MachineryDepth,
    machinery_functions: usize,
    bodies: Vec<SymbolId>,
    parameters: usize,
    stack: Vec<usize>,
    found: Vec<CallSite<'p>>,
}

impl SiteSearch<'_, '_> {
    fn called_wrapper(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let call = called_expression(expr)?;
        let Expression::Identifier(id) = &call.callee else {
            return None;
        };
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())?;
        self.wrappers.contains(&symbol).then_some(symbol)
    }

    fn declared_wrapper(&self, stmt: &Statement<'_>) -> Option<SymbolId> {
        declaration_symbol(stmt).filter(|symbol| self.wrappers.contains(symbol))
    }
}

impl<'p> VisitJs<'p> for SiteSearch<'p, '_> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        let entered = self.units.enter(it);
        let function = entered && var_function_or_declaration(it).is_some();
        let body = self.declared_wrapper(it);
        self.machinery_functions += usize::from(function);
        self.bodies.extend(body);
        walk_js::walk_statement(self, it);
        if body.is_some() {
            self.bodies.pop();
        }
        self.machinery_functions -= usize::from(function);
        self.units.leave(entered);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'p>) {
        self.parameters += 1;
        walk_js::walk_formal_parameters(self, it);
        self.parameters -= 1;
    }

    fn visit_expression(&mut self, it: &Expression<'p>) {
        let wrapper = if self.machinery_functions > 0 {
            None
        } else {
            self.called_wrapper(it)
        };
        let Some(wrapper) = wrapper else {
            walk_js::walk_expression(self, it);
            return;
        };
        let expression: &'p Expression<'p> = self.alloc(it);
        let Some(call) = called_expression(expression) else {
            walk_js::walk_expression(self, it);
            return;
        };
        let id = self.found.len();
        self.found.push(CallSite {
            wrapper,
            span: expression.span(),
            arguments: &call.arguments,
            ancestors: self.stack.clone(),
            body_of: self.bodies.last().copied(),
            in_parameters: self.parameters > 0,
        });
        self.stack.push(id);
        walk_js::walk_expression(self, it);
        self.stack.pop();
    }
}

pub(crate) struct Sites<'p> {
    pub(crate) all: Vec<CallSite<'p>>,
    by_wrapper: FxHashMap<SymbolId, Vec<usize>>,
    nested: FxHashSet<usize>,
}

impl Sites<'_> {
    pub(crate) fn of(&self, wrapper: SymbolId) -> &[usize] {
        self.by_wrapper
            .get(&wrapper)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn holds_another_site(&self, id: usize) -> bool {
        self.nested.contains(&id)
    }
}

pub(crate) fn collect_sites<'p>(
    view: &Program<'p>,
    scoping: &Scoping,
    shapes: &[WrapperShape],
) -> Sites<'p> {
    let mut search = SiteSearch {
        scoping,
        wrappers: shapes.iter().map(|shape| shape.symbol).collect(),
        units: MachineryDepth::every_unit(),
        machinery_functions: 0,
        bodies: Vec::new(),
        parameters: 0,
        stack: Vec::new(),
        found: Vec::new(),
    };
    search.visit_program(view);
    let mut by_wrapper: FxHashMap<SymbolId, Vec<usize>> = FxHashMap::default();
    let mut nested: FxHashSet<usize> = FxHashSet::default();
    for (id, site) in search.found.iter().enumerate() {
        by_wrapper.entry(site.wrapper).or_default().push(id);
        nested.extend(site.ancestors.iter().copied());
    }
    Sites {
        all: search.found,
        by_wrapper,
        nested,
    }
}

pub(crate) struct FunctionsAt<'p> {
    wanted: FxHashMap<Span, Option<&'p Function<'p>>>,
    missing: usize,
}

impl<'p> FunctionsAt<'p> {
    pub(crate) fn search(view: &Program<'p>, shapes: &[WrapperShape]) -> Self {
        let wanted: FxHashMap<Span, Option<&'p Function<'p>>> =
            shapes.iter().map(|shape| (shape.function, None)).collect();
        let mut search = FunctionsAt {
            missing: wanted.len(),
            wanted,
        };
        if search.missing > 0 {
            search.visit_program(view);
        }
        search
    }

    pub(crate) fn get(&self, span: Span) -> Option<&'p Function<'p>> {
        self.wanted.get(&span).copied().flatten()
    }
}

impl<'p> VisitJs<'p> for FunctionsAt<'p> {
    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        if self.missing == 0 {
            return;
        }
        if matches!(self.wanted.get(&it.span), Some(None)) {
            let found: &'p Function<'p> = self.alloc(it);
            self.wanted.insert(it.span, Some(found));
            self.missing -= 1;
        }
        walk_js::walk_function(self, it, flags);
    }
}
