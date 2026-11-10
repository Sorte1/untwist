use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::{mismatches, node_or_skip, patched_after_named_rotator, Mode};

fn options(unsafe_transforms: bool, conservative: bool) -> Options {
    Options {
        unsafe_transforms,
        extended: !conservative,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn run(source: &str, unsafe_transforms: bool, conservative: bool) -> Output {
    Pipeline::new(options(unsafe_transforms, conservative)).run(source)
}

fn sum_key_write(suffix: &str) -> String {
    patched_after_named_rotator(
        suffix,
        "true",
        "var g = this; function put(k, v) { g[k + 1] = v; } put(String.fromCharCode(98, 81), function () { return 'patched'; });",
    )
}

fn calls_left(out: &Output, decoder: &str) -> bool {
    out.code().contains(&format!("{decoder}(0)"))
}

fn messages(out: &Output) -> Vec<&str> {
    out.notes.iter().map(|note| note.message.as_str()).collect()
}

#[test]
fn a_sum_key_that_cannot_end_a_watched_name_is_not_a_write_to_the_decoder() {
    let out = run(&sum_key_write("Q"), false, false);
    let seen = messages(&out);
    assert!(
        !seen
            .iter()
            .any(|m| m.contains(notes::STRING_ARRAY_GLOBAL_REACH)),
        "{seen:#?}"
    );
}

#[test]
fn a_sum_key_on_the_global_object_refuses_as_a_possible_built_in_write() {
    let out = run(&sum_key_write("Q"), false, false);
    assert!(calls_left(&out, "bQ"), "{}", out.code());
    let seen = messages(&out);
    assert!(
        seen.iter()
            .any(|m| m.contains(notes::BUILTIN_ALIAS_PATCHED)),
        "{seen:#?}"
    );
}

#[test]
fn a_sum_key_that_may_end_a_watched_name_refuses() {
    let out = run(&sum_key_write("Q1"), false, false);
    assert!(calls_left(&out, "bQ1"), "{}", out.code());
    let seen = messages(&out);
    assert!(
        seen.iter()
            .any(|m| m.contains(notes::STRING_ARRAY_GLOBAL_REACH)),
        "{seen:#?}"
    );
}

#[test]
fn node_agrees_on_sum_keys_and_the_mutant_diverges_when_decoded() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = vec![
        ("sum key misses".to_string(), sum_key_write("Q")),
        ("sum key may hit".to_string(), sum_key_write("Q1")),
    ];
    let safe = |source: &str| run(source, false, false).code().to_string();
    let conservative = |source: &str| run(source, false, true).code().to_string();
    let modes: [Mode<'_>; 2] = [("safe", &safe), ("conservative", &conservative)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let decoded = |source: &str| run(source, true, false).code().to_string();
    let modes: [Mode<'_>; 1] = [("decoded", &decoded)];
    assert_eq!(mismatches(&node, &programs[1..], &modes).len(), 1);
}
