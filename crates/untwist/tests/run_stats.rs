use untwist::{Options, Pipeline, RenameMode};

const SPLIT_STRINGS_BEHIND_A_DECODER: &str = "\
function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0x0) + b(0x1));";

const A_FREE_NAME_IN_THE_WAY: &str = "\
var a = 1;
function f(x, y) { return a + x + y; }
console.log(f(2, input));";

fn options() -> Options {
    Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

#[test]
fn a_decoded_string_array_records_the_steps_units_and_call_depth() {
    let out = Pipeline::new(options()).run(SPLIT_STRINGS_BEHIND_A_DECODER);
    assert!(out.code().contains("\"Hello\""), "{}", out.code());
    let stats = &out.stats;
    assert!(stats.decode.units >= 1, "{stats:?}");
    assert!(stats.decode.steps > 0, "{stats:?}");
    assert!(
        stats.decode.peak_unit_steps <= stats.decode.steps,
        "{stats:?}"
    );
    assert!(stats.decode.peak_call_depth >= 1, "{stats:?}");
}

#[test]
fn a_program_without_a_decoder_records_nothing() {
    let out = Pipeline::new(options()).run("var a = !0;");
    assert_eq!(out.stats.decode, untwist::DecodeStats::default());
    assert_eq!(out.stats.collisions, untwist::CollisionStats::default());
}

#[test]
fn simple_renaming_counts_why_it_skipped_names() {
    let out = Pipeline::new(Options {
        rename: RenameMode::Simple,
        ..options()
    })
    .run(A_FREE_NAME_IN_THE_WAY);
    let collisions = &out.stats.collisions;
    assert!(collisions.total() > 0, "{collisions:?} {}", out.code());
    assert!(
        collisions.free_reference + collisions.bound_in_scope > 0,
        "{collisions:?}"
    );
}

#[test]
fn the_stats_serialise_with_the_names_both_reports_use() {
    let out = Pipeline::new(options()).run(SPLIT_STRINGS_BEHIND_A_DECODER);
    let value = serde_json::to_value(out.stats).unwrap();
    for key in ["steps", "peakUnitSteps", "units", "peakCallDepth"] {
        assert!(value["decode"].get(key).is_some(), "{value}");
    }
    for key in ["boundInScope", "freeReference", "label", "excluded"] {
        assert!(value["collisions"].get(key).is_some(), "{value}");
    }
}
