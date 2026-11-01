use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::matcher::Factory;
use crate::scan::{is_direct_eval, Frame, Policy, Scan, Step};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Captured {
    Binding(String),
    Parameter(String),
    Arguments,
    Eval,
}

impl Captured {
    pub fn describe(&self) -> String {
        match self {
            Self::Binding(name) => format!("it reads {name}, a binding of the bundle's scope"),
            Self::Parameter(name) => format!("it reads {name}, a factory parameter no module maps"),
            Self::Arguments => "it reads the factory's own arguments".to_string(),
            Self::Eval => "it calls eval directly".to_string(),
        }
    }
}

pub struct Allowed<'s> {
    pub mapped_params: usize,
    pub helpers: &'s FxHashSet<SymbolId>,
}

const KNOWN_PARAMS: usize = 8;

#[derive(Clone, Copy)]
pub struct Region<'k, 'a> {
    inside: Span,
    params: Option<&'k FormalParameters<'a>>,
    symbols: [Option<SymbolId>; KNOWN_PARAMS],
    mapped_params: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    Local,
    Free,
    Param(usize),
    Helper(SymbolId),
    Binding(SymbolId),
    Unmapped(SymbolId),
    Arguments,
}

impl Reach {
    pub fn captured(self, scoping: &Scoping) -> Option<Captured> {
        let name = |symbol| scoping.symbol_name(symbol).to_string();
        match self {
            Self::Binding(symbol) => Some(Captured::Binding(name(symbol))),
            Self::Unmapped(symbol) => Some(Captured::Parameter(name(symbol))),
            Self::Arguments => Some(Captured::Arguments),
            Self::Local | Self::Free | Self::Param(_) | Self::Helper(_) => None,
        }
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

impl<'k, 'a> Region<'k, 'a> {
    pub fn factory(factory: Factory<'k, 'a>, mapped_params: usize) -> Self {
        let mut symbols = [None; KNOWN_PARAMS];
        for (slot, item) in symbols.iter_mut().zip(&factory.params.items) {
            if let BindingPattern::BindingIdentifier(id) = &item.pattern {
                *slot = id.symbol_id.get();
            }
        }
        Self {
            inside: factory.body.span,
            params: Some(factory.params),
            symbols,
            mapped_params,
        }
    }

    pub fn statement(statement: &Statement<'_>) -> Self {
        Self {
            inside: statement.span(),
            params: None,
            symbols: [None; KNOWN_PARAMS],
            mapped_params: 0,
        }
    }

    fn parameter(&self, index: usize, symbol: SymbolId) -> Reach {
        match index < self.mapped_params {
            true => Reach::Param(index),
            false => Reach::Unmapped(symbol),
        }
    }

    fn parameter_index(&self, declared: Span) -> Option<usize> {
        let params = self.params?;
        if !contains(params.span, declared) {
            return None;
        }
        let index = params
            .items
            .iter()
            .position(|item| contains(item.span, declared))
            .unwrap_or(params.items.len());
        Some(index)
    }

    #[inline(always)]
    fn resolved(
        &self,
        scoping: &Scoping,
        symbol: SymbolId,
        helpers: &FxHashSet<SymbolId>,
    ) -> Reach {
        if let Some(index) = self.symbols.iter().position(|s| *s == Some(symbol)) {
            return self.parameter(index, symbol);
        }
        let declared = scoping.symbol_span(symbol);
        if contains(self.inside, declared) {
            return Reach::Local;
        }
        if let Some(index) = self.parameter_index(declared) {
            return self.parameter(index, symbol);
        }
        match helpers.contains(&symbol) {
            true => Reach::Helper(symbol),
            false => Reach::Binding(symbol),
        }
    }

    #[inline(always)]
    pub fn reach(
        &self,
        (scoping, helpers): (&Scoping, &FxHashSet<SymbolId>),
        frame: Frame,
        it: &IdentifierReference<'_>,
    ) -> Reach {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| scoping.get_reference(id).symbol_id());
        match symbol {
            Some(symbol) => self.resolved(scoping, symbol, helpers),
            None if it.name == "arguments" && !frame.receiver() => Reach::Arguments,
            None => Reach::Free,
        }
    }
}

pub struct Check<'s, 'k, 'a> {
    scoping: &'s Scoping,
    region: Region<'k, 'a>,
    helpers: &'s FxHashSet<SymbolId>,
    captured: Option<Captured>,
}

impl<'s, 'k, 'a> Check<'s, 'k, 'a> {
    pub fn factory(scoping: &'s Scoping, factory: Factory<'k, 'a>, allowed: &Allowed<'s>) -> Self {
        Self {
            scoping,
            region: Region::factory(factory, allowed.mapped_params),
            helpers: allowed.helpers,
            captured: None,
        }
    }

    pub fn into_captured(self) -> Option<Captured> {
        self.captured
    }
}

impl<'a> Policy<'a> for Check<'_, '_, 'a> {
    fn done(&self) -> bool {
        self.captured.is_some()
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        if self.captured.is_none() {
            let reach = self.region.reach((self.scoping, self.helpers), frame, it);
            self.captured = reach.captured(self.scoping);
        }
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        if self.captured.is_none() && is_direct_eval(self.scoping, &it.callee) {
            self.captured = Some(Captured::Eval);
        }
        Step::Walk
    }
}

pub fn captured_by_factory(
    scoping: &Scoping,
    factory: Factory<'_, '_>,
    allowed: &Allowed<'_>,
) -> Option<Captured> {
    let mut check = Check::factory(scoping, factory, allowed);
    let mut scan = Scan::new(&mut check);
    scan.visit_formal_parameters(factory.params);
    scan.visit_function_body(factory.body);
    check.captured
}

pub fn captured_by_statement(
    scoping: &Scoping,
    statement: &Statement<'_>,
    helpers: &FxHashSet<SymbolId>,
) -> Option<Captured> {
    let mut check = Check {
        scoping,
        region: Region::statement(statement),
        helpers,
        captured: None,
    };
    Scan::new(&mut check).visit_statement(statement);
    check.captured
}
