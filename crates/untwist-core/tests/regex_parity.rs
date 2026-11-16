use std::path::PathBuf;

use oxc_allocator::Allocator;
use untwist_core::parse::{early_errors, Early, has_error, parse_program};

const NODE_VERSION: &str = "22.23.1";

fn rows() -> Vec<(bool, String, String)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/regex-parity")
        .join(format!("node-{NODE_VERSION}.tsv"));
    let text = std::fs::read_to_string(path).expect("rows");
    text.lines()
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let ok = fields.next() == Some("1");
            let flags = fields.next().expect("flags").to_string();
            let pattern = fields.next().expect("pattern").to_string();
            (ok, flags, pattern)
        })
        .collect()
}

fn accepted(pattern: &str, flags: &str) -> bool {
    let source = format!("/{pattern}/{flags};");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    !has_error(&parsed.diagnostics)
        && !matches!(early_errors(&parsed.program), Early::SyntaxError(_))
}

#[test]
fn every_pinned_regular_expression_is_accepted_exactly_when_node_accepts_it() {
    let all = rows();
    assert!(all.len() >= 13_000, "{}", all.len());
    let wrong: Vec<String> = all
        .iter()
        .filter(|(ok, flags, pattern)| accepted(pattern, flags) != *ok)
        .map(|(ok, flags, pattern)| format!("/{pattern}/{flags}: node accepts it: {ok}"))
        .collect();
    assert!(wrong.is_empty(), "{} rows differ:\n{}", wrong.len(), wrong.join("\n"));
}
