use std::sync::atomic::{AtomicBool, Ordering};

static WRAPPER_GUARDS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_wrapper_guards_for_tests(on: bool) {
    WRAPPER_GUARDS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn wrapper_guards_dropped() -> bool {
    WRAPPER_GUARDS_OFF.load(Ordering::Relaxed)
}

static ROTATOR_PLACEMENT_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_rotator_placement_guards_for_tests(on: bool) {
    ROTATOR_PLACEMENT_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn rotator_placement_dropped() -> bool {
    ROTATOR_PLACEMENT_OFF.load(Ordering::Relaxed)
}

static INERT_PREFIX_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_inert_prefix_guards_for_tests(on: bool) {
    INERT_PREFIX_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn inert_prefix_dropped() -> bool {
    INERT_PREFIX_OFF.load(Ordering::Relaxed)
}

static DIRECT_ARRAY_USES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_direct_array_uses_for_tests(on: bool) {
    DIRECT_ARRAY_USES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn direct_array_uses_trusted() -> bool {
    DIRECT_ARRAY_USES_TRUSTED.load(Ordering::Relaxed)
}

static BUILTIN_IMPORTS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_builtin_import_guards_for_tests(on: bool) {
    BUILTIN_IMPORTS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn builtin_imports_dropped() -> bool {
    BUILTIN_IMPORTS_OFF.load(Ordering::Relaxed)
}

static OBJECT_VALUES_UNCHECKED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn accept_any_object_value_before_the_rotator_for_tests(on: bool) {
    OBJECT_VALUES_UNCHECKED.store(on, Ordering::SeqCst);
}

pub(crate) fn object_values_unchecked() -> bool {
    OBJECT_VALUES_UNCHECKED.load(Ordering::Relaxed)
}

static DECODER_NAME_WATCH_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_decoder_name_watch_for_tests(on: bool) {
    DECODER_NAME_WATCH_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn decoder_name_watch_dropped() -> bool {
    DECODER_NAME_WATCH_OFF.load(Ordering::Relaxed)
}

static EXTERNAL_WRITE_ORDER_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_external_write_order_for_tests(on: bool) {
    EXTERNAL_WRITE_ORDER_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn external_write_order_trusted() -> bool {
    EXTERNAL_WRITE_ORDER_TRUSTED.load(Ordering::Relaxed)
}

static INERT_STATEMENTS_UNCHECKED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn read_every_statement_as_unable_to_call_for_tests(on: bool) {
    INERT_STATEMENTS_UNCHECKED.store(on, Ordering::SeqCst);
}

pub(crate) fn inert_statements_unchecked() -> bool {
    INERT_STATEMENTS_UNCHECKED.load(Ordering::Relaxed)
}

static GLOBAL_BINDING_WRITES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_global_binding_writes_for_tests(on: bool) {
    GLOBAL_BINDING_WRITES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn global_binding_writes_trusted() -> bool {
    GLOBAL_BINDING_WRITES_TRUSTED.load(Ordering::Relaxed)
}

static STRAIGHT_LINE_GUARDS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_straight_line_guards_for_tests(on: bool) {
    STRAIGHT_LINE_GUARDS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn straight_line_guards_dropped() -> bool {
    STRAIGHT_LINE_GUARDS_OFF.load(Ordering::Relaxed)
}

static LEGACY_ARRAY_GUARDS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_legacy_array_guards_for_tests(on: bool) {
    LEGACY_ARRAY_GUARDS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn legacy_array_guards_dropped() -> bool {
    LEGACY_ARRAY_GUARDS_OFF.load(Ordering::Relaxed)
}

static MINIFIED_ROTATORS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_minified_rotators_for_tests(on: bool) {
    MINIFIED_ROTATORS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn minified_rotators_trusted() -> bool {
    MINIFIED_ROTATORS_TRUSTED.load(Ordering::Relaxed)
}

static DECLARED_MACHINERY_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_declared_machinery_for_tests(on: bool) {
    DECLARED_MACHINERY_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn declared_machinery_trusted() -> bool {
    DECLARED_MACHINERY_TRUSTED.load(Ordering::Relaxed)
}

static UNSAFE_NAMES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_unsafe_machinery_names_for_tests(on: bool) {
    UNSAFE_NAMES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn unsafe_names_trusted() -> bool {
    UNSAFE_NAMES_TRUSTED.load(Ordering::Relaxed)
}

static EARLY_READ_GUARDS_OFF: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_early_read_guards_for_tests(on: bool) {
    EARLY_READ_GUARDS_OFF.store(on, Ordering::SeqCst);
}

pub(crate) fn early_read_guards_dropped() -> bool {
    EARLY_READ_GUARDS_OFF.load(Ordering::Relaxed)
}

static NESTED_ROTATOR_BUILTINS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_nested_rotator_builtins_for_tests(on: bool) {
    NESTED_ROTATOR_BUILTINS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn nested_rotator_builtins_trusted() -> bool {
    NESTED_ROTATOR_BUILTINS_TRUSTED.load(Ordering::Relaxed)
}

static LEGACY_ROTATION_PATCHES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_legacy_rotation_patches_for_tests(on: bool) {
    LEGACY_ROTATION_PATCHES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn legacy_rotation_patches_trusted() -> bool {
    LEGACY_ROTATION_PATCHES_TRUSTED.load(Ordering::Relaxed)
}

static INLINED_ROTATOR_VARS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_inlined_rotator_vars_for_tests(on: bool) {
    INLINED_ROTATOR_VARS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn inlined_rotator_vars_trusted() -> bool {
    INLINED_ROTATOR_VARS_TRUSTED.load(Ordering::Relaxed)
}

static DEAD_MEMBER_RESULTS_FOLDED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn fold_dead_member_results_in_callees_for_tests(on: bool) {
    DEAD_MEMBER_RESULTS_FOLDED.store(on, Ordering::SeqCst);
}

pub(crate) fn dead_member_callees_folded() -> bool {
    DEAD_MEMBER_RESULTS_FOLDED.load(Ordering::Relaxed)
}

static WRITE_ONLY_ORDER_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_write_only_order_for_tests(on: bool) {
    WRITE_ONLY_ORDER_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn write_only_order_trusted() -> bool {
    WRITE_ONLY_ORDER_TRUSTED.load(Ordering::Relaxed)
}

static DECLARATOR_WRITES_IGNORED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn ignore_declarator_writes_for_tests(on: bool) {
    DECLARATOR_WRITES_IGNORED.store(on, Ordering::SeqCst);
}

pub(crate) fn declarator_writes_ignored() -> bool {
    DECLARATOR_WRITES_IGNORED.load(Ordering::Relaxed)
}

static NESTED_LITERAL_WRITES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_nested_literal_writes_for_tests(on: bool) {
    NESTED_LITERAL_WRITES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn nested_literal_writes_trusted() -> bool {
    NESTED_LITERAL_WRITES_TRUSTED.load(Ordering::Relaxed)
}

static SETTLED_RIGHTS_DROPPED_ANYWHERE: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn drop_any_settled_right_of_if_tests_for_tests(on: bool) {
    SETTLED_RIGHTS_DROPPED_ANYWHERE.store(on, Ordering::SeqCst);
}

pub(crate) fn settled_rights_dropped_anywhere() -> bool {
    SETTLED_RIGHTS_DROPPED_ANYWHERE.load(Ordering::Relaxed)
}

static WRAPPER_CONSTANTS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_wrapper_constants_for_tests(on: bool) {
    WRAPPER_CONSTANTS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn wrapper_constants_trusted() -> bool {
    WRAPPER_CONSTANTS_TRUSTED.load(Ordering::Relaxed)
}

static SINGLE_USE_INDEX_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_single_use_index_for_tests(on: bool) {
    SINGLE_USE_INDEX_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn single_use_index_trusted() -> bool {
    SINGLE_USE_INDEX_TRUSTED.load(Ordering::Relaxed)
}

static ALIAS_ROOT_NAMES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_alias_root_names_for_tests(on: bool) {
    ALIAS_ROOT_NAMES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn alias_root_names_trusted() -> bool {
    ALIAS_ROOT_NAMES_TRUSTED.load(Ordering::Relaxed)
}

static INERT_STATEMENTS_MOVED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn move_inert_statements_into_rotators_for_tests(on: bool) {
    INERT_STATEMENTS_MOVED.store(on, Ordering::SeqCst);
}

pub(crate) fn inert_statements_moved() -> bool {
    INERT_STATEMENTS_MOVED.load(Ordering::Relaxed)
}

static BUILTIN_ALIASES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_builtin_aliases_for_tests(on: bool) {
    BUILTIN_ALIASES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn builtin_aliases_trusted() -> bool {
    BUILTIN_ALIASES_TRUSTED.load(Ordering::Relaxed)
}

static FUNCTION_METHOD_WRITES_IGNORED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn ignore_function_method_writes_for_tests(on: bool) {
    FUNCTION_METHOD_WRITES_IGNORED.store(on, Ordering::SeqCst);
}

pub(crate) fn function_method_writes_ignored() -> bool {
    FUNCTION_METHOD_WRITES_IGNORED.load(Ordering::Relaxed)
}

static INLINE_RETURNERS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_inline_returners_for_tests(on: bool) {
    INLINE_RETURNERS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn inline_returners_trusted() -> bool {
    INLINE_RETURNERS_TRUSTED.load(Ordering::Relaxed)
}

static COOKIE_BLOCK_VARS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_cookie_block_vars_for_tests(on: bool) {
    COOKIE_BLOCK_VARS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn cookie_block_vars_trusted() -> bool {
    COOKIE_BLOCK_VARS_TRUSTED.load(Ordering::Relaxed)
}

static CALLEE_OBJECTS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_callee_objects_for_tests(on: bool) {
    CALLEE_OBJECTS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn callee_objects_trusted() -> bool {
    CALLEE_OBJECTS_TRUSTED.load(Ordering::Relaxed)
}

static NESTED_WRITES_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_nested_write_targets_for_tests(on: bool) {
    NESTED_WRITES_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn nested_writes_trusted() -> bool {
    NESTED_WRITES_TRUSTED.load(Ordering::Relaxed)
}

static UNSEALED_ROTATORS_REMOVED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn remove_unsealed_rotators_for_tests(on: bool) {
    UNSEALED_ROTATORS_REMOVED.store(on, Ordering::SeqCst);
}

pub(crate) fn unsealed_rotators_removed() -> bool {
    UNSEALED_ROTATORS_REMOVED.load(Ordering::Relaxed)
}

static ROTATOR_LOOP_BINDINGS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_rotator_loop_bindings_for_tests(on: bool) {
    ROTATOR_LOOP_BINDINGS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn rotator_loop_bindings_trusted() -> bool {
    ROTATOR_LOOP_BINDINGS_TRUSTED.load(Ordering::Relaxed)
}

static WRITTEN_FIRST_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_holders_written_first_for_tests(on: bool) {
    WRITTEN_FIRST_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn written_first_trusted() -> bool {
    WRITTEN_FIRST_TRUSTED.load(Ordering::Relaxed)
}

static SEQUENCE_OBJECT_READS_TRUSTED: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn trust_sequence_object_reads_for_tests(on: bool) {
    SEQUENCE_OBJECT_READS_TRUSTED.store(on, Ordering::SeqCst);
}

pub(crate) fn sequence_object_reads_trusted() -> bool {
    SEQUENCE_OBJECT_READS_TRUSTED.load(Ordering::Relaxed)
}
