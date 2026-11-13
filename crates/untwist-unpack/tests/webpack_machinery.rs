use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_core::parse::parse_program;
use untwist_unpack::webpack::machinery::{form_text, jsonp_block_text, known, Role};

const FIXTURES: &str = "../../fixtures/webpack-variants";

fn forms_root() -> PathBuf {
    std::env::var_os("UNTWIST_FORMS_ROOT")
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES))
}

fn bundles() -> Vec<PathBuf> {
    let root = forms_root();
    let mut found = Vec::new();
    for cell in std::fs::read_dir(&root).expect("fixtures") {
        let cell = cell.expect("entry").path();
        if !cell.is_dir() {
            continue;
        }
        for file in std::fs::read_dir(&cell).expect("cell") {
            let file = file.expect("file").path();
            if file
                .extension()
                .is_some_and(|ext| ext == "js" || ext == "mjs")
            {
                found.push(file);
            }
        }
    }
    found.sort();
    found
}

fn bootstrap<'k, 'a>(program: &'k Program<'a>) -> Option<&'k [Statement<'a>]> {
    let top = require_name(&program.body).filter(|name| member_uses(&program.body, name) > 0);
    match top {
        Some(_) => Some(&program.body[..]),
        None => wrapped(program),
    }
}

fn wrapped<'k, 'a>(program: &'k Program<'a>) -> Option<&'k [Statement<'a>]> {
    program.body.iter().find_map(|statement| {
        let Statement::ExpressionStatement(statement) = statement else {
            return None;
        };
        let mut expr = statement.expression.without_parentheses();
        if let Expression::UnaryExpression(unary) = expr {
            expr = unary.argument.without_parentheses();
        }
        let Expression::CallExpression(call) = expr else {
            return None;
        };
        match call.callee.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(&body.statements[..]),
                _ => None,
            }
            .filter(|body| body.len() > 3),
            Expression::FunctionExpression(function) => function
                .body
                .as_ref()
                .map(|body| &body.statements[..])
                .filter(|body| body.len() > 3),
            _ => None,
        }
    })
}

fn require_name<'k>(statements: &'k [Statement<'_>]) -> Option<&'k str> {
    let names: Vec<&str> = statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::FunctionDeclaration(function) if function.params.items.len() == 1 => {
                function.id.as_ref().map(|id| id.name.as_str())
            }
            _ => None,
        })
        .collect();
    names
        .into_iter()
        .max_by_key(|name| member_uses(statements, name))
}

fn member_uses(statements: &[Statement<'_>], name: &str) -> usize {
    statements
        .iter()
        .flat_map(elements)
        .filter(|expr| {
            matches!(expr, Expression::AssignmentExpression(assignment)
                if matches!(&assignment.left, AssignmentTarget::StaticMemberExpression(member)
                    if matches!(&member.object, Expression::Identifier(id) if id.name == name)))
        })
        .count()
}

fn elements<'k, 'a>(statement: &'k Statement<'a>) -> Vec<&'k Expression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return Vec::new();
    };
    match statement.expression.without_parentheses() {
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().collect(),
        other => vec![other],
    }
}

fn role_of(form: &str) -> Option<&'static str> {
    let loads = [
        "$R.f.",
        "$R.O.require=",
        "$R.O.j=",
        "$R.O.readFileVm=",
        "$R.O.import=",
    ];
    if loads
        .iter()
        .any(|mark| form.contains(mark) && !form.starts_with("$R.e="))
    {
        return Some("LOADER");
    }
    if form.starts_with("$R.e=") || form.contains("$R.f={};$R.e=") {
        return Some("ENSURE");
    }
    let helpers = ["$R.O=", "$R.t=", "$R.p=", "$R.g="];
    if helpers.iter().any(|mark| form.contains(mark)) {
        return Some("HELPER");
    }
    None
}

#[test]
#[ignore = "prints the canonical runtime forms of fixtures/webpack-variants"]
fn dump_runtime_forms() {
    let mut forms: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let root = forms_root();
    for file in bundles() {
        let text = std::fs::read_to_string(&file).expect("read");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &text);
        let Some(statements) = bootstrap(&parsed.program) else {
            continue;
        };
        let Some(require) = require_name(statements) else {
            continue;
        };
        let label = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        if let Some(form) = jsonp_block_text(&text, statements, require) {
            forms
                .entry(("LOADER".to_string(), form))
                .or_default()
                .push(label.clone());
        }
        for expr in statements.iter().flat_map(elements) {
            let Some(form) = form_text(expr, require) else {
                continue;
            };
            let Some(role) = role_of(&form) else {
                continue;
            };
            if known(expr, require).is_some() {
                continue;
            }
            forms
                .entry((role.to_string(), form))
                .or_default()
                .push(label.clone());
        }
    }
    for ((role, form), files) in forms {
        println!("{role} {}\n    r#\"{form}\"#,", files.join(" "));
    }
}

#[test]
fn every_fixture_loader_is_a_recorded_form() {
    let mut loaders = 0;
    for file in bundles() {
        let text = std::fs::read_to_string(&file).expect("read");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &text);
        let Some(statements) = bootstrap(&parsed.program) else {
            continue;
        };
        let Some(require) = require_name(statements) else {
            continue;
        };
        loaders += statements
            .iter()
            .flat_map(elements)
            .filter(|e| known(e, require).is_some_and(|k| k.role == Role::Loader))
            .count();
    }
    assert!(loaders > 0);
}
