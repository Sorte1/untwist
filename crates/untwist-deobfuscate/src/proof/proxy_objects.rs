use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SymbolId};
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use untwist_core::ast_util::js_text::js_text;

fn property_key_str<'e>(key: &PropertyKey<'e>) -> Option<Cow<'e, str>> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(Cow::Borrowed(id.name.as_str())),
        PropertyKey::StringLiteral(literal) => Some(js_text(literal.value)),
        _ => None,
    }
}

struct ProxyObjectScan<'s> {
    scoping: &'s Scoping,
    candidates: FxHashMap<SymbolId, FxHashSet<String>>,
}

impl ProxyObjectScan<'_> {
    fn mark_written(&mut self, object: &Expression<'_>) {
        if let Expression::Identifier(id) = object.without_parentheses() {
            if let Some(symbol_id) = id
                .reference_id
                .get()
                .and_then(|r| self.scoping.get_reference(r).symbol_id())
            {
                self.candidates.remove(&symbol_id);
            }
        }
    }
}

impl<'a> VisitJs<'a> for ProxyObjectScan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Expression::ObjectExpression(object) = init.without_parentheses() {
                if let Some(symbol_id) = id.symbol_id.get() {
                    let mut functions = FxHashSet::default();
                    for property in &object.properties {
                        if let ObjectPropertyKind::ObjectProperty(property) = property {
                            if matches!(
                                property.value.without_parentheses(),
                                Expression::FunctionExpression(_)
                            ) {
                                if let Some(key) = property_key_str(&property.key) {
                                    functions.insert(key.to_string());
                                }
                            }
                        }
                    }
                    self.candidates.insert(symbol_id, functions);
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        match it {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                if let Some(symbol_id) = id
                    .reference_id
                    .get()
                    .and_then(|r| self.scoping.get_reference(r).symbol_id())
                {
                    self.candidates.remove(&symbol_id);
                }
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                self.mark_written(&member.object);
                self.visit_expression(&member.object);
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.mark_written(&member.object);
                self.visit_expression(&member.object);
                self.visit_expression(&member.expression);
            }
            _ => walk_js::walk_assignment_target(self, it),
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            match it.argument.without_parentheses() {
                Expression::StaticMemberExpression(member) => self.mark_written(&member.object),
                Expression::ComputedMemberExpression(member) => self.mark_written(&member.object),
                _ => {}
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if !matches!(it.object.without_parentheses(), Expression::Identifier(_)) {
            self.visit_expression(&it.object);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if !matches!(it.object.without_parentheses(), Expression::Identifier(_)) {
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
            self.candidates.remove(&symbol_id);
        }
    }
}

pub(super) fn scan_proxy_functions(
    program: &Program<'_>,
    scoping: &Scoping,
) -> FxHashMap<SymbolId, FxHashSet<String>> {
    let mut scan = ProxyObjectScan {
        scoping,
        candidates: FxHashMap::default(),
    };
    scan.visit_program(program);
    scan.candidates
}
