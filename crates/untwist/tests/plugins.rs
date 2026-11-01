mod common;

use common::WEBPACK_4;
use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::{Expression, Program, Statement};
use untwist::{Options, Pipeline, Plugins, Stage, StopReason};
use untwist_core::pass::{Cx, Pass};

struct DoubleOddNumbers;

impl<'a> Pass<'a> for DoubleOddNumbers {
    fn name(&self) -> &'static str {
        "test-double-odd-numbers"
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::NumericLiteral(literal) = expr else {
            return;
        };
        if literal.value % 2.0 == 0.0 {
            return;
        }
        literal.value *= 2.0;
        literal.raw = None;
        cx.changed();
    }
}

fn double<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(DoubleOddNumbers)
}

fn with(stage: Stage) -> Options {
    let mut plugins = Plugins::new();
    plugins.register(stage, double as untwist::PluginCtor);
    Options {
        plugins,
        ..Options::default()
    }
}

#[test]
fn an_after_parse_plugin_changes_the_output_and_is_reported() {
    let out = Pipeline::new(with(Stage::AfterParse)).run("x(1);");
    assert_eq!(out.code(), "x(2);");
    assert!(out
        .stages
        .iter()
        .any(|stage| stage.stage == "plugin:after-parse"));
}

#[test]
fn every_hook_point_exists_and_is_reported() {
    for stage in Stage::ALL {
        let out = Pipeline::new(with(stage)).run(WEBPACK_4);
        let name = format!("plugin:{}", stage.as_str());
        assert!(
            out.stages.iter().any(|reported| reported.stage == name),
            "{name} did not run: {:?}",
            out.stages.iter().map(|s| s.stage).collect::<Vec<_>>()
        );
    }
}

#[test]
fn no_plugin_means_no_plugin_stage() {
    let out = Pipeline::new(Options::default()).run("x(1);");
    assert!(!out
        .stages
        .iter()
        .any(|stage| stage.stage.starts_with("plugin:")));
}

#[test]
fn an_after_unpack_plugin_reaches_the_extracted_modules() {
    let out = Pipeline::new(with(Stage::AfterUnpack)).run(WEBPACK_4);
    let bundle = out.bundle.expect("a bundle was detected");
    let codes: Vec<&str> = bundle
        .modules
        .iter()
        .map(|module| module.code.as_str())
        .collect();
    assert!(
        codes.iter().any(|code| code.contains("module.exports = 2")),
        "{codes:?}"
    );
}

struct CountSequences;

impl<'a> Pass<'a> for CountSequences {
    fn name(&self) -> &'static str {
        "test-count-sequences"
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if matches!(expr, Expression::SequenceExpression(_)) {
            cx.note("saw a sequence expression", None);
        }
    }
}

fn count_sequences<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(CountSequences)
}

#[test]
fn after_parse_sees_what_prepare_destroys_and_after_prepare_does_not() {
    let saw = |stage: Stage| {
        let mut plugins = Plugins::new();
        plugins.register(stage, count_sequences as untwist::PluginCtor);
        Pipeline::new(Options {
            plugins,
            unpack: false,
            ..Options::default()
        })
        .run("x(), y();")
        .notes
        .iter()
        .any(|note| note.message == "saw a sequence expression")
    };
    assert!(saw(Stage::AfterParse));
    assert!(!saw(Stage::AfterPrepare));
}

struct Noop;

impl<'a> Pass<'a> for Noop {
    fn name(&self) -> &'static str {
        "test-noop"
    }
}

fn noop<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(Noop)
}

#[test]
fn every_hook_is_reported_where_it_runs() {
    let mut plugins = Plugins::new();
    for stage in Stage::ALL {
        plugins.register(stage, noop as untwist::PluginCtor);
    }
    let out = Pipeline::new(Options {
        plugins,
        ..Options::default()
    })
    .run(WEBPACK_4);
    let names: Vec<&str> = out.stages.iter().map(|stage| stage.stage).collect();
    assert_eq!(
        names,
        [
            "plugin:after-parse",
            "prepare",
            "plugin:after-prepare",
            "deobfuscate",
            "plugin:after-deobfuscate",
            "unminify",
            "plugin:after-unminify",
            "protections",
            "jsx",
            "cleanup",
            "lexical",
            "unpack",
            "plugin:after-unpack",
        ]
    );
}

struct AlwaysChanges;

impl<'a> Pass<'a> for AlwaysChanges {
    fn name(&self) -> &'static str {
        "test-always-changes"
    }

    fn exit_program(&mut self, _program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        cx.changed();
    }
}

fn always_changes<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(AlwaysChanges)
}

#[test]
fn after_unpack_reports_a_module_that_did_not_converge() {
    let mut plugins = Plugins::new();
    plugins.register(Stage::AfterUnpack, always_changes as untwist::PluginCtor);
    let out = Pipeline::new(Options {
        plugins,
        max_iterations: Some(2),
        ..Options::default()
    })
    .run(WEBPACK_4);
    let row = out
        .stages
        .iter()
        .find(|stage| stage.stage == "plugin:after-unpack")
        .expect("the after-unpack row is reported");
    assert_ne!(row.stop, StopReason::Converged, "{row:?}");
    assert!(row.iterations >= 2, "{row:?}");
    assert_eq!(row.per_pass, [("test-always-changes", 4)], "{row:?}");
}

#[test]
fn after_unpack_concatenates_every_modules_iterations() {
    let mut plugins = Plugins::new();
    plugins.register(Stage::AfterUnpack, always_changes as untwist::PluginCtor);
    let out = Pipeline::new(Options {
        plugins,
        max_iterations: Some(2),
        ..Options::default()
    })
    .run(WEBPACK_4);
    let row = out
        .stages
        .iter()
        .find(|stage| stage.stage == "plugin:after-unpack")
        .expect("the after-unpack row is reported");
    assert_eq!(row.iterations, 2, "{row:?}");
    assert_eq!(row.iteration_elapsed.len(), 4, "{row:?}");
    assert!(row
        .iteration_elapsed
        .iter()
        .all(|timing| timing.rebuild().is_some()));
}

struct MergeDeclarations;

fn single_var(statement: &Statement<'_>) -> bool {
    matches!(statement, Statement::VariableDeclaration(declaration) if declaration.declarations.len() == 1)
}

impl<'a> Pass<'a> for MergeDeclarations {
    fn name(&self) -> &'static str {
        "test-merge-declarations"
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        let Some(at) =
            (1..stmts.len()).find(|&at| single_var(&stmts[at - 1]) && single_var(&stmts[at]))
        else {
            return;
        };
        let Statement::VariableDeclaration(second) = stmts.remove(at) else {
            return;
        };
        if let Statement::VariableDeclaration(first) = &mut stmts[at - 1] {
            first.declarations.extend(second.unbox().declarations);
            cx.changed();
        }
    }
}

fn merge_declarations<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(MergeDeclarations)
}

#[test]
fn deobfuscate_prepares_again_after_an_after_prepare_plugin_edits() {
    let mut plugins = Plugins::new();
    plugins.register(
        Stage::AfterPrepare,
        merge_declarations as untwist::PluginCtor,
    );
    let out = Pipeline::new(Options {
        plugins,
        unminify: false,
        unpack: false,
        ..Options::default()
    })
    .run("var a = f(), b = g();\nh(a, b);");
    assert_eq!(out.code(), "var a = f();\nvar b = g();\nh(a, b);");
}
