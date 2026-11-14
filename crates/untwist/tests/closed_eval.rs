use std::path::{Path, PathBuf};

use untwist::{Options, Output, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, patched_after_rotator, Mode};

const DECODER_CALL: &str = "console.log(b(";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Safe,
    Unsafe,
    Conservative,
}

fn options(lane: Lane) -> Options {
    Options {
        unsafe_transforms: lane == Lane::Unsafe,
        extended: lane != Lane::Conservative,
        unpack: false,
        jsx: false,
        disabled_passes: vec!["inline-layers"],
        ..Options::default()
    }
}

fn run(source: &str, lane: Lane) -> Output {
    Pipeline::new(options(lane)).run(source)
}

fn decoded(out: &Output) -> bool {
    !out.code().contains(DECODER_CALL)
}

const CLOSED: [(&str, &str); 4] = [
    (
        "a closed read of require in a function",
        "function load() { try { return typeof eval('require;'); } catch (e) { return 'threw ' + e.constructor.name; } }\nconsole.log(load());",
    ),
    (
        "a closed read of a global at the top level",
        "console.log(eval('typeof Math.max'));",
    ),
    (
        "a closed payload with its own function and catch binding",
        "console.log(eval('(function (q) { try { return q + 1; } catch (z) { return z; } })(1)'));",
    ),
    (
        "a closed call of a global",
        "eval('console.log(\"from eval\")');",
    ),
];

const OPEN: [(&str, &str, &str); 8] = [
    (
        "a",
        "an argument that is not a literal",
        "var s = 'typeof ' + String.fromCharCode(98);\nfunction f() { var t = b(1); return eval(s) + t.length; }\nconsole.log(f());",
    ),
    (
        "c",
        "a payload var that redeclares a local",
        "function f() { var t = b(1); eval('var t = \"shadow\"'); return t; }\nconsole.log(f());",
    ),
    (
        "c",
        "a sloppy block function that redeclares a local",
        "function g() { var t = b(1); eval('{ function t() {} }'); return typeof t; }\nconsole.log(g());",
    ),
    (
        "d",
        "a nested eval that reads the decoder",
        "function f() { var t = b(1); return eval(\"eval('typeof ' + String.fromCharCode(98))\") + t.length; }\nconsole.log(f());",
    ),
    (
        "d",
        "an arguments read that sees a dead parameter write",
        "function k(c) { c = b(1); return eval('arguments[0]'); }\nconsole.log(k('arg'));",
    ),
    (
        "d",
        "a this read",
        "console.log(eval('typeof this'));",
    ),
    (
        "e",
        "a payload that reads the decoder's name",
        "function f() { var t = b(1); return eval('typeof b') + t.length; }\nconsole.log(f());",
    ),
    (
        "e",
        "a payload that writes a local",
        "function f() { var t = b(1); eval('t = \"written\"'); return t; }\nconsole.log(f());",
    ),
];

const SPELLS_THE_DECODER: &str = "a payload that reads the decoder's name";

fn program(patch: &str) -> String {
    patched_after_rotator("true", patch)
}

#[test]
fn a_closed_literal_eval_lets_the_string_array_decode() {
    for (label, patch) in CLOSED {
        let source = program(patch);
        let out = run(&source, Lane::Safe);
        assert!(decoded(&out), "{label}: {}", out.code());
        assert!(
            !out.notes
                .iter()
                .any(|note| note.message.contains("direct eval")),
            "{label}: {:?}",
            out.notes
        );
    }
}

#[test]
fn an_open_eval_still_observes_the_scope_by_default() {
    for (condition, label, patch) in OPEN {
        let out = run(&program(patch), Lane::Safe);
        assert!(!decoded(&out), "({condition}) {label}: {}", out.code());
    }
}

#[test]
fn under_unsafe_every_eval_is_assumed_not_to_observe_but_a_with_still_is() {
    for (condition, label, patch) in OPEN
        .iter()
        .filter(|(_, label, _)| *label != SPELLS_THE_DECODER)
    {
        let out = run(&program(patch), Lane::Unsafe);
        assert!(decoded(&out), "({condition}) {label}: {}", out.code());
    }
    let with = program("var o = { q: 1 };\nwith (o) { console.log(q); }");
    assert!(!decoded(&run(&with, Lane::Unsafe)), "with under --unsafe");
    assert!(!decoded(&run(&with, Lane::Safe)), "with by default");
}

#[test]
fn conservative_keeps_every_eval_an_observer() {
    for (label, patch) in CLOSED {
        let out = run(&program(patch), Lane::Conservative);
        assert!(!decoded(&out), "{label}: {}", out.code());
    }
}

fn lane_runner(lane: Lane) -> impl Fn(&str) -> String {
    move |source: &str| run(source, lane).code().to_string()
}

const UNSAFE_WITNESSES: [&str; 3] = [
    "an argument that is not a literal",
    "a nested eval that reads the decoder",
    "an arguments read that sees a dead parameter write",
];

#[test]
fn node_agrees_by_default_and_unsafe_diverges_only_on_its_a_l1_witnesses() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs: Vec<(String, String)> = CLOSED
        .iter()
        .map(|(label, patch)| (label.to_string(), program(patch)))
        .collect();
    programs.extend(
        OPEN.iter()
            .map(|(_, label, patch)| (label.to_string(), program(patch))),
    );
    let safe = lane_runner(Lane::Safe);
    let conservative = lane_runner(Lane::Conservative);
    let modes: [Mode<'_>; 2] = [("safe", &safe), ("conservative", &conservative)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let unsafe_lane = lane_runner(Lane::Unsafe);
    let modes: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let mut diverged: Vec<String> = mismatches(&node, &programs, &modes)
        .into_iter()
        .filter_map(|row| row.split(" (unsafe)").next().map(str::to_string))
        .collect();
    diverged.sort();
    let mut want: Vec<String> = UNSAFE_WITNESSES.iter().map(|w| w.to_string()).collect();
    want.sort();
    assert_eq!(diverged, want);
}

fn game_old() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game_old.js")
}

fn undecoded_references(out: &Output) -> usize {
    out.notes
        .iter()
        .filter_map(|note| {
            note.message
                .strip_prefix("decoder machinery kept: ")?
                .split_once(" reference(s)")?
                .0
                .parse::<usize>()
                .ok()
        })
        .sum()
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "game_old.js; run with cargo test --release -p untwist --test closed_eval"
)]
fn game_old_drops_the_eval_hazard_and_decodes_every_string_under_unsafe() {
    let Ok(source) = std::fs::read_to_string(game_old()) else {
        eprintln!("game_old.js is absent; skipped");
        return;
    };
    let safe = run(&source, Lane::Safe);
    assert!(
        !safe
            .notes
            .iter()
            .any(|note| note.message.contains("direct eval")),
        "{:?}",
        safe.notes
    );
    let unsafe_lane = run(&source, Lane::Unsafe);
    let left = undecoded_references(&unsafe_lane);
    eprintln!("game_old unsafe: {left} decoder references left");
    assert_eq!(left, 0, "{:?}", unsafe_lane.notes);
}

const ASSUMED_EVAL_NOTE: &str = "direct eval may reach the decoder";

fn notes_the_assumption(out: &Output) -> bool {
    out.notes
        .iter()
        .any(|note| note.message.contains(ASSUMED_EVAL_NOTE))
}

#[test]
fn under_unsafe_a_closed_eval_needs_no_assumption_and_notes_none() {
    for (label, patch) in CLOSED {
        let out = run(&program(patch), Lane::Unsafe);
        assert!(decoded(&out), "{label}: {}", out.code());
        assert!(!notes_the_assumption(&out), "{label}: {:?}", out.notes);
    }
}

#[test]
fn under_unsafe_an_open_eval_still_notes_the_assumption() {
    for (condition, label, patch) in OPEN
        .iter()
        .filter(|(_, label, _)| *label != SPELLS_THE_DECODER)
    {
        let out = run(&program(patch), Lane::Unsafe);
        assert!(
            notes_the_assumption(&out),
            "({condition}) {label}: {:?}",
            out.notes
        );
    }
}
