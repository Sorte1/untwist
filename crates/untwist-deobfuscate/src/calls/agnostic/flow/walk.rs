use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::GetSpan;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;

use super::collect::{Collector, Key};

impl<'a> VisitJs<'a> for Collector<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.fn_stack.push(it.span.start);
        let patterns = std::mem::take(&mut self.pattern_depth);
        walk_js::walk_function(self, it, flags);
        self.pattern_depth = patterns;
        self.fn_stack.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.fn_stack.push(it.span.start);
        if let Some(body) = it.get_expression() {
            let ret = self.ret_node(it.span.start);
            self.route_into(body, ret);
        }
        let patterns = std::mem::take(&mut self.pattern_depth);
        walk_js::walk_arrow_function_expression(self, it);
        self.pattern_depth = patterns;
        self.fn_stack.pop();
    }

    fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
        self.pattern_depth += 1;
        walk_js::walk_object_pattern(self, it);
        self.pattern_depth -= 1;
    }

    fn visit_array_pattern(&mut self, it: &ArrayPattern<'a>) {
        self.pattern_depth += 1;
        walk_js::walk_array_pattern(self, it);
        self.pattern_depth -= 1;
    }

    fn visit_array_assignment_target(&mut self, it: &ArrayAssignmentTarget<'a>) {
        self.pattern_depth += 1;
        walk_js::walk_array_assignment_target(self, it);
        self.pattern_depth -= 1;
    }

    fn visit_object_assignment_target(&mut self, it: &ObjectAssignmentTarget<'a>) {
        self.pattern_depth += 1;
        walk_js::walk_object_assignment_target(self, it);
        self.pattern_depth -= 1;
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.pattern_depth > 0 {
            if let Some(bind) = it.symbol_id.get().and_then(|s| self.bind_node(s)) {
                let any = self.any_node();
                self.edge(any, bind);
            }
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let (SimpleAssignmentTarget::AssignmentTargetIdentifier(id), true) =
            (it, self.pattern_depth > 0)
        {
            let bind = self.ident_node(id);
            let any = self.any_node();
            self.edge(any, bind);
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        self.sink_expr(&it.init, it.span);
        walk_js::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        let bind = self.ident_node(&it.binding);
        let any = self.any_node();
        self.edge(any, bind);
        if let Some(init) = it.init.as_ref() {
            self.sink_expr(init, it.span);
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator == BinaryOperator::Instanceof {
            self.sink_expr(&it.left, it.span);
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.sink_expr(&it.source, it.span);
        if let Some(options) = it.options.as_ref() {
            self.sink_expr(options, it.span);
        }
        walk_js::walk_import_expression(self, it);
    }

    fn visit_v8_intrinsic_expression(&mut self, it: &V8IntrinsicExpression<'a>) {
        self.sink_arguments(&it.arguments, it.span);
        self.sink_spreads(&it.arguments, it.span);
        walk_js::walk_v8_intrinsic_expression(self, it);
    }

    fn visit_jsx_expression_container(&mut self, it: &JSXExpressionContainer<'a>) {
        if let Some(expr) = it.expression.as_expression() {
            self.sink_expr(expr, it.span);
        }
        walk_js::walk_jsx_expression_container(self, it);
    }

    fn visit_jsx_spread_attribute(&mut self, it: &JSXSpreadAttribute<'a>) {
        self.sink_expr(&it.argument, it.span);
        walk_js::walk_jsx_spread_attribute(self, it);
    }

    fn visit_jsx_spread_child(&mut self, it: &JSXSpreadChild<'a>) {
        self.sink_expr(&it.expression, it.span);
        walk_js::walk_jsx_spread_child(self, it);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let (Some(&top), Some(argument)) = (self.fn_stack.last(), it.argument.as_ref()) {
            let ret = self.ret_node(top);
            self.route_into(argument, ret);
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
        if self.reads_closed_runner(it) {
            self.read_node(it.span);
        }
        let spread = Self::has_spread(&it.arguments);
        self.sink_spreads(&it.arguments, it.span);
        match self.resolved_callee(&it.callee).filter(|_| !spread) {
            Some(start) => self.bind_arguments(start, &it.arguments, it.span),
            None => {
                let callee = it.callee.without_parentheses();
                self.sink_expr(callee, it.span);
                if let Some(member) = callee.as_member_expression() {
                    if self.forwards(member) {
                        self.sink_expr(member.object(), it.span);
                    }
                }
                if !self.inert_native(callee) {
                    self.sink_arguments(&it.arguments, it.span);
                }
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.sink_expr(&it.callee, it.span);
        self.sink_arguments(&it.arguments, it.span);
        self.sink_spreads(&it.arguments, it.span);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.sink_expr(&it.tag, it.span);
        for expression in &it.quasi.expressions {
            self.sink_expr(expression, it.span);
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let routes = matches!(
            it.operator,
            AssignmentOperator::Assign
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalNullish
        );
        if routes {
            match &it.left {
                AssignmentTarget::AssignmentTargetIdentifier(id) => {
                    let bind = self.ident_node(id);
                    self.route_into(&it.right, bind);
                }
                AssignmentTarget::ArrayAssignmentTarget(_)
                | AssignmentTarget::ObjectAssignmentTarget(_) => {
                    self.sink_expr(&it.right, it.span);
                }
                other => {
                    if let Some(member) = other.as_member_expression() {
                        let key = self.key_of_member(member);
                        self.store_expr(&it.right, &key, it.span);
                    }
                }
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = it.init.as_ref() {
            match &it.id {
                BindingPattern::BindingIdentifier(id) => {
                    if let Some(bind) = id.symbol_id.get().and_then(|s| self.bind_node(s)) {
                        self.route_into(init, bind);
                    }
                }
                _ => self.sink_expr(init, it.span),
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        self.sink_expr(&it.right, it.span);
        walk_js::walk_assignment_pattern(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if let Some(initializer) = it.initializer.as_ref() {
            self.sink_expr(initializer, it.span);
        }
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        for property in &it.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(prop) => match prop.kind {
                    PropertyKind::Init => {
                        let key = self.key_of_property(&prop.key);
                        self.store_expr(&prop.value, &key, it.span);
                    }
                    PropertyKind::Get | PropertyKind::Set => {
                        if let Some(start) = self.accessor_start(&prop.value) {
                            let ret = self.ret_node(start);
                            self.sink(ret, it.span);
                        }
                    }
                },
                ObjectPropertyKind::SpreadProperty(spread) => {
                    self.sink_expr(&spread.argument, it.span);
                }
            }
        }
        walk_js::walk_object_expression(self, it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        for element in &it.elements {
            match element {
                ArrayExpressionElement::SpreadElement(spread) => {
                    self.sink_expr(&spread.argument, it.span);
                }
                other => {
                    if let Some(value) = other.as_expression() {
                        let holder = self.field_node(&Key::Index);
                        self.route_into(value, holder);
                    }
                }
            }
        }
        walk_js::walk_array_expression(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(heritage) = it.heritage.as_ref() {
            self.sink_expr(&heritage.expression, it.span);
        }
        for element in &it.body.body {
            match element {
                ClassElement::PropertyDefinition(prop) => {
                    if let Some(value) = prop.value.as_ref() {
                        self.sink_expr(value, prop.span);
                    }
                }
                ClassElement::AccessorProperty(prop) => {
                    if let Some(value) = prop.value.as_ref() {
                        self.sink_expr(value, prop.span);
                    }
                }
                ClassElement::MethodDefinition(method) => {
                    if matches!(method.kind, MethodDefinitionKind::Get) {
                        let ret = self.ret_node(method.value.span.start);
                        self.sink(ret, method.span);
                    }
                }
                _ => {}
            }
        }
        walk_js::walk_class(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.sink_expr(&it.right, it.span);
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.sink_expr(&it.right, it.span);
        self.pattern_depth += 1;
        self.visit_for_statement_left(&it.left);
        self.pattern_depth -= 1;
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
        self.sink_expr(&it.argument, it.span);
        walk_js::walk_throw_statement(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.sink_expr(&it.object, it.span);
        self.visit_expression(&it.object);
        self.with_depth += 1;
        self.visit_statement(&it.body);
        self.with_depth -= 1;
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        if let Some(argument) = it.argument.as_ref() {
            self.sink_expr(argument, it.span);
        }
        walk_js::walk_yield_expression(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        self.sink_expr(&it.argument, it.span);
        walk_js::walk_await_expression(self, it);
    }
}
