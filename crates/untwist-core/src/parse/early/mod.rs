use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::{OxcDiagnostic, Severity};
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::Span;

mod annex_b;
mod class_octal;
mod failure;
mod newer;
mod templates;

pub use failure::{parse_failure, ParseFailure};

pub enum Early {
    Clean,
    SyntaxError(Vec<OxcDiagnostic>),
    UnmodelledAnnexB(Vec<Span>),
}

impl Early {
    pub fn is_clean(&self) -> bool {
        matches!(self, Early::Clean)
    }
}

fn errors(diagnostics: Vec<OxcDiagnostic>) -> Vec<OxcDiagnostic> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect()
}

pub fn early_errors(program: &Program<'_>) -> Early {
    let alloc = Allocator::default();
    let source_type = program.source_type;
    let options = ParseOptions {
        parse_regular_expression: true,
        preserve_parens: true,
        allow_return_outside_function: !source_type.is_module(),
        ..ParseOptions::default()
    };
    let checked = Parser::new(&alloc, program.source_text, source_type)
        .with_options(options)
        .parse();
    let parse_errors = errors(checked.diagnostics.into_vec());
    if !parse_errors.is_empty() {
        return Early::SyntaxError(parse_errors);
    }
    let built = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(&checked.program);
    let scoping = built.semantic.scoping();
    let annex_b = annex_b::AnnexB::scan(&checked.program, scoping);
    let octals = class_octal::ClassOctals::scan(&checked.program, scoping);
    let mut found = newer::unread_by_node(&checked.program);
    found.extend(
        errors(built.diagnostics.into_vec())
            .into_iter()
            .filter(|diagnostic| {
                templates::semantic_listed(diagnostic)
                    && !annex_b.explains(diagnostic)
                    && !octals.explains(diagnostic)
            }),
    );
    match (found.is_empty(), annex_b.unmodelled()) {
        (false, _) => Early::SyntaxError(found),
        (true, []) => Early::Clean,
        (true, spans) => Early::UnmodelledAnnexB(spans.to_vec()),
    }
}
