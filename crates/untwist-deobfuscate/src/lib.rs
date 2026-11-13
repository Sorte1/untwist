mod alias_forms;
pub mod array_rotator;
pub mod bindings;
pub mod bridge;
pub mod call_checks;
pub mod calls;
mod constant_test;
pub mod control_flow_object;
pub mod control_flow_switch;
pub mod dead_assignment;
pub mod dead_code;
pub mod debug_protection;
pub mod decoded;
pub mod decoded_strings;
pub mod decoder;
pub mod directives;
pub mod driver;
pub mod evaluate_globals;
pub mod fold_chains;
pub mod fused;
pub mod init_order;
pub mod layers;
pub mod legacy_reach;
pub mod merge_object_assignments;
pub mod mutants;
mod note_set;
pub mod notes;
pub mod object_props;
mod primitive_names;
pub mod probe;
pub mod proof;
mod protection_shape;
pub mod pure_calls;
#[cfg(feature = "refusal-spans")]
pub mod refusal_spans;
pub mod removal;
pub mod self_defending;
pub mod shell_scope;
pub mod stamp;
pub mod string_array;
pub mod units;
pub mod unstamp;
pub mod value_flow;
pub mod var_transformation;
pub mod vm_interpreter;
mod wrapper_args;
pub mod wrapper_calls;
mod wrapper_chain;
mod wrapper_consts;
mod wrapper_effects;
mod wrapper_immediate;
mod wrapper_sites;
pub mod wrappers;
mod write_only;

pub use driver::{
    cleanup_ctors, cleanup_fixpoint_ctors, deobfuscate_ctors, drop_cleared_observation,
    protections_ctors, run_cleanup, run_deobfuscate, run_deobfuscate_prepared,
    run_deobfuscate_sharing, run_final_machinery, run_machinery_sweep, run_protections,
    CLEANUP_FIXPOINT_ORDER, CLEANUP_ORDER, DEOBFUSCATE_MAX_ROUNDS, DEOBFUSCATE_ORDER,
    INLINE_DECODED, NO_STRING_ARRAY_NOTE, OBJECT_PROPS, PROTECTIONS_ORDER,
    STRING_ARRAY_MAX_ITERATIONS,
};

pub fn reset_run_state() {
    calls::forget_call_flags();
    write_only::forget_write_only_flags();
    decoded::forget_alias_refusals();
    string_array::legacy::forget_lifts();
    legacy_reach::forget();
    init_order::clear_kept_order();
}
