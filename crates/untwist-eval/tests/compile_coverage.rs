use std::collections::BTreeSet;

const COMPILED: &[(&str, &str)] = &[
    ("expr.rs", include_str!("../src/compile/expr.rs")),
    ("stmt.rs", include_str!("../src/compile/stmt.rs")),
    ("loops.rs", include_str!("../src/compile/loops.rs")),
    ("assign.rs", include_str!("../src/compile/assign.rs")),
];

const VARIANTS: &[(&str, &[&str])] = &[
    (
        "Expression",
        &[
            "BooleanLiteral",
            "NullLiteral",
            "NumericLiteral",
            "BigIntLiteral",
            "RegExpLiteral",
            "StringLiteral",
            "TemplateLiteral",
            "Identifier",
            "Super",
            "ArrayExpression",
            "ArrowFunctionExpression",
            "AssignmentExpression",
            "AwaitExpression",
            "BinaryExpression",
            "CallExpression",
            "ChainExpression",
            "ClassExpression",
            "ConditionalExpression",
            "FunctionExpression",
            "ImportExpression",
            "LogicalExpression",
            "NewExpression",
            "ObjectExpression",
            "ParenthesizedExpression",
            "SequenceExpression",
            "TaggedTemplateExpression",
            "ThisExpression",
            "UnaryExpression",
            "UpdateExpression",
            "YieldExpression",
            "PrivateInExpression",
            "ImportMeta",
            "NewTarget",
            "JSXElement",
            "JSXFragment",
            "TSAsExpression",
            "TSSatisfiesExpression",
            "TSTypeAssertion",
            "TSNonNullExpression",
            "TSInstantiationExpression",
            "V8IntrinsicExpression",
            "ComputedMemberExpression",
            "StaticMemberExpression",
            "PrivateFieldExpression",
        ],
    ),
    (
        "Statement",
        &[
            "BlockStatement",
            "BreakStatement",
            "ContinueStatement",
            "DebuggerStatement",
            "DoWhileStatement",
            "EmptyStatement",
            "ExpressionStatement",
            "ForInStatement",
            "ForOfStatement",
            "ForStatement",
            "IfStatement",
            "LabeledStatement",
            "ReturnStatement",
            "SwitchStatement",
            "ThrowStatement",
            "TryStatement",
            "WhileStatement",
            "WithStatement",
            "VariableDeclaration",
            "FunctionDeclaration",
            "ClassDeclaration",
            "TSTypeAliasDeclaration",
            "TSInterfaceDeclaration",
            "TSEnumDeclaration",
            "TSExternalModuleDeclaration",
            "TSNamespaceDeclaration",
            "TSGlobalDeclaration",
            "TSImportEqualsDeclaration",
            "ImportDeclaration",
            "ExportAllDeclaration",
            "ExportDefaultDeclaration",
            "ExportDeclaration",
            "ExportNamedDeclaration",
            "ExportFromDeclaration",
            "TSExportAssignment",
            "TSNamespaceExportDeclaration",
        ],
    ),
    (
        "AssignmentTarget",
        &[
            "AssignmentTargetIdentifier",
            "TSAsExpression",
            "TSSatisfiesExpression",
            "TSNonNullExpression",
            "TSTypeAssertion",
            "ComputedMemberExpression",
            "StaticMemberExpression",
            "PrivateFieldExpression",
            "ArrayAssignmentTarget",
            "ObjectAssignmentTarget",
        ],
    ),
    (
        "SimpleAssignmentTarget",
        &[
            "AssignmentTargetIdentifier",
            "TSAsExpression",
            "TSSatisfiesExpression",
            "TSNonNullExpression",
            "TSTypeAssertion",
            "ComputedMemberExpression",
            "StaticMemberExpression",
            "PrivateFieldExpression",
        ],
    ),
    (
        "ChainElement",
        &[
            "CallExpression",
            "TSNonNullExpression",
            "ComputedMemberExpression",
            "StaticMemberExpression",
            "PrivateFieldExpression",
        ],
    ),
    (
        "ForStatementLeft",
        &[
            "VariableDeclaration",
            "AssignmentTargetIdentifier",
            "TSAsExpression",
            "TSSatisfiesExpression",
            "TSNonNullExpression",
            "TSTypeAssertion",
            "ComputedMemberExpression",
            "StaticMemberExpression",
            "PrivateFieldExpression",
            "ArrayAssignmentTarget",
            "ObjectAssignmentTarget",
        ],
    ),
];

const MEMBERS: [&str; 3] = [
    "ComputedMemberExpression",
    "StaticMemberExpression",
    "PrivateFieldExpression",
];

struct Arm {
    dispatch: &'static str,
    arm: &'static str,
    file: &'static str,
    counterpart: &'static str,
}

const fn arm(
    dispatch: &'static str,
    arm: &'static str,
    file: &'static str,
    counterpart: &'static str,
) -> Arm {
    Arm {
        dispatch,
        arm,
        file,
        counterpart,
    }
}

const INNER: &str = "Expression";
const COMPOUND: &str = "Expression";
const STATEMENT: &str = "Statement";
const TARGET: &str = "AssignmentTarget";
const SIMPLE: &str = "SimpleAssignmentTarget";
const CHAIN: &str = "ChainElement";
const FOR_LEFT: &str = "ForStatementLeft";

const TABLE: &[Arm] = &[
    arm(
        INNER,
        "Expression::NumericLiteral",
        "expr.rs",
        "Expression::NumericLiteral(literal) =>",
    ),
    arm(
        INNER,
        "Expression::StringLiteral",
        "expr.rs",
        "Expression::StringLiteral(literal) =>",
    ),
    arm(
        INNER,
        "Expression::BooleanLiteral",
        "expr.rs",
        "Expression::BooleanLiteral(literal) =>",
    ),
    arm(
        INNER,
        "Expression::NullLiteral",
        "expr.rs",
        "Expression::NullLiteral(_) =>",
    ),
    arm(
        INNER,
        "Expression::Identifier",
        "expr.rs",
        "Expression::Identifier(id) => identifier(",
    ),
    arm(
        INNER,
        "Expression::ParenthesizedExpression",
        "expr.rs",
        "Expression::ParenthesizedExpression(paren) =>",
    ),
    arm(
        INNER,
        "Expression::UnaryExpression",
        "expr.rs",
        "Expression::UnaryExpression(unary) => compile_unary(",
    ),
    arm(
        INNER,
        "Expression::BigIntLiteral",
        "expr.rs",
        "Expression::BigIntLiteral(_) => refuse::expression(&\"BigInt literal\")",
    ),
    arm(
        INNER,
        "_",
        "expr.rs",
        "_ => compile_compound(expr, scope, cx)",
    ),
    arm(
        COMPOUND,
        "Expression::BinaryExpression",
        "expr.rs",
        "Expression::BinaryExpression(binary) => compile_binary(",
    ),
    arm(
        COMPOUND,
        "Expression::LogicalExpression",
        "expr.rs",
        "Expression::LogicalExpression(logical) => compile_logical(",
    ),
    arm(
        COMPOUND,
        "Expression::ConditionalExpression",
        "expr.rs",
        "Expression::ConditionalExpression(conditional) =>",
    ),
    arm(
        COMPOUND,
        "Expression::SequenceExpression",
        "expr.rs",
        "Expression::SequenceExpression(sequence) =>",
    ),
    arm(
        COMPOUND,
        "Expression::TemplateLiteral",
        "expr.rs",
        "Expression::TemplateLiteral(template) => compile_template(",
    ),
    arm(
        COMPOUND,
        "Expression::ArrayExpression",
        "expr.rs",
        "Expression::ArrayExpression(array) => compile_array(",
    ),
    arm(
        COMPOUND,
        "Expression::ObjectExpression",
        "expr.rs",
        "Expression::ObjectExpression(object) => compile_object(",
    ),
    arm(
        COMPOUND,
        "Expression::FunctionExpression",
        "expr.rs",
        "Expression::FunctionExpression(function) =>",
    ),
    arm(
        COMPOUND,
        "Expression::ArrowFunctionExpression",
        "expr.rs",
        "Expression::ArrowFunctionExpression(arrow) =>",
    ),
    arm(
        COMPOUND,
        "Expression::ThisExpression",
        "expr.rs",
        "Expression::ThisExpression(_) =>",
    ),
    arm(
        COMPOUND,
        "Expression::StaticMemberExpression",
        "expr.rs",
        "Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) =>",
    ),
    arm(
        COMPOUND,
        "Expression::ComputedMemberExpression",
        "expr.rs",
        "Some(member) => compile_member_value(member, scope, cx)",
    ),
    arm(
        COMPOUND,
        "Expression::PrivateFieldExpression",
        "expr.rs",
        "Expression::PrivateFieldExpression(_) => refused(&PRIVATE_FIELD)",
    ),
    arm(
        COMPOUND,
        "as_member_expression",
        "expr.rs",
        "match expr.as_member_expression()",
    ),
    arm(
        COMPOUND,
        "Expression::ChainExpression",
        "expr.rs",
        "Expression::ChainExpression(chain) => compile_chain(",
    ),
    arm(
        COMPOUND,
        "Expression::AssignmentExpression",
        "expr.rs",
        "Expression::AssignmentExpression(assignment) => compile_assignment(",
    ),
    arm(
        COMPOUND,
        "Expression::UpdateExpression",
        "expr.rs",
        "Expression::UpdateExpression(update) => compile_update(",
    ),
    arm(
        COMPOUND,
        "Expression::CallExpression",
        "expr.rs",
        "Expression::CallExpression(call) =>",
    ),
    arm(
        COMPOUND,
        "Expression::NewExpression",
        "expr.rs",
        "Expression::NewExpression(new) =>",
    ),
    arm(
        COMPOUND,
        "Expression::RegExpLiteral",
        "expr.rs",
        "Expression::RegExpLiteral(regexp) =>",
    ),
    arm(COMPOUND, "_", "expr.rs", "_ => refuse::unported()"),
    arm(
        STATEMENT,
        "Statement::EmptyStatement",
        "stmt.rs",
        "Statement::EmptyStatement(_)",
    ),
    arm(
        STATEMENT,
        "Statement::FunctionDeclaration",
        "stmt.rs",
        "| Statement::FunctionDeclaration(_)",
    ),
    arm(
        STATEMENT,
        "Statement::DebuggerStatement",
        "stmt.rs",
        "| Statement::DebuggerStatement(_) => statement(|_, _, _| Ok(Completion::Normal))",
    ),
    arm(
        STATEMENT,
        "Statement::ExpressionStatement",
        "stmt.rs",
        "Statement::ExpressionStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::ReturnStatement",
        "stmt.rs",
        "Statement::ReturnStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::VariableDeclaration",
        "stmt.rs",
        "Statement::VariableDeclaration(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::BlockStatement",
        "stmt.rs",
        "Statement::BlockStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::IfStatement",
        "stmt.rs",
        "Statement::IfStatement(node) => compile_if(",
    ),
    arm(
        STATEMENT,
        "Statement::ThrowStatement",
        "stmt.rs",
        "Statement::ThrowStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::LabeledStatement",
        "stmt.rs",
        "Statement::LabeledStatement(node) => compile_labeled(",
    ),
    arm(
        STATEMENT,
        "Statement::ForStatement",
        "loops.rs",
        "Statement::ForStatement(node) => compile_for(",
    ),
    arm(
        STATEMENT,
        "Statement::WhileStatement",
        "loops.rs",
        "Statement::WhileStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::DoWhileStatement",
        "loops.rs",
        "Statement::DoWhileStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::ForInStatement",
        "loops.rs",
        "Statement::ForInStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::ForOfStatement",
        "loops.rs",
        "Statement::ForOfStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::BreakStatement",
        "stmt.rs",
        "Statement::BreakStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::ContinueStatement",
        "stmt.rs",
        "Statement::ContinueStatement(node) =>",
    ),
    arm(
        STATEMENT,
        "Statement::TryStatement",
        "stmt.rs",
        "Statement::TryStatement(node) => match compile_try(",
    ),
    arm(
        STATEMENT,
        "Statement::SwitchStatement",
        "stmt.rs",
        "Statement::SwitchStatement(node) => match compile_switch(",
    ),
    arm(STATEMENT, "_", "stmt.rs", "None => refuse::statement(),"),
    arm(
        TARGET,
        "AssignmentTarget::AssignmentTargetIdentifier",
        "assign.rs",
        "if let AssignmentTarget::AssignmentTargetIdentifier(id) = target",
    ),
    arm(
        TARGET,
        "as_member_expression",
        "assign.rs",
        "match target.as_member_expression()",
    ),
    arm(
        SIMPLE,
        "SimpleAssignmentTarget::AssignmentTargetIdentifier",
        "assign.rs",
        "if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = target",
    ),
    arm(
        SIMPLE,
        "as_member_expression",
        "assign.rs",
        "match target.as_member_expression()",
    ),
    arm(SIMPLE, "_", "assign.rs", "None => Err(&UPDATE_TARGET),"),
    arm(
        TARGET,
        "_",
        "assign.rs",
        "None => Err(&DESTRUCTURING_TARGET),",
    ),
    arm(
        CHAIN,
        "ChainElement::StaticMemberExpression",
        "expr.rs",
        "element => match element.as_member_expression()",
    ),
    arm(
        CHAIN,
        "ChainElement::ComputedMemberExpression",
        "expr.rs",
        "return compile_computed_read_link(node, scope, cx);",
    ),
    arm(
        CHAIN,
        "ChainElement::PrivateFieldExpression",
        "expr.rs",
        "ChainElement::PrivateFieldExpression(_) => return refused(&PRIVATE_FIELD)",
    ),
    arm(
        CHAIN,
        "ChainElement::CallExpression",
        "expr.rs",
        "ChainElement::CallExpression(call) => compile_call_link(call, scope, cx)",
    ),
    arm(
        CHAIN,
        "_",
        "expr.rs",
        "None => return refused(&\"chain element\")",
    ),
    arm(
        FOR_LEFT,
        "ForStatementLeft::VariableDeclaration",
        "loops.rs",
        "let ForStatementLeft::VariableDeclaration(declaration) = left else",
    ),
    arm(
        FOR_LEFT,
        "ForStatementLeft::AssignmentTargetIdentifier",
        "loops.rs",
        "if let ForStatementLeft::AssignmentTargetIdentifier(id) = left",
    ),
    arm(
        FOR_LEFT,
        "as_member_expression",
        "loops.rs",
        "match left.as_member_expression()",
    ),
    arm(
        FOR_LEFT,
        "_",
        "loops.rs",
        "None => Left::Refuse(&FOR_IN_TARGET)",
    ),
];

fn named(dispatch: &str, variant: &str) -> bool {
    let arm = format!("{dispatch}::{variant}");
    TABLE
        .iter()
        .any(|row| row.dispatch == dispatch && row.arm == arm)
}

fn routed(dispatch: &str, arm: &str) -> bool {
    TABLE
        .iter()
        .any(|row| row.dispatch == dispatch && row.arm == arm)
}

fn covered(dispatch: &str, variant: &str) -> bool {
    named(dispatch, variant)
        || (MEMBERS.contains(&variant) && routed(dispatch, "as_member_expression"))
        || routed(dispatch, "_")
}

#[test]
fn every_frozen_variant_is_named_by_a_compiled_dispatch_or_routed_to_refuse() {
    let missing: Vec<String> = VARIANTS
        .iter()
        .flat_map(|(dispatch, variants)| {
            variants
                .iter()
                .filter(|variant| !covered(dispatch, variant))
                .map(move |variant| format!("{dispatch}::{variant}"))
        })
        .collect();
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn every_row_has_its_compiled_counterpart() {
    let mut missing = Vec::new();
    for row in TABLE {
        let compiled = COMPILED
            .iter()
            .find(|(file, _)| *file == row.file)
            .map(|(_, source)| *source)
            .unwrap_or_default();
        if !compiled.contains(row.counterpart) {
            missing.push(format!(
                "{}: {} has no compiled counterpart `{}` in compile/{}",
                row.dispatch, row.arm, row.counterpart, row.file
            ));
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn the_coverage_table_names_no_variant_the_frozen_list_lacks() {
    let known: BTreeSet<String> = VARIANTS
        .iter()
        .flat_map(|(dispatch, variants)| {
            variants
                .iter()
                .map(move |variant| format!("{dispatch}::{variant}"))
        })
        .chain(["_".to_string(), "as_member_expression".to_string()])
        .collect();
    let stale: Vec<String> = TABLE
        .iter()
        .filter(|row| !known.contains(row.arm))
        .map(|row| format!("{}: {}", row.dispatch, row.arm))
        .collect();
    assert!(stale.is_empty(), "{}", stale.join("\n"));
}
