use oxc_ast::ast::Program;
use oxc_diagnostics::{OxcDiagnostic, Severity};
use oxc_semantic::SemanticBuilder;

pub fn early_errors(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(program)
        .diagnostics
        .into_vec()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect()
}
