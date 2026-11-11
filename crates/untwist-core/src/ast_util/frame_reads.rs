use oxc_ast::ast::{
    AssignmentTargetPropertyProperty, BindingProperty, ComputedMemberExpression, Expression,
    Program, PropertyKey, StaticMemberExpression,
};
use oxc_ast_visit::{walk_js, VisitJs};

const FRAME_PROPERTIES: [&str; 2] = ["arguments", "caller"];

fn is_frame_property(name: &str) -> bool {
    FRAME_PROPERTIES.contains(&name)
}

fn is_never_a_function(object: &Expression<'_>) -> bool {
    match object.without_parentheses() {
        Expression::Identifier(id) => id.name == "arguments",
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::TemplateLiteral(_)
        | Expression::ArrayExpression(_)
        | Expression::ObjectExpression(_) => true,
        _ => false,
    }
}

fn static_key_name<'k>(key: &'k PropertyKey<'_>) -> Option<&'k str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

#[derive(Default)]
struct FrameReads {
    found: bool,
}

impl<'a> VisitJs<'a> for FrameReads {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if is_frame_property(it.property.name.as_str()) && !is_never_a_function(&it.object) {
            self.found = true;
            return;
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(literal) = it.expression.without_parentheses() {
            if literal.value.as_str().is_some_and(is_frame_property)
                && !is_never_a_function(&it.object)
            {
                self.found = true;
                return;
            }
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        self.found |= static_key_name(&it.key).is_some_and(is_frame_property);
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &AssignmentTargetPropertyProperty<'a>,
    ) {
        self.found |= static_key_name(&it.name).is_some_and(is_frame_property);
        walk_js::walk_assignment_target_property_property(self, it);
    }
}

pub fn reads_function_frame(program: &Program<'_>) -> bool {
    let mut scan = FrameReads::default();
    scan.visit_program(program);
    scan.found
}
