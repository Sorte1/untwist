use crate::ast_util::js_text::js_text;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

const NAME: &str = "name";

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

fn is_never_a_function(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}

fn is_name_text(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => js_text(literal.value) == NAME,
        Expression::TemplateLiteral(template) => {
            template.expressions.is_empty()
                && template
                    .quasis
                    .first()
                    .and_then(|quasi| quasi.value.cooked)
                    .is_some_and(|cooked| js_text(cooked) == NAME)
        }
        _ => false,
    }
}

fn is_name_key(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name.as_str() == NAME,
        PropertyKey::PrivateIdentifier(_) => false,
        other => other.as_expression().is_some_and(is_name_text),
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

    fn assigned_member_object<'e, 'a>(
        target: &'e AssignmentTarget<'a>,
    ) -> Option<&'e Expression<'a>> {
        match target {
            AssignmentTarget::StaticMemberExpression(member) if member.property.name == NAME => {
                Some(&member.object)
            }
            AssignmentTarget::ComputedMemberExpression(member)
                if is_name_text(&member.expression) =>
            {
                Some(&member.object)
            }
            _ => None,
        }
    }
}

impl<'a> VisitJs<'a> for Walker<'_> {
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
            if let Some(object) = Self::assigned_member_object(&it.left) {
                self.visit_expression(object);
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
        if it.binding.name == NAME {
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
        if is_name_key(&it.name) {
            self.out.name_read = true;
        }
        walk_js::walk_assignment_target_property_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if is_name_key(&it.key) {
            self.out.name_read = true;
        }
        walk_js::walk_binding_property(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if it.property.name == NAME {
            self.read_on(&it.object);
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if is_name_text(&it.expression) {
            self.read_on(&it.object);
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}
