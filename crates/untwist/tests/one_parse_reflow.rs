#![cfg(debug_assertions)]

use untwist::{Options, Pipeline, RenameMode};
use untwist_core::reflow::{recording, reflow_recorded, Witnessed};
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
    assert!(every_mode(&high).compared > 0);
}

#[test]
fn corpus_npm_reflows_as_the_round_loop_does() {
    assert!(every_mode(&js_files_in("fixtures/corpus-npm")).compared > 0);
}

#[test]
fn the_behaviour_corpus_reflows_as_the_round_loop_does() {
    assert!(every_mode(&js_files_in("fixtures/behaviour")).compared > 0);
}

#[test]
fn the_diff_sets_reflow_as_the_round_loop_does() {
    let mut compared = 0;
    for dir in DIFF_SETS {
        compared += every_mode(&js_files_in(dir)).compared;
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
