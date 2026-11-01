use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program, Statement};
use sha2::{Digest, Sha256};
use untwist_core::parse::parse_program;

const LAYERS: &str = "fixtures/layers";
const FORMS: &[&str] = &[
    "W-eval",
    "W-Function",
    "W-packer",
    "I-stmt",
    "I-expr",
    "I-Function",
    "I-eval-indirect",
];
const SWEEP: &[&str] = &[
    "fixtures/behaviour",
    "fixtures/eval",
    "fixtures/unminify",
    "fixtures/deobfuscate-gaps",
    "fixtures/jsx",
    "fixtures/corpus-npm",
    "fixtures/reference",
    "fixtures/untranspile",
    "fixtures/unpack-bundlers",
];
const SWEEP_LAYERS: &[&str] = &[];

struct Row {
    form: String,
    expect: String,
    sha256: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest() -> BTreeMap<String, Row> {
    let path = root().join(LAYERS).join("MANIFEST");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let mut rows = BTreeMap::new();
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let fields: Vec<&str> = line.split('\t').collect();
        let [rel, form, _origin, expect, sha256] = fields[..] else {
            panic!("MANIFEST line has {} fields: {line}", fields.len());
        };
        rows.insert(
            rel.to_string(),
            Row {
                form: form.to_string(),
                expect: expect.to_string(),
                sha256: sha256.to_string(),
            },
        );
    }
    rows
}

fn js_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            js_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path);
        }
    }
}

fn layer_fixtures() -> Vec<(String, PathBuf)> {
    let base = root().join(LAYERS);
    let mut files = Vec::new();
    js_files(&base, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let rel = path
                .strip_prefix(&base)
                .unwrap_or(&path)
                .display()
                .to_string();
            (rel, path)
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn callee_form(callee: &Expression) -> Option<&'static str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) if id.name == "eval" => Some("W-eval"),
        Expression::Identifier(id) if id.name == "Function" => Some("W-Function"),
        Expression::StaticMemberExpression(member) if member.property.name == "eval" => {
            Some("W-eval")
        }
        Expression::SequenceExpression(seq) => match seq.expressions.last() {
            Some(Expression::Identifier(id)) if id.name == "eval" => Some("W-eval"),
            _ => None,
        },
        Expression::CallExpression(inner) => callee_form(&inner.callee),
        Expression::NewExpression(inner) => callee_form(&inner.callee),
        _ => None,
    }
}

fn is_packer_argument(expr: &Expression) -> bool {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return false;
    };
    matches!(call.callee.without_parentheses(), Expression::FunctionExpression(f) if f.params.items.len() == 6)
        && call.arguments.len() == 6
}

fn probe(program: &Program) -> Option<&'static str> {
    let statements: Vec<&Statement> = program
        .body
        .iter()
        .filter(|s| !matches!(s, Statement::EmptyStatement(_)))
        .collect();
    let [Statement::ExpressionStatement(only)] = statements[..] else {
        return None;
    };
    let (callee, first) = match only.expression.without_parentheses() {
        Expression::CallExpression(call) => (&call.callee, call.arguments.first()),
        Expression::NewExpression(call) => (&call.callee, call.arguments.first()),
        _ => return None,
    };
    let form = callee_form(callee)?;
    let packed = first
        .and_then(|arg| arg.as_expression())
        .is_some_and(is_packer_argument);
    Some(if form == "W-eval" && packed {
        "W-packer"
    } else {
        form
    })
}

fn probed(source: &str) -> Option<&'static str> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    probe(&parsed.program)
}

#[test]
fn the_manifest_lists_every_layer_fixture_with_its_form_and_hash() {
    let rows = manifest();
    let fixtures = layer_fixtures();
    let listed: Vec<&String> = rows.keys().collect();
    let found: Vec<&String> = fixtures.iter().map(|(rel, _)| rel).collect();
    assert_eq!(listed, found, "MANIFEST and fixtures/layers disagree");
    for (rel, path) in &fixtures {
        let row = &rows[rel];
        let bytes = std::fs::read(path).unwrap_or_else(|err| panic!("read {rel}: {err}"));
        assert_eq!(hex(&bytes), row.sha256, "{rel}: the hash moved");
        assert!(
            FORMS.contains(&row.form.as_str()),
            "{rel}: form {}",
            row.form
        );
        assert!(!row.expect.is_empty(), "{rel}: no expected outcome");
    }
}

#[test]
fn every_whole_program_fixture_is_probed_as_its_form_and_no_inline_one_is() {
    let rows = manifest();
    let mut wrong = Vec::new();
    for (rel, path) in layer_fixtures() {
        let source = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{rel}: {err}"));
        let form = rows[&rel].form.as_str();
        let want = form.starts_with("W-").then_some(form);
        let got = probed(&source);
        if got != want {
            wrong.push(format!("{rel}: probe {got:?}, MANIFEST {form}"));
        }
    }
    assert!(wrong.is_empty(), "the L-D matrix disagrees: {wrong:?}");
}

#[test]
fn no_input_of_the_sweep_is_a_whole_program_layer() {
    let mut files = Vec::new();
    for dir in SWEEP {
        js_files(&root().join(dir), &mut files);
    }
    files.extend(untwist_testing::real_world_path("game.js"));
    files.sort();
    let mut found = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        if let Some(form) = probed(&source) {
            let rel = file.strip_prefix(root()).unwrap_or(file);
            found.push(format!("{} {form}", rel.display()));
        }
    }
    assert!(files.len() > 400, "the sweep shrank to {}", files.len());
    assert_eq!(found, SWEEP_LAYERS);
}
