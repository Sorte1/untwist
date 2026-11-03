use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

use super::collect::{Collector, Key};

impl<'a> VisitJs<'a> for Collector<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.fn_stack.push(it.span.start);
        walk_js::walk_function(self, it, flags);
        self.fn_stack.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.fn_stack.push(it.span.start);
        if let Some(node) = it.get_expression().and_then(|e| self.node_of(e)) {
            let ret = self.ret_node(it.span.start);
            self.edge(node, ret);
        }
        walk_js::walk_arrow_function_expression(self, it);
        self.fn_stack.pop();
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let (Some(&top), Some(argument)) = (self.fn_stack.last(), it.argument.as_ref()) {
            if let Some(node) = self.node_of(argument) {
                let ret = self.ret_node(top);
                self.edge(node, ret);
            }
        }
        walk_js::walk_return_statement(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        let node = self.read_node(it.span());
        let key = self.key_of_member(it);
        let holder = self.field_node(&key);
        self.edge(holder, node);
        walk_js::walk_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(callee) = self.resolved_callee(&it.callee) {
            self.bind_arguments(&callee, &it.arguments);
        } else {
            let callee = it.callee.without_parentheses();
            if let Some(node) = self.node_of(callee) {
                self.sink(node, it.span);
            }
            if let Some(member) = callee.as_member_expression() {
                if self.forwards(member) {
                    if let Some(node) = self.node_of(member.object()) {
                        self.sink(node, it.span);
                    }
                }
            }
            if !self.inert_native(callee) {
                self.sink_arguments(&it.arguments, it.span);
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if let Some(node) = self.node_of(&it.callee) {
            self.sink(node, it.span);
        }
        self.sink_arguments(&it.arguments, it.span);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if let Some(node) = self.node_of(&it.tag) {
            self.sink(node, it.span);
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let Some(value) = self.node_of(&it.right) {
                if let Some(member) = it.left.as_member_expression() {
                    let key = self.key_of_member(member);
                    self.store(value, &key, it.span);
                } else if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                    if let Some(bind) = self.symbol_of(id).and_then(|s| self.bind_node(s)) {
                        self.edge(value, bind);
                    }
                }
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, it.init.as_ref()) {
            if let Some(value) = self.node_of(init) {
                if let Some(bind) = id.symbol_id.get().and_then(|s| self.bind_node(s)) {
                    self.edge(value, bind);
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        for property in &it.properties {
            if let ObjectPropertyKind::ObjectProperty(prop) = property {
                if let Some(value) = self.node_of(&prop.value) {
                    let key = self.key_of_property(&prop.key);
                    self.store(value, &key, it.span);
                }
            }
        }
        walk_js::walk_object_expression(self, it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        for element in &it.elements {
            if let Some(value) = element.as_expression().and_then(|e| self.node_of(e)) {
                let holder = self.field_node(&Key::Index);
                self.edge(value, holder);
            }
        }
        walk_js::walk_array_expression(self, it);
    }
}
