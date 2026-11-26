use oxc_ast::ast::{
    AssignmentExpression, AssignmentOperator, AssignmentTarget, Expression, IdentifierReference,
    ObjectPropertyKind, Program, PropertyKind, Statement,
};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::generate::is_identifier_name;

const MODULE: &str = "module";

const EXPORTS: &str = "exports";

pub type ExportNames = FxHashMap<SymbolId, String>;

struct Found<'s> {
    scoping: &'s Scoping,
    names: FxHashMap<SymbolId, Option<String>>,
}

impl Found<'_> {
    fn note(&mut self, key: Option<&str>, value: &Expression<'_>) {
        let Expression::Identifier(value) = value.without_parentheses() else {
            return;
        };
        let Some(symbol) = self.root_symbol(value) else {
            return;
        };
        let key = key
            .filter(|name| is_identifier_name(name))
            .map(str::to_string);
        self.names
            .entry(symbol)
            .and_modify(|seen| *seen = None)
            .or_insert(key);
    }

    fn root_symbol(&self, value: &IdentifierReference<'_>) -> Option<SymbolId> {
        value
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .filter(|symbol| self.scoping.symbol_scope_id(*symbol) == self.scoping.root_scope_id())
    }

    fn is_unbound(&self, id: &IdentifierReference<'_>, name: &str) -> bool {
        id.name == name
            && id.reference_id.get().is_some_and(|reference| {
                self.scoping.get_reference(reference).symbol_id().is_none()
            })
    }

    fn is_exports_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.is_unbound(id, EXPORTS),
            Expression::StaticMemberExpression(member) => {
                member.property.name == EXPORTS
                    && matches!(member.object.without_parentheses(),
                        Expression::Identifier(id) if self.is_unbound(id, MODULE))
            }
            _ => false,
        }
    }

    fn assignment(&mut self, assignment: &AssignmentExpression<'_>) {
        if assignment.operator != AssignmentOperator::Assign {
            return;
        }
        let AssignmentTarget::StaticMemberExpression(target) = &assignment.left else {
            return;
        };
        if target.property.name == EXPORTS
            && matches!(target.object.without_parentheses(),
                Expression::Identifier(id) if self.is_unbound(id, MODULE))
        {
            self.object(&assignment.right);
            return;
        }
        if self.is_exports_object(&target.object) {
            self.note(Some(target.property.name.as_str()), &assignment.right);
        }
    }

    fn object(&mut self, value: &Expression<'_>) {
        let Expression::ObjectExpression(object) = value.without_parentheses() else {
            return;
        };
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            if property.computed || property.method || property.kind != PropertyKind::Init {
                continue;
            }
            let key = property.key.static_name();
            self.note(key.as_ref().and_then(|name| name.as_str()), &property.value);
        }
    }
}

pub fn export_names(program: &Program<'_>, scoping: &Scoping) -> ExportNames {
    let mut found = Found {
        scoping,
        names: FxHashMap::default(),
    };
    for statement in &program.body {
        let Statement::ExpressionStatement(statement) = statement else {
            continue;
        };
        if let Expression::AssignmentExpression(assignment) =
            statement.expression.without_parentheses()
        {
            found.assignment(assignment);
        }
    }
    found
        .names
        .into_iter()
        .filter_map(|(symbol, key)| Some((symbol, key?)))
        .collect()
}
