use untwist_core::catalogue;

pub use crate::driver::NO_STRING_ARRAY_NOTE;
pub use untwist_eval::decode::DECODE_ERROR_MARKER as DECODE_ERROR;

pub const STRING_ARRAY_FOUND: &str = "string array found:";
pub const SIMPLE_ARRAY_ACCESS: &str = "simple string array not inlined: unexpected access";
pub const ARRAY_ELEMENT_NOT_FOUND: &str = "element";
pub const ARRAY_READS_NOT_ALL_SUBSTITUTED: &str =
    "the array declaration was kept, not every read was substituted";
pub const ARRAY_READ_BEFORE_DECLARATION: &str =
    "a read of the array may run before its declaration, so the array was left in place";
pub const ARRAY_ALIAS_READ_BEFORE_DECLARATION: &str =
    "a read of the alias may run before its declaration, so the alias was left in place";
pub const ARRAY_ALIAS_NAME_NOT_UNIQUE: &str = "alias not resolved, the name";
pub const ARRAY_ALIAS_ESCAPES: &str =
    "the alias binding is written or read as a value rather than only indexed, so it was left in \
     place";

pub const ROTATOR_FOUND: &str = "string array rotator found";
pub const ROTATOR_NONE: &str = "string array rotator: none";

pub const DECODERS_FOUND: &str = "string array decoders found:";

pub const OBJECT_MISSING_PROP: &str = "object not inlined: property";
pub const OBJECT_PROP_VALUE_MISSING: &str = "the value of property";
pub const OBJECT_PROPS_PARTIAL: &str = "object literal kept,";
pub const OBJECT_AMBIGUOUS_KEY: &str = "the object literal defines";
pub const OBJECT_READ_BEFORE_DECLARATION: &str =
    "a read of the object may run before its declaration, so the object was left in place";
pub const OBJECT_PROPS_SPANS_EXHAUSTED: &str =
    "the synthetic span counter is exhausted, so the substituted value could not be given a span \
     outside the source range and the read was left alone";

pub const WRAPPER_ALIAS_NOT_REMOVABLE: &str =
    "the alias is not declared exactly once by a single-declarator declaration in a statement \
     list, so it was left in place";
pub const WRAPPER_ALIAS_NOT_CONSTANT: &str =
    "the alias binding is written again after it is declared, so it was left in place";
pub const WRAPPER_ALIAS_ALREADY_INITIALISED: &str =
    "the alias declaration has an initialiser, so the assignment to it was left in place";
pub const WRAPPER_ALIAS_EXTRA_WRITES: &str =
    "the alias binding is written somewhere other than the aliasing assignment, so it was left in \
     place";
pub const WRAPPER_ALIAS_NOT_AN_IDENTIFIER: &str =
    "the alias assignment's right-hand side is not an identifier";
pub const WRAPPER_ALIAS_NO_SPAN: &str = "the alias assignment has no usable span";
pub const WRAPPER_ALIAS_READ_BEFORE_THE_ALIAS: &str =
    "the alias may be read before its declaration runs, so it was left in place";
pub const WRAPPER_ALIAS_SELF_ASSIGNMENT: &str =
    "a self-assignment of the decoder was left in place";
pub const WRAPPER_OBJECT_ALIAS_THIS: &str =
    "an object holds the decoder and a decoder reads this, so a call through the object may \
     differ, and it was left in place";
pub const WRAPPER_OBJECT_ALIAS_OTHER_USE: &str =
    "an object holds the decoder and is used other than by reading that one property, so it was \
     left in place";

pub const WRAPPER_BODY_MISSING: &str = "the wrapper body was not found at its recorded span";
pub const WRAPPER_SITE_KEPT: &str = "a wrapper call site was left alone";
pub const WRAPPER_DEFERRED: &str = "wrapper kept: a call site was deferred to the next iteration";
pub const WRAPPER_NESTED_FUNCTION: &str =
    "the wrapper returns a call holding a function, arrow or class, whose bindings name-based \
     substitution would capture, so it was left in place";
pub const WRAPPER_SITE_SCOPED_ARGUMENT: &str =
    "an argument holds a function, arrow or class whose scopes the substituted clone cannot carry";
pub const WRAPPER_SITE_IMPURE_ARGUMENT: &str =
    "an argument is not side-effect-free, so substituting the body would run it again, drop it, or \
     reorder it";
pub const WRAPPER_BODY_NOT_SELF_CONTAINED: &str =
    "the wrapper body reads something other than its own parameters and the stamped machinery, so \
     the clone would resolve it in the call site's scope and it was left in place";
pub const WRAPPER_BINDING_UNSTABLE: &str =
    "the wrapper's binding is written again, or it is declared in a block whose name code outside \
     the block may reach, so a call may reach another function and it was left in place";
pub const WRAPPER_READ_BEFORE_DECLARATION: &str =
    "the wrapper is a var bound to a function expression that may be read before its declaration \
     runs, where it is still undefined, so it was left in place";
pub const WRAPPER_PARTIAL: &str = "wrapper kept,";
pub const WRAPPER_ROUNDS: &str = "decoder wrappers did not settle after";

pub const TERNARY_NOT_FOUND: &str = "the decoder's ternary argument could not be split";
pub const TERNARY_HOLDS_A_SCOPE: &str =
    "the decoder's ternary argument holds a function, arrow or class whose scopes the split clone \
     cannot carry, so the call site was left alone";
pub const NO_LITERAL_INITIALISER: &str = "the external constant has no literal initialiser";
pub const NO_PRECEDING_ASSIGNMENT: &str = "no literal assignment precedes this external reference";
pub const GLOBAL_EXTERNAL_BINDING: &str =
    "the external variable is a binding of the script's global scope, which another script or a \
     write through the global object may change, so the call site was left alone";
pub const WRITTEN_BESIDE_THE_READ: &str =
    "the external variable is also written in the statement that reads it, so the call site was \
     left alone";
pub const ASSIGNMENT_NOT_BESIDE_THE_DECLARATION: &str =
    "the external variable's declaration and its literal assignment are not direct members of the \
     same statement list, so the call site was left alone";
pub const USE_BEFORE_INITIALISATION: &str =
    "the external constant may be read before its initialiser runs, so the call site was left \
     alone";
pub const NON_LITERAL_WRITE: &str =
    "the external variable is written somewhere other than its literal assignments, so the call \
     site was left alone";
pub const REFERENCE_NOT_DOMINATED: &str =
    "an external reference is not a later member of the assignment's own statement list, so the \
     call site was left alone";

pub const DECODE_BAILED: &str = "decoding bailed";
pub const DECODE_PARTIAL: &str = "decoder returned";
pub const NON_STRING_RESULTS: &str = "some call sites decoded to non-strings";
pub const NON_PRIMITIVE_RESULT: &str =
    "the call site decoded to an object, which has no literal form, so it was left alone";
pub const MACHINERY_INCOMPLETE: &str =
    "the stamped machinery is incomplete, so no call site was decoded";
pub const STRING_ARRAY_ESCAPES: &str =
    "the string array is reachable from code outside its declaration, its decoders and a \
     recognised rotator, which may reorder or rewrite it, so the unit's call sites were left \
     alone";
pub const ROTATOR_NOT_TOP_LEVEL: &str =
    "the string array's rotator is not a statement of the program body, so whether and when it \
     runs is not proven, and the unit's call sites were left alone";
pub const CODE_BEFORE_ROTATOR: &str =
    "a statement that may run code precedes the rotator, so a call site may see the array before \
     it is rotated, and the unit's call sites were left alone";
pub const MACHINERY_NOT_INITIALISED: &str =
    "the string array or a decoder is not a function declaration, so its binding may still be \
     unset when the rotator runs, and the unit's call sites were left alone";
pub const ROTATOR_NOT_SEALED: &str =
    "the rotator reads or writes something other than its own bindings, the string array and its \
     decoders, so its effect is not proven, and the unit's call sites were left alone";
pub const WITH_STATEMENT: &str =
    "the program contains a with statement, under which an identifier may resolve to an object \
     property instead of the decoder, so no string-array unit was decoded";
pub const MACHINERY_IN_A_BLOCK: &str =
    "this function is declared in a block and its name is also used outside that block, where \
     sloppy-mode code may reach it, so it was left unrenamed and undecoded";
pub const DIRECT_EVAL_SCOPE: &str =
    "a direct eval can see this string array's scope and resolve its names at run time, so its \
     unit was left unrenamed and undecoded";
pub const DECODER_USED_AS_VALUE: &str =
    "a decoder of the unit is used other than as the callee of a call, so it may run from where \
     no call site was collected, and the unit's call sites were left alone";
pub const MACHINERY_NAME_SPELLED: &str =
    "a string or property name in the program spells this function's name, so code the program \
     builds at run time may reach it by that name, and it was left unrenamed and undecoded";
pub const DECODER_WRITTEN_OUTSIDE: &str =
    "the decoder's binding is assigned outside its own body, so a call site may reach another \
     function, and it was left unrenamed and undecoded";
pub const MACHINERY_REDECLARED: &str =
    "this function's name is declared again, or by a function in a block that may assign it, so \
     a call may reach another function, and it was left unrenamed and undecoded";
pub const STRING_ARRAY_DECLARED_UNDEFINED: &str =
    "an entry of the string array reads `undefined`, which a scope here declares or a with \
     statement or direct eval may supply, so the entry may hold that binding's value rather than \
     undefined, and the string array was left undecoded";
pub const SIMPLE_ARRAY_READ_NOT_A_VALUE: &str =
    "a read of this array is called, used as a base, iterated or destructured rather than used as \
     a value, so the array was not inlined";
pub const SIMPLE_ARRAY_DIRECT_EVAL: &str =
    "a direct eval can see this array's scope and may reach the array by name at run time, so \
     the array was not inlined";
pub const SIMPLE_ARRAY_WITH: &str =
    "a with statement can see this array's scope, so a name there may resolve to an object \
     property instead of the array, and the array was not inlined";
pub const SIMPLE_ARRAY_GLOBAL_REACH: &str =
    "code that may hold the global object, or code from text, may reach this array by a key that \
     may be its name or by an enumeration that yields values, so the array was not inlined";
pub const SIMPLE_ARRAY_NAME_OBSERVED: &str =
    "code may observe the names of the global object, where this array is a property, so its \
     declaration was kept and only its reads were inlined";
pub const BLOCK_FUNCTION_NAME: &str =
    "a function declared in a block shares this binding's name, and sloppy code also binds that \
     name in the enclosing function (Annex B.3.3), so reads there may not reach this binding and \
     it was not inlined";
pub const ARRAY_ALIAS_NAME_OBSERVED: &str =
    "code may observe the names of the global object, where this alias is a property, so the \
     alias was kept";
pub const MACHINERY_NAME_OBSERVED: &str =
    "code may observe the names of the global object, where this unit's string array function \
     or decoders are properties, so the machinery was kept as written and only its calls were \
     inlined";
pub const MACHINERY_REMOVED_UNDER_OUTSIDE_PREMISE: &str =
    "the machinery was dropped under the outside-code premise: code from outside the file (a \
     require result or the loader itself) is handed a value that may hold the global object, and \
     that code does not depend on the names the obfuscator picked for the machinery";
pub const STRING_ARRAY_GLOBAL_REACH: &str =
    "code that may hold the global object, or code from text, may reach the string array's \
     function by a key that may be its name or by an enumeration that yields values, so the \
     unit's call sites were left alone";
pub const WEBPACK_BOOTSTRAP_UNPROVEN: &str =
    "the webpack bootstrap's module table and require function were not proven to stay inside \
     the bootstrap, so its dispatch and require.d were judged as unknown code";
pub const SIMPLE_ARRAY_UNDEFINED_NOT_GLOBAL: &str =
    "an entry of this array reads `undefined` through a binding a scope here declares, or \
     through a with statement or direct eval that may supply one, so the entry is not the \
     global undefined and the array was not inlined";
pub const UNATTRIBUTED_DECODER: &str =
    "does not reference exactly one stamped string array, so it belongs to no unit and its call \
     sites were left alone";
pub const DECODED_CALL_SITES: &str = "decoded call sites";
pub const DECODER_NOT_READ_ONLY: &str =
    "a decoder of the unit does more than read from the string array, so its calls may change or \
     expose the array, and the unit's call sites were left alone";
pub const DECODER_READS_OUTER_BINDING: &str =
    "a decoder of the unit reads a binding outside itself other than a standard global, so its \
     result may depend on when it runs, and the unit's call sites were left alone";
pub const BUILTIN_PATCHED: &str =
    "the program replaces a standard built-in that the unit's decoders or rotator use, so \
     decoding with the standard one is not proven, and the unit's call sites were left alone";

pub const DECODED_NAME: &str =
    "a decoded string spells the name of this string array or one of its decoders, so code built \
     from it may reach the machinery; the unit was left undecoded under its original names";
pub const MACHINERY_NAME_UNREADABLE: &str =
    "this function's original name cannot be read back from the source text, so it could not be \
     restored if the unit were later refused; the unit was left stamped and undecoded";
pub const UNSTAMP_BLOCKED: &str =
    "the original name of a refused unit's machinery is now bound by another declaration that a \
     restored name would collide with or capture, so its names could not be restored";
pub const UNSTAMP_FREE_REFERENCE: &str =
    "the original name of a refused unit's machinery is also used where nothing declares it, so \
     restoring it would let that reference reach the machinery, and its names were not restored";

const MACHINERY_KEPT: &str = "decoder machinery kept";
pub const MACHINERY_REFERENCES_KEPT: &str =
    " reference(s) to the string array or its decoders did not decode";
pub const MACHINERY_NOT_A_LIST_MEMBER: &str =
    "decoder machinery kept: a machinery statement is not a member of a statement list";
pub const MACHINERY_NO_USABLE_SPAN: &str =
    "decoder machinery kept: a machinery statement has no usable span";
pub const MACHINERY_PARTIAL_REMOVAL: &str = "removed";

pub const CONTROL_FLOW_MISSING_PROP: &str = "control_flow_missing_prop";
pub const CONTROL_FLOW_DUPLICATE_KEY: &str = "control_flow_duplicate_key";
pub const CONTROL_FLOW_NON_CONSTANT_KEY: &str =
    "control-flow object not inlined: a computed key is not a compile-time constant string";
pub const CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY: &str =
    "control-flow object kept: a function property is read without being called, so its identity \
     must stay shared";
pub const CONTROL_FLOW_USE_BEFORE_INIT: &str =
    "control_flow_use_before_init: a read of the control-flow object may run before the \
     declaration initialises it, so the object was left in place";
pub const CONTROL_FLOW_SPANS_EXHAUSTED: &str =
    "control-flow object keys not folded: the synthetic span counter is exhausted, so the \
     completed object could not be given a span outside the source range";
pub const CONTROL_FLOW_SHAPE_LOST: &str =
    "control-flow object kept: the declared object was not found at its recorded span, or no \
     longer reads as a control-flow object";
pub const CONTROL_FLOW_OBJECT_PARTIAL: &str = "control-flow object kept,";
pub const CONTROL_FLOW_LIVE_MUTATION: &str =
    "control-flow object kept: the declaration has a statically dead branch, but the binding is \
     written or escapes outside every dead branch, so the mutation may really run";
pub const CONTROL_FLOW_DEAD_WRITE_KEPT: &str =
    "a write to the control-flow object inside a statically dead branch sits in a statement that \
     is not itself wholly dead, so the write was left in place; it never runs, but the binding it \
     assigns is no longer declared";
pub const LOGICAL_FORWARDER_ARGUMENT: &str =
    "control-flow object kept: the property forwards to a logical operator, which evaluates its \
     right operand only when the left is falsy, and an argument is neither a literal nor a \
     binding whose evaluation cannot throw, so moving it into the operand could skip a throw the \
     call performs";

pub const SWITCH_SEQUENCE: &str = "control-flow switch sequence does not match the case labels";
pub const SWITCH_FALLTHROUGH: &str =
    "control-flow switch not flattened: a case falls through into another case the dispatch loop \
     runs again on its own turn, or out of the switch before the sequence ends";
pub const SWITCH_CASE_EXIT: &str =
    "control-flow switch not flattened: a case exits the dispatch loop with break or a labeled \
     continue";
pub const SWITCH_CASE_SHADOW: &str =
    "control-flow switch not flattened: a case declares a binding that would collide once hoisted";

pub const DEAD_CODE_UNMERGEABLE: &str =
    "the taken branch declares a binding that cannot be merged; the conditional is kept";
pub const DEAD_CODE_DISCARDED_FUNCTION: &str =
    "dead-code: the branch an always-true or always-false string comparison never takes \
     declares a function in sloppy-mode code, which also binds its name in the enclosing \
     function, so the conditional was kept";

pub const MERGE_USE_BEFORE_INIT: &str =
    "merge_object_use_before_init: the object's only read may run before the declaration \
     initialises it, so the literal was left in place";
pub const MERGE_ADOPTED_DYNAMIC_KEY: &str =
    "a computed key that is no literal was merged into the object \
     literal because the conservative mode keeps the reference behaviour; { [k]: v } sets an own property where \
     obj[k] = v would set the prototype if k is \"__proto__\", and a key that runs code now runs \
     before the binding is initialised";
pub const MERGE_SPANS_EXHAUSTED: &str =
    "merged object not inlined: the synthetic span counter is exhausted, so the literal could not \
     be moved to its single reference";

pub const GLOBAL_NOT_EVALUATED: &str = "was not evaluated: the folder could not prove a result";

pub const VAR_FUNCTION_READS_ARGUMENTS: &str =
    "the function references arguments, which aliases its simple parameters, so the assignment \
     was left as a parameter";
pub const VAR_ARGUMENTS_PROPERTY_ALIAS: &str =
    "a property named arguments is read somewhere in the program, and it may alias a function's \
     simple parameters, so no assignment was rewritten as a var declaration";
pub const VAR_PARAMETER_WRITTEN_AGAIN: &str =
    "the parameter is written somewhere other than this assignment, so it was left as a parameter";
pub const VAR_PARAMETER_USED_BEFORE_THE_ASSIGNMENT: &str =
    "the parameter is read, or reachable from a nested function, before the assignment runs, so \
     it was left as a parameter";
pub const VAR_DECLARATION_NOT_REBUILT: &str =
    "the assignment statement could not be rebuilt as a var declaration, so it was left in place";

pub const DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE: &str =
    "the hoisted var is read, or reachable from a nested function, before the assignment runs, so \
     the declaration was left empty";

pub const INIT_ORDER_BUDGET: &str = "init-order budget exhausted";

pub const INIT_ORDER_OVERFLOW: &str =
    "init-order table not built: the program has more statements, references or function values \
     than a 32-bit index holds, so every candidate that needs the table is refused";

pub const SELF_DEFENDING_REFERENCE_KEPT: &str =
    "a call controller was found but a reference to it survives outside the statements this pass \
     removes, so the block was left in place";
pub const SELF_DEFENDING_NO_USABLE_SPAN: &str =
    "a matched statement has no usable span, so the block was left in place";

pub const DEBUG_PROTECTION_REFERENCE_KEPT: &str =
    "a debug protection function was found but a reference to it survives outside the statements \
     this pass removes, so the function was left in place";
pub const DEBUG_PROTECTION_NO_USABLE_SPAN: &str =
    "a matched statement has no usable span, so the function was left in place";

pub const DEOBFUSCATE_SPANS_EXHAUSTED: &str =
    "deobfuscate: the process has issued every synthetic span; inserted nodes can no longer be \
     given a span outside the source range, so edits that need one are refused";
pub const SCOPE_HAZARD: &str =
    "may reach the decoder by a name this stage renames or removes; deobfuscating anyway";
pub const ROUND_CAP: &str = "deobfuscate round cap";
pub const DECODE_DEFERRED_AT_STOP: &str =
    "string-array decoding was still deferred when the deobfuscate rounds stopped, so its call \
     sites were left alone";
pub const PREPARATION_STOPPED_EARLY: &str = "preparation stopped early";
const PLANNED_AND_APPLIED: &str = "and applied";
pub const EDIT_PLAN_INCOMPLETE: &str = " of its edits";
pub const PROTECTION_REMOVAL_PLAN_INCOMPLETE: &str = " of its protection removals";
pub const ALIAS_REMOVAL_PLAN_INCOMPLETE: &str = " of its alias removals";

pub fn string_array_found(length: usize) -> String {
    format!("{STRING_ARRAY_FOUND} {length} entries")
}

pub fn simple_array_access(name: &str) -> String {
    format!("{SIMPLE_ARRAY_ACCESS} {name}")
}

fn breakdown(details: &[(String, usize)]) -> String {
    details
        .iter()
        .map(|(name, count)| format!("{name} \u{d7}{count}"))
        .collect::<Vec<String>>()
        .join(", ")
}

pub fn simple_array_accesses(total: usize, details: &[(String, usize)]) -> String {
    format!(
        "{SIMPLE_ARRAY_ACCESS} at {total} sites ({})",
        breakdown(details)
    )
}

pub fn array_element_not_found(index: usize) -> String {
    format!("{ARRAY_ELEMENT_NOT_FOUND} {index} was not found in the array literal")
}

pub fn array_alias_name_not_unique(name: &str) -> String {
    format!("{ARRAY_ALIAS_NAME_NOT_UNIQUE} {name} is not unique")
}

pub fn decoders_found(count: usize) -> String {
    format!("{DECODERS_FOUND} {count}")
}

pub fn object_missing_prop(name: &str) -> String {
    format!("{OBJECT_MISSING_PROP} {name} is not defined")
}

pub fn object_prop_value_missing(name: &str) -> String {
    format!("{OBJECT_PROP_VALUE_MISSING} {name} was not found")
}

pub fn object_props_partial(substituted: usize, reads: usize) -> String {
    format!("{OBJECT_PROPS_PARTIAL} {substituted} of {reads} reads substituted")
}

pub fn object_ambiguous_key(name: &str) -> String {
    format!(
        "{OBJECT_AMBIGUOUS_KEY} {name} more than once or through __proto__, so which value a read \
         sees is not decidable from the literal"
    )
}

pub fn wrapper_site_scoped_argument() -> String {
    format!("{WRAPPER_SITE_KEPT}: {WRAPPER_SITE_SCOPED_ARGUMENT}")
}

pub fn wrapper_site_impure_argument() -> String {
    format!("{WRAPPER_SITE_KEPT}: {WRAPPER_SITE_IMPURE_ARGUMENT}")
}

pub fn wrapper_partial(rewritten: usize, references: usize) -> String {
    format!("{WRAPPER_PARTIAL} {rewritten} of {references} references rewritten")
}

pub fn wrapper_rounds(cap: usize) -> String {
    format!("{WRAPPER_ROUNDS} {cap} rounds")
}

pub const DECODE_BUDGET_SPENT: &str = "decoding stopped partway because this input's heap \
     budget is spent, so the remaining call sites were left encoded";

pub fn decode_bailed(bail: impl std::fmt::Display) -> String {
    format!("{DECODE_BAILED}: {bail}")
}

pub fn decode_partial(got: usize, want: usize) -> String {
    format!("{DECODE_PARTIAL} {got} values for {want} call sites")
}

pub fn decode_error() -> String {
    format!("{DECODE_ERROR}: call site did not decode to a string")
}

pub fn unattributed_decoder(name: &str) -> String {
    format!("{name} {UNATTRIBUTED_DECODER}")
}

pub fn decoded_call_sites(planned: usize, applied: usize, wanted: usize) -> String {
    format!("planned {planned} {PLANNED_AND_APPLIED} {applied} of {wanted} {DECODED_CALL_SITES}")
}

pub fn machinery_references_kept(count: usize) -> String {
    format!("{MACHINERY_KEPT}: {count}{MACHINERY_REFERENCES_KEPT}")
}

pub fn machinery_removed(applied: usize, wanted: usize) -> String {
    format!("{MACHINERY_PARTIAL_REMOVAL} {applied} of {wanted} statements")
}

pub fn control_flow_missing_prop(name: &str) -> String {
    format!("{CONTROL_FLOW_MISSING_PROP}: {name}")
}

pub fn control_flow_duplicate_key(name: &str) -> String {
    format!(
        "{CONTROL_FLOW_DUPLICATE_KEY}: {name} is assigned to an object that already defines it, so \
         which value a read sees is not decidable from the folded literal"
    )
}

pub fn control_flow_uncalled_function_properties(total: usize) -> String {
    format!("{CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY}; {total} such reads")
}

pub fn logical_forwarder_arguments(total: usize) -> String {
    format!("{LOGICAL_FORWARDER_ARGUMENT}; {total} such calls")
}

pub fn control_flow_object_partial(substituted: usize, reads: usize) -> String {
    format!("{CONTROL_FLOW_OBJECT_PARTIAL} {substituted} of {reads} reads substituted")
}

pub fn switch_sequence(label: &str) -> String {
    format!("{SWITCH_SEQUENCE}: {label}")
}

pub fn global_not_evaluated(name: &str) -> String {
    format!("{name} {GLOBAL_NOT_EVALUATED}")
}

pub fn init_order_budget(name: &str) -> String {
    format!("{INIT_ORDER_BUDGET}: a value that reads {name} was treated as running first")
}

pub fn planned_and_applied_edits(pass: &str, wanted: usize, applied: usize) -> String {
    format!("{pass}: planned {wanted} {PLANNED_AND_APPLIED} {applied}{EDIT_PLAN_INCOMPLETE}")
}

pub fn planned_and_applied_removals(pass: &str, wanted: usize, applied: usize) -> String {
    format!(
        "{pass}: planned {wanted} {PLANNED_AND_APPLIED} \
         {applied}{PROTECTION_REMOVAL_PLAN_INCOMPLETE}"
    )
}

pub fn planned_and_applied_alias_removals(wanted: usize, removed: usize) -> String {
    format!(
        "find-string-array: planned {wanted} {PLANNED_AND_APPLIED} \
         {removed}{ALIAS_REMOVAL_PLAN_INCOMPLETE}"
    )
}

pub fn scope_hazard(hazard: impl std::fmt::Display) -> String {
    format!("{hazard} {SCOPE_HAZARD}")
}

pub fn round_cap(max: u32) -> String {
    format!("{ROUND_CAP} {max} reached with changes pending")
}

pub fn preparation_stopped_early(stop: &str) -> String {
    format!("{PREPARATION_STOPPED_EARLY}: {stop}")
}

catalogue!(
    ALL_NOTES,
    STRING_ARRAY_FOUND,
    SIMPLE_ARRAY_ACCESS,
    ARRAY_READ_BEFORE_DECLARATION,
    ARRAY_ALIAS_READ_BEFORE_DECLARATION,
    ARRAY_ALIAS_NAME_NOT_UNIQUE,
    ARRAY_ALIAS_ESCAPES,
    ROTATOR_FOUND,
    ROTATOR_NONE,
    DECODERS_FOUND,
    OBJECT_MISSING_PROP,
    OBJECT_AMBIGUOUS_KEY,
    OBJECT_READ_BEFORE_DECLARATION,
    WRAPPER_ALIAS_NOT_REMOVABLE,
    WRAPPER_ALIAS_NOT_CONSTANT,
    WRAPPER_ALIAS_ALREADY_INITIALISED,
    WRAPPER_ALIAS_EXTRA_WRITES,
    WRAPPER_ALIAS_READ_BEFORE_THE_ALIAS,
    WRAPPER_ALIAS_SELF_ASSIGNMENT,
    WRAPPER_OBJECT_ALIAS_THIS,
    WRAPPER_OBJECT_ALIAS_OTHER_USE,
    WRAPPER_SITE_KEPT,
    WRAPPER_DEFERRED,
    WRAPPER_NESTED_FUNCTION,
    WRAPPER_SITE_SCOPED_ARGUMENT,
    WRAPPER_SITE_IMPURE_ARGUMENT,
    WRAPPER_BODY_NOT_SELF_CONTAINED,
    WRAPPER_BINDING_UNSTABLE,
    WRAPPER_READ_BEFORE_DECLARATION,
    WRAPPER_PARTIAL,
    TERNARY_HOLDS_A_SCOPE,
    NO_LITERAL_INITIALISER,
    NO_PRECEDING_ASSIGNMENT,
    WRITTEN_BESIDE_THE_READ,
    GLOBAL_EXTERNAL_BINDING,
    ASSIGNMENT_NOT_BESIDE_THE_DECLARATION,
    USE_BEFORE_INITIALISATION,
    NON_LITERAL_WRITE,
    REFERENCE_NOT_DOMINATED,
    DECODE_BAILED,
    DECODE_BUDGET_SPENT,
    DECODE_ERROR,
    NON_STRING_RESULTS,
    NON_PRIMITIVE_RESULT,
    MACHINERY_INCOMPLETE,
    STRING_ARRAY_ESCAPES,
    ROTATOR_NOT_TOP_LEVEL,
    CODE_BEFORE_ROTATOR,
    MACHINERY_NOT_INITIALISED,
    ROTATOR_NOT_SEALED,
    WITH_STATEMENT,
    DIRECT_EVAL_SCOPE,
    MACHINERY_IN_A_BLOCK,
    DECODER_USED_AS_VALUE,
    MACHINERY_NAME_SPELLED,
    DECODER_WRITTEN_OUTSIDE,
    MACHINERY_REDECLARED,
    STRING_ARRAY_DECLARED_UNDEFINED,
    SIMPLE_ARRAY_UNDEFINED_NOT_GLOBAL,
    SIMPLE_ARRAY_READ_NOT_A_VALUE,
    SIMPLE_ARRAY_DIRECT_EVAL,
    SIMPLE_ARRAY_WITH,
    SIMPLE_ARRAY_GLOBAL_REACH,
    SIMPLE_ARRAY_NAME_OBSERVED,
    ARRAY_ALIAS_NAME_OBSERVED,
    BLOCK_FUNCTION_NAME,
    MACHINERY_NAME_OBSERVED,
    MACHINERY_REMOVED_UNDER_OUTSIDE_PREMISE,
    STRING_ARRAY_GLOBAL_REACH,
    WEBPACK_BOOTSTRAP_UNPROVEN,
    UNATTRIBUTED_DECODER,
    DECODED_CALL_SITES,
    DECODER_NOT_READ_ONLY,
    DECODER_READS_OUTER_BINDING,
    BUILTIN_PATCHED,
    DECODED_NAME,
    UNSTAMP_BLOCKED,
    UNSTAMP_FREE_REFERENCE,
    MACHINERY_REFERENCES_KEPT,
    MACHINERY_NOT_A_LIST_MEMBER,
    CONTROL_FLOW_MISSING_PROP,
    CONTROL_FLOW_DUPLICATE_KEY,
    CONTROL_FLOW_NON_CONSTANT_KEY,
    CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY,
    CONTROL_FLOW_USE_BEFORE_INIT,
    CONTROL_FLOW_OBJECT_PARTIAL,
    CONTROL_FLOW_LIVE_MUTATION,
    CONTROL_FLOW_DEAD_WRITE_KEPT,
    LOGICAL_FORWARDER_ARGUMENT,
    SWITCH_SEQUENCE,
    SWITCH_FALLTHROUGH,
    SWITCH_CASE_EXIT,
    SWITCH_CASE_SHADOW,
    MERGE_USE_BEFORE_INIT,
    MERGE_ADOPTED_DYNAMIC_KEY,
    DEAD_CODE_DISCARDED_FUNCTION,
    GLOBAL_NOT_EVALUATED,
    VAR_FUNCTION_READS_ARGUMENTS,
    VAR_ARGUMENTS_PROPERTY_ALIAS,
    VAR_PARAMETER_WRITTEN_AGAIN,
    VAR_PARAMETER_USED_BEFORE_THE_ASSIGNMENT,
    DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE,
    INIT_ORDER_BUDGET,
    SELF_DEFENDING_REFERENCE_KEPT,
    DEBUG_PROTECTION_REFERENCE_KEPT,
    SCOPE_HAZARD,
    NO_STRING_ARRAY_NOTE,
);

catalogue!(
    GUARD_NOTES,
    MACHINERY_NAME_UNREADABLE,
    ARRAY_ELEMENT_NOT_FOUND,
    ARRAY_READS_NOT_ALL_SUBSTITUTED,
    ALIAS_REMOVAL_PLAN_INCOMPLETE,
    OBJECT_PROP_VALUE_MISSING,
    OBJECT_PROPS_PARTIAL,
    OBJECT_PROPS_SPANS_EXHAUSTED,
    WRAPPER_ALIAS_NOT_AN_IDENTIFIER,
    WRAPPER_ALIAS_NO_SPAN,
    WRAPPER_BODY_MISSING,
    WRAPPER_ROUNDS,
    TERNARY_NOT_FOUND,
    DECODE_PARTIAL,
    MACHINERY_NO_USABLE_SPAN,
    MACHINERY_PARTIAL_REMOVAL,
    CONTROL_FLOW_SPANS_EXHAUSTED,
    CONTROL_FLOW_SHAPE_LOST,
    DEAD_CODE_UNMERGEABLE,
    MERGE_SPANS_EXHAUSTED,
    VAR_DECLARATION_NOT_REBUILT,
    SELF_DEFENDING_NO_USABLE_SPAN,
    DEBUG_PROTECTION_NO_USABLE_SPAN,
    DEOBFUSCATE_SPANS_EXHAUSTED,
    ROUND_CAP,
    DECODE_DEFERRED_AT_STOP,
    PREPARATION_STOPPED_EARLY,
    INIT_ORDER_OVERFLOW,
    EDIT_PLAN_INCOMPLETE,
    PROTECTION_REMOVAL_PLAN_INCOMPLETE,
);
