mod common;

use common::{CHUNK_A, CHUNK_B, WEBPACK_4};
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist::{Options, Pipeline, Plugins, Stage, UNPACK_PANICKED};
use untwist_core::pass::{Cx, Pass};

struct PanicsOnEveryModule;

impl<'a> Pass<'a> for PanicsOnEveryModule {
    fn name(&self) -> &'static str {
        "test-panics-on-every-module"
    }

    fn enter_program(&mut self, _program: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {
        panic!("the test plugin panicked on purpose");
    }
}

fn panicking<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(PanicsOnEveryModule)
}

fn options() -> Options {
    let mut plugins = Plugins::new();
    plugins.register(Stage::AfterUnpack, panicking as untwist::PluginCtor);
    Options {
        plugins,
        ..Options::default()
    }
}

#[test]
fn a_panic_in_the_unpack_stage_loses_the_bundle_and_nothing_else() {
    let out = Pipeline::new(options()).run_many(&[CHUNK_A, CHUNK_B]);
    assert!(out.bundle.is_none());
    assert_eq!(out.codes.len(), 2);
    assert!(out.codes.iter().all(|code| code.contains("webpackJsonp")));
    assert!(out.panicked.is_empty(), "{:?}", out.panicked);
    assert!(
        out.notes
            .iter()
            .any(|note| note.pass == "pipeline" && note.message == UNPACK_PANICKED),
        "{:?}",
        out.notes
    );
}

#[test]
fn a_panic_in_the_unpack_stage_is_flagged_on_the_output() {
    let out = Pipeline::new(options()).run_many(&[CHUNK_A, CHUNK_B]);
    assert!(out.unpack_panicked);
    let clean = Pipeline::new(Options::default()).run_many(&[CHUNK_A, CHUNK_B]);
    assert!(!clean.unpack_panicked);
}

#[test]
fn without_the_plugin_the_same_inputs_unpack() {
    let out = Pipeline::new(Options::default()).run_many(&[CHUNK_A, CHUNK_B]);
    assert!(out.bundle.is_some());
    assert!(!out.notes.iter().any(|note| note.message == UNPACK_PANICKED));
}

#[test]
fn a_single_input_that_panics_while_unpacking_is_still_written() {
    let out = Pipeline::new(options()).run(WEBPACK_4);
    assert!(out.bundle.is_none());
    assert!(out.code().contains("exports"));
    assert_eq!(out.panicked, Vec::<usize>::new());
}
