use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::collect::collect;
use untwist_rename::mangle::{decide, decide_every_site_for_tests};
use untwist_rename::oracle::OuterRule;
use untwist_testing::{js_files_in, real_world_fixture, Allocator};

const CORPORA: [&str; 4] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/corpus-npm",
];
const PAST_THE_CAP: usize = 1_100;
const CAP: usize = 999;

fn abort_notes(name: &str, source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let collected = collect(&parsed.program, &sem.scoping, OuterRule::ProgramWide);
    let mut notes = Vec::new();
    for extended in [true, false] {
        let fast = decide(&collected, &sem.scoping, extended);
        let every = decide_every_site_for_tests(&collected, &sem.scoping, extended);
        assert_eq!(fast, every, "{name} extended={extended}");
        notes.extend(
            fast.notes
                .into_iter()
                .filter(|note| note.starts_with("ran out of names")),
        );
    }
    notes
}

fn both_paths_agree(name: &str, source: &str) -> bool {
    !abort_notes(name, source).is_empty()
}

fn ran_out_for(name: &str) -> Vec<String> {
    vec![format!("ran out of names for {name}; leaving the program unrenamed"); 2]
}

fn functions_with_one_param(count: usize, last_extra: &str) -> String {
    (0..count)
        .map(|index| {
            let extra = if index + 1 == count { last_extra } else { "" };
            format!("function g{index}(a{extra}) {{ return a; }}\n")
        })
        .collect()
}

fn read_ahead_past_the_cap() -> String {
    let objects: String = (0..CAP)
        .map(|index| format!("var a{index} = {{}};\n"))
        .collect();
    format!("{objects}var q = r;\nvar r = {{}};\n")
}

fn exhausted_early() -> String {
    (0..PAST_THE_CAP)
        .map(|index| {
            let next = index + 1;
            format!("function g{index}(a) {{ var b = g{next}; return a + b; }}\n")
        })
        .collect()
}

fn exhausted_late() -> String {
    let functions =
        (0..PAST_THE_CAP / 2).map(|index| format!("function h{index}(a) {{ return a; }}\n"));
    let objects = (0..PAST_THE_CAP).map(|index| format!("var o{index} = {{}};\n"));
    functions.chain(objects).collect()
}

#[test]
fn a_program_that_exhausts_early_or_late_gets_the_same_plan_from_both_paths() {
    assert!(both_paths_agree("early", &exhausted_early()));
    assert!(both_paths_agree("late", &exhausted_late()));
}

#[test]
fn a_program_exactly_at_the_cap_is_renamed_by_both_paths() {
    let notes = abort_notes("at the cap", &functions_with_one_param(CAP, ""));
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_program_that_runs_out_only_at_its_last_site_names_that_site() {
    let source = functions_with_one_param(CAP, ", zlast");
    assert_eq!(abort_notes("last site", &source), ran_out_for("zlast"));
}

#[test]
fn a_program_that_runs_out_inside_the_read_ahead_names_the_binding_read() {
    let source = read_ahead_past_the_cap();
    assert_eq!(abort_notes("read ahead", &source), ran_out_for("r"));
}

#[test]
fn the_corpora_get_the_same_plan_from_both_paths() {
    let mut compared = 0;
    for corpus in CORPORA {
        for (name, source) in js_files_in(corpus) {
            both_paths_agree(&name, &source);
            compared += 1;
        }
    }
    assert!(compared >= 50, "{compared} files compared");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-rename --test mangle_exhaustion"
)]
fn game_js_gets_the_same_plan_from_both_paths() {
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    assert!(both_paths_agree("game.js", &source));
}
