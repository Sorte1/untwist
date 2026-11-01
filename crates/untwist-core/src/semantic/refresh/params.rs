use oxc_ast::ast::{BindingIdentifier, CatchParameter, Expression, FormalParameters};
use oxc_ast_visit::VisitJs;
use oxc_syntax::symbol::SymbolId;

use super::{Region, Walk, FRAMES};

struct ParamNames<'w> {
    early: &'w mut Vec<SymbolId>,
    symbols: usize,
    complete: bool,
}

impl<'a> VisitJs<'a> for ParamNames<'_> {
    fn visit_expression(&mut self, _it: &Expression<'a>) {}

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        match it.symbol_id.get() {
            Some(symbol) if symbol.index() < self.symbols => self.early.push(symbol),
            _ => self.complete = false,
        }
    }
}

impl Walk<'_, '_> {
    pub(super) fn enter_params(&mut self, params: &FormalParameters<'_>) {
        if !self.open_params(Region::Function) {
            return;
        }
        let mut names = ParamNames {
            early: &mut self.early,
            symbols: self.scoping.symbols_len(),
            complete: true,
        };
        names.visit_formal_parameters(params);
        if !names.complete {
            self.fail();
        }
    }

    pub(super) fn enter_catch_param(&mut self, param: &CatchParameter<'_>) {
        if !self.open_params(Region::Catch) {
            return;
        }
        let mut names = ParamNames {
            early: &mut self.early,
            symbols: self.scoping.symbols_len(),
            complete: true,
        };
        names.visit_binding_pattern(&param.pattern);
        if !names.complete {
            self.fail();
        }
    }

    fn open_params(&mut self, region: Region) -> bool {
        if self.early.capacity() == 0 {
            self.early.reserve(FRAMES);
        }
        match self.frames.last_mut() {
            Some(frame) if frame.region == region && !frame.in_params => {
                frame.in_params = true;
                true
            }
            _ => {
                self.fail();
                false
            }
        }
    }

    pub(super) fn leave_params(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.in_params = false;
        }
    }
}
