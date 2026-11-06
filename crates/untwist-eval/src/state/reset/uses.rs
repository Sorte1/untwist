use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, AssignmentTargetPattern, BindingIdentifier,
    BlockStatement, CatchClause, Expression, ForInStatement, ForOfStatement, ForStatement,
    Function, IdentifierReference, MemberExpression, SimpleAssignmentTarget, Statement,
    SwitchStatement, WithStatement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use super::{own_key, target_key_of, Keys};
use crate::mutants::reset_names_loose;
use crate::shadow::{self, Scoped, Shadows};
use crate::stack;

struct Uses<'s, 'a> {
    keys: &'s Keys<'a>,
    shadows: Shadows<'a>,
    value_bindings: u32,
    array_bindings: u32,
    value_refs: u32,
    array_writes: u32,
    state_accesses: u32,
    memo_reads: u32,
    opaque: bool,
}

impl<'a> Uses<'_, 'a> {
    fn proven(&self) -> bool {
        self.value_bindings == 1
            && self.array_bindings == 1
            && self.value_refs == 2
            && self.array_writes == 0
            && self.state_accesses == 2
            && self.memo_reads == 0
            && !self.opaque
            && !self.shadows.unresolved
    }

    fn is_own_memo(&self, expr: &Expression<'a>) -> bool {
        expr.without_parentheses()
            .as_member_expression()
            .and_then(|member| own_key(member, self.keys.decoder))
            == Some(self.keys.memo)
    }

    fn is_value(&self, name: &str) -> bool {
        name == self.keys.value && self.shadows.reaches(name)
    }

    fn is_array(&self, name: &str) -> bool {
        name == self.keys.array && self.shadows.reaches(name)
    }

    fn enter_root(&mut self, function: &Function<'a>) {
        if let Some(id) = &function.id {
            self.visit_binding_identifier(id);
        }
        self.visit_formal_parameters(&function.params);
        if let Some(body) = &function.body {
            self.shadows.note_top_functions(&body.statements);
            self.visit_function_body(body);
        }
    }
}

impl<'a> Scoped<'a> for Uses<'_, 'a> {
    fn shadows(&mut self) -> &mut Shadows<'a> {
        &mut self.shadows
    }
}

impl<'a> VisitJs<'a> for Uses<'_, 'a> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        shadow::function(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        shadow::arrow(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        shadow::block(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        shadow::for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        shadow::for_in(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        shadow::for_of(self, it);
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        shadow::switch(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        shadow::catch(self, it);
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.opaque = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let name = it.name.as_str();
        self.value_bindings += u32::from(self.is_value(name));
        self.array_bindings += u32::from(self.is_array(name));
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.value_refs += u32::from(self.is_value(name));
        self.opaque |= name == "eval";
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.array_writes += u32::from(self.is_array(id.name.as_str()));
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_pattern(&mut self, _it: &AssignmentTargetPattern<'a>) {
        self.opaque = true;
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let written = target_key_of(&it.left, self.keys.decoder);
        if written == Some(self.keys.state) || written == Some(self.keys.memo) {
            self.state_accesses += u32::from(written == Some(self.keys.state));
            self.visit_expression(&it.right);
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if let MemberExpression::ComputedMemberExpression(computed) = it {
            if self.is_own_memo(&computed.object) {
                self.visit_expression(&computed.expression);
                return;
            }
        }
        let key = own_key(it, self.keys.decoder);
        self.state_accesses += u32::from(key == Some(self.keys.state));
        self.memo_reads += u32::from(key == Some(self.keys.memo));
        walk_js::walk_member_expression(self, it);
    }
}

pub(super) fn uses_are_exact<'a>(function: &Function<'a>, keys: &Keys<'a>) -> bool {
    let mut uses = Uses {
        keys,
        shadows: Shadows::new(vec![keys.value, keys.array], reset_names_loose()),
        value_bindings: 0,
        array_bindings: 0,
        value_refs: 0,
        array_writes: 0,
        state_accesses: 0,
        memo_reads: 0,
        opaque: false,
    };
    uses.enter_root(function);
    uses.proven()
}
