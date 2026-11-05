#![cfg(debug_assertions)]

use untwist::{Options, Pipeline, RenameMode};
use untwist_core::reflow::{
    flip_bundle_answer_for_tests, misplace_an_edit_for_tests, recording, reflow_recorded, Witnessed,
};
use untwist_testing::{generated_bundle_present, js_files_in, real_world_fixture};

const MODES: [(&str, bool, bool); 4] = [
    ("d", false, true),
    ("p", false, false),
    ("u", true, true),
    ("up", true, false),
];

const DIFF_SETS: [&str; 6] = [
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/unminify",
    "fixtures/jsx",
    "fixtures/deobfuscate-gaps",
    "fixtures/unpack-gaps",
];

const MANGLE_SETS: [&str; 3] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
];

#[derive(Default)]
struct Tally {
    compared: usize,
    in_rounds: usize,
    placed: usize,
    reparsed: usize,
}

impl Tally {
    fn absorb(&mut self, label: &str, witnessed: Witnessed) {
        assert!(
            witnessed.differing.is_empty(),
            "{label}: the one-parse reflow differs from the round loop: {:?}",
            witnessed.differing
        );
        self.compared += witnessed.compared;
        self.in_rounds += witnessed.in_rounds;
        self.placed += witnessed.placed;
        self.reparsed += witnessed.reparsed;
    }
}

fn mode(unsafe_transforms: bool, extended: bool) -> Options {
    Options {
        unsafe_transforms,
        extended,
        ..Options::default()
    }
}

fn witness(tally: &mut Tally, inputs: &[(String, String)], options: &Options) {
    for (label, source) in inputs {
        let pipeline = Pipeline::new(options.clone());
        let (_, witnessed) = recording(|| pipeline.run(source));
        tally.absorb(label, witnessed);
    }
}

fn every_mode(inputs: &[(String, String)]) -> Tally {
    assert!(!inputs.is_empty(), "no inputs");
    let mut tally = Tally::default();
    for (_, unsafe_transforms, extended) in MODES {
        witness(&mut tally, inputs, &mode(unsafe_transforms, extended));
    }
    tally
}

fn game_js(index: usize) -> Option<Tally> {
    let (_, unsafe_transforms, extended) = MODES[index];
    let mut tally = Tally::default();
    let game = vec![("game.js".to_string(), real_world_fixture("game.js")?)];
    witness(&mut tally, &game, &mode(unsafe_transforms, extended));
    assert!(tally.compared > 0);
    assert!(
        tally.in_rounds > 0,
        "game.js no longer needs a second round"
    );
    assert_eq!(
        (tally.placed, tally.reparsed),
        (1, 0),
        "game.js was reflowed through a second parse"
    );
    Some(tally)
}

#[test]
fn game_js_default_reflows_as_the_round_loop_does() {
    game_js(0);
}

#[test]
fn game_js_conservative_reflows_as_the_round_loop_does() {
    game_js(1);
}

#[test]
fn game_js_unsafe_reflows_as_the_round_loop_does() {
    game_js(2);
}

#[test]
fn game_js_unsafe_conservative_reflows_as_the_round_loop_does() {
    game_js(3);
}

#[test]
fn the_high_preset_reflows_as_the_round_loop_does() {
    let high = js_files_in("fixtures/reference/deobfuscate")
        .into_iter()
        .filter(|(label, _)| label.ends_with("obfuscator.io-high.js"))
        .collect::<Vec<_>>();
    let tally = every_mode(&high);
    assert!(tally.compared > 0 && tally.placed > 0);
}

#[test]
fn corpus_npm_reflows_as_the_round_loop_does() {
    let tally = every_mode(&js_files_in("fixtures/corpus-npm"));
    assert!(tally.compared > 0 && tally.placed > 0);
}

#[test]
fn the_behaviour_corpus_reflows_as_the_round_loop_does() {
    let tally = every_mode(&js_files_in("fixtures/behaviour"));
    assert!(tally.compared > 0 && tally.placed > 0);
}

#[test]
fn the_diff_sets_reflow_as_the_round_loop_does() {
    let mut compared = 0;
    for dir in DIFF_SETS {
        let tally = every_mode(&js_files_in(dir));
        assert!(tally.placed > 0, "{dir}: nothing was placed");
        compared += tally.compared;
    }
    let mangle = Options {
        rename: RenameMode::Mangle,
        ..Options::default()
    };
    let mut tally = Tally::default();
    for dir in MANGLE_SETS {
        witness(&mut tally, &js_files_in(dir), &mangle);
    }
    assert!(compared > 0 && tally.compared > 0);
}

#[test]
fn the_generated_bundle_reflows_as_the_round_loop_does() {
    if !generated_bundle_present() {
        return;
    }
    let bundle = js_files_in("fixtures/generated");
    let options = Options {
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    };
    let mut tally = Tally::default();
    witness(&mut tally, &bundle, &options);
    assert!(tally.compared > 1, "the bundle's modules were not reflowed");
    assert_eq!(tally.placed, 1);
}

const GUARD_IN_AN_ELEMENT: &str = "var a = [function () { var ctrl = function () { var f = !![]; \
return function (c, fn) { var r = f ? function () {
  if (fn) {
    var x = fn.apply(c, arguments);
    fn = null;
    return x;
  }
} : function () {}; f = ![]; return r; }; }(); },
  1,
  2
];";

const GUARD_IN_AN_ELEMENT_REFLOWED: &str = "var a = [function () { var ctrl = function () { var f = !![]; \
return function (c, fn) { var r = f ? function () {if (fn) {var x = fn.apply(c, arguments);fn = null;return x;}} \
: function () {}; f = ![]; return r; }; }(); }, 1, 2];";

#[test]
fn a_compacted_guard_lets_its_array_collapse_in_one_parse() {
    let (reflowed, witnessed) = recording(|| reflow_recorded(GUARD_IN_AN_ELEMENT.to_string()));
    assert_eq!(reflowed.0, GUARD_IN_AN_ELEMENT_REFLOWED);
    assert_eq!(reflowed.1.tally().compacted, 1);
    assert_eq!((witnessed.compared, witnessed.in_rounds), (1, 1));
    assert!(witnessed.differing.is_empty(), "{:?}", witnessed.differing);
}

#[test]
fn nested_line_breaks_cascade_in_one_parse() {
    let sources = [
        "var a = [[{\n  x: 1\n}, 2, 3], [4, 5, 6], [7, 8, 9]];",
        "var b = [[[{ a: 1,\n b: 2 }, 1, 2], 3, 4], 5, 6];",
        "var c = [function () {\n  return 1;\n}, 2, 3];",
        "var d = [[1, 2, 3], `\n`, 4];",
    ];
    let inputs = sources
        .iter()
        .enumerate()
        .map(|(index, source)| (format!("nested {index}"), (*source).to_string()))
        .collect::<Vec<_>>();
    let tally = every_mode(&inputs);
    assert!(tally.compared > 0);
}

const PLACEMENT_CASES: [&str; 24] = [
    "(\"use strict\");\ndefine([\"a\", \"b\", \"c\"], function (a, b, c) { return [a, b, c]; });",
    "var a = [/* c */ { a: 1 }, 2, 3];\nvar b = [2, /* d */ { b: 1, c: 2 }, 3, (x) => x];",
    "var a = [\"é\", \"日本\", \"😀\"]; var b = [x, \"→\", y, \"\\u2028\"];",
    "var a = [/* @__PURE__ */ f(), g(), h()]; var b = [g(), /* @__PURE__ */ f(), h()];",
    "// lead\nvar a = [1, 2, 3];\nvar b = [1, /* c */ 2, 3];\nvar c = [\n// own line\n1, 2, 3];",
    "var a = [1, 2, 3 /* trailing */, 4];\nvar b = [(x) => x, 1, 2]; // after",
    "\"use strict\";\nvar a = [1, 2, 3];\nfunction f() { \"use strict\"; return [4, 5, 6]; }",
    "var a = [`a\nb`, 1, 2]; var b = [[1, 2, 3], `\n  ]`, [4, 5, 6]];",
    "var a = [1, , 3, 4]; var b = [, 1, 2]; var c = [1, 2, ,];",
    "var a = [(x, y), z, w]; var b = [(function () {}), 1, 2]; var c = [(() => 1), 2, 3];",
    "var a = [(x || y) && z, 1, 2]; var b = [(function () {})(), 1, 2]; var c = [(x) => x, 1, 2];",
    "var a = [...x, y, z]; var b = [/a*/, 1, 2]; var c = [/[/]/, 1, 2];",
    "var a = [\"function(\", \"[\\n\", 1]; var f = function(){ return \"function() {}\"; };",
    "var a = [async function(){}, function*(){}, async function*(){}]; export default function(){}",
    "var o = { function() {}, async function() {}, get a() { return [1, 2, 3]; }, b: function(){} };",
    "class A { function() {} async m() { return [1, 2, 3]; } static s = function(){}; }",
    "var a = [[[{ a: 1,\n b: 2 }, 1, 2], 3, 4], 5, 6]; var b = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];",
    "var a = `${[1, 2, 3]} and ${function(){ return [4, 5, 6]; }}`;",
    "var a = [<a />, <b>x</b>, <c>\n  y\n</c>];",
    "var a = [function () {\n  return 1;\n}, 2, 3]; var b = [{ a: 1 }, { b: 2 }, { c: 3, d: 4 }];",
    "var a = [\"\u{e000}\", 1, 2]; var f = function () { var x = \"\u{e000}\"; return [x, 1, 2]; };",
    "if (a) x = [1, 2, 3]; else y = function(){ return [4, 5, 6]; };",
    "var a = [1, 2, 3], b = [[4, 5, 6], 7, 8]; f([1, 2, 3], [4, 5, 6], function () { return [7, 8, 9]; });",
    "for (var a = [1, 2, 3], i = 0; i < 3; i++) g(function(){ return [i, 2, 3]; });",
];

#[test]
fn edits_placed_from_the_printed_program_equal_the_reparse() {
    let inputs = PLACEMENT_CASES
        .iter()
        .enumerate()
        .map(|(index, source)| (format!("case {index}"), (*source).to_string()))
        .collect::<Vec<_>>();
    let tally = every_mode(&inputs);
    assert!(
        tally.placed > tally.reparsed,
        "{} {}",
        tally.placed,
        tally.reparsed
    );
    let guarded = vec![("guard".to_string(), GUARD_IN_AN_ELEMENT.to_string())];
    assert!(every_mode(&guarded).placed > 0);
}

fn differing_with(mutant: fn(bool), source: &str, options: &Options) -> Vec<String> {
    let pipeline = Pipeline::new(options.clone());
    mutant(true);
    let (_, witnessed) = recording(|| pipeline.run(source));
    mutant(false);
    witnessed.differing
}

#[test]
fn an_edit_placed_one_byte_off_turns_the_comparison_red() {
    let source = "var a = [1, 2, 3];";
    let differing = differing_with(misplace_an_edit_for_tests, source, &Options::default());
    assert!(!differing.is_empty(), "the misplaced edit went unseen");
    let (_, witnessed) = recording(|| Pipeline::new(Options::default()).run(source));
    assert!(witnessed.differing.is_empty() && witnessed.placed == 1);
}

#[test]
fn the_bundle_check_answers_alike_on_the_unpack_fixtures() {
    let mut tally = Tally::default();
    let fixtures = [
        js_files_in("fixtures/reference/unpack"),
        js_files_in("fixtures/unpack-gaps"),
    ]
    .concat();
    for (_, unsafe_transforms, extended) in MODES {
        witness(&mut tally, &fixtures, &mode(unsafe_transforms, extended));
    }
    assert!(tally.placed > 0);
    let bundle = fixtures
        .iter()
        .find(|(label, _)| label.contains("webpack"))
        .map(|(_, source)| source.clone())
        .unwrap_or_default();
    let differing = differing_with(flip_bundle_answer_for_tests, &bundle, &Options::default());
    assert!(!differing.is_empty(), "a flipped bundle answer went unseen");
}
