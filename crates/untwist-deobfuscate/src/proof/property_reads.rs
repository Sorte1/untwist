use std::borrow::Cow;

use oxc_ast::ast::*;
use oxc_ast::{
    match_assignment_target, match_declaration, match_expression, match_member_expression,
    match_module_declaration,
};

pub(super) enum KeyRead<'k, 'a> {
    Static(Cow<'k, str>),
    Computed(&'k Expression<'a>),
    Private,
    Nested,
    Defines,
    Unkeyed,
    Unmodelled,
}

pub(super) fn statement_read<'k, 'a>(statement: &'k Statement<'a>) -> KeyRead<'k, 'a> {
    match statement {
        Statement::WithStatement(_) => KeyRead::Unmodelled,
        Statement::ForInStatement(_) | Statement::ForOfStatement(_) => KeyRead::Unkeyed,
        Statement::BlockStatement(_) | Statement::BreakStatement(_) => KeyRead::Unkeyed,
        Statement::ContinueStatement(_) | Statement::DebuggerStatement(_) => KeyRead::Unkeyed,
        Statement::DoWhileStatement(_) | Statement::EmptyStatement(_) => KeyRead::Unkeyed,
        Statement::ExpressionStatement(_) | Statement::ForStatement(_) => KeyRead::Unkeyed,
        Statement::IfStatement(_) | Statement::LabeledStatement(_) => KeyRead::Unkeyed,
        Statement::ReturnStatement(_) | Statement::SwitchStatement(_) => KeyRead::Unkeyed,
        Statement::ThrowStatement(_) | Statement::TryStatement(_) => KeyRead::Unkeyed,
        Statement::WhileStatement(_) => KeyRead::Unkeyed,
        match_declaration!(Statement) => declaration_read(statement.to_declaration()),
        match_module_declaration!(Statement) => module_read(statement.to_module_declaration()),
    }
}

fn declaration_read<'k, 'a>(declaration: &'k Declaration<'a>) -> KeyRead<'k, 'a> {
    match declaration {
        Declaration::VariableDeclaration(_)
        | Declaration::FunctionDeclaration(_)
        | Declaration::ClassDeclaration(_) => KeyRead::Unkeyed,
        Declaration::TSTypeAliasDeclaration(_)
        | Declaration::TSInterfaceDeclaration(_)
        | Declaration::TSEnumDeclaration(_)
        | Declaration::TSExternalModuleDeclaration(_)
        | Declaration::TSNamespaceDeclaration(_)
        | Declaration::TSGlobalDeclaration(_)
        | Declaration::TSImportEqualsDeclaration(_) => KeyRead::Unmodelled,
    }
}

fn module_read<'k, 'a>(declaration: &'k ModuleDeclaration<'a>) -> KeyRead<'k, 'a> {
    match declaration {
        ModuleDeclaration::ImportDeclaration(_)
        | ModuleDeclaration::TSExportAssignment(_)
        | ModuleDeclaration::TSNamespaceExportDeclaration(_) => KeyRead::Unmodelled,
        ModuleDeclaration::ExportAllDeclaration(_)
        | ModuleDeclaration::ExportDefaultDeclaration(_)
        | ModuleDeclaration::ExportDeclaration(_)
        | ModuleDeclaration::ExportNamedDeclaration(_)
        | ModuleDeclaration::ExportFromDeclaration(_) => KeyRead::Unkeyed,
    }
}

pub(super) fn expression_read<'k, 'a>(expr: &'k Expression<'a>) -> KeyRead<'k, 'a> {
    match expr {
        match_member_expression!(Expression) => KeyRead::Nested,
        Expression::PrivateInExpression(_) => KeyRead::Private,
        Expression::ImportMeta(_) | Expression::ImportExpression(_) => KeyRead::Unmodelled,
        Expression::V8IntrinsicExpression(_) | Expression::TSAsExpression(_) => KeyRead::Unmodelled,
        Expression::TSSatisfiesExpression(_) | Expression::TSTypeAssertion(_) => {
            KeyRead::Unmodelled
        }
        Expression::TSNonNullExpression(_) => KeyRead::Unmodelled,
        Expression::TSInstantiationExpression(_) => KeyRead::Unmodelled,
        Expression::BooleanLiteral(_) | Expression::NullLiteral(_) => KeyRead::Unkeyed,
        Expression::NumericLiteral(_) | Expression::BigIntLiteral(_) => KeyRead::Unkeyed,
        Expression::RegExpLiteral(_) | Expression::StringLiteral(_) => KeyRead::Unkeyed,
        Expression::TemplateLiteral(_) | Expression::Identifier(_) => KeyRead::Unkeyed,
        Expression::Super(_) | Expression::ThisExpression(_) => KeyRead::Unkeyed,
        Expression::NewTarget(_) | Expression::ArrayExpression(_) => KeyRead::Unkeyed,
        Expression::ObjectExpression(_) | Expression::ClassExpression(_) => KeyRead::Unkeyed,
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
            KeyRead::Unkeyed
        }
        Expression::AssignmentExpression(_) | Expression::UpdateExpression(_) => KeyRead::Unkeyed,
        Expression::BinaryExpression(_) | Expression::LogicalExpression(_) => KeyRead::Unkeyed,
        Expression::UnaryExpression(_) | Expression::ConditionalExpression(_) => KeyRead::Unkeyed,
        Expression::SequenceExpression(_) | Expression::ParenthesizedExpression(_) => {
            KeyRead::Unkeyed
        }
        Expression::CallExpression(_) | Expression::NewExpression(_) => KeyRead::Unkeyed,
        Expression::ChainExpression(_) | Expression::TaggedTemplateExpression(_) => {
            KeyRead::Unkeyed
        }
        Expression::AwaitExpression(_) | Expression::YieldExpression(_) => KeyRead::Unkeyed,
        Expression::JSXElement(_) | Expression::JSXFragment(_) => KeyRead::Unkeyed,
    }
}

pub(super) fn member_read<'k, 'a>(member: &'k MemberExpression<'a>) -> KeyRead<'k, 'a> {
    match member {
        MemberExpression::StaticMemberExpression(m) => {
            KeyRead::Static(Cow::Borrowed(m.property.name.as_str()))
        }
        MemberExpression::ComputedMemberExpression(m) => KeyRead::Computed(&m.expression),
        MemberExpression::PrivateFieldExpression(_) => KeyRead::Private,
    }
}

pub(super) fn chain_element_read<'k, 'a>(element: &'k ChainElement<'a>) -> KeyRead<'k, 'a> {
    match element {
        ChainElement::CallExpression(_) => KeyRead::Unkeyed,
        ChainElement::TSNonNullExpression(_) => KeyRead::Unmodelled,
        match_member_expression!(ChainElement) => KeyRead::Nested,
    }
}

pub(super) fn binary_read<'k, 'a>(binary: &'k BinaryExpression<'a>) -> KeyRead<'k, 'a> {
    if binary.operator == BinaryOperator::In {
        KeyRead::Computed(&binary.left)
    } else {
        KeyRead::Unkeyed
    }
}

pub(super) fn simple_target_read<'k, 'a>(
    target: &'k SimpleAssignmentTarget<'a>,
) -> KeyRead<'k, 'a> {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(_) => KeyRead::Unkeyed,
        SimpleAssignmentTarget::TSAsExpression(_)
        | SimpleAssignmentTarget::TSSatisfiesExpression(_)
        | SimpleAssignmentTarget::TSNonNullExpression(_)
        | SimpleAssignmentTarget::TSTypeAssertion(_) => KeyRead::Unmodelled,
        match_member_expression!(SimpleAssignmentTarget) => KeyRead::Nested,
    }
}

pub(super) fn argument_read<'k, 'a>(argument: &'k Argument<'a>) -> KeyRead<'k, 'a> {
    match argument {
        Argument::SpreadElement(_) => KeyRead::Unkeyed,
        match_expression!(Argument) => KeyRead::Unkeyed,
    }
}

pub(super) fn array_element_read<'k, 'a>(
    element: &'k ArrayExpressionElement<'a>,
) -> KeyRead<'k, 'a> {
    match element {
        ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => {
            KeyRead::Unkeyed
        }
        match_expression!(ArrayExpressionElement) => KeyRead::Unkeyed,
    }
}

pub(super) fn property_key_read<'k, 'a>(
    key: &'k PropertyKey<'a>,
    computed: bool,
) -> KeyRead<'k, 'a> {
    match key {
        PropertyKey::StaticIdentifier(id) => KeyRead::Static(Cow::Borrowed(id.name.as_str())),
        PropertyKey::PrivateIdentifier(_) => KeyRead::Private,
        match_expression!(PropertyKey) => match (
            computed,
            key.static_name().and_then(|name| name.into_cow_str()),
        ) {
            (true, _) => KeyRead::Computed(key.to_expression()),
            (false, Some(name)) => KeyRead::Static(name),
            (false, None) => KeyRead::Unmodelled,
        },
    }
}

pub(super) fn binding_pattern_read<'k, 'a>(pattern: &'k BindingPattern<'a>) -> KeyRead<'k, 'a> {
    match pattern {
        BindingPattern::BindingIdentifier(_)
        | BindingPattern::ObjectPattern(_)
        | BindingPattern::ArrayPattern(_)
        | BindingPattern::AssignmentPattern(_) => KeyRead::Unkeyed,
    }
}

pub(super) fn binding_property_read<'k, 'a>(property: &'k BindingProperty<'a>) -> KeyRead<'k, 'a> {
    property_key_read(&property.key, property.computed)
}

pub(super) fn assignment_pattern_read<'k, 'a>(
    pattern: &'k AssignmentTargetPattern<'a>,
) -> KeyRead<'k, 'a> {
    match pattern {
        AssignmentTargetPattern::ArrayAssignmentTarget(_)
        | AssignmentTargetPattern::ObjectAssignmentTarget(_) => KeyRead::Unkeyed,
    }
}

pub(super) fn maybe_default_read<'k, 'a>(
    target: &'k AssignmentTargetMaybeDefault<'a>,
) -> KeyRead<'k, 'a> {
    match target {
        AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(_) => KeyRead::Unkeyed,
        match_assignment_target!(AssignmentTargetMaybeDefault) => KeyRead::Unkeyed,
    }
}

pub(super) fn assignment_property_read<'k, 'a>(
    property: &'k AssignmentTargetProperty<'a>,
) -> KeyRead<'k, 'a> {
    match property {
        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(shorthand) => {
            KeyRead::Static(Cow::Borrowed(shorthand.binding.name.as_str()))
        }
        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
            property_key_read(&property.name, property.computed)
        }
    }
}

fn sets_prototype(property: &ObjectProperty<'_>) -> bool {
    property.kind == PropertyKind::Init
        && !property.method
        && !property.shorthand
        && !property.computed
        && property
            .key
            .static_name()
            .and_then(|name| name.into_cow_str())
            .is_some_and(|name| name == "__proto__")
}

pub(super) fn object_property_read<'k, 'a>(kind: &'k ObjectPropertyKind<'a>) -> KeyRead<'k, 'a> {
    match kind {
        ObjectPropertyKind::ObjectProperty(property) if sets_prototype(property) => {
            KeyRead::Unmodelled
        }
        ObjectPropertyKind::ObjectProperty(_) => KeyRead::Defines,
        ObjectPropertyKind::SpreadProperty(_) => KeyRead::Unkeyed,
    }
}

pub(super) fn class_element_read<'k, 'a>(element: &'k ClassElement<'a>) -> KeyRead<'k, 'a> {
    match element {
        ClassElement::StaticBlock(_) => KeyRead::Unkeyed,
        ClassElement::MethodDefinition(_)
        | ClassElement::PropertyDefinition(_)
        | ClassElement::AccessorProperty(_) => KeyRead::Defines,
        ClassElement::TSIndexSignature(_) => KeyRead::Unmodelled,
    }
}

pub(super) fn jsx_name_read<'k, 'a>(name: &'k JSXElementName<'a>) -> KeyRead<'k, 'a> {
    match name {
        JSXElementName::Identifier(_)
        | JSXElementName::IdentifierReference(_)
        | JSXElementName::NamespacedName(_)
        | JSXElementName::ThisExpression(_) => KeyRead::Unkeyed,
        JSXElementName::MemberExpression(_) => KeyRead::Nested,
    }
}

pub(super) fn jsx_object_read<'k, 'a>(
    object: &'k JSXMemberExpressionObject<'a>,
) -> KeyRead<'k, 'a> {
    match object {
        JSXMemberExpressionObject::IdentifierReference(_)
        | JSXMemberExpressionObject::ThisExpression(_) => KeyRead::Unkeyed,
        JSXMemberExpressionObject::MemberExpression(_) => KeyRead::Nested,
    }
}

pub(super) fn jsx_member_read<'k, 'a>(member: &'k JSXMemberExpression<'a>) -> KeyRead<'k, 'a> {
    KeyRead::Static(Cow::Borrowed(member.property.name.as_str()))
}
