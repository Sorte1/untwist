use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

const GLOBAL_NAMES: [&str; 6] = ["globalThis", "global", "self", "window", "Function", "eval"];

struct Reach<'s, 'n> {
    scoping: &'s Scoping,
    slot: &'n str,
    global_param: Option<SymbolId>,
    functions: usize,
    writing: bool,
    found: bool,
}

impl Reach<'_, '_> {
    fn global_name(&self, id: &IdentifierReference<'_>) -> bool {
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        match symbol {
            Some(symbol) => Some(symbol) == self.global_param,
            None => GLOBAL_NAMES.contains(&id.name.as_str()),
        }
    }

    fn global_object(&self, expr: &Expression<'_>) -> Option<bool> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.global_name(id).then_some(true),
            Expression::ThisExpression(_) => (self.functions > 0).then_some(false),
            _ => None,
        }
    }

    fn key(&self, key: &PropertyKey<'_>) -> bool {
        match key {
            PropertyKey::StaticIdentifier(id) => id.name == self.slot,
            PropertyKey::StringLiteral(literal) => literal.value.as_str() == Some(self.slot),
            _ => false,
        }
    }
}

impl<'a> VisitJs<'a> for Reach<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= self.global_name(it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.found |= it.property.name == self.slot;
        if self.global_object(&it.object).is_none() {
            let writing = std::mem::replace(&mut self.writing, false);
            self.visit_expression(&it.object);
            self.writing = writing;
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let key = match it.expression.without_parentheses() {
            Expression::StringLiteral(literal) => Some(literal.value.as_str() == Some(self.slot)),
            Expression::NumericLiteral(_) => Some(false),
            _ => None,
        };
        self.found |= key == Some(true);
        let writing = std::mem::replace(&mut self.writing, false);
        match self.global_object(&it.object) {
            Some(_) => self.found |= writing && key.is_none(),
            None => self.visit_expression(&it.object),
        }
        self.visit_expression(&it.expression);
        self.writing = writing;
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        let writing = std::mem::replace(&mut self.writing, true);
        walk_js::walk_simple_assignment_target(self, it);
        self.writing = writing;
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let skipped = match (&it.operator, it.argument.without_parentheses()) {
            (UnaryOperator::Typeof, Expression::Identifier(id)) => self.global_name(id),
            _ => false,
        };
        if skipped {
            return;
        }
        let writing = std::mem::replace(&mut self.writing, it.operator == UnaryOperator::Delete);
        walk_js::walk_unary_expression(self, it);
        self.writing = writing;
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if !it.operator.is_equality() {
            walk_js::walk_binary_expression(self, it);
            return;
        }
        for side in [&it.left, &it.right] {
            let compared = matches!(side.without_parentheses(),
                Expression::Identifier(id) if self.global_name(id));
            if !compared {
                self.visit_expression(side);
            }
        }
    }

    fn visit_property_key(&mut self, it: &PropertyKey<'a>) {
        self.found |= self.key(it);
        walk_js::walk_property_key(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.functions += 1;
        walk_js::walk_function(self, it, flags);
        self.functions -= 1;
    }
}

pub fn reaches_slot(
    scoping: &Scoping,
    body: &FunctionBody<'_>,
    slot: &str,
    global_param: Option<SymbolId>,
) -> bool {
    let mut reach = Reach {
        scoping,
        slot,
        global_param,
        functions: 0,
        writing: false,
        found: false,
    };
    reach.visit_function_body(body);
    reach.found
}
