#![allow(dead_code)]

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program, Statement};
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassCtor, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_unpack::model::{Bundle, BundleKind, Module};

pub fn parse<'a>(alloc: &'a Allocator, source: &'a str) -> Program<'a> {
    let parsed = untwist_core::parse::parse_program(alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    parsed.program
}

pub trait PrintedModule {
    fn code(&self) -> String;
}

impl PrintedModule for Module<'_> {
    fn code(&self) -> String {
        print(self.program(), PrintStyle::Pretty)
    }
}

pub fn print_module(module: &Module<'_>) -> String {
    module.code().trim_end().to_string()
}

pub fn prepared<'a>(alloc: &'a Allocator, source: &'a str) -> Program<'a> {
    let mut program = parse(alloc, source);
    let sem = build_scoping(&program);
    let mut group =
        PassGroup::from_ctors(&untwist_unminify::prepare_ctors(), PassOptions::default());
    let (sem, _) = run_to_fixpoint(alloc, &mut program, sem, &mut group);
    let mut ctors: Vec<PassCtor<'a>> = untwist_unminify::transpile_ctors();
    ctors.extend(untwist_unminify::unminify_ctors());
    let mut group = PassGroup::from_ctors(&ctors, PassOptions::default());
    let (sem, _) = run_to_fixpoint(alloc, &mut program, sem, &mut group);
    let bundle = untwist_unpack::detect::holds_a_bundle(&program);
    let opts = PassOptions {
        keep_captured_vars: bundle,
        keep_var_positions: bundle,
        ..PassOptions::default()
    };
    let mut group = PassGroup::from_ctors(&untwist_unminify::lexical_ctors(), opts);
    untwist_unminify::run_last_traversal(alloc, &mut program, sem, &mut group, false);
    program
}

pub fn webpack4_without_module_value() -> String {
    let value = "console.log(e);";
    let source = fixture("webpack-4.js");
    assert!(source.contains(value));
    source.replacen(value, "console.log(e.exports);", 1)
}

pub fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/reference/unpack")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

pub fn first_expression_statement<'k, 'a>(program: &'k Program<'a>) -> &'k Expression<'a> {
    let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
        panic!("first statement is not an expression statement");
    };
    untwist_unpack::matcher::unwrap_statement_expression(&stmt.expression)
}

pub fn bundle_of<'a>(
    alloc: &'a Allocator,
    sources: &[(&str, &'a str, bool)],
    entry: Option<&str>,
) -> Bundle<'a> {
    let mut bundle = Bundle::new(BundleKind::Webpack, entry.map(str::to_string));
    for (id, source, is_esm) in sources {
        let mut program = parse(alloc, source);
        program.source_type = if *is_esm {
            SourceType::mjs()
        } else {
            SourceType::cjs()
        };
        let is_entry = entry == Some(*id);
        bundle.modules.insert(
            (*id).to_string(),
            Module::new((*id).to_string(), program, is_entry),
        );
    }
    bundle
}

pub fn module_summary(bundle: &Bundle<'_>) -> Vec<(String, String, bool)> {
    bundle
        .modules
        .values()
        .map(|module| {
            (
                module.id.clone(),
                module.path.as_str().to_string(),
                module.is_entry,
            )
        })
        .collect()
}

pub fn bundle_from<'a>(alloc: &'a Allocator, source: &'a str) -> Option<Bundle<'a>> {
    let program = prepared(alloc, source);
    untwist_unpack::detect::unpack_program(alloc, &program)
}

fn thread_cpu() -> Option<std::time::Duration> {
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let nanos = stat.split_whitespace().next()?.parse().ok()?;
    Some(std::time::Duration::from_nanos(nanos))
}

pub fn cpu_or_wall<T>(work: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let (cpu, wall) = (thread_cpu(), std::time::Instant::now());
    let done = work();
    let spent = match (cpu, thread_cpu()) {
        (Some(before), Some(after)) => after.saturating_sub(before),
        _ => wall.elapsed(),
    };
    (done, spent)
}
