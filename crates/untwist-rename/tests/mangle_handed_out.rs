use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::collect::collect;
use untwist_rename::generate::{title_case, uid_candidate, uid_candidates, UID_CANDIDATES};
use untwist_rename::mangle::{decide, decide_by_rescan_for_tests};
use untwist_rename::oracle::{NameOracle, OuterRule};
use untwist_rename::reserved::{Excluded, PSEUDO_GLOBALS, RESERVED_WORDS};
use untwist_testing::{js_files_in, real_world_fixture, Allocator};

const CORPORA: [&str; 4] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/corpus-npm",
];
const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";

fn both_takes_agree(name: &str, source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let mut aborted = false;
    for extended in [true, false] {
        let skipping = decide(&collected, &sem.scoping, extended);
        let rescanning = decide_by_rescan_for_tests(&collected, &sem.scoping, extended);
        assert_eq!(skipping, rescanning, "{name} extended={extended}");
        aborted |= skipping
            .notes
            .iter()
            .any(|note| note.starts_with("ran out of names"));
    }
    aborted
}

fn same_base_strings(bases: usize, per_base: usize) -> String {
    LETTERS
        .chars()
        .take(bases)
        .flat_map(|letter| {
            (0..per_base).map(move |index| format!("var x{letter}{index} = 'q{letter}';\n"))
        })
        .collect()
}

fn six_bases(per_base: usize) -> String {
    (0..per_base)
        .map(|index| {
            format!(
                "function g{index}(a{index}) {{ try {{}} catch (c{index}) {{}} }}\n\
                 class K{index} {{}}\nvar o{index} = {{}};\nvar r{index} = [];\n"
            )
        })
        .collect()
}

fn names_skipped_by_one_binding_and_taken_by_its_owner() -> String {
    (0..40)
        .map(|round| {
            let owned = 4 * round + 3;
            format!(
                "function a{round}() {{}}\nfunction b{round}() {{}}\nfunction c{round}() {{}}\n\
                 function f{owned}() {{}}\nvar k{round} = {{}};\n"
            )
        })
        .collect()
}

fn names_taken_elsewhere() -> String {
    let mut source = String::from("f3; f5; p4; vO2;\nf6: for (;;) break f6;\n");
    for index in 0..60 {
        source.push_str(&format!(
            "var m{index} = require('f{index}');\nfunction g{index}(a, b) {{ return a + b; }}\n\
             var o{index} = {{}};\n"
        ));
    }
    source
}

fn objects_then_strings(per_base: usize) -> String {
    let objects = (0..per_base).map(|index| format!("var o{index} = {{}};\n"));
    let strings = (0..per_base).map(|index| format!("var s{index} = 'qa';\n"));
    objects.chain(strings).collect()
}

fn every_letter(line: impl Fn(char) -> String) -> String {
    LETTERS.chars().map(line).collect()
}

fn a_global_in_front_of_every_base(per_base: usize) -> String {
    every_letter(|letter| format!("vLSQ{letter};\n")) + &same_base_strings(26, per_base)
}

fn a_label_in_front_of_every_base(per_base: usize) -> String {
    every_letter(|letter| format!("vLSQ{letter}: for (;;) break vLSQ{letter};\n"))
        + &same_base_strings(26, per_base)
}

fn a_kept_binding_in_front_of_every_base(per_base: usize) -> String {
    every_letter(|letter| format!("var vLSQ{letter} = 1;\n")) + &same_base_strings(26, per_base)
}

fn owners_of_early_candidates_declared_late(per_base: usize) -> String {
    let late = every_letter(|letter| {
        format!("var vLSQ{letter} = 'q{letter}';\nvar vLSQ{letter}7 = 'q{letter}';\n")
    });
    every_letter(|letter| format!("var vLSQ{letter}5 = 1;\n"))
        + &same_base_strings(26, per_base)
        + &late
}

fn an_own_name_inside_the_skipped_run(per_base: usize) -> String {
    same_base_strings(26, per_base)
        + "var vLSQa3 = 'qa'; var f = 1; function f2() {} function g(f) {} f5;\n\
           e: for (;;) break e; try {} catch (e) {} try {} catch (e2) {}\n"
        + &same_base_strings(26, per_base)
}

fn functions_params_and_catches_behind_a_global_and_a_label(count: usize) -> String {
    let mut source = String::from("f; f3: for (;;) break f3; p2;\n");
    for index in 0..count {
        source.push_str(&format!(
            "function f{index}(p{index}, e) {{ var f = 1; try {{}} catch (e{index}) {{}} }}\n\
             function h{index}(p) {{}}\n"
        ));
    }
    source
}

const STALL_BINDINGS: usize = 300;

fn oracle<'s>(
    scoping: &'s oxc_semantic::Scoping,
    collected: &'s untwist_rename::collect::Collected,
    keeps_other_names: bool,
) -> NameOracle<'s> {
    if !keeps_other_names {
        return NameOracle::new(scoping, collected);
    }
    let mut kept = Excluded::from_scoping(scoping);
    for symbol in scoping.symbol_ids() {
        let name = scoping.symbol_name(symbol);
        if !name.starts_with('x') {
            kept.insert(name);
        }
    }
    NameOracle::keeping(scoping, collected, kept)
}

fn reserved_words_reachable_as_a_base() -> Vec<(&'static str, String)> {
    let reachable: Vec<(&'static str, String)> = RESERVED_WORDS
        .iter()
        .chain(PSEUDO_GLOBALS.iter())
        .filter_map(|&word| {
            let read = format!("\u{e9}{}", word.strip_prefix('v')?);
            (format!("v{}", title_case(&read)) == word).then_some((word, read))
        })
        .collect();
    assert!(!reachable.is_empty());
    reachable
}

fn reads_of(read: &str, count: usize) -> String {
    (0..count)
        .map(|index| format!("var x{index} = {read};\n"))
        .collect()
}

fn probes_to_name_every_string_behind(prefix: &str, keeps_other_names: bool) -> usize {
    let source = format!("{prefix}\n{}", same_base_strings(1, STALL_BINDINGS));
    probes_to_name_every_binding(&source, "vLSQa", keeps_other_names)
}

fn probes_to_name_every_binding(source: &str, base: &str, keeps_other_names: bool) -> usize {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let mut skipping = oracle(&sem.scoping, &collected, keeps_other_names);
    let mut rescanning = oracle(&sem.scoping, &collected, keeps_other_names);
    let mut probes = 0;
    for symbol in sem.scoping.symbol_ids() {
        if !sem.scoping.symbol_name(symbol).starts_with('x') {
            continue;
        }
        let (taken, examined) = skipping.take_uid_counting_probes_for_tests(symbol, base);
        let rescanned = rescanning.take_uid_by_rescan_for_tests(symbol, base);
        assert_eq!(
            taken, rescanned,
            "{source:.60} keeps_other_names={keeps_other_names}"
        );
        probes += examined;
    }
    probes
}

#[test]
fn a_candidate_refused_to_every_binding_is_probed_once_per_base_not_once_per_binding() {
    let prefixes = [
        "",
        "vLSQa;",
        "vLSQa: for (;;) break vLSQa;",
        "var vLSQa = 1;",
        "vLSQa; vLSQa2: for (;;) break vLSQa2; var vLSQa3 = 1;",
    ];
    let stalled: Vec<String> = prefixes
        .iter()
        .flat_map(|prefix| [true, false].map(|keeps| (*prefix, keeps)))
        .map(|(prefix, keeps)| {
            (
                prefix,
                keeps,
                probes_to_name_every_string_behind(prefix, keeps),
            )
        })
        .filter(|&(_, _, probes)| probes > 2 * STALL_BINDINGS)
        .map(|(prefix, keeps, probes)| format!("{prefix:?} keeps_other_names={keeps}: {probes}"))
        .collect();
    assert!(
        stalled.is_empty(),
        "probes for {STALL_BINDINGS} bindings: {stalled:#?}"
    );
}

#[test]
fn a_single_candidate_matches_the_candidate_sequence_at_every_position() {
    for base in ["f", "vLSQa", "p"] {
        let sequence: Vec<String> = uid_candidates(base).collect();
        assert_eq!(sequence.len(), UID_CANDIDATES);
        for (position, expected) in sequence.iter().enumerate() {
            assert_eq!(uid_candidate(base, position).as_ref(), Some(expected));
        }
        assert_eq!(uid_candidate(base, UID_CANDIDATES), None);
    }
}

#[test]
fn same_base_bindings_under_the_cap_get_the_same_plan_from_both_takes() {
    assert!(!both_takes_agree("strings", &same_base_strings(26, 40)));
    assert!(!both_takes_agree("six bases", &six_bases(150)));
}

#[test]
fn candidates_skipped_by_one_binding_or_taken_by_other_names_get_the_same_plan_from_both_takes() {
    assert!(!both_takes_agree(
        "owned",
        &names_skipped_by_one_binding_and_taken_by_its_owner()
    ));
    assert!(!both_takes_agree("taken", &names_taken_elsewhere()));
}

#[test]
fn programs_at_and_past_the_cap_get_the_same_plan_from_both_takes() {
    assert!(!both_takes_agree("at the cap", &objects_then_strings(999)));
    assert!(both_takes_agree(
        "past the cap",
        &objects_then_strings(1_000)
    ));
}

#[test]
fn the_corpora_get_the_same_plan_from_both_takes() {
    let mut compared = 0;
    for corpus in CORPORA {
        for (name, source) in js_files_in(corpus) {
            both_takes_agree(&name, &source);
            compared += 1;
        }
    }
    assert!(compared >= 50, "{compared} files compared");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "25,740 bindings; run with cargo test --release -p untwist-rename --test mangle_handed_out"
)]
fn twenty_six_bases_of_990_bindings_get_the_same_plan_from_both_takes() {
    assert!(!both_takes_agree("26 x 990", &same_base_strings(26, 990)));
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-rename --test mangle_handed_out"
)]
fn game_js_gets_the_same_plan_from_both_takes() {
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    assert!(both_takes_agree("game.js", &source));
}

#[test]
fn candidates_refused_to_every_binding_get_the_same_plan_from_both_takes() {
    assert!(!both_takes_agree("plain", &same_base_strings(26, 120)));
    assert!(!both_takes_agree(
        "global",
        &a_global_in_front_of_every_base(120)
    ));
    assert!(!both_takes_agree(
        "label",
        &a_label_in_front_of_every_base(120)
    ));
    assert!(!both_takes_agree(
        "kept",
        &a_kept_binding_in_front_of_every_base(120)
    ));
}

#[test]
fn a_binding_that_owns_a_refused_candidate_gets_the_same_plan_from_both_takes() {
    assert!(!both_takes_agree(
        "owner declared late",
        &owners_of_early_candidates_declared_late(120)
    ));
    assert!(!both_takes_agree(
        "own name in the skipped run",
        &an_own_name_inside_the_skipped_run(60)
    ));
    assert!(!both_takes_agree(
        "mixed",
        &functions_params_and_catches_behind_a_global_and_a_label(300)
    ));
}

#[test]
fn a_base_that_is_a_reserved_word_is_probed_once_per_binding() {
    let source_of = |read: &str| reads_of(read, STALL_BINDINGS);
    let stalled: Vec<String> = reserved_words_reachable_as_a_base()
        .into_iter()
        .flat_map(|(word, read)| [true, false].map(|keeps| (word, read.clone(), keeps)))
        .map(|(word, read, keeps)| {
            let probes = probes_to_name_every_binding(&source_of(&read), word, keeps);
            (word, keeps, probes)
        })
        .filter(|&(_, _, probes)| probes > 2 * STALL_BINDINGS)
        .map(|(word, keeps, probes)| format!("{word} keeps_other_names={keeps}: {probes}"))
        .collect();
    assert!(
        stalled.is_empty(),
        "probes for {STALL_BINDINGS} bindings: {stalled:#?}"
    );
}

#[test]
fn a_base_that_is_a_reserved_word_gets_the_same_plan_from_both_takes() {
    for (word, read) in reserved_words_reachable_as_a_base() {
        let owners: String = (2..9)
            .map(|suffix| format!("var {word}{suffix} = 1;\n"))
            .collect();
        let global = format!("{word}4;\n{word}6: for (;;) break {word}6;\n");
        assert!(!both_takes_agree(word, &reads_of(&read, 400)));
        assert!(!both_takes_agree(
            word,
            &format!("{owners}{}", reads_of(&read, 400))
        ));
        assert!(!both_takes_agree(
            word,
            &format!("{global}{}{owners}", reads_of(&read, 400))
        ));
    }
    let mixed: String = (0..300)
        .flat_map(|index| {
            reserved_words_reachable_as_a_base()
                .into_iter()
                .map(move |(word, read)| format!("var y{word}{index} = {read};\n"))
        })
        .collect();
    assert!(!both_takes_agree("mixed", &mixed));
}
