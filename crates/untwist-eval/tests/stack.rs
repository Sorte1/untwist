mod boundary;
mod common;

use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use boundary::{
    assert_at_the_grow_boundary, assert_the_answer_is_stackers, descend_through_segments,
    descend_to_the_boundary, FRAME_PAD_BYTES,
};
use oxc_ast::ast::{Program, Statement};
use untwist_eval::budget::{Budget, CALL_STACK_GROWTH};
use untwist_eval::interp::Interp;
use untwist_eval::stack;
use untwist_eval::value::Value;

const WORKER_STACK_BYTES: usize = 512 * 1024;
const SEEDED_WORKER_STACK_BYTES: usize = 2 * 1024 * 1024;
const SMALL_THREAD_STACK_BYTES: usize = 128 * 1024;
const SEGMENTS_THE_THROW_CROSSES: usize = 2;
const THROW_ACROSS_SEGMENTS: &str = "var caught = 0;
function d(n) { if (n === 0) { throw 7; } return d(n - 1) + 1; }
try { d(900); } catch (e) { caught = e; }
caught";

#[test]
fn a_rayon_worker_reaches_the_grow_boundary_where_stacker_does() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(WORKER_STACK_BYTES)
        .build()
        .expect("a one-thread pool builds");
    let reached = pool.install(|| descend_through_segments(0, 1));
    assert_eq!(reached.len(), 2, "{reached:?}");
    reached.iter().for_each(assert_at_the_grow_boundary);
}

#[test]
fn a_rayon_worker_on_its_seeded_stack_reaches_the_grow_boundary_where_stacker_does() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(SEEDED_WORKER_STACK_BYTES)
        .build()
        .expect("a one-thread pool builds");
    let reached = pool.install(|| stack::on_this_threads_stack(|| descend_through_segments(0, 1)));
    assert_eq!(reached.len(), 2, "{reached:?}");
    reached.iter().for_each(assert_at_the_grow_boundary);
}

#[test]
fn segments_grown_inside_grown_segments_each_reach_their_own_boundary() {
    let reached = descend_through_segments(0, 3);
    assert_eq!(reached.len(), 4, "{reached:?}");
    reached.iter().for_each(assert_at_the_grow_boundary);
    let segment_levels = CALL_STACK_GROWTH / FRAME_PAD_BYTES;
    assert!(
        reached
            .iter()
            .skip(1)
            .all(|boundary| boundary.levels < segment_levels),
        "{reached:?}"
    );
    assert_the_answer_is_stackers("after three nested segments returned");
}

#[test]
fn a_grown_segment_entered_from_shallow_code_reaches_its_boundary_and_the_caller_keeps_its_own() {
    assert!(stack::has_room());
    let inner = stack::grow(CALL_STACK_GROWTH, descend_to_the_boundary);
    assert_at_the_grow_boundary(&inner);
    assert_the_answer_is_stackers("after a segment entered from shallow code returned");
    assert_at_the_grow_boundary(&descend_to_the_boundary());
}

#[test]
fn a_panic_out_of_a_grown_segment_leaves_the_callers_answer_stackers() {
    assert!(stack::has_room());
    let caught = catch_unwind(AssertUnwindSafe(|| {
        stack::grow(CALL_STACK_GROWTH, || {
            panic_any(descend_to_the_boundary().levels)
        })
    }));
    let levels = caught
        .err()
        .and_then(|payload| payload.downcast::<usize>().ok());
    assert!(levels.is_some_and(|levels| *levels > 0));
    assert_the_answer_is_stackers("after a panic left a grown segment");
    assert_at_the_grow_boundary(&descend_to_the_boundary());
}

fn run_counting(program: &Program<'_>) -> (Option<f64>, u32, u32) {
    let mut interp = Interp::new(program.source_text, Budget::default());
    let env = interp.global_env();
    let mut last = None;
    if interp.hoist(&program.body, &env).is_ok() {
        for statement in &program.body {
            last = match statement {
                Statement::ExpressionStatement(node) => {
                    match interp.eval_root_expression(&node.expression) {
                        Ok(Value::Number(number)) => Some(number),
                        _ => None,
                    }
                }
                other => interp.run_root_statement(other).ok().and(None),
            };
        }
    }
    (
        last,
        interp.budget.call_depth(),
        interp.budget.nesting_depth(),
    )
}

#[test]
fn a_throw_across_grown_segments_leaves_the_call_and_nesting_counters_balanced() {
    let outcome = std::thread::Builder::new()
        .stack_size(SMALL_THREAD_STACK_BYTES)
        .spawn(|| {
            let before = stack::segments_entered_on_this_thread_for_tests();
            let outcome = common::with_program(THROW_ACROSS_SEGMENTS, run_counting);
            (
                outcome,
                stack::segments_entered_on_this_thread_for_tests() - before,
            )
        })
        .expect("the small-stack thread spawns")
        .join()
        .expect("the small-stack thread completed");
    assert_eq!(outcome.0, (Some(7.0), 0, 0));
    assert!(outcome.1 >= SEGMENTS_THE_THROW_CROSSES, "{outcome:?}");
}

fn rust_files_under(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| {
            if path.is_dir() {
                rust_files_under(&path)
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                vec![path]
            } else {
                Vec::new()
            }
        })
        .collect()
}

#[test]
fn every_stack_switch_in_the_workspace_goes_through_the_stack_module() {
    let switches = [
        concat!("stacker", "::grow"),
        concat!("stacker", "::maybe_grow"),
        concat!("psm", "::on_stack"),
        concat!("psm", "::replace_stack"),
    ];
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files: Vec<PathBuf> = ["crates", "xtask"]
        .iter()
        .flat_map(|root| rust_files_under(&workspace.join(root)))
        .collect();
    assert!(files.len() > 100, "the scan found the workspace's sources");
    let offenders: Vec<&PathBuf> = files
        .iter()
        .filter(|path| {
            !path.ends_with("untwist-eval/src/stack.rs")
                && !path.ends_with("untwist-eval/tests/stack_foreign.rs")
        })
        .filter(|path| {
            std::fs::read_to_string(path)
                .is_ok_and(|text| switches.iter().any(|switch| text.contains(switch)))
        })
        .collect();
    assert!(offenders.is_empty(), "{offenders:?}");
}
