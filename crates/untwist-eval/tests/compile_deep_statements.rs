mod common;
mod wide_shapes;

use std::ffi::OsString;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{compile_census, frozen, record_units, UnitRecord};
use untwist_testing::capped_child::{self, Cap};
use wide_shapes::{Shape, Source, SHAPES};

const PARSE_STACK: usize = 1 << 30;

fn compiled(source: String) -> (UnitRecord, u64) {
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, &source));
    assert!(parsed.diagnostics.is_empty());
    let program = &parsed.program;
    let run = || eval_script_in(program, Budget::new(50_000_000), |_, _| ());
    let mut compiled = Vec::new();
    let census = compile_census(|| compiled = record_units(run));
    frozen("compile_deep_statements", &source, &compiled);
    (compiled.remove(0), census.calls)
}

fn on_a_2_mib_thread(source: String, value: &'static str) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let (record, calls) = compiled(source);
            assert_eq!(record.values, [value], "{record:?}");
            assert_eq!(calls, 1);
        })
        .expect("a thread")
        .join()
        .expect("the row's thread finished, with the compiled tree dropped");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 30,000 nested for statements on a 2 MiB thread"
)]
fn a_30000_deep_for_nest_runs_and_drops_compiled_on_a_2_mib_thread() {
    let source = format!(
        "function f() {{ {}{{ return 1; }} }} f();",
        "for (;;) ".repeat(30_000)
    );
    on_a_2_mib_thread(source, "number:3ff0000000000000");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 10,000 nested switch statements on a 2 MiB thread"
)]
fn a_10000_deep_switch_nest_runs_and_drops_compiled_on_a_2_mib_thread() {
    let source = format!(
        "function f() {{ {}return 1;{} }} f();",
        "switch (1) { case 1: ".repeat(10_000),
        " }".repeat(10_000)
    );
    on_a_2_mib_thread(source, "number:3ff0000000000000");
}

const KINDS: usize = 2_000;

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: a nest of every ported statement kind on a 2 MiB thread"
)]
fn a_nest_of_every_ported_statement_kind_runs_and_drops_compiled_on_a_2_mib_thread() {
    let open = "if (1) { while (1) { for (;;) { switch (1) { case 1: try { { ";
    let close = " } } catch (e) { } } } } }";
    let source = format!(
        "function f() {{ {}return 'deep';{} }} f();",
        open.repeat(KINDS),
        close.repeat(KINDS)
    );
    on_a_2_mib_thread(source, "string:deep");
}

const RUNS: usize = 99_990;
const BAILS: usize = 150_000;
const NESTING: &str = "nesting depth cap exceeded";
const CHAIN: &str = "scope chain too deep";

fn top_level(shape: fn(usize) -> String, levels: usize) -> UnitRecord {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || compiled(format!("{} 'done';", shape(levels))).0)
        .expect("a thread")
        .join()
        .expect("the row's thread finished, with the compiled tree dropped")
}

fn deep_outcomes(shape: fn(usize) -> String, at_runs: Option<&str>) {
    let record = top_level(shape, RUNS);
    assert_eq!(record.bail.as_deref(), at_runs, "{record:?}");
    if at_runs.is_none() {
        assert_eq!(record.values, ["string:done"], "{record:?}");
    }
    let record = top_level(shape, BAILS);
    assert_eq!(record.bail.as_deref(), Some(NESTING), "{record:?}");
}

fn deep_row(shape: fn(usize) -> String) {
    deep_outcomes(shape, None);
}

fn nest(open: &str, levels: usize, close: &str) -> String {
    format!("{}{}", open.repeat(levels), close.repeat(levels))
}

fn lexical_blocks(levels: usize) -> String {
    nest("{ let a = 1; a; ", levels, "} ")
}

fn labels(levels: usize) -> String {
    let labels: String = (0..levels).map(|level| format!("l{level}: ")).collect();
    format!("{labels};")
}

fn try_finally(levels: usize) -> String {
    format!(
        "{}{}",
        "try { ".repeat(levels),
        "} finally { } ".repeat(levels)
    )
}

fn catches(levels: usize) -> String {
    nest("try { throw 0; } catch (e) { let c = e; ", levels, "} ")
}

fn switches(levels: usize) -> String {
    nest("switch (1) { case 1: let s = 1; ", levels, "} ")
}

fn do_whiles(levels: usize) -> String {
    format!("{};{}", "do ".repeat(levels), " while (0);".repeat(levels))
}

fn for_ofs(levels: usize) -> String {
    format!("{};", "for (const k of [1]) ".repeat(levels))
}

fn for_ins(levels: usize) -> String {
    format!("{};", "for (var k in {p: 1}) ".repeat(levels))
}

fn for_lets(levels: usize) -> String {
    format!("{};", "for (let i = 0; i < 1; i++) ".repeat(levels))
}

fn plain_blocks(levels: usize) -> String {
    nest("{ ", levels, "} ")
}

fn plain_catches(levels: usize) -> String {
    nest("try { throw 0; } catch { ", levels, "} ")
}

fn finally_bodies(levels: usize) -> String {
    nest("try { } finally { ", levels, "} ")
}

fn labelled_loops(levels: usize) -> String {
    let open: String = (0..levels)
        .map(|level| format!("l{level}: for (;;) {{ "))
        .collect();
    format!("{open}break l0; {}", "} ".repeat(levels))
}

fn for_in_lets(levels: usize) -> String {
    format!("{};", "for (let k in {p: 1}) ".repeat(levels))
}

fn for_of_vars(levels: usize) -> String {
    format!("{};", "for (var k of [1]) ".repeat(levels))
}

fn for_of_members(levels: usize) -> String {
    format!("var o = {{}}; {};", "for (o.k of [1]) ".repeat(levels))
}

fn plain_switches(levels: usize) -> String {
    nest("switch (1) { case 1: ", levels, "} ")
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_lexical_block_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(lexical_blocks);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_label_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(labels);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_try_finally_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(try_finally);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_catch_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(catches);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_switch_nest_with_lexicals_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(switches);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_do_while_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(do_whiles);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_for_of_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(for_ofs);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_for_in_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(for_ins);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_per_iteration_for_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(for_lets);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_plain_block_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(plain_blocks);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_catch_nest_without_lexicals_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(plain_catches);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_finally_body_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(finally_bodies);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_labelled_loop_nest_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_outcomes(labelled_loops, Some(NESTING));
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_for_in_let_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(for_in_lets);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_for_of_var_nest_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(for_of_vars);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_for_of_member_nest_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_outcomes(for_of_members, Some(CHAIN));
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150,000 levels on a 2 MiB thread"
)]
fn a_deep_switch_nest_without_lexicals_runs_bails_and_drops_compiled_on_a_2_mib_thread() {
    deep_row(plain_switches);
}

const VISITED: [usize; 2] = [12_000, 100_000];

fn both_tiers(key: &str, run: impl Fn()) -> Vec<UnitRecord> {
    let compiled = record_units(&run);
    frozen("compile_deep_statements tiers", key, &compiled);
    assert!(!compiled.is_empty());
    compiled
}

fn script_tiers(source: &str, decoding: bool) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, source));
    assert!(parsed.diagnostics.is_empty());
    let program = &parsed.program;
    both_tiers(&format!("script {decoding}\n{source}"), || {
        if decoding {
            eval_script_decoding(program, Budget::new(50_000_000), |_| ());
        } else {
            eval_script_in(program, Budget::new(50_000_000), |_, _| ());
        }
    })
}

fn session_tiers(source: &str) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, source));
    assert!(parsed.diagnostics.is_empty());
    let program = common::decoder_program(&parsed.program, "a", &["b"], None);
    let calls = common::calls_for(&parsed.program, 0);
    both_tiers(&format!("session\n{source}"), || {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let _ = evaluator.decode(&program, &calls);
    })
}

#[derive(Clone, Copy)]
enum Entry {
    Script,
    Decoding,
    Session,
}

fn visited(entry: Entry, shape: fn(usize) -> String, levels: usize) -> Vec<UnitRecord> {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let source = shape(levels);
            match entry {
                Entry::Script => script_tiers(&source, false),
                Entry::Decoding => script_tiers(&source, true),
                Entry::Session => session_tiers(&source),
            }
        })
        .expect("a thread")
        .join()
        .expect("the row's thread finished on its 2 MiB stack")
}

fn visited_row(entry: Entry, shape: fn(usize) -> String, value: &str) {
    let [runs, bails] = VISITED;
    for record in visited(entry, shape, runs) {
        assert_eq!(record.bail, None, "{record:?}");
        assert!(record.values.iter().all(|seen| seen == value), "{record:?}");
    }
    for record in visited(entry, shape, bails) {
        assert_eq!(record.bail.as_deref(), Some(NESTING), "{record:?}");
    }
}

fn negations(levels: usize) -> String {
    format!("{}1", "!".repeat(levels))
}

fn refused_row(entry: Entry, shape: fn(usize) -> String) {
    let refusals: Vec<Option<String>> = VISITED
        .into_iter()
        .flat_map(|levels| visited(entry, shape, levels))
        .map(|record| record.bail)
        .collect();
    assert!(refusals[0].is_some(), "{refusals:?}");
    assert!(
        refusals.iter().all(|bail| *bail == refusals[0]),
        "{refusals:?}"
    );
}

fn binding_pattern(levels: usize) -> String {
    format!(
        "var {} = 0;",
        nest("[", levels, "]").replacen("[]", "[a]", 1)
    )
}

fn target_pattern(levels: usize) -> String {
    format!(
        "var a; {} = 0;",
        nest("[", levels, "]").replacen("[]", "[a]", 1)
    )
}

fn function_binding_patterns(levels: usize) -> String {
    format!("(function(){{ {} }})()", binding_pattern(levels))
}

fn function_target_patterns(levels: usize) -> String {
    format!("(function(){{ {} }})()", target_pattern(levels))
}

fn decoder_binding_patterns(levels: usize) -> String {
    session_decoder(binding_pattern(levels))
}

fn decoder_target_patterns(levels: usize) -> String {
    session_decoder(target_pattern(levels))
}

fn functions(levels: usize) -> String {
    nest("(function(){ ", levels, "})(); ")
}

fn decoder_functions(levels: usize) -> String {
    session_decoder(functions(levels))
}

fn function_blocks(levels: usize) -> String {
    format!("(function(){{ {} }})()", nest("{ ", levels, "} "))
}

fn function_negations(levels: usize) -> String {
    format!("(function(){{ return {}; }})()", negations(levels))
}

fn decoding_blocks(levels: usize) -> String {
    format!("{} 'done';", nest("{ ", levels, "0; } "))
}

fn decoding_negations(levels: usize) -> String {
    format!("{} + 'done';", negations(levels))
}

fn session_decoder(body: String) -> String {
    format!(
        "{}\nfunction b(i) {{ {body} i = i - 0x10; var e = a(); return e[i]; }}\nb(0x10);\n",
        wide_shapes::ARRAY
    )
}

fn decoder_blocks(levels: usize) -> String {
    session_decoder(nest("{ ", levels, "0; } "))
}

fn decoder_negations(levels: usize) -> String {
    session_decoder(format!("{};", negations(levels)))
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 100,000 levels on a 2 MiB thread"
)]
fn a_function_body_nest_runs_or_bails_as_the_walker_through_the_arguments_scan() {
    visited_row(Entry::Script, function_blocks, "undefined");
    visited_row(Entry::Script, function_negations, "bool:true");
    refused_row(Entry::Script, function_binding_patterns);
    refused_row(Entry::Script, function_target_patterns);
    refused_row(Entry::Script, functions);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 100,000 levels on a 2 MiB thread"
)]
fn a_decoding_program_nest_runs_or_bails_as_the_walker_through_the_dead_zone_scan() {
    visited_row(Entry::Decoding, decoding_blocks, "string:done");
    visited_row(Entry::Decoding, decoding_negations, "string:truedone");
    refused_row(Entry::Decoding, binding_pattern);
    refused_row(Entry::Decoding, target_pattern);
    refused_row(Entry::Decoding, functions);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 100,000 levels on a 2 MiB thread"
)]
fn a_decoder_nest_runs_or_bails_as_the_walker_through_the_state_judge() {
    visited_row(Entry::Session, decoder_blocks, "string:4");
    visited_row(Entry::Session, decoder_negations, "string:4");
    refused_row(Entry::Session, decoder_binding_patterns);
    refused_row(Entry::Session, decoder_target_patterns);
    refused_row(Entry::Session, decoder_functions);
}

const WIDE_CHILD: &str = "UNTWIST_WIDE_SHAPE_CHILD";
const WIDE_CEILING: Cap = Cap {
    memory_max: "64M",
    timeout_s: 120,
};

fn decode_session(source: &str) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty());
    let program = common::decoder_program(&parsed.program, "a", &["b"], Some(0));
    let calls = common::calls_for(&parsed.program, 0);
    let mut evaluator = InterpEvaluator::new(Budget::default());
    let _ = evaluator.decode(&program, &calls);
}

fn run_wide(source: &Source) {
    match source {
        Source::Script(text) => {
            let alloc = Allocator::default();
            let parsed = parse_program(&alloc, text);
            assert!(parsed.diagnostics.is_empty());
            eval_script_in(&parsed.program, Budget::new(50_000_000), |_, _| ());
        }
        Source::Session(text) => decode_session(text),
    }
}

#[test]
fn wide_shape_child() {
    let Ok(request) = std::env::var(WIDE_CHILD) else {
        return;
    };
    let name = request.as_str();
    let width = SHAPES
        .iter()
        .find(|shape| shape.name == name)
        .map_or(0, |shape| shape.width);
    let source = wide_shapes::source(name, width).expect("a known wide shape");
    let mut records = Vec::new();
    let census = compile_census(|| records = record_units(|| run_wide(&source)));
    for record in &records {
        println!(
            "WIDE {:?} {:?} steps {}",
            record.values, record.bail, record.steps_used
        );
    }
    println!("CALLS {}", census.calls);
}

struct Child {
    records: String,
    census: String,
    peak_kb: u64,
}

fn wide_child(name: &str) -> Result<Child, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let args: Vec<OsString> = ["wide_shape_child", "--exact", "--nocapture"]
        .into_iter()
        .map(OsString::from)
        .collect();
    let env = [(OsString::from(WIDE_CHILD), OsString::from(name))];
    let outcome = capped_child::run(exe.as_os_str(), &args, &env, None, WIDE_CEILING);
    if outcome.exit_code != Some(0) {
        return Err(format!("{name}: {outcome:?}"));
    }
    let stdout = String::from_utf8_lossy(&outcome.stdout);
    let lines = |prefix: &str| -> Vec<&str> {
        stdout
            .lines()
            .filter(|line| line.starts_with(prefix))
            .collect()
    };
    Ok(Child {
        records: lines("WIDE ").join("\n"),
        census: lines("CALLS ").join(""),
        peak_kb: outcome.peak_rss_kb,
    })
}

fn wide_row(shape: &Shape) -> Result<(), String> {
    let compiled = wide_child(shape.name)?;
    frozen(
        "wide shapes",
        &format!("{} x {}", shape.name, shape.width),
        std::slice::from_ref(&compiled.records),
    );
    println!(
        "wide {} x {}: compiled {} kB; {}",
        shape.name, shape.width, compiled.peak_kb, compiled.census
    );
    let calls = format!("CALLS {}", shape.calls);
    let checks = [
        (!compiled.records.is_empty(), "a recorded unit"),
        (
            compiled
                .records
                .lines()
                .all(|record| record.contains(" None steps ")),
            "no bail",
        ),
        (
            compiled.census == calls,
            "exactly the shape's compiled calls",
        ),
    ];
    match checks.iter().find(|(held, _)| !held) {
        Some((_, what)) => Err(format!(
            "{}: {what}: compiled {} kB {} {}",
            shape.name, compiled.peak_kb, compiled.records, compiled.census
        )),
        None => Ok(()),
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: every wide shape in capped children"
)]
fn every_wide_shape_runs_compiled_as_frozen_under_a_64_mb_ceiling() {
    if std::env::var_os(WIDE_CHILD).is_some() {
        return;
    }
    let failures: Vec<String> = SHAPES
        .iter()
        .filter_map(|shape| wide_row(shape).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
