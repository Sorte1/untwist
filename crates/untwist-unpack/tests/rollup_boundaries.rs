use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_unpack::{finish_with, unpack_program_with, UnpackOptions};

const DIR: &str = "fixtures/unpack-rollup-boundaries";
const SCORES: &str = "SCORES";
const NAMES: &str = "NAMES";
const UNNAMED: [&str; 2] = ["module", "chunk_module"];
const BLESS: &str = "UNTWIST_BLESS";
const HOST: &str = "hostConfig";
const ASSUMED: UnpackOptions = UnpackOptions {
    split_hoisted: true,
    assume_modules: true,
    conservative: false,
    trust_free_loader: false,
    trust_legacy_reads: false,
    trust_builtins: false,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cells() -> Vec<String> {
    let manifest = std::fs::read_to_string(root().join(DIR).join("MANIFEST")).expect("MANIFEST");
    manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .filter_map(|line| line.split('/').next().map(str::to_string))
        .collect()
}

fn truth(cell: &str) -> Vec<(u32, String)> {
    let text = std::fs::read_to_string(root().join(DIR).join("truth").join(format!("{cell}.tsv")))
        .expect("truth");
    text.lines()
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            (fields[0].parse().expect("start"), fields[3].to_string())
        })
        .collect()
}

fn called<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut FunctionBody<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let mut expression = &mut statement.expression;
    while let Expression::UnaryExpression(unary) = expression {
        expression = &mut unary.argument;
    }
    let Expression::CallExpression(call) = expression.without_parentheses_mut() else {
        return None;
    };
    match call.callee.without_parentheses_mut() {
        Expression::FunctionExpression(function) => function.body.as_deref_mut(),
        _ => None,
    }
}

fn hosts(body: &FunctionBody<'_>) -> bool {
    body.statements.iter().any(|statement| {
        let Statement::VariableDeclaration(declaration) = statement else {
            return false;
        };
        declaration.declarations[0]
            .id
            .get_binding_identifier()
            .is_some_and(|id| id.name == HOST)
    })
}

fn inner<'k, 'a>(program: &'k mut Program<'a>) -> &'k mut FunctionBody<'a> {
    let body = called(&mut program.body[0]).expect("iife");
    if !hosts(body) {
        return body;
    }
    let at = body
        .statements
        .iter_mut()
        .position(|statement| called(statement).is_some())
        .expect("inner iife");
    called(&mut body.statements[at]).expect("inner iife")
}

fn units<'a>(alloc: &'a Allocator, statement: &Statement<'a>, out: &mut AVec<'a, Statement<'a>>) {
    let builder = AstBuilder::new(alloc);
    match statement {
        Statement::VariableDeclaration(declaration) if declaration.declarations.len() > 1 => {
            for declarator in &declaration.declarations {
                let single = AVec::from_iter_in([declarator.clone_in(alloc)], &builder);
                out.push(Statement::new_variable_declaration(
                    declarator.span,
                    declaration.kind,
                    single,
                    false,
                    &builder,
                ));
            }
        }
        Statement::ExpressionStatement(expression) => match &expression.expression {
            Expression::SequenceExpression(sequence) => {
                for part in &sequence.expressions {
                    out.push(Statement::new_expression_statement(
                        part.span(),
                        part.clone_in(alloc),
                        &builder,
                    ));
                }
            }
            _ => out.push(statement.clone_in(alloc)),
        },
        _ => out.push(statement.clone_in(alloc)),
    }
}

struct Score {
    truth: usize,
    cuts: usize,
    hits: usize,
    modules: usize,
    names: Vec<(String, String, Option<String>)>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verdict {
    Exact,
    Defined,
    Generic,
    Misleading,
}

fn stem_of(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.strip_suffix(".js").unwrap_or(file);
    let stem = match stem.rsplit_once('-') {
        Some((base, suffix)) if suffix.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => stem,
    };
    stem.to_ascii_lowercase()
}

fn verdict(path: &str, label: &str, origin: Option<&str>) -> Verdict {
    let stem = stem_of(path);
    let generic = UNNAMED.iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    });
    if generic {
        return Verdict::Generic;
    }
    if stem == stem_of(label) {
        return Verdict::Exact;
    }
    match origin == Some(label) {
        true => Verdict::Defined,
        false => Verdict::Misleading,
    }
}

fn same(binding: &str, name: &str) -> bool {
    let base = match binding.rsplit_once('$') {
        Some((base, suffix)) if suffix.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => binding,
    };
    binding == name || base == name
}

fn binds(statement: &Statement<'_>, name: &str) -> bool {
    let id = |id: Option<&BindingIdentifier<'_>>| id.is_some_and(|id| same(&id.name, name));
    let vars = |declaration: &VariableDeclaration<'_>| {
        declaration
            .declarations
            .iter()
            .any(|declarator| id(declarator.id.get_binding_identifier()))
    };
    match statement {
        Statement::FunctionDeclaration(function) => id(function.id.as_ref()),
        Statement::ClassDeclaration(class) => id(class.id.as_ref()),
        Statement::VariableDeclaration(declaration) => vars(declaration),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::FunctionDeclaration(function) => id(function.id.as_ref()),
            Declaration::ClassDeclaration(class) => id(class.id.as_ref()),
            Declaration::VariableDeclaration(declaration) => vars(declaration),
            _ => false,
        },
        _ => false,
    }
}

fn declared_at(program: &Program<'_>, name: &str) -> Option<u32> {
    program
        .body
        .iter()
        .find(|statement| binds(statement, name))
        .map(|statement| statement.span().start)
}

fn majority(labels: Vec<&str>) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for label in labels {
        *counts.entry(label).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(label, _)| label.to_string())
}

fn score(cell: &str) -> Score {
    let text =
        std::fs::read_to_string(root().join(DIR).join(cell).join("index.js")).expect("bundle");
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, &text);
    let body = inner(&mut parsed.program);
    let mut split = AVec::new_in(&AstBuilder::new(&alloc));
    for statement in body.statements.iter() {
        units(&alloc, statement, &mut split);
    }
    body.statements = split;
    let labels = truth(cell);
    let label_of: BTreeMap<u32, &str> = labels
        .iter()
        .map(|(at, label)| (*at, label.as_str()))
        .collect();
    let mut bundle = unpack_program_with(&alloc, &parsed.program, ASSUMED);
    if let Some(bundle) = &mut bundle {
        finish_with(&alloc, bundle, &[], ASSUMED);
    }
    let mut placed: BTreeMap<u32, String> = BTreeMap::new();
    let modules = bundle.as_ref().map_or(1, |bundle| bundle.modules.len());
    if let Some(bundle) = &bundle {
        for (id, module) in &bundle.modules {
            let mut stack: Vec<&Statement<'_>> = module.program().body.iter().collect();
            while let Some(statement) = stack.pop() {
                let start = statement.span().start;
                if label_of.contains_key(&start) {
                    placed.insert(start, id.clone());
                    continue;
                }
                if let Statement::ExpressionStatement(expression) = statement {
                    let mut call = &expression.expression;
                    while let Expression::UnaryExpression(unary) = call {
                        call = &unary.argument;
                    }
                    if let Expression::CallExpression(call) = call {
                        if let Expression::FunctionExpression(function) = &call.callee {
                            if let Some(body) = &function.body {
                                stack.extend(body.statements.iter());
                            }
                        }
                    }
                }
            }
        }
    }
    let mut names = Vec::new();
    if let Some(bundle) = &bundle {
        for (id, module) in &bundle.modules {
            if module.is_entry || module.path.as_str() == "chunk_shared.js" || id == "shell" {
                continue;
            }
            let inside = placed
                .iter()
                .filter(|(_, owner)| *owner == id)
                .filter_map(|(at, _)| label_of.get(at).copied())
                .collect();
            if let Some(label) = majority(inside) {
                let file = module.path.as_str().rsplit('/').next().unwrap_or_default();
                let name = file.strip_suffix(".js").unwrap_or(file);
                let name = match name.rsplit_once('-') {
                    Some((base, suffix)) if suffix.bytes().all(|b| b.is_ascii_digit()) => base,
                    _ => name,
                };
                let origin = declared_at(module.program(), name)
                    .and_then(|at| label_of.get(&at))
                    .map(|label| label.to_string());
                names.push((module.path.as_str().to_string(), label, origin));
            }
        }
    }
    let mut result = Score {
        truth: 0,
        cuts: 0,
        hits: 0,
        modules,
        names,
    };
    let order: Vec<&(u32, String)> = labels.iter().collect();
    for pair in order.windows(2) {
        let real = pair[0].1 != pair[1].1;
        let module = |at: u32| placed.get(&at).map(String::as_str).unwrap_or("index");
        let cut = module(pair[0].0) != module(pair[1].0);
        result.truth += usize::from(real);
        result.cuts += usize::from(cut);
        result.hits += usize::from(real && cut);
    }
    result
}

fn ratio(part: usize, whole: usize) -> String {
    match whole {
        0 => "-".to_string(),
        _ => format!("{:.2}", part as f64 / whole as f64),
    }
}

#[test]
fn inferred_module_boundaries_score_against_rollups_own_source_map() {
    let mut lines = Vec::new();
    let mut totals: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for cell in cells() {
        let found = score(&cell);
        let group = match cell.contains("-min-") {
            true => "minified",
            false => "plain",
        };
        for key in ["all", group] {
            let total = totals.entry(key.to_string()).or_default();
            total.0 += found.truth;
            total.1 += found.cuts;
            total.2 += found.hits;
        }
        lines.push(format!(
            "{cell}\tmodules {}\ttrue {}\tcuts {}\thits {}\tP {}\tR {}",
            found.modules,
            found.truth,
            found.cuts,
            found.hits,
            ratio(found.hits, found.cuts),
            ratio(found.hits, found.truth)
        ));
    }
    for (key, (truth, cuts, hits)) in &totals {
        lines.push(format!(
            "total {key}\ttrue {truth}\tcuts {cuts}\thits {hits}\tP {}\tR {}",
            ratio(*hits, *cuts),
            ratio(*hits, *truth)
        ));
    }
    let text = format!("{}\n", lines.join("\n"));
    let path = root().join(DIR).join(SCORES);
    if std::env::var_os(BLESS).is_some() {
        std::fs::write(&path, &text).expect("write SCORES");
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        committed, text,
        "boundary scores moved (rerun with {BLESS}=1 to accept)"
    );
}

#[test]
fn inferred_module_names_score_against_rollups_own_source_map() {
    let mut lines = Vec::new();
    let mut totals: BTreeMap<Verdict, usize> = BTreeMap::new();
    for cell in cells() {
        let found = score(&cell);
        let mut counts: BTreeMap<Verdict, usize> = BTreeMap::new();
        let mut wrong = Vec::new();
        for (path, label, origin) in &found.names {
            let verdict = verdict(path, label, origin.as_deref());
            *counts.entry(verdict).or_default() += 1;
            *totals.entry(verdict).or_default() += 1;
            if verdict == Verdict::Misleading {
                wrong.push(format!("{path}={label}"));
            }
        }
        let count = |verdict| counts.get(&verdict).copied().unwrap_or_default();
        lines.push(format!(
            "{cell}\texact {}\tdefined {}\tgeneric {}\tmisleading {}\t{}",
            count(Verdict::Exact),
            count(Verdict::Defined),
            count(Verdict::Generic),
            count(Verdict::Misleading),
            wrong.join(" ")
        ));
    }
    let total = |verdict| totals.get(&verdict).copied().unwrap_or_default();
    lines.push(format!(
        "total\texact {}\tdefined {}\tgeneric {}\tmisleading {}",
        total(Verdict::Exact),
        total(Verdict::Defined),
        total(Verdict::Generic),
        total(Verdict::Misleading)
    ));
    let text = format!("{}\n", lines.join("\n"));
    let path = root().join(DIR).join(NAMES);
    if std::env::var_os(BLESS).is_some() {
        std::fs::write(&path, &text).expect("write NAMES");
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        committed, text,
        "module names moved (rerun with {BLESS}=1 to accept)"
    );
}
