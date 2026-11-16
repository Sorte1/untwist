use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::{OxcDiagnostic, Severity};
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};

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
    check(program.source_text, program.source_type)
}

fn blanked(text: &str, spans: &[Span]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for span in spans {
        let range = span.start as usize..span.end as usize;
        if let Some(part) = bytes.get_mut(range) {
            part.fill(b' ');
            if let Some(first) = part.first_mut() {
                *first = b';';
            }
        }
    }
    String::from_utf8(bytes).unwrap_or_default()
}

fn check(text: &str, source_type: SourceType) -> Early {
    let alloc = Allocator::default();
    let options = ParseOptions {
        preserve_parens: true,
        allow_return_outside_function: !source_type.is_module(),
        ..ParseOptions::default()
    };
    let checked = Parser::new(&alloc, text, source_type)
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
    let (annex_b, mut hoisting) = annex_b::AnnexB::scan(&checked.program, scoping);
    let octals = class_octal::ClassOctals::scan(&checked.program, scoping);
    let mut found = newer::unread_by_node(&checked.program);
    let diagnostics = errors(built.diagnostics.into_vec());
    let hidden_by_clause = annex_b.clause_shares_name(scoping)
        || diagnostics
            .iter()
            .any(|diagnostic| annex_b.explains_by_clause(diagnostic));
    let (semantic, mut read_by_node): (Vec<_>, Vec<_>) = diagnostics
        .into_iter()
        .filter(|diagnostic| {
            templates::semantic_listed(diagnostic)
                && !annex_b.explains(diagnostic)
                && !octals.explains(diagnostic)
                && !hoisting.explains(diagnostic)
        })
        .partition(|diagnostic| !hoisting.read_by_v8(diagnostic));
    found.extend(semantic);
    if hidden_by_clause {
        let without = blanked(text, &annex_b.clause_functions());
        if let Early::SyntaxError(behind) = check(&without, source_type) {
            found.extend(behind);
        }
    }
    let (missed, v8_reads) = hoisting.take_found();
    found.extend(missed);
    read_by_node.extend(v8_reads);
    let spans = annex_b.unmodelled();
    if found.is_empty() {
        return match (read_by_node.is_empty(), spans.is_empty()) {
            (false, _) => Early::ReadByNode(read_by_node),
            (true, true) => Early::Clean,
            (true, false) => Early::UnmodelledAnnexB(spans.to_vec()),
        };
    }
    let redeclared = found.iter().all(annex_b::AnnexB::is_redeclaration);
    if spans.is_empty() || !redeclared {
        return Early::SyntaxError(found);
    }
    let without = blanked(text, &annex_b.unmodelled_functions());
    match check(&without, source_type) {
        Early::SyntaxError(_) => Early::SyntaxError(found),
        _ => Early::UnmodelledAnnexB(spans.to_vec()),
    }
}
