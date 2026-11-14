use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_traverse::{Ancestor, TraverseCtx};

use crate::ast_util::code_text::may_run_code_text;
use crate::ast_util::function_names::function_names;
use crate::semantic::{build_scoping, has_module_syntax, names_read_by_another_input};

const PROTO: &str = "__proto__";

const MODULE: &str = "module";
const EXPORTS: &str = "exports";

pub fn reads_names(program: &Program<'_>, scoping: &Scoping) -> bool {
    function_names(program, scoping).name_read || may_run_code_text(program, scoping)
}

fn exports_values(program: &Program<'_>, scoping: &Scoping) -> bool {
    let free = scoping.root_unresolved_references();
    has_module_syntax(program) || free.contains_key(MODULE) || free.contains_key(EXPORTS)
}

pub fn names_observed(program: &Program<'_>, scoping: &Scoping) -> bool {
    names_read_by_another_input()
        || exports_values(program, scoping)
        || reads_names(program, scoping)
}

pub fn text_observes_names(text: &str) -> bool {
    let alloc = oxc_allocator::Allocator::default();
    let parsed = crate::parse::parse_program(&alloc, text);
    names_observed(&parsed.program, &build_scoping(&parsed.program).scoping)
}

pub fn named_as<'a>(
    name: &'a str,
    value: Expression<'a>,
    builder: &AstBuilder<'a>,
) -> Expression<'a> {
    if !value.is_anonymous_function_definition() {
        return value;
    }
    let proto = name == PROTO;
    let key = match proto {
        true => PropertyKey::new_string_literal(SPAN, name, None, builder),
        false => PropertyKey::new_static_identifier(SPAN, name, builder),
    };
    let property = ObjectPropertyKind::new_object_property(
        SPAN,
        PropertyKind::Init,
        key,
        value,
        false,
        false,
        proto,
        builder,
    );
    let object =
        Expression::new_object_expression(SPAN, AVec::from_array_in([property], builder), builder);
    Expression::new_static_member_expression(
        SPAN,
        object,
        IdentifierName::new(SPAN, name, builder),
        false,
        builder,
    )
}

fn binds_name(pattern: &BindingPattern<'_>) -> bool {
    matches!(pattern, BindingPattern::BindingIdentifier(_))
}

fn targets_name(target: &AssignmentTarget<'_>) -> bool {
    matches!(target, AssignmentTarget::AssignmentTargetIdentifier(_))
}

fn names_on(operator: AssignmentOperator) -> bool {
    matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalNullish
    )
}

fn is_proto_key(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name == PROTO,
        PropertyKey::StringLiteral(literal) => literal.value == PROTO,
        _ => false,
    }
}

fn names_value(ancestor: &Ancestor<'_, '_>) -> bool {
    match ancestor {
        Ancestor::VariableDeclaratorInit(it) => binds_name(it.id()),
        Ancestor::AssignmentExpressionRight(it) => {
            names_on(*it.operator()) && targets_name(it.left())
        }
        Ancestor::AssignmentPatternRight(it) => binds_name(it.left()),
        Ancestor::FormalParameterInitializer(it) => binds_name(it.pattern()),
        Ancestor::AssignmentTargetWithDefaultInit(it) => targets_name(it.binding()),
        Ancestor::AssignmentTargetPropertyIdentifierInit(_)
        | Ancestor::PropertyDefinitionValue(_)
        | Ancestor::AccessorPropertyValue(_)
        | Ancestor::ExportDefaultDeclarationDeclaration(_) => true,
        Ancestor::ObjectPropertyValue(it) => {
            *it.kind() == PropertyKind::Init
                && !*it.method()
                && (*it.computed() || !is_proto_key(it.key()))
        }
        _ => false,
    }
}

pub fn in_naming_slot(ctx: &TraverseCtx<'_, ()>) -> bool {
    ctx.ancestors()
        .find(|ancestor| !matches!(ancestor, Ancestor::ParenthesizedExpressionExpression(_)))
        .is_some_and(|ancestor| names_value(&ancestor))
}

pub fn unnamed<'a>(value: Expression<'a>, builder: &AstBuilder<'a>) -> Expression<'a> {
    if !value.is_anonymous_function_definition() {
        return value;
    }
    let parts = AVec::from_array_in([Expression::new_number_0(SPAN, builder), value], builder);
    Expression::new_sequence_expression(SPAN, parts, builder)
}

pub fn without_unnamed<'k, 'a>(value: &'k Expression<'a>) -> &'k Expression<'a> {
    let Expression::SequenceExpression(sequence) = value else {
        return value;
    };
    match sequence.expressions.as_slice() {
        [Expression::NumericLiteral(zero), inner]
            if zero.value == 0.0 && inner.is_anonymous_function_definition() =>
        {
            inner
        }
        _ => value,
    }
}
