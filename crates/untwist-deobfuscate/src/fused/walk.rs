use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use super::Member;

pub(crate) struct Walk<M>(pub(crate) M);

macro_rules! nested {
    ($($hook:ident($node:ty, $walk:ident);)*) => {
        $(
            fn $hook(&mut self, it: &$node) {
                let saved = self.0.enter_nested();
                walk_js::$walk(self, it);
                self.0.leave_nested(saved);
            }
        )*
    };
}

macro_rules! element {
    ($($hook:ident($node:ty, $walk:ident);)*) => {
        $(
            fn $hook(&mut self, it: &$node) {
                let saved = self.0.enter_class_element();
                walk_js::$walk(self, it);
                self.0.leave_class_element(saved);
            }
        )*
    };
}

impl<'a, M: Member<'a>> VisitJs<'a> for Walk<M> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        let saved = self.0.enter_statements(it);
        walk_js::walk_statements(self, it);
        self.0.leave_statements(saved);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        let saved = self.0.enter_statement(it);
        walk_js::walk_statement(self, it);
        self.0.leave_statement(saved);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let Some(saved) = self.0.enter_expression(it) else {
            return;
        };
        walk_js::walk_expression(self, it);
        self.0.leave_expression(saved);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.0.identifier_reference(it);
        walk_js::walk_identifier_reference(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.0.binding_identifier(it);
        walk_js::walk_binding_identifier(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.0.assignment_expression(it);
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let saved = self.0.enter_variable_declarator(it);
        walk_js::walk_variable_declarator(self, it);
        self.0.leave_variable_declarator(saved);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        self.0.formal_parameter(it);
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_catch_parameter(&mut self, it: &CatchParameter<'a>) {
        self.0.catch_parameter(it);
        walk_js::walk_catch_parameter(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let saved = self.0.enter_function(it);
        walk_js::walk_function(self, it, flags);
        self.0.leave_function(saved);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let saved = self.0.enter_arrow(it);
        walk_js::walk_arrow_function_expression(self, it);
        self.0.leave_arrow(saved);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let saved = self.0.enter_class(it);
        walk_js::walk_class(self, it);
        self.0.leave_class(saved);
    }

    fn visit_class_heritage(&mut self, it: &ClassHeritage<'a>) {
        let saved = self.0.enter_class_heritage();
        walk_js::walk_class_heritage(self, it);
        self.0.leave_class_heritage(saved);
    }

    fn visit_property_key(&mut self, it: &PropertyKey<'a>) {
        let saved = self.0.enter_property_key();
        walk_js::walk_property_key(self, it);
        self.0.leave_property_key(saved);
    }

    element! {
        visit_method_definition(MethodDefinition<'a>, walk_method_definition);
        visit_property_definition(PropertyDefinition<'a>, walk_property_definition);
        visit_accessor_property(AccessorProperty<'a>, walk_accessor_property);
    }

    nested! {
        visit_class_body(ClassBody<'a>, walk_class_body);
        visit_for_statement(ForStatement<'a>, walk_for_statement);
        visit_for_in_statement(ForInStatement<'a>, walk_for_in_statement);
        visit_for_of_statement(ForOfStatement<'a>, walk_for_of_statement);
        visit_while_statement(WhileStatement<'a>, walk_while_statement);
        visit_do_while_statement(DoWhileStatement<'a>, walk_do_while_statement);
    }
}
