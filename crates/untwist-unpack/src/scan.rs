use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;

#[derive(Clone, Copy, Default)]
pub struct Frame {
    pub functions: u32,
    pub arrows: u32,
    pub members: u32,
}

impl Frame {
    pub fn receiver(self) -> bool {
        self.functions + self.members > 0
    }

    pub fn deferred(self) -> bool {
        self.functions + self.arrows + self.members > 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Walk,
    Skip,
}

pub trait Policy<'a> {
    fn done(&self) -> bool {
        false
    }

    fn reference(&mut self, _frame: Frame, _it: &IdentifierReference<'a>) {}

    fn binding(&mut self, _it: &BindingIdentifier<'a>) {}

    fn label(&mut self, _it: &LabelIdentifier<'a>) {}

    fn call(&mut self, _frame: Frame, _it: &CallExpression<'a>) -> Step {
        Step::Walk
    }

    fn static_member(&mut self, _it: &StaticMemberExpression<'a>) -> Step {
        Step::Walk
    }

    fn computed_member(&mut self, _it: &ComputedMemberExpression<'a>) {}

    fn unary(&mut self, _it: &UnaryExpression<'a>) -> Step {
        Step::Walk
    }

    fn target(&mut self, _it: &SimpleAssignmentTarget<'a>) -> Step {
        Step::Walk
    }

    fn tagged(&mut self, _it: &TaggedTemplateExpression<'a>) {}

    fn operand(&mut self, _it: &IdentifierReference<'a>) {}

    fn item<'k>(&mut self, it: &'k Expression<'a>) -> &'k Expression<'a> {
        it
    }

    fn statement(&mut self, _it: &ExpressionStatement<'a>) {}

    fn declarator(&mut self, _it: &VariableDeclarator<'a>) {}

    fn dynamic_import(&mut self) {}

    fn this(&mut self, _frame: Frame) {}

    fn new_target(&mut self, _frame: Frame) {}

    fn import_meta(&mut self) {}

    fn returns(&mut self, _frame: Frame) {}

    fn with(&mut self);
}

pub struct Scan<'p, P: ?Sized> {
    policy: &'p mut P,
    frame: Frame,
}

impl<'p, P: ?Sized> Scan<'p, P> {
    pub fn new(policy: &'p mut P) -> Self {
        Self {
            policy,
            frame: Frame::default(),
        }
    }
}

impl<'a, P: Policy<'a> + ?Sized> Scan<'_, P> {
    fn operand(&mut self, expression: &Expression<'a>) {
        if let Expression::Identifier(id) = expression.without_parentheses() {
            self.policy.operand(id);
        }
    }

    fn member_value(&mut self, value: Option<&Expression<'a>>) {
        if let Some(value) = value {
            self.frame.members += 1;
            self.visit_expression(value);
            self.frame.members -= 1;
        }
    }
}

pub fn is_direct_eval(scoping: &Scoping, callee: &Expression<'_>) -> bool {
    let Expression::Identifier(id) = callee.without_parentheses() else {
        return false;
    };
    id.name == "eval"
        && id
            .reference_id
            .get()
            .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

impl<'a, P: Policy<'a> + ?Sized> VisitJs<'a> for Scan<'_, P> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.policy.done() {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.policy.statement(it);
        let items = match &it.expression {
            Expression::SequenceExpression(sequence) => sequence.expressions.as_slice(),
            other => std::slice::from_ref(other),
        };
        for item in items {
            let item = self.policy.item(item);
            self.visit_expression(item);
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.policy.declarator(it);
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.policy.dynamic_import();
        walk_js::walk_import_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.policy.reference(self.frame, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.policy.binding(it);
    }

    fn visit_label_identifier(&mut self, it: &LabelIdentifier<'a>) {
        self.policy.label(it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.policy.call(self.frame, it) == Step::Walk {
            walk_js::walk_call_expression(self, it);
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.policy.static_member(it) == Step::Walk {
            walk_js::walk_static_member_expression(self, it);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.policy.computed_member(it);
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if self.policy.unary(it) == Step::Walk {
            walk_js::walk_unary_expression(self, it);
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if self.policy.target(it) == Step::Walk {
            walk_js::walk_simple_assignment_target(self, it);
        }
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.policy.tagged(it);
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        for expression in &it.expressions {
            self.operand(expression);
        }
        walk_js::walk_template_literal(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.operand(&it.left);
        self.operand(&it.right);
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.operand(&it.test);
        walk_js::walk_conditional_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.operand(&it.test);
        walk_js::walk_if_statement(self, it);
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.policy.this(self.frame);
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.policy.new_target(self.frame);
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.policy.import_meta();
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.policy.returns(self.frame);
        walk_js::walk_return_statement(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.policy.with();
        walk_js::walk_with_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.frame.functions += 1;
        walk_js::walk_function(self, it, flags);
        self.frame.functions -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.frame.arrows += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.frame.arrows -= 1;
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.visit_decorators(&it.decorators);
        if it.computed {
            self.visit_property_key(&it.key);
        }
        self.member_value(it.value.as_ref());
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        self.visit_decorators(&it.decorators);
        if it.computed {
            self.visit_property_key(&it.key);
        }
        self.member_value(it.value.as_ref());
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.frame.members += 1;
        walk_js::walk_static_block(self, it);
        self.frame.members -= 1;
    }
}
