#![cfg(all(target_os = "linux", not(debug_assertions)))]

#[path = "depth_rows/channel.rs"]
mod channel;
#[path = "depth_rows/edge_cases.rs"]
mod edge_cases;
#[path = "depth_rows/harness.rs"]
mod harness;
#[path = "depth_rows/heap.rs"]
mod heap;
#[path = "depth_rows/memory.rs"]
mod memory;
#[path = "depth_rows/refusal.rs"]
mod refusal;
#[path = "depth_rows/supervisor.rs"]
mod supervisor;
#[path = "depth_rows/sweeps.rs"]
mod sweeps;

use harness::{Rows, Scratch};

fn refusal_rows(rows: &mut Rows, scratch: &Scratch) {
    refusal::print_refusals(rows, scratch);
    refusal::parentheses_and_nesting(rows, scratch);
    refusal::linear_chains_run_and_parse(rows, scratch);
    refusal::outputs_and_reports(rows, scratch);
    refusal::a_deep_input_among_fixtures(rows, scratch);
    refusal::a_refused_ladder_stays_silent(rows, scratch);
    refusal::many_deep_inputs_share_the_reserve(rows, scratch);
    refusal::unique_rename_cannot_deadlock(rows, scratch);
    refusal::refused_text_is_never_parsed(rows, scratch);
    refusal::the_cumulative_speculative_bound(rows, scratch);
    refusal::the_rung_does_not_depend_on_jobs(rows, scratch);
}

fn memory_rows(rows: &mut Rows, scratch: &Scratch) {
    memory::string_chains(rows, scratch);
    memory::a_deep_decoded_text_refuses_its_unit(rows, scratch);
    memory::one_heap_budget_ends_decoding_partway(rows, scratch);
    memory::two_units_share_one_budget(rows, scratch);
    memory::four_decoded_texts_share_the_reserve(rows, scratch);
    memory::the_scan_goal_matches_each_site(rows, scratch);
    memory::live_prefixes_bail_at_the_heap_budget(rows, scratch);
}

fn heap_rows(rows: &mut Rows, scratch: &Scratch) {
    heap::inlined_text_stays_inside_the_budget(rows, scratch);
    heap::objects_closures_and_scopes_are_charged(rows, scratch);
    heap::parsed_arrays_and_arguments_are_charged(rows, scratch);
    heap::small_strings_carry_their_header(rows, scratch);
    heap::shrinks_and_projections_are_credited(rows, scratch);
    heap::a_consequent_chain_lifts_in_one_visit(rows, scratch);
    heap::frames_and_native_loops_are_bounded(rows, scratch);
    heap::every_closure_call_checks_the_heap(rows, scratch);
}

fn supervisor_rows(rows: &mut Rows, scratch: &Scratch) {
    supervisor::a_stack_overflow_refuses_the_input(rows, scratch);
    supervisor::an_abort_and_a_memory_kill_refuse_the_input(rows, scratch);
    supervisor::under_four_jobs_only_the_dying_input_is_refused(rows, scratch);
    supervisor::stdin_input_dies(rows);
    supervisor::the_fallback_runs_in_process(rows);
    supervisor::normal_exits_are_not_crashes(rows, scratch);
    supervisor::unpack_and_emit_deaths(rows);
    channel::a_death_inside_an_output_frame(rows);
    channel::a_death_inside_a_record_refuses_its_input(rows, scratch);
    channel::the_worker_is_the_oom_killers_first_choice(rows, scratch);
    channel::a_stdin_writer_that_cannot_start_falls_back(rows);
}

#[test]
fn depth_rows() {
    let _serial = harness::one_row_set_at_a_time();
    let scratch = Scratch::new();
    let mut rows = Rows::new();
    refusal_rows(&mut rows, &scratch);
    memory_rows(&mut rows, &scratch);
    heap_rows(&mut rows, &scratch);
    supervisor_rows(&mut rows, &scratch);
    refusal::no_class_kills_the_process(&mut rows, &scratch);
    sweeps::no_class_kills_a_sixty_four_mib_rung(&mut rows, &scratch);
    sweeps::premise_and_print_estimate(&mut rows, &scratch);
    supervisor::the_supervised_run_equals_the_in_process_run(&mut rows, &scratch);
    rows.finish();
}

#[test]
fn edge_case_rows() {
    let _serial = harness::one_row_set_at_a_time();
    let scratch = Scratch::new();
    let mut rows = Rows::new();
    edge_cases::json_parse_probes_stay_inside_the_print_budget(&mut rows, &scratch);
    edge_cases::inlined_json_is_charged_to_the_print_room(&mut rows, &scratch);
    edge_cases::decoding_does_not_depend_on_jobs_or_co_inputs(&mut rows, &scratch);
    edge_cases::a_death_after_the_last_input_refuses_the_inputs_it_serves(&mut rows, &scratch);
    edge_cases::a_panic_before_the_parse_is_a_refusal(&mut rows, &scratch);
    rows.finish();
}
