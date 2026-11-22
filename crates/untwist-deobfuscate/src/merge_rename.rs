use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;

pub(crate) fn rename_is_observable(
    branch: &Statement<'_>,
    renamed: &[SymbolId],
    cx: &Cx<'_, '_>,
) -> bool {
    if renamed.is_empty() {
        return false;
    }
    let scoping = cx.scoping();
    let names: FxHashSet<&str> = renamed
        .iter()
        .map(|symbol| scoping.symbol_name(*symbol))
        .collect();
    let classes = renamed
        .iter()
        .any(|symbol| scoping.symbol_flags(*symbol).is_class());
    if cx.names_observed() && (classes || names_a_value(branch, &names)) {
        return true;
    }
    hoists_a_block_function(branch, &names)
}

fn names_a_value(branch: &Statement<'_>, names: &FxHashSet<&str>) -> bool {
    let mut finder = NamedEvaluation {
        names,
        found: false,
    };
    finder.visit_statement(branch);
    finder.found
}

fn hoists_a_block_function(branch: &Statement<'_>, names: &FxHashSet<&str>) -> bool {
    let mut finder = BlockFunctions {
        names,
        found: false,
    };
    finder.visit_statement(branch);
    finder.found
}

struct NamedEvaluation<'n> {
    names: &'n FxHashSet<&'n str>,
    found: bool,
}

impl NamedEvaluation<'_> {
    fn note(&mut self, name: &str, value: Option<&Expression<'_>>) {
        self.found |= self.names.contains(name)
            && value.is_some_and(Expression::is_anonymous_function_definition);
    }
}

impl<'a> Visit<'a> for NamedEvaluation<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(id) = it.id.get_binding_identifier() {
            self.note(&id.name, it.init.as_ref());
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        if let Some(id) = it.left.get_binding_identifier() {
            self.note(&id.name, Some(&it.right));
        }
        walk::walk_assignment_pattern(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let naming = matches!(
            it.operator,
            AssignmentOperator::Assign
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish
        );
        if let (true, AssignmentTarget::AssignmentTargetIdentifier(id)) = (naming, &it.left) {
            self.note(&id.name, Some(&it.right));
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.binding {
            self.note(&id.name, Some(&it.init));
        }
        walk::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.note(&it.binding.name, it.init.as_ref());
        walk::walk_assignment_target_property_identifier(self, it);
    }
}

struct BlockFunctions<'n> {
    names: &'n FxHashSet<&'n str>,
    found: bool,
}

impl<'a> Visit<'a> for BlockFunctions<'_> {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let named = it
            .id
            .as_ref()
            .is_some_and(|id| self.names.contains(id.name.as_str()));
        self.found |= it.is_declaration() && named;
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}
