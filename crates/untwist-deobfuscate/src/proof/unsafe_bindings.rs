use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::{Scoping, SymbolId};
use rustc_hash::FxHashSet;

struct UnsafeBindingScan<'s> {
    scoping: &'s Scoping,
    unsafe_bindings: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for UnsafeBindingScan<'_> {
    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if !matches!(it.callee.without_parentheses(), Expression::Identifier(_)) {
            self.visit_expression(&it.callee);
        }
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let is_prototype_object = it.property.name.as_str() == "prototype"
            && matches!(it.object.without_parentheses(), Expression::Identifier(_));
        if !is_prototype_object {
            self.visit_expression(&it.object);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let is_prototype_key = matches!(
            it.expression.without_parentheses(),
            Expression::StringLiteral(literal) if literal.value == "prototype"
        );
        let is_prototype_object = is_prototype_key
            && matches!(it.object.without_parentheses(), Expression::Identifier(_));
        if !is_prototype_object {
            self.visit_expression(&it.object);
        }
        self.visit_expression(&it.expression);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol_id) = it
            .reference_id
            .get()
            .and_then(|r| self.scoping.get_reference(r).symbol_id())
        {
            self.unsafe_bindings.insert(symbol_id);
        }
    }
}

pub(super) fn scan_unsafe_bindings(
    program: &Program<'_>,
    scoping: &Scoping,
) -> FxHashSet<SymbolId> {
    let mut scan = UnsafeBindingScan {
        scoping,
        unsafe_bindings: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.unsafe_bindings
}
