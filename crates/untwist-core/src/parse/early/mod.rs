use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::{OxcDiagnostic, Severity};
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::Span;

mod annex_b;
mod class_octal;
mod failure;
mod hoisting;
mod newer;
mod regex;
mod templates;

pub use failure::{parse_failure, ParseFailure};

pub enum Early {
    Clean,
    SyntaxError(Vec<OxcDiagnostic>),
    ReadByNode(Vec<OxcDiagnostic>),
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
    let mut hoisting = hoisting::Hoisting::scan(&checked.program, scoping);
    let mut found = newer::unread_by_node(&checked.program);
    let (semantic, mut read_by_node): (Vec<_>, Vec<_>) = errors(built.diagnostics.into_vec())
        .into_iter()
        .filter(|diagnostic| {
            templates::semantic_listed(diagnostic)
                && !annex_b.explains(diagnostic)
                && !octals.explains(diagnostic)
                && !hoisting.explains(diagnostic)
        })
        .partition(|diagnostic| !hoisting.read_by_v8(diagnostic));
    found.extend(semantic);
    let (missed, v8_reads) = hoisting.take_found();
    found.extend(missed);
    read_by_node.extend(v8_reads);
    let spans = annex_b.unmodelled();
    let unmodelled_only = !spans.is_empty()
        && found
            .iter()
            .all(|diagnostic| annex_b.touches_unmodelled(diagnostic));
    match (unmodelled_only, found.is_empty(), read_by_node.is_empty()) {
        (false, false, _) => Early::SyntaxError(found),
        (_, _, false) => Early::ReadByNode(read_by_node),
        (true, _, true) => Early::UnmodelledAnnexB(spans.to_vec()),
        (false, true, true) => Early::Clean,
    }
}
