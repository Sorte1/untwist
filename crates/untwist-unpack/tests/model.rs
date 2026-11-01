mod common;

use common::{parse, print_module};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_unpack::model::{module_from_body, Bundle, BundleKind, Module};
use untwist_unpack::path::PosixPath;

fn first_factory_body<'k, 'a>(program: &'k Program<'a>) -> &'k FunctionBody<'a> {
    let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
        panic!("expression statement");
    };
    let Expression::FunctionExpression(function) = &stmt.expression else {
        panic!("function expression");
    };
    function.body.as_deref().expect("block body")
}

#[test]
fn extraction_clones_the_body_and_leaves_the_source_alone() {
    let alloc = Allocator::default();
    let source = "(function (module, exports, require) { 'use strict'; module.exports = 1; });";
    let program = parse(&alloc, source);
    let extracted = module_from_body(&alloc, &program, first_factory_body(&program));
    let module = Module::new("3".to_string(), extracted, false);
    assert_eq!(print_module(&module), "module.exports = 1;");
    assert_eq!(
        untwist_core::print::print(&program, untwist_core::print::PrintStyle::Compact),
        "(function(module,exports,require){\"use strict\";module.exports=1});"
    );
}

#[test]
fn module_comments_come_from_the_factory_body_only() {
    let alloc = Allocator::default();
    let source = "/* outer */ (function () { /* inner */ x(); });";
    let program = parse(&alloc, source);
    let extracted = module_from_body(&alloc, &program, first_factory_body(&program));
    let module = Module::new("0".to_string(), extracted, false);
    let code = print_module(&module);
    assert!(code.contains("inner"), "{code}");
    assert!(!code.contains("outer"), "{code}");
}

#[test]
fn printing_follows_program_mut_edits() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "(function () { x(); });");
    let extracted = module_from_body(&alloc, &program, first_factory_body(&program));
    let mut module = Module::new("0".to_string(), extracted, false);
    assert_eq!(print_module(&module), "x();");
    module.program_mut().body.clear();
    assert_eq!(print_module(&module), "");
}

#[test]
fn default_paths_follow_the_entry_flag() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "(function () {});");
    let body = first_factory_body(&program);
    let entry = Module::new(
        "2".to_string(),
        module_from_body(&alloc, &program, body),
        true,
    );
    let other = Module::new(
        "2wwy".to_string(),
        module_from_body(&alloc, &program, body),
        false,
    );
    assert_eq!(entry.path, PosixPath::parse("index.js").expect("safe"));
    assert_eq!(other.path, PosixPath::parse("2wwy.js").expect("safe"));
}

#[test]
fn bundle_keeps_insertion_order_and_finds_the_entry() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "(function () {});");
    let body = first_factory_body(&program);
    let mut bundle = Bundle::new(BundleKind::Webpack, Some("2".to_string()));
    for (id, is_entry) in [("1", false), ("2", true), ("3", false)] {
        bundle.modules.insert(
            id.to_string(),
            Module::new(
                id.to_string(),
                module_from_body(&alloc, &program, body),
                is_entry,
            ),
        );
    }
    assert_eq!(
        bundle.modules.keys().cloned().collect::<Vec<_>>(),
        vec!["1", "2", "3"]
    );
    assert_eq!(bundle.entry().map(|module| module.id.as_str()), Some("2"));
    assert_eq!(bundle.path_of("3").map(PosixPath::as_str), Some("3.js"));
    assert_eq!(bundle.kind.as_str(), "webpack");
}

#[test]
fn every_kind_names_itself_in_lower_case_for_bundle_json() {
    let names: Vec<&str> = untwist_unpack::model::BundleKind::ALL
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "webpack",
            "browserify",
            "esbuild",
            "rollup",
            "systemjs",
            "amd",
            "umd",
            "parcel",
            "metro",
            "turbopack",
            "closure"
        ]
    );
}

fn factory_bodies<'k, 'a>(program: &'k Program<'a>) -> Vec<&'k FunctionBody<'a>> {
    let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
        panic!("expression statement");
    };
    let Expression::ArrayExpression(array) = &stmt.expression else {
        panic!("array");
    };
    array
        .elements
        .iter()
        .map(|element| match element {
            ArrayExpressionElement::FunctionExpression(function) => {
                function.body.as_deref().expect("block body")
            }
            _ => panic!("function expression"),
        })
        .collect()
}

fn comment_texts<'a>(program: &Program<'a>) -> Vec<&'a str> {
    program
        .comments
        .iter()
        .map(|comment| comment.span.source_text(program.source_text))
        .collect()
}

#[test]
fn a_module_keeps_exactly_the_comments_inside_its_body() {
    let alloc = Allocator::default();
    let source = "/*a*/[/*b*/function(){/*c*/x();//d\n}/*e*/,function(){},function(){/*f*/y()/*g*/}/*h*/];//i\n";
    let program = parse(&alloc, source);
    let kept: Vec<Vec<&str>> = factory_bodies(&program)
        .into_iter()
        .map(|body| comment_texts(&module_from_body(&alloc, &program, body)))
        .collect();
    assert_eq!(
        kept,
        vec![vec!["/*c*/", "//d"], vec![], vec!["/*f*/", "/*g*/"]]
    );
}

const STRESS_FACTORIES: usize = 100_000;
const STRESS_SECONDS: u64 = 30;

#[test]
fn extracting_many_commented_bodies_takes_linear_time() {
    let factories: Vec<String> = (0..STRESS_FACTORIES)
        .map(|i| format!("function(){{/*m{i}*/x{i}()}}"))
        .collect();
    let source = format!("[{}];\n", factories.join(","));
    let alloc = Allocator::default();
    let program = parse(&alloc, &source);
    let started = std::time::Instant::now();
    let kept: usize = factory_bodies(&program)
        .into_iter()
        .map(|body| module_from_body(&alloc, &program, body).comments.len())
        .sum();
    assert_eq!(kept, STRESS_FACTORIES);
    let elapsed = started.elapsed();
    assert!(elapsed.as_secs() < STRESS_SECONDS, "{elapsed:?}");
}
