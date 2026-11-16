use oxc_diagnostics::{OxcDiagnostic, Severity};

use super::templates::parser_listed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseFailure {
    SyntaxError,
    Unsupported,
}

const WRAPPER_REDECLARED: &str = "in its CommonJS wrapper, whose parameter";
const CALL_TARGET: &str = "Cannot assign to this expression";
const YIELD_NAME: &str = "A 'yield' expression is only allowed in a generator body.";
const ASSIGNMENTS: [&str; 12] = [
    ">>>=", "<<=", ">>=", "**=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
];

fn label_text<'s>(source: &'s str, diagnostic: &OxcDiagnostic) -> Option<(usize, &'s str)> {
    let label = diagnostic.labels.first()?;
    let start = label.offset() as usize;
    let end = start + label.len() as usize;
    Some((end, source.get(start..end)?))
}

fn word_follows(rest: &str, word: &str) -> bool {
    rest.strip_prefix(word).is_some_and(|after| {
        after
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_' || next == '$'))
    })
}

fn assigns_or_counts(after: &str) -> bool {
    let simple = after.starts_with('=') && !after.starts_with("==") && !after.starts_with("=>");
    simple
        || ASSIGNMENTS
            .iter()
            .any(|operator| after.starts_with(operator))
        || after.starts_with("++")
        || after.starts_with("--")
        || word_follows(after, "in")
        || word_follows(after, "of")
}

fn plain_call(text: &str) -> bool {
    let Some(open) = argument_list_start(text) else {
        return false;
    };
    let callee = &text[..open];
    !callee.contains("?.") && !word_follows(callee, "new") && !callee.trim().is_empty()
}

fn argument_list_start(text: &str) -> Option<usize> {
    if !text.ends_with(')') {
        return None;
    }
    let mut depth = 0usize;
    for (at, byte) in text.bytes().enumerate().rev() {
        match byte {
            b')' => depth += 1,
            b'(' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

fn call_as_target(source: &str, diagnostic: &OxcDiagnostic) -> bool {
    let Some((end, text)) = label_text(source, diagnostic) else {
        return false;
    };
    if !plain_call(text) {
        return false;
    }
    let start = end - text.len();
    let after = source[end..].trim_start_matches(|c: char| c.is_whitespace() || c == ')');
    let before = source[..start].trim_end_matches(|c: char| c.is_whitespace() || c == '(');
    assigns_or_counts(after) || before.ends_with("++") || before.ends_with("--")
}

fn yield_iterated(source: &str, diagnostic: &OxcDiagnostic) -> bool {
    let Some((end, text)) = label_text(source, diagnostic) else {
        return false;
    };
    text == "yield" && word_follows(source[end..].trim_start(), "of")
}

fn private_optional_target(source: &str, diagnostic: &OxcDiagnostic) -> bool {
    label_text(source, diagnostic).is_some_and(|(_, text)| text.contains("?.#"))
}

fn read_by_node(source: &str, diagnostic: &OxcDiagnostic) -> bool {
    match diagnostic.message.as_ref() {
        CALL_TARGET => {
            call_as_target(source, diagnostic) || private_optional_target(source, diagnostic)
        }
        YIELD_NAME => yield_iterated(source, diagnostic),
        _ => false,
    }
}

fn rejected_by_node(diagnostic: &OxcDiagnostic) -> bool {
    diagnostic.message.contains(WRAPPER_REDECLARED) || parser_listed(diagnostic)
}

pub fn parse_failure(source: &str, diagnostics: &[OxcDiagnostic]) -> ParseFailure {
    let mut errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error);
    let Some(first) = errors.next() else {
        return ParseFailure::SyntaxError;
    };
    let unsupported = std::iter::once(first)
        .chain(errors)
        .any(|diagnostic| read_by_node(source, diagnostic));
    match unsupported || !rejected_by_node(first) {
        true => ParseFailure::Unsupported,
        false => ParseFailure::SyntaxError,
    }
}
