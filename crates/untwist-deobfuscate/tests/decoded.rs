mod common;

use oxc_allocator::Allocator;

const UNIT: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
";

const OPTIONAL_CHAIN_CALL: &str = "f(b?.(0x1));";

const PRE_FLIGHT: [&str; 10] = [
    "const z = () => b(0x1);",
    "class K { m() { return b(0x1); } }",
    "for (const q of [b(0x1)]) { f(q); }",
    "for (const q of b(0x1)) { f(q); }",
    OPTIONAL_CHAIN_CALL,
    "f(`x${b(0x1)}y`);",
    "f({ k: b(0x1) });",
    "f([b(0x1)]);",
    "async function g() { return await b(0x1); }",
    "function* g() { yield b(0x1); }",
];

#[test]
fn every_call_position_in_the_pre_flight_list_is_substituted() {
    for tail in PRE_FLIGHT {
        let (code, _notes) = common::run_passes(&format!("{UNIT}{tail}"));
        assert!(code.contains("\"Hello World!\""), "{tail}: {code}");
    }
}

#[test]
fn every_pre_flight_position_loses_its_decoder_call() {
    for tail in PRE_FLIGHT {
        let (code, _notes) = common::run_passes(&format!("{UNIT}{tail}"));
        let decoded = code.matches("\"Hello World!\"").count();
        assert!(!code.contains("__STRING_ARRAY__"), "{tail}: {code}");
        assert_eq!(decoded, 1, "{tail}: {code}");
    }
}

#[test]
fn an_optional_chain_call_is_substituted_and_the_output_reparses() {
    let (code, notes) = common::run_passes(&format!("{UNIT}{OPTIONAL_CHAIN_CALL}"));
    assert!(code.contains("f(\"Hello World!\")"), "{code}");
    assert!(!code.contains("?."), "{code}");
    assert!(
        !notes
            .iter()
            .any(|note| note.contains("untwist:decode_error") || note.contains("not numbered")),
        "{notes:?}"
    );
    let alloc = Allocator::default();
    let reparsed = untwist_core::parse::parse_program(&alloc, &code);
    assert!(
        reparsed.diagnostics.is_empty(),
        "output does not re-parse: {:?}\n{code}",
        reparsed.diagnostics
    );
}

#[test]
fn every_literal_call_site_becomes_its_decoded_string() {
    let (code, _notes) = common::run_passes(
        "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));",
    );
    assert!(
        code.contains("console[\"log\"](\"Hello World!\")"),
        "{code}"
    );
}

#[test]
fn an_out_of_range_index_becomes_void_zero_and_is_reported() {
    let (code, notes) = common::run_passes(
        "\
function a() { var h = ['log']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0xc811));",
    );
    assert!(code.contains("console.log(void 0)"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("untwist:decode_error")),
        "{notes:?}"
    );
}

#[test]
fn a_void_zero_array_entry_stays_void_zero() {
    let (code, notes) = common::run_passes(
        "\
function a() { var h = ['log', void 0]; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console[b(0x0)](b(0x1));",
    );
    assert!(code.contains("console[\"log\"](void 0)"), "{code}");
    assert!(notes
        .iter()
        .any(|note| note.contains("untwist:decode_error")));
}

#[test]
fn a_program_with_no_decoder_is_untouched() {
    let (code, notes) = common::run_passes("console.log(1);");
    assert_eq!(code.trim(), "console.log(1);");
    assert_eq!(
        notes,
        vec![untwist_deobfuscate::NO_STRING_ARRAY_NOTE.to_string()],
        "{notes:?}"
    );
}

#[test]
fn a_second_run_of_the_pass_changes_nothing() {
    let (code, _notes) = common::run_passes(&format!("{UNIT}console[b(0x0)](b(0x1));"));
    let alloc = Allocator::default();
    let (again, notes) = common::run_ctors(&alloc, &code, &[untwist_deobfuscate::decoded::boxed]);
    assert_eq!(code, again, "{again}");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_stamped_name_that_is_not_part_of_the_machinery_is_reported() {
    let alloc = Allocator::default();
    let (_code, notes) = common::run_ctors(
        &alloc,
        "\
let __DECODE_0__ = 1;
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_1__(c, d) { var e = __STRING_ARRAY__(); __DECODE_1__ = function (f, g) { return e[f]; }; return __DECODE_1__(c, d); }
console.log(__DECODE_1__(0x0));",
        &[untwist_deobfuscate::decoded::boxed],
    );
    assert!(
        notes
            .iter()
            .any(|note| note.contains("the stamped machinery is incomplete")),
        "{notes:?}"
    );
}

#[test]
fn a_gap_below_a_decoders_number_is_not_a_refusal() {
    let alloc = Allocator::default();
    let (code, notes) = common::run_ctors(
        &alloc,
        "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_1__(c, d) { var e = __STRING_ARRAY__(); __DECODE_1__ = function (f, g) { return e[f]; }; return __DECODE_1__(c, d); }
console.log(__DECODE_1__(0x0));",
        &[untwist_deobfuscate::decoded::boxed],
    );
    assert!(code.contains("console.log(\"log\")"), "{code}");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_decoder_that_exhausts_the_budget_bails_into_a_note_and_changes_nothing() {
    let alloc = Allocator::default();
    let (code, notes) = common::run_ctors(
        &alloc,
        "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { while (true) { f = f + 1; } }; return __DECODE_0__(c, d); }
console.log(__DECODE_0__(0x0));",
        &[untwist_deobfuscate::decoded::boxed],
    );
    assert!(code.contains("console.log(__DECODE_0__(0))"), "{code}");
    assert!(
        notes.iter().any(|note| note.contains("decoding bailed")),
        "{notes:?}"
    );
}

const TWO_DECODERS: &str = "\
function __STRING_ARRAY__() { var h = ['log', '3', 'Hello World!', 'Goodbye']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); __DECODE_0__ = function (f, g) { f = f - 0x0; return e[f]; }; return __DECODE_0__(c, d); }
function __DECODE_1__(c, d) { var e = __STRING_ARRAY__(); __DECODE_1__ = function (f, g) { f = f - 0x0; return e[f]; }; return __DECODE_1__(c, d); }
";

fn decode_only(tail: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let source = format!("{TWO_DECODERS}{tail}");
    let (code, notes) = common::run_ctors(&alloc, &source, &[untwist_deobfuscate::decoded::boxed]);
    let check = Allocator::default();
    let reparsed = untwist_core::parse::parse_program(&check, &code);
    assert!(
        reparsed.diagnostics.is_empty(),
        "output does not re-parse: {:?}\n{code}",
        reparsed.diagnostics
    );
    (code, notes)
}

#[test]
fn a_decoder_call_inside_a_ternary_argument_keeps_the_outer_decoder_layer() {
    let (code, _notes) = decode_only("f(__DECODE_0__(c ? __DECODE_1__(0x1) : 0x2));");
    assert!(
        code.contains("f(c ? \"Goodbye\" : \"Hello World!\")"),
        "{code}"
    );
}

#[test]
fn a_nested_decoder_call_is_decoded_through_both_layers() {
    let (code, _notes) = decode_only("f(__DECODE_0__(__DECODE_1__(0x1)));");
    assert!(code.contains("f(\"Goodbye\")"), "{code}");
}

#[test]
fn an_optional_call_with_a_ternary_argument_is_split_and_decoded() {
    let (code, _notes) = decode_only("f(__DECODE_0__?.(q ? 0x1 : 0x2));");
    assert!(code.contains("f(q ? \"3\" : \"Hello World!\")"), "{code}");
    assert!(!code.contains("?."), "{code}");
}

#[test]
fn an_optional_call_that_is_a_member_object_is_substituted_in_place() {
    let (code, _notes) = decode_only("f(__DECODE_0__?.(0x1).foo);");
    assert!(code.contains("f(\"3\".foo)"), "{code}");
    assert!(!code.contains("__DECODE_0__?.(1)"), "{code}");
}

const ESCAPE_HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return String.fromCharCode(e[c].charCodeAt(0)); }
";

fn escape_notes(patch: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let source = format!("{ESCAPE_HEADER}{patch}\nconsole.log(__DECODE_0__(0x0));\n");
    common::run_ctors(&alloc, &source, &[untwist_deobfuscate::decoded::boxed])
}

fn escape_refused(notes: &[String]) -> bool {
    notes.iter().any(|note| {
        note.starts_with(untwist_deobfuscate::notes::BUILTIN_PATCHED)
            || note == untwist_deobfuscate::notes::STRING_ARRAY_GLOBAL_REACH
    })
}

const CONSTRUCTOR_ESCAPE_HEADER: &str = "\
function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return ''.constructor.fromCharCode(e[c].charCodeAt(0)); }
";

fn constructor_escape_notes(patch: &str) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let source = format!("{CONSTRUCTOR_ESCAPE_HEADER}{patch}\nconsole.log(__DECODE_0__(0x0));\n");
    common::run_ctors(&alloc, &source, &[untwist_deobfuscate::decoded::boxed])
}

#[test]
fn a_decoder_reached_through_string_constructor_still_refuses_a_patch_on_the_bare_name() {
    let patch = "String.fromCharCode = function () { return 'X'; };";
    let (_code, notes) = constructor_escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_patch_reached_through_string_constructor_refuses_the_unit() {
    let patch = "''.constructor.fromCharCode = function () { return 'X'; };";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_function_escape_the_decoder_never_reads_still_decodes() {
    let patch = "function useFunction(x) { return x; }\nuseFunction(Function);";
    let (code, notes) = escape_notes(patch);
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(code.contains("\"l\""), "{code}");
}

#[test]
fn an_eval_string_patching_a_used_builtin_refuses_the_unit() {
    let patch = "eval(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_new_function_string_patching_a_used_builtin_refuses_the_unit() {
    let patch = "new Function(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_literal_settimeout_string_patching_a_used_builtin_refuses_the_unit() {
    let patch = "setTimeout(\"String.fromCharCode = function () { return 'X'; };\", 0);";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_constructor_call_string_patching_a_used_builtin_refuses_the_unit() {
    let patch = "var x = function () {};\n\
                 x.constructor(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_harmless_new_function_string_still_decodes() {
    let patch = "new Function('return this')();";
    let (code, notes) = escape_notes(patch);
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(code.contains("\"l\""), "{code}");
}

#[test]
fn a_core_js_style_define_built_in_polyfill_still_decodes() {
    let patch = "\
        function defineBuiltIn(target, key, value) {\n\
          target[key] = value;\n\
        }\n\
        defineBuiltIn(Object.prototype, 'toString', function () {\n\
          return '[object Object]';\n\
        });\n\
        defineBuiltIn(Array.prototype, 'toString', function () {\n\
          return '[object Array]';\n\
        });";
    let (code, notes) = escape_notes(patch);
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(code.contains("\"l\""), "{code}");
}

#[test]
fn a_top_level_define_built_in_beside_a_sloppy_this_refuses_the_unit() {
    let patch = "\
        function defineBuiltIn(target, key, value) {\n\
          target[key] = value;\n\
        }\n\
        defineBuiltIn(Object.prototype, 'toString', function () {\n\
          return '[object Object]';\n\
        });\n\
        defineBuiltIn(Array.prototype, 'toString', function () {\n\
          return this.join(',');\n\
        });";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_shadowed_undefined_binding_cannot_capture_the_substituted_value() {
    let (code, _notes) = common::run_passes(
        "\
function a() { var h = ['log', undefined]; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
var arr = [undefined, 'log'];
function f() { var undefined = 'BAD'; return arr[0]; }
console[b(0x0)](f(), arr[1]);",
    );
    assert!(!code.contains("return undefined;"), "{code}");
    assert!(code.contains("return void 0;"), "{code}");
}

#[test]
fn an_indirect_eval_via_a_sequence_expression_refuses_the_unit() {
    let patch = "(0, eval)(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_computed_global_object_member_naming_function_refuses_the_unit() {
    let patch =
        "new (globalThis['Function'])(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn function_passed_as_an_argument_to_reflect_construct_refuses_the_unit() {
    let patch =
        "Reflect.construct(Function, [\"String.fromCharCode = function () { return 'X'; };\"])();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn window_eval_refuses_the_unit() {
    let patch = "window.eval(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn array_constructor_constructor_via_a_variable_refuses_the_unit() {
    let patch = "var c = [].constructor.constructor;\n\
                 c(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn function_prototype_constructor_refuses_the_unit() {
    let patch =
        "Function.prototype.constructor(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_root_captured_into_a_variable_and_then_called_refuses_the_unit() {
    let patch = "var t = eval;\nt(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_root_captured_into_a_variable_and_then_constructed_refuses_the_unit() {
    let patch =
        "let f = Function;\nnew f(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn assigning_a_sink_root_to_a_plain_variable_refuses_the_unit() {
    let patch = "var x;\nx = setTimeout;";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_root_read_through_an_array_literal_index_refuses_the_unit() {
    let patch = "[eval][0](\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_root_read_through_an_object_literal_property_refuses_the_unit() {
    let patch = "({ e: eval }).e(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn top_level_this_eval_refuses_the_unit() {
    let patch = "this.eval(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn top_level_this_computed_function_refuses_the_unit() {
    let patch = "new (this['Function'])(\"String.fromCharCode = function () { return 'X'; };\")();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sloppy_functions_this_eval_refuses_the_unit() {
    let patch = "(function () {\n\
                 this.eval(\"String.fromCharCode = function () { return 'X'; };\");\n\
                 })();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_plainly_called_named_function_this_eval_refuses_the_unit() {
    let patch = "function f() {\n\
                 this.eval(\"String.fromCharCode = function () { return 'X'; };\");\n\
                 }\n\
                 f();";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_method_extracted_and_called_bare_refuses_the_unit() {
    let patch = "var o = { m: function () { return this; } };\n\
                 var g = o.m;\n\
                 g().eval(\"String.fromCharCode = function () { return 'X'; };\");";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_root_passed_to_array_map_refuses_the_unit() {
    let patch = "[\"String.fromCharCode = function () { return 'X'; };\"].map(eval);";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_root_passed_to_an_unresolved_free_global_refuses_the_unit() {
    let patch = "unknownFn(eval);";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_new_only_constructor_using_this_computed_key_still_decodes() {
    let patch = "function C(x) {\n\
                 this['k'] = x;\n\
                 }\n\
                 new C(1);";
    let (code, notes) = escape_notes(patch);
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(code.contains("\"l\""), "{code}");
}

#[test]
fn a_plain_value_through_a_proxy_object_dispatch_still_decodes() {
    let patch = "var O = { m: function (x) { return x; } };\n\
                 O['m'](function () {});";
    let (code, notes) = escape_notes(patch);
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(code.contains("\"l\""), "{code}");
}

#[test]
fn a_runner_handed_to_a_user_function_is_taken_as_a_value() {
    let patch = "var O = { m: function (x) { return x; } };\n\
                 O['m'](Function);";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_proxy_object_passed_to_another_function_no_longer_exempts_its_argument() {
    let patch = "function h(x) {\n\
                 return x;\n\
                 }\n\
                 var O = { m: function (x) { return x; } };\n\
                 h(O);\n\
                 O['m'](Function);";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_proxy_object_with_a_member_written_outside_the_literal_no_longer_exempts_its_argument() {
    let patch = "var O = { m: function (x) { return x; } };\n\
                 O['k'] = eval;\n\
                 O['m'](Function);";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_sink_key_that_is_a_call_that_never_resolves_eventually_refuses() {
    let patch = "function neverInlines() {\n\
                 return \"String.fromCharCode = function () { return 'X'; };\";\n\
                 }\n\
                 eval(neverInlines());";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn an_iteration_whose_only_change_is_a_defer_still_refuses_without_a_full_extra_iteration() {
    let alloc = Allocator::default();
    let patch = "function neverInlines() {\n\
                 return \"String.fromCharCode = function () { return 'X'; };\";\n\
                 }\n\
                 eval(neverInlines());";
    let source = format!("{ESCAPE_HEADER}{patch}\nconsole.log(__DECODE_0__(0x0));\n");
    let (_code, report) =
        common::run_ctors_report(&alloc, &source, &[untwist_deobfuscate::decoded::boxed]);
    assert!(
        report.iterations <= 2,
        "a defer-only iteration must resolve via the single-pass re-run, not a full extra \
         fixpoint iteration: {} iterations, stop={:?}",
        report.iterations,
        report.stop
    );
    let notes: Vec<String> = report.notes.into_iter().map(|note| note.message).collect();
    assert!(escape_refused(&notes), "{notes:?}");
}

fn assert_two_phase_matches_full_rescan(patch: &str) {
    use rustc_hash::FxHashSet;
    use untwist_deobfuscate::bridge::build_requests;
    use untwist_deobfuscate::proof::{
        build_escape_context, resolve_pending_sites, scan_escaping_roots_resolved, scan_pending,
    };

    let alloc = Allocator::default();
    let source = format!("{ESCAPE_HEADER}{patch}\nconsole.log(__DECODE_0__(0x0));\n");
    let parsed = untwist_core::parse::parse_program(&alloc, &source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{patch}: {:?}",
        parsed.diagnostics
    );
    let sem = untwist_core::semantic::build_scoping(&parsed.program);
    let scoping = &sem.scoping;
    let plan = build_requests(&parsed.program, scoping);
    let all_own_spans: FxHashSet<_> = plan
        .requests
        .iter()
        .flat_map(|request| request.calls.iter().map(|call| call.span))
        .collect();
    let globals: FxHashSet<&'static str> = untwist_eval::builtins::all_root_names()
        .iter()
        .copied()
        .collect();
    let context = build_escape_context(&parsed.program, scoping);
    let resolve = |_span: oxc_span::Span| None;

    let two_phase_scan = scan_pending(&parsed.program, scoping, &globals, &context, &all_own_spans);
    let two_phase = resolve_pending_sites(&two_phase_scan, &globals, &context, &resolve);
    let old_rescan =
        scan_escaping_roots_resolved(&parsed.program, scoping, &globals, &context, &resolve);

    assert_eq!(
        two_phase.names, old_rescan.names,
        "{patch}: two-phase E must equal the old full-rescan E"
    );
    assert_eq!(
        two_phase.span.is_some(),
        old_rescan.span.is_some(),
        "{patch}: two-phase and the old full rescan must agree on whether anything escaped"
    );
}

#[test]
fn the_two_phase_escape_scan_matches_the_old_full_rescan_on_every_known_row() {
    for patch in [
        "var S = String;",
        "f(String);",
        "window.String.fromCharCode = function () { return 'X'; };",
        "globalThis['String'];",
        "Object.defineProperty(String, 'x', { value: 1 });",
        "Object.assign(String, { x: 1 });",
        "(0, String).fromCharCode = function () { return 'X'; };",
        "[String][0].fromCharCode = function () { return 'X'; };",
        "this.eval(\"String.fromCharCode = function () { return 'X'; };\");",
        "new (this['Function'])(\"String.fromCharCode = function () { return 'X'; };\")();",
        "(function () {\n\
         this.eval(\"String.fromCharCode = function () { return 'X'; };\");\n\
         })();",
        "var O = { m: function (x) { return x; } };\n\
         O['m'](Function);",
        "function h(x) {\n\
         return x;\n\
         }\n\
         var O = { m: function (x) { return x; } };\n\
         h(O);\n\
         O['m'](Function);",
        "var O = { m: function (x) { return x; } };\n\
         O['k'] = eval;\n\
         O['m'](Function);",
    ] {
        assert_two_phase_matches_full_rescan(patch);
    }
}

#[test]
fn a_terser_compressed_unit_decodes_and_loses_its_machinery() {
    let (code, notes) = common::run_passes(
        "\
function b(r, t) { return r -= 415, a()[r]; }
function a() { var r = ['log', 'Hello World!']; return (a = function () { return r; })(); }
console[b(415)](b(416));",
    );
    assert!(code.contains("console.log(\"Hello World!\")"), "{code}");
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(notes.is_empty(), "{notes:?}");
}

const TERSER_UNIT: &str = "\
function b(r, t) { return r -= 0, a()[r]; }
function a() { var r = ['Hello World!', 'log']; return (a = function () { return r; })(); }
";

#[test]
fn a_terser_rotator_that_calls_the_array_directly_is_run() {
    let (code, notes) = common::run_passes(&format!(
        "{TERSER_UNIT}(function (r, t) {{ for (var e = b, n = a();;) try {{ if (e(0) === 'log') break; n.push(n.shift()); }} catch (r) {{ n.push(n.shift()); }} }})();\nconsole[b(0)](b(1));"
    ));
    assert!(code.contains("console.log(\"Hello World!\")"), "{code}");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_terser_rotator_that_calls_the_array_twice_is_refused() {
    let (code, notes) = common::run_passes(&format!(
        "{TERSER_UNIT}(function (r, t) {{ for (var e = b, n = a(), m = a();;) try {{ if (e(0) === 'log') break; n.push(m.shift()); }} catch (r) {{ n.push(n.shift()); }} }})();\nconsole[b(0)](b(1));"
    ));
    assert!(!code.contains("console.log(\"Hello World!\")"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("the rotator reads or writes")),
        "{notes:?}"
    );
}

#[test]
fn a_terser_rotator_that_runs_code_before_calling_the_array_is_refused() {
    let (code, notes) = common::run_passes(&format!(
        "{TERSER_UNIT}(function (r, t) {{ var e = b; e(1); var n = a(); for (;;) try {{ if (e(0) === 'log') break; n.push(n.shift()); }} catch (r) {{ n.push(n.shift()); }} }})();\nconsole[b(0)](b(1));"
    ));
    assert!(!code.contains("console.log(\"Hello World!\")"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("the rotator reads or writes")),
        "{notes:?}"
    );
}
