use oxc_ast::ast::{BindingPattern, Expression, Function};
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use rustc_hash::FxHashSet;
use untwist_core::pass::PassOptions;

use crate::stamp::{is_machinery_name, may_name_machinery, original_name};

pub const CALL_SOUNDNESS_CHECKS: &str = "decoder-calls/soundness-checks";

pub fn call_checks_apply(opts: &PassOptions) -> bool {
    !opts.unsafe_transforms && !opts.rule_disabled(CALL_SOUNDNESS_CHECKS)
}

pub fn is_plain_function(function: &Function<'_>) -> bool {
    if function.r#async || function.generator || function.params.rest.is_some() {
        return false;
    }
    let mut names = FxHashSet::default();
    function.params.items.iter().all(|item| {
        item.initializer.is_none()
            && matches!(
                &item.pattern,
                BindingPattern::BindingIdentifier(id) if names.insert(id.name.as_str())
            )
    })
}

pub fn evaluates_every_operand(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::AssignmentExpression(assignment) => !assignment.operator.is_logical(),
        Expression::CallExpression(call) => !call.optional,
        Expression::StaticMemberExpression(member) => !member.optional,
        Expression::ComputedMemberExpression(member) => !member.optional,
        Expression::SequenceExpression(_)
        | Expression::ParenthesizedExpression(_)
        | Expression::UnaryExpression(_)
        | Expression::BinaryExpression(_)
        | Expression::NewExpression(_) => true,
        _ => false,
    }
}

pub fn is_under_with(scoping: &Scoping, scope: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| scoping.scope_flags(ancestor).contains(ScopeFlags::With))
}

pub fn input_binds_a_machinery_name(source_text: &str, scoping: &Scoping) -> bool {
    scoping.symbol_ids().any(|symbol| {
        let name = scoping.symbol_name(symbol);
        may_name_machinery(name)
            && is_machinery_name(name)
            && original_name(source_text, scoping, symbol) == Some(name)
    })
}
