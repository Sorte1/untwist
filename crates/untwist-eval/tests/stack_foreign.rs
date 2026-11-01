mod boundary;
mod common;

use std::process::Command;

use boundary::{
    assert_at_the_grow_boundary, assert_the_answer_is_stackers, descend_to_the_boundary,
};
use untwist_eval::stack;

const SCENARIO_VARIABLE: &str = "UNTWIST_FOREIGN_STACK_SCENARIO";
const OUTCOME_PREFIX: &str = "foreign-stack-outcome:";
const THREAD_STACK_BYTES: usize = 512 * 1024;
const FOREIGN_SEGMENT_BYTES: usize = 1024 * 1024;
const LARGE_FOREIGN_SEGMENT_BYTES: usize = 8 * 1024 * 1024;
const SMALL_FOREIGN_SEGMENT_BYTES: usize = 256 * 1024;
const DEEP: &str =
    "var a=JSON.parse('['.repeat(20000)+']'.repeat(20000)); JSON.stringify(a).length";
const DEEP_RESULT: &str = "Ok(\"number:40000\")";

fn on_a_small_thread(scenario: fn() -> String) -> String {
    std::thread::Builder::new()
        .stack_size(THREAD_STACK_BYTES)
        .spawn(scenario)
        .expect("the small-stack thread spawns")
        .join()
        .expect("the small-stack thread completed")
}

fn in_a_child_process(test_name: &str, scenario: fn() -> String) -> String {
    if std::env::var_os(SCENARIO_VARIABLE).is_some() {
        let outcome = on_a_small_thread(scenario);
        println!("{OUTCOME_PREFIX}{outcome}");
        return outcome;
    }
    let output = Command::new(std::env::current_exe().expect("the test binary has a path"))
        .args([test_name, "--exact", "--test-threads=1", "--nocapture"])
        .env(SCENARIO_VARIABLE, test_name)
        .output()
        .expect("the child test process runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
        .lines()
        .find_map(|line| line.split_once(OUTCOME_PREFIX))
        .map(|(_, outcome)| outcome.to_string())
        .unwrap_or_else(|| panic!("the child reported no outcome: {stdout}"))
}

fn outer_after_a_foreign_segment() -> String {
    let first = common::eval("1+1");
    let inside = stacker::grow(FOREIGN_SEGMENT_BYTES, || common::eval("2+2"));
    let deep = common::eval(DEEP);
    format!("{first:?} {inside:?} {deep:?}")
}

#[test]
fn a_deep_evaluation_after_a_foreign_segment_returns_runs_where_stacker_says() {
    assert_eq!(
        in_a_child_process(
            "a_deep_evaluation_after_a_foreign_segment_returns_runs_where_stacker_says",
            outer_after_a_foreign_segment,
        ),
        format!("Ok(\"number:2\") Ok(\"number:4\") {DEEP_RESULT}")
    );
}

fn a_segment_mapped_where_a_freed_one_was() -> String {
    let first = stacker::grow(LARGE_FOREIGN_SEGMENT_BYTES, || common::eval("1+1"));
    let reused = stacker::grow(SMALL_FOREIGN_SEGMENT_BYTES, || common::eval(DEEP));
    format!("{first:?} {reused:?}")
}

#[test]
fn a_deep_evaluation_on_a_segment_mapped_after_a_freed_one_runs_where_stacker_says() {
    assert_eq!(
        in_a_child_process(
            "a_deep_evaluation_on_a_segment_mapped_after_a_freed_one_runs_where_stacker_says",
            a_segment_mapped_where_a_freed_one_was,
        ),
        format!("Ok(\"number:2\") {DEEP_RESULT}")
    );
}

fn ours_after_a_foreign_segment_inside_it() -> String {
    stack::grow(FOREIGN_SEGMENT_BYTES, || {
        let first = common::eval("1+1");
        let inside = stacker::grow(SMALL_FOREIGN_SEGMENT_BYTES, || common::eval("2+2"));
        let deep = common::eval(DEEP);
        format!("{first:?} {inside:?} {deep:?}")
    })
}

#[test]
fn a_deep_evaluation_on_our_segment_after_a_foreign_one_inside_it_runs_where_stacker_says() {
    assert_eq!(
        in_a_child_process(
            "a_deep_evaluation_on_our_segment_after_a_foreign_one_inside_it_runs_where_stacker_says",
            ours_after_a_foreign_segment_inside_it,
        ),
        format!("Ok(\"number:2\") Ok(\"number:4\") {DEEP_RESULT}")
    );
}

fn foreign_after_ours_inside_it() -> String {
    stacker::grow(FOREIGN_SEGMENT_BYTES, || {
        let first = common::eval("1+1");
        let inside = stack::grow(SMALL_FOREIGN_SEGMENT_BYTES, || common::eval("2+2"));
        let deep = common::eval(DEEP);
        format!("{first:?} {inside:?} {deep:?}")
    })
}

#[test]
fn a_deep_evaluation_on_a_foreign_segment_after_ours_inside_it_runs_where_stacker_says() {
    assert_eq!(
        in_a_child_process(
            "a_deep_evaluation_on_a_foreign_segment_after_ours_inside_it_runs_where_stacker_says",
            foreign_after_ours_inside_it,
        ),
        format!("Ok(\"number:2\") Ok(\"number:4\") {DEEP_RESULT}")
    );
}

fn descend_after(prepare: impl FnOnce()) {
    prepare();
    assert_the_answer_is_stackers("after the foreign segments of the preparation");
    assert_at_the_grow_boundary(&descend_to_the_boundary());
}

fn descend_across_foreign_segments() {
    descend_after(|| {
        stacker::grow(FOREIGN_SEGMENT_BYTES, descend_to_the_boundary);
    });
    stacker::grow(LARGE_FOREIGN_SEGMENT_BYTES, || {
        descend_after(|| assert!(stack::has_room()))
    });
    stacker::grow(SMALL_FOREIGN_SEGMENT_BYTES, || descend_after(|| ()));
    stack::grow(FOREIGN_SEGMENT_BYTES, || {
        descend_after(|| {
            stacker::grow(SMALL_FOREIGN_SEGMENT_BYTES, descend_to_the_boundary);
        })
    });
}

#[test]
fn every_answer_after_foreign_segments_is_stackers_at_every_level() {
    let outcome = std::thread::Builder::new()
        .stack_size(THREAD_STACK_BYTES)
        .spawn(descend_across_foreign_segments)
        .expect("the small-stack thread spawns")
        .join();
    assert!(outcome.is_ok());
}
