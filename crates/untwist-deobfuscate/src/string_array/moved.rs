use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use crate::stamp::DynamicScopes;

#[derive(Default)]
struct Names {
    found: Vec<Option<SymbolId>>,
}

impl<'a> VisitJs<'a> for Names {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found.push(it.symbol_id.get());
    }
}

#[derive(Default)]
struct Moved {
    escapes: bool,
    arrows: usize,
    vars: Vec<Option<SymbolId>>,
}

impl<'a> VisitJs<'a> for Moved {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        self.escapes |= it.is_declaration();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.arrows += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.arrows -= 1;
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.escapes = true;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.escapes = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.escapes = true;
    }

    fn visit_await_expression(&mut self, _it: &AwaitExpression<'a>) {
        self.escapes = true;
    }

    fn visit_yield_expression(&mut self, _it: &YieldExpression<'a>) {
        self.escapes = true;
    }

    fn visit_return_statement(&mut self, _it: &ReturnStatement<'a>) {
        self.escapes = true;
    }

    fn visit_break_statement(&mut self, _it: &BreakStatement<'a>) {
        self.escapes = true;
    }

    fn visit_continue_statement(&mut self, _it: &ContinueStatement<'a>) {
        self.escapes = true;
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.escapes |= it.r#await;
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.escapes |= it.name == "arguments" || it.name == "eval";
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        self.escapes |= disposes(it);
        if it.kind == VariableDeclarationKind::Var && self.arrows == 0 {
            let mut names = Names::default();
            for declarator in &it.declarations {
                names.visit_binding_pattern(&declarator.id);
            }
            self.vars.extend(names.found);
        }
        walk_js::walk_variable_declaration(self, it);
    }
}

pub(super) fn moved_vars(stmt: &Statement<'_>) -> Option<Vec<SymbolId>> {
    let mut moved = Moved::default();
    moved.visit_statement(stmt);
    if moved.escapes {
        return None;
    }
    moved.vars.into_iter().collect()
}

pub(super) fn disposes(it: &VariableDeclaration<'_>) -> bool {
    matches!(
        it.kind,
        VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing
    )
}

struct Closures<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    depth: usize,
    found: bool,
}

impl<'a> VisitJs<'a> for Closures<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.depth += 1;
        walk_js::walk_class(self, it);
        self.depth -= 1;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.found |= self.depth > 0 && symbol == Some(self.symbol);
    }
}

pub(super) fn read_by_closure(stmt: &Statement<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let mut closures = Closures {
        scoping,
        symbol,
        depth: 0,
        found: false,
    };
    closures.visit_statement(stmt);
    closures.found
}

pub(super) fn never_rebound(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_redeclarations(symbol).is_empty()
        && DynamicScopes::of(scoping)
            .refusal(scoping, symbol)
            .is_none()
        && scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|reference| !scoping.get_reference(*reference).is_write())
}

macro_rules! loop_aware_visits {
    () => {
        fn visit_function(
            &mut self,
            it: &mut oxc_ast::ast::Function<'a>,
            flags: oxc_syntax::scope::ScopeFlags,
        ) {
            let was = std::mem::replace(&mut self.looped, false);
            oxc_ast_visit::walk_mut::walk_function(self, it, flags);
            self.looped = was;
        }

        fn visit_arrow_function_expression(
            &mut self,
            it: &mut oxc_ast::ast::ArrowFunctionExpression<'a>,
        ) {
            let was = std::mem::replace(&mut self.looped, false);
            oxc_ast_visit::walk_mut::walk_arrow_function_expression(self, it);
            self.looped = was;
        }

        fn visit_static_block(&mut self, it: &mut oxc_ast::ast::StaticBlock<'a>) {
            let was = std::mem::replace(&mut self.looped, false);
            oxc_ast_visit::walk_mut::walk_static_block(self, it);
            self.looped = was;
        }

        fn visit_for_statement(&mut self, it: &mut oxc_ast::ast::ForStatement<'a>) {
            let was = std::mem::replace(&mut self.looped, true);
            oxc_ast_visit::walk_mut::walk_for_statement(self, it);
            self.looped = was;
        }

        fn visit_for_in_statement(&mut self, it: &mut oxc_ast::ast::ForInStatement<'a>) {
            let was = std::mem::replace(&mut self.looped, true);
            oxc_ast_visit::walk_mut::walk_for_in_statement(self, it);
            self.looped = was;
        }

        fn visit_for_of_statement(&mut self, it: &mut oxc_ast::ast::ForOfStatement<'a>) {
            let was = std::mem::replace(&mut self.looped, true);
            oxc_ast_visit::walk_mut::walk_for_of_statement(self, it);
            self.looped = was;
        }

        fn visit_while_statement(&mut self, it: &mut oxc_ast::ast::WhileStatement<'a>) {
            let was = std::mem::replace(&mut self.looped, true);
            oxc_ast_visit::walk_mut::walk_while_statement(self, it);
            self.looped = was;
        }

        fn visit_do_while_statement(&mut self, it: &mut oxc_ast::ast::DoWhileStatement<'a>) {
            let was = std::mem::replace(&mut self.looped, true);
            oxc_ast_visit::walk_mut::walk_do_while_statement(self, it);
            self.looped = was;
        }
    };
}

pub(super) use loop_aware_visits;
