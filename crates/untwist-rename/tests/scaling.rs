mod common;

use std::time::{Duration, Instant};

use common::render;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::PrintStyle;
use untwist_core::semantic::build_scoping;
use untwist_rename::collisions::{self, Collisions};
use untwist_rename::usage::collect_naming_usage;
use untwist_rename::{RenameConfig, RenameMode};

const REFUSALS_PER_BINDING: u64 = 4;

fn refusals(source: &str, mode: RenameMode, rename_all: bool) -> u64 {
    collisions::take();
    let config = RenameConfig {
        mode,
        rename_all,
        extended: true,
    };
    render(source, config, PrintStyle::Compact, true);
    let Collisions {
        bound_in_scope,
        free_reference,
        label,
        excluded,
    } = collisions::take();
    [bound_in_scope, free_reference, label, excluded]
        .into_iter()
        .map(u64::from)
        .sum()
}

fn globals_covering_the_sequence(count: usize) -> String {
    let globals: Vec<String> = (0..count)
        .map(|index| format!("g{index} = {index}"))
        .collect();
    let locals: Vec<String> = (0..count)
        .map(|index| format!("l{index} = {index}"))
        .collect();
    let reads: Vec<String> = (0..count)
        .map(|index| format!("g{index}, l{index}"))
        .collect();
    format!(
        "var {};\nfunction f() {{ var {}; return [{}]; }}\n",
        globals.join(", "),
        locals.join(", "),
        reads.join(", ")
    )
}

fn shorthand_chain(count: usize) -> String {
    let bindings: Vec<String> = (0..count)
        .map(|index| format!("key{index} = o[{index}]"))
        .collect();
    let literals: Vec<String> = (0..count)
        .map(|index| format!("{{ key{}: key{index} }}", index + 1))
        .collect();
    format!(
        "function f(o) {{ var {}; return [{}]; }}\n",
        bindings.join(", "),
        literals.join(", ")
    )
}

#[test]
fn an_exhausted_sequence_window_is_not_probed_again_for_every_binding() {
    let count = 4_300;
    let refused = refusals(
        &globals_covering_the_sequence(count),
        RenameMode::Simple,
        true,
    );
    let bound = REFUSALS_PER_BINDING * 2 * count as u64 + 2 * 4_096;
    assert!(
        refused < bound,
        "{count} locals behind {count} free globals cost {refused} refusals, over {bound}"
    );
}

#[test]
fn a_chain_of_shorthand_hints_resolves_in_linear_refusals() {
    let count = 2_000;
    let refused = refusals(&shorthand_chain(count), RenameMode::None, false);
    let bound = REFUSALS_PER_BINDING * count as u64;
    assert!(
        refused < bound,
        "a {count}-link chain cost {refused} refusals, over {bound}"
    );
}

#[test]
fn the_chain_is_still_fully_restored() {
    let (output, _) = render(
        &shorthand_chain(50),
        RenameConfig {
            mode: RenameMode::None,
            rename_all: false,
            extended: true,
        },
        PrintStyle::Compact,
        true,
    );
    for index in 1..=50 {
        assert!(
            output.contains(&format!("{{key{index}}}")),
            "{index}: {output}"
        );
    }
}

fn fastest(mut work: impl FnMut()) -> Duration {
    (0..3)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
fn object_literal_keys_are_looked_up_in_constant_time() {
    let count = 20_000;
    let bindings: Vec<String> = (0..count)
        .map(|index| format!("v{index} = {index}"))
        .collect();
    let keys: Vec<String> = (0..count)
        .map(|index| format!("k{index}: v{index}"))
        .collect();
    let unhinted: Vec<String> = (0..count)
        .map(|index| format!("w{index} = {index}"))
        .collect();
    let source = format!(
        "var {};\nvar {};\nuse({{ {} }});\n",
        bindings.join(", "),
        unhinted.join(", "),
        keys.join(", ")
    );
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let sem = build_scoping(&parsed.program);
    let walk = fastest(|| {
        collect_naming_usage(&parsed.program, &sem.scoping);
    });
    let usage = collect_naming_usage(&parsed.program, &sem.scoping);
    let symbols: Vec<_> = sem.scoping.symbol_ids().collect();
    let mut found = 0;
    let lookups = fastest(|| {
        found = symbols
            .iter()
            .filter(|&&symbol| usage.object_literal_key(symbol).is_some())
            .count();
    });
    assert_eq!(found, count);
    assert!(
        lookups < walk * 2,
        "{} lookups took {lookups:?}, the walk that built them {walk:?}",
        symbols.len()
    );
}

#[test]
fn an_exhausted_unique_counter_is_not_probed_again_for_every_binding() {
    let count = 2_000;
    let globals: Vec<String> = (1..=4_096).map(|index| format!("num{index}")).collect();
    let locals: Vec<String> = (0..count)
        .map(|index| format!("_0x{index:x} = {index}"))
        .collect();
    let reads: Vec<String> = (0..count).map(|index| format!("_0x{index:x}")).collect();
    let source = format!(
        "typeof [{}];\nfunction f() {{ var {}; return [{}]; }}\n",
        globals.join(", "),
        locals.join(", "),
        reads.join(", ")
    );
    let refused = refusals(&source, RenameMode::Unique, true);
    let bound = REFUSALS_PER_BINDING * count as u64 + 2 * 4_096;
    assert!(
        refused < bound,
        "{count} numbers behind 4096 taken names cost {refused} refusals, over {bound}"
    );
}
