use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::clock::Instant;
use untwist_core::pass::{
    replayed_report, scoping_after_traversal, IterationTiming, PassCtor, PassGroup, RunReport,
};
use untwist_core::semantic::{Semantics, Shape};

use crate::{
    block_statements, computed_properties, for_to_while, function_to_arrow, infinity,
    invert_boolean_logic, json_parse, let_to_const, logical_to_if, merge_else_if, merge_strings,
    number_expressions, remove_double_not, sequence, split_for_loop_vars,
    split_variable_declarations, string_literal_in_template, ternary_to_if,
    truncate_number_literal, typeof_undefined, unary_expressions, unminify_booleans, untranspile,
    unwrap_comma_call, var_to_let, void_to_undefined, yoda,
};

pub fn unminify_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        block_statements::boxed,
        computed_properties::boxed,
        for_to_while::boxed,
        infinity::boxed,
        invert_boolean_logic::boxed,
        json_parse::boxed,
        logical_to_if::boxed,
        merge_else_if::boxed,
        merge_strings::boxed,
        number_expressions::boxed,
        remove_double_not::boxed,
        sequence::boxed,
        split_for_loop_vars::boxed,
        split_variable_declarations::boxed,
        string_literal_in_template::boxed,
        ternary_to_if::boxed,
        truncate_number_literal::boxed,
        typeof_undefined::boxed,
        unary_expressions::boxed,
        unminify_booleans::boxed,
        void_to_undefined::boxed,
        yoda::boxed,
        unwrap_comma_call::boxed,
    ]
}

pub fn lexical_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![
        let_to_const::boxed,
        var_to_let::boxed,
        function_to_arrow::boxed,
        untranspile::merge_member_declarations::boxed,
    ]
}

pub fn run_last_traversal<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    group: &mut PassGroup<'a>,
    keep_scoping: bool,
) -> (Option<Semantics>, RunReport) {
    group.begin_iteration();
    group.opts().scoping_patched.set(sem.patched);
    group.opts().scoping_canonical.set(sem.canonical);
    let (stats, before) = (sem.stats, Shape::of(&sem.scoping));
    let started = Instant::now();
    let traversed = oxc_traverse::traverse_mut(group, alloc, program, sem.scoping, ());
    let mut timing = IterationTiming::traversed(started.elapsed());
    let refreshed = Instant::now();
    let kept = keep_scoping
        .then(|| scoping_after_traversal(program, traversed, stats, before, group))
        .inspect(|_| timing.set_rebuild(refreshed.elapsed()));
    let notes = group.take_notes();
    let mut report = replayed_report(group, notes);
    report.total_changes = group.total_changes();
    report.iteration_elapsed = vec![timing];
    (kept, report)
}
