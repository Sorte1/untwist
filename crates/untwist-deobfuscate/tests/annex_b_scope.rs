use oxc_allocator::Allocator;
use untwist::{Options, Pipeline};
use untwist_core::parse::parse_program;
use untwist_core::semantic::{build_scoping, is_hoisted_block_function};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/annex_b_scope_rows.txt");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect<'r> {
    Inlined(&'r str),
    RefusedHere,
    KeptElsewhere,
}

struct Row<'r> {
    label: &'r str,
    expect: Expect<'r>,
    program: &'r str,
}

fn expect_of(field: &str) -> Expect<'_> {
    match field {
        "-" => Expect::RefusedHere,
        "=" => Expect::KeptElsewhere,
        absent => Expect::Inlined(absent),
    }
}

fn rows() -> Vec<Row<'static>> {
    ROWS.lines()
        .filter_map(|line| {
            let (label, rest) = line.split_once('|')?;
            let (expect, program) = rest.split_once('|')?;
            Some(Row {
                label,
                expect: expect_of(expect),
                program,
            })
        })
        .collect()
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn reads_a_hoisted_block_function(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let scoping = build_scoping(&parsed.program).scoping;
    let reaches = scoping
        .symbol_ids()
        .filter(|symbol| is_hoisted_block_function(&scoping, *symbol))
        .any(|symbol| {
            scoping
                .get_resolved_reference_ids(symbol)
                .iter()
                .any(|reference| scoping.get_reference(*reference).is_read())
        });
    reaches
}

#[test]
fn a_block_function_refuses_a_binding_only_where_its_annex_b_binding_reaches_a_read() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .iter()
        .map(|row| (row.label.to_string(), row.program.to_string()))
        .collect();
    assert!(programs.len() >= 41, "found {} rows", programs.len());
    let safe = Options::default();
    let risky = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&risky, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (wrong, witnesses) = node_rows::unsafe_dynamic_code_witnesses(wrong, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(witnesses.len(), 1, "{witnesses:?}");
}

#[test]
fn a_binding_no_annex_b_binding_reaches_is_still_inlined_in_both_modes() {
    let safe = Options::default();
    let risky = unsafe_options();
    let mut kept = Vec::new();
    for row in rows() {
        let Expect::Inlined(absent) = row.expect else {
            continue;
        };
        for (mode, options) in [("default", &safe), ("unsafe", &risky)] {
            let code = transformed(options, row.program);
            if code.contains(absent) {
                kept.push(format!("{} ({mode}) kept {absent}:\n{code}", row.label));
            }
        }
    }
    assert!(kept.is_empty(), "{}", kept.join("\n"));
}

#[test]
fn every_row_that_keeps_its_read_names_the_refusal_or_reads_the_hoisted_function() {
    let safe = Options::default();
    let mut silent = Vec::new();
    for row in rows()
        .iter()
        .filter(|row| row.expect == Expect::RefusedHere)
    {
        let output = Pipeline::new(safe.clone()).run(row.program);
        let noted = output.notes.iter().any(|note| {
            note.message.contains("Annex B.3.3") || note.message.contains("not unique")
        });
        if !noted && !reads_a_hoisted_block_function(row.program) {
            silent.push(row.label);
        }
    }
    assert!(silent.is_empty(), "no refusal note for {silent:?}");
}
