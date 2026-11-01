mod common;

use common::{CHUNK_A, CHUNK_B};
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use untwist::{Options, Pipeline, Plugins, Stage};
use untwist_core::pass::{Cx, Pass};

const WEBPACK_4_ON_ONE_LINE: &str = "!function(e){var a={};function n(t){if(a[t])return a[t].exports;var r=a[t]={exports:{}};return e[t].call(r.exports,r,r.exports,n),r.exports}return n(n.s=0)}([function(e,t,i){const lib=i(1);console.log(lib)},function(e,t,i){e.exports=1}]);";

struct NotesEveryNumber;

impl<'a> Pass<'a> for NotesEveryNumber {
    fn name(&self) -> &'static str {
        "test-notes-every-number"
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if let Expression::NumericLiteral(literal) = expr {
            cx.note("a number", Some(literal.span));
        }
    }
}

fn noting<'a>(_alloc: &'a Allocator) -> Box<dyn Pass<'a> + 'a> {
    Box::new(NotesEveryNumber)
}

#[test]
fn an_after_unpack_notes_span_is_an_offset_into_the_final_text() {
    let mut plugins = Plugins::new();
    plugins.register(Stage::AfterUnpack, noting as untwist::PluginCtor);
    let out = Pipeline::new(Options {
        plugins,
        ..Options::default()
    })
    .run(WEBPACK_4_ON_ONE_LINE);
    let spans: Vec<_> = out
        .notes
        .iter()
        .filter(|note| note.pass == "test-notes-every-number")
        .filter_map(|note| note.span)
        .collect();
    assert!(!spans.is_empty(), "{:?}", out.notes);
    for span in spans {
        let text = out
            .code()
            .get(span.start as usize..span.end as usize)
            .unwrap_or_else(|| panic!("{span:?} is outside the final text"));
        assert!(text.parse::<f64>().is_ok(), "{text:?} at {span:?}");
    }
}

#[test]
fn the_codes_do_not_depend_on_unpack() {
    let with = Pipeline::new(Options::default()).run_many(&[CHUNK_A, CHUNK_B]);
    let without = Pipeline::new(Options {
        unpack: false,
        ..Options::default()
    })
    .run_many(&[CHUNK_A, CHUNK_B]);
    assert_eq!(with.codes, without.codes);
    assert!(with.bundle.is_some());
    assert!(without.bundle.is_none());
}

#[test]
fn the_second_inputs_stage_rounds_start_at_one() {
    let out = Pipeline::new(Options::default()).run_many(&[CHUNK_A, CHUNK_B]);
    let second_input = out
        .stages
        .iter()
        .rposition(|stage| stage.stage == "prepare")
        .expect("the second input's prepare row");
    let unminify = out.stages[second_input..]
        .iter()
        .find(|stage| stage.stage == "unminify")
        .expect("the second input's unminify row");
    assert_eq!(unminify.round, 1, "{:?}", out.stages);
}
