use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentTarget, BindingPattern, BlockStatement, CatchClause,
    Expression, ForInStatement, ForOfStatement, ForStatement, Function, IdentifierReference,
    Statement, SwitchStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use crate::shadow::{self, Scoped, Shadows};
use crate::stack;

pub(crate) const UNSOUND_DEFAULT: &str =
    "a parameter default that reads its own parameter, a later one or a binding of the function body";

struct Mentions<'a> {
    names: Vec<&'a str>,
}

impl<'a> VisitJs<'a> for Mentions<'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.names.push(it.name.as_str());
    }
}

pub(crate) fn mentions<'a>(initializer: &Expression<'a>) -> Vec<&'a str> {
    let mut mentions = Mentions { names: Vec::new() };
    mentions.visit_expression(initializer);
    let mut names = mentions.names;
    names.sort_unstable();
    names.dedup();
    names
}

pub(crate) struct Unbound<'a> {
    pub(crate) parameters: Vec<&'a str>,
    pub(crate) body: Vec<&'a str>,
}

struct Reads<'n, 'a> {
    unbound: &'n Unbound<'a>,
    shadows: Shadows<'a>,
    nesting: u32,
    found: bool,
}

impl<'a> Scoped<'a> for Reads<'_, 'a> {
    fn shadows(&mut self) -> &mut Shadows<'a> {
        &mut self.shadows
    }
}

impl<'a> VisitJs<'a> for Reads<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        self.nesting += 1;
        shadow::function(self, it);
        self.nesting -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nesting += 1;
        shadow::arrow(self, it);
        self.nesting -= 1;
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        shadow::block(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        shadow::for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        shadow::for_in(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        shadow::for_of(self, it);
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        shadow::switch(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        shadow::catch(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        if !self.shadows.reaches(name) {
            return;
        }
        let deferred = self.nesting > 0 && self.unbound.body.contains(&name);
        if deferred || self.unbound.parameters.contains(&name) {
            self.found = true;
        }
    }
}

pub(crate) fn reads_unbound<'a>(initializer: &Expression<'a>, unbound: &Unbound<'a>) -> bool {
    if unbound.parameters.is_empty() && unbound.body.is_empty() {
        return false;
    }
    let targets = unbound
        .parameters
        .iter()
        .chain(&unbound.body)
        .copied()
        .collect();
    let mut reads = Reads {
        unbound,
        shadows: Shadows::new(targets, false),
        nesting: 0,
        found: false,
    };
    reads.visit_expression(initializer);
    reads.found || reads.shadows.unresolved
}
