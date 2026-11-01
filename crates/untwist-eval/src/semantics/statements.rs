use oxc_ast::ast::{LabelIdentifier, Statement, VariableDeclarationKind};

pub(crate) fn statement_label<'a>(label: &Option<LabelIdentifier<'a>>) -> Option<&'a str> {
    label.as_ref().map(|l| l.name.as_str())
}

pub(crate) const ANNEX_B_FUNCTION: &str =
    "a function declaration outside a function body's top level, whose Annex B binding the evaluator does not model";

pub(crate) fn declares_function(statements: &[Statement<'_>]) -> bool {
    statements
        .iter()
        .any(|statement| matches!(statement, Statement::FunctionDeclaration(_)))
}

pub(crate) fn block_declares_bindings(statements: &[Statement<'_>]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::FunctionDeclaration(_) | Statement::ClassDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.kind != VariableDeclarationKind::Var
        }
        _ => false,
    })
}
