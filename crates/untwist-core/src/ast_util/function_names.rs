use crate::ast_util::code_text::literal_text;
use crate::ast_util::name_keys::{
    call_reads_name, is_never_a_function, key_may_name, member_key, property_key_may_name, reader,
    NAME,
};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

#[derive(Default)]
pub struct FunctionNames {
    pub givers: Vec<SymbolId>,
    pub name_read: bool,
}

pub fn function_names(program: &Program<'_>, scoping: &Scoping) -> FunctionNames {
    let mut walker = Walker {
        scoping,
        out: FunctionNames::default(),
    };
    walker.visit_program(program);
    walker.out
}

struct Walker<'s> {
    scoping: &'s Scoping,
    out: FunctionNames,
}

fn is_anonymous_function(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => function.id.is_none(),
        Expression::ArrowFunctionExpression(_) => true,
        Expression::ClassExpression(class) => class.id.is_none(),
        _ => false,
    }
}

fn names_on_assignment(operator: AssignmentOperator) -> bool {
    matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalNullish
    )
}

impl Walker<'_> {
    fn give(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.out.givers.push(symbol);
        }
    }

    fn give_pattern(&mut self, pattern: &BindingPattern<'_>, value: Option<&Expression<'_>>) {
        if let (BindingPattern::BindingIdentifier(id), Some(value)) = (pattern, value) {
            if is_anonymous_function(value) {
                self.give(id);
            }
        }
    }

    fn give_reference(&mut self, id: &IdentifierReference<'_>, value: &Expression<'_>) {
        if !is_anonymous_function(value) {
            return;
        }
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol {
            self.out.givers.push(symbol);
        }
    }

    fn read_on(&mut self, object: &Expression<'_>) {
        if !is_never_a_function(object) {
            self.out.name_read = true;
        }
    }

    fn visit_member_parts<'a>(&mut self, member: &MemberExpression<'a>) {
        match member {
            MemberExpression::StaticMemberExpression(it) => self.visit_expression(&it.object),
            MemberExpression::ComputedMemberExpression(it) => {
                self.visit_expression(&it.object);
                self.visit_expression(&it.expression);
            }
            MemberExpression::PrivateFieldExpression(it) => self.visit_expression(&it.object),
        }
    }

    fn visit_reader_call<'a>(&mut self, it: &CallExpression<'a>) -> bool {
        let Some(member) = it.callee.without_parentheses().as_member_expression() else {
            return false;
        };
        let Some(found) = member_key(member).and_then(|key| reader(&key)) else {
            return false;
        };
        if call_reads_name(found, &it.arguments) {
            self.out.name_read = true;
        }
        self.visit_member_parts(member);
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
        true
    }
}

impl<'a> VisitJs<'a> for Walker<'_> {
    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.out.name_read = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.give(id);
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(id) = &it.id {
            self.give(id);
        }
        walk_js::walk_class(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.give_pattern(&it.id, it.init.as_ref());
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        self.give_pattern(&it.left, Some(&it.right));
        walk_js::walk_assignment_pattern(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        self.give_pattern(&it.pattern, it.initializer.as_deref());
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if names_on_assignment(it.operator) {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                self.give_reference(id, &it.right);
            }
        }
        if it.operator == AssignmentOperator::Assign {
            if let Some(member) = it.left.as_member_expression() {
                self.visit_member_parts(member);
                self.visit_expression(&it.right);
                return;
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.binding {
            self.give_reference(id, &it.init);
        }
        walk_js::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if it.binding.name == NAME || reader(&it.binding.name).is_some() {
            self.out.name_read = true;
        }
        if let Some(init) = &it.init {
            self.give_reference(&it.binding, init);
        }
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        if property_key_may_name(&it.name, it.computed) {
            self.out.name_read = true;
        }
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if property_key_may_name(&it.key, it.computed) {
            self.out.name_read = true;
        }
        walk_js::walk_binding_property(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == NAME || reader(&it.property.name).is_some() {
            self.read_on(&it.object);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let reads = literal_text(&it.expression).is_some_and(|key| reader(&key).is_some());
        if reads || key_may_name(&it.expression) {
            self.read_on(&it.object);
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !self.visit_reader_call(it) {
            walk_js::walk_call_expression(self, it);
        }
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        let spreads = it.properties.iter().any(|property| match property {
            ObjectPropertyKind::SpreadProperty(spread) => !is_never_a_function(&spread.argument),
            ObjectPropertyKind::ObjectProperty(_) => false,
        });
        if spreads {
            self.out.name_read = true;
        }
        walk_js::walk_object_expression(self, it);
    }

    fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
        if it.rest.is_some() {
            self.out.name_read = true;
        }
        walk_js::walk_object_pattern(self, it);
    }

    fn visit_object_assignment_target(&mut self, it: &ObjectAssignmentTarget<'a>) {
        if it.rest.is_some() {
            self.out.name_read = true;
        }
        walk_js::walk_object_assignment_target(self, it);
    }
}
