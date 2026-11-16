use std::path::PathBuf;

use oxc_allocator::Allocator;
use untwist_core::parse::{early_errors, has_error, parse_named_module, parse_named_script, Early};

const NODE_VERSION: &str = "22.23.1";

struct Row {
    script: bool,
    module: bool,
    source: String,
}

fn rows() -> Vec<Row> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/redeclaration-parity")
        .join(format!("node-{NODE_VERSION}.tsv"));
    let text = std::fs::read_to_string(path).expect("rows");
    text.lines()
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let script = fields.next() == Some("1");
            let module = fields.next() == Some("1");
            let source = fields.next().expect("source").to_string();
            Row {
                script,
                module,
                source,
            }
        })
        .collect()
}

fn accepted(source: &str, module: bool) -> bool {
    let alloc = Allocator::default();
    let parsed = match module {
        true => parse_named_module(&alloc, source),
        false => parse_named_script(&alloc, source),
    };
    !has_error(&parsed.diagnostics)
        && !matches!(early_errors(&parsed.program), Early::SyntaxError(_))
}

#[test]
fn every_pinned_redeclaration_is_refused_exactly_when_node_rejects_it() {
    let all = rows();
    assert!(all.len() >= 24_000, "{}", all.len());
    let wrong: Vec<String> = all
        .iter()
        .flat_map(|row| {
            [(false, row.script), (true, row.module)]
                .into_iter()
                .filter(|(module, ok)| accepted(&row.source, *module) != *ok)
                .map(|(module, ok)| {
                    let goal = if module { "module" } else { "script" };
                    format!("{goal}: {}: node accepts it: {ok}", row.source)
                })
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} rows differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
