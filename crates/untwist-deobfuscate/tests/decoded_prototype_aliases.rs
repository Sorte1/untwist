mod common;

use oxc_allocator::Allocator;

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
    notes
        .iter()
        .any(|note| note.starts_with(untwist_deobfuscate::notes::BUILTIN_PATCHED))
}

#[test]
fn a_built_in_reached_through_an_alias_or_an_escape_refuses_the_unit() {
    for patch in [
        "var S = String;",
        "f(String);",
        "window.String.fromCharCode = function () { return 'X'; };",
        "globalThis['String'];",
        "Object.defineProperty(String, 'x', { value: 1 });",
        "Object.assign(String, { x: 1 });",
        "(0, String).fromCharCode = function () { return 'X'; };",
        "[String][0].fromCharCode = function () { return 'X'; };",
    ] {
        let (code, notes) = escape_notes(patch);
        assert!(escape_refused(&notes), "{patch}: {notes:?}");
        assert!(code.contains("__DECODE_0__("), "{patch}: {code}");
    }
}

#[test]
fn a_global_object_alias_reaching_a_used_built_in_refuses_the_unit() {
    let patch = "var w = window;\nw.String.fromCharCode = function () { return 'X'; };";
    let (_code, notes) = escape_notes(patch);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_an_escaped_alias_is_patched_is_refused_before_any_decode() {
    let source = "\
        var P = Array.prototype;\n\
        P.push = function (x) { return [x][0]; };\n\
        P.shift = function () { return 0; };\n\
        function __STRING_ARRAY__() { var h = ['3', '1', '2']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }\n\
        function __DECODE_0__(c, d) {\n\
          var e = __STRING_ARRAY__();\n\
          return (__DECODE_0__ = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), __DECODE_0__(c, d);\n\
        }\n\
        (function (c, d) {\n\
          var k = __DECODE_0__, f = c();\n\
          while (!![]) {\n\
            try {\n\
              var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;\n\
              if (g === d) break;\n\
              else f['push'](f['shift']());\n\
            } catch (h) { f['push'](f['shift']()); }\n\
          }\n\
        })(__STRING_ARRAY__, 0xe);\n\
        console.log(__DECODE_0__(0x0), __DECODE_0__(0x1), __DECODE_0__(0x2));\n";
    let (_code, notes) = common::run_passes(source);
    assert!(refused_before_the_rotator(&notes), "{notes:?}");
}

fn rotator_source(patch: &str) -> String {
    format!(
        "{patch}\n\
        function __STRING_ARRAY__() {{ var h = ['3', '1', '2']; __STRING_ARRAY__ = function () {{ return h; }}; return __STRING_ARRAY__(); }}\n\
        function __DECODE_0__(c, d) {{\n\
          var e = __STRING_ARRAY__();\n\
          return (__DECODE_0__ = function (f, g) {{ f = f - 0x0; var r = [e[f], e[f]]; r['push'](r['shift']()); return r[0]; }}), __DECODE_0__(c, d);\n\
        }}\n\
        console.log(__DECODE_0__(0x0), __DECODE_0__(0x1), __DECODE_0__(0x2));\n"
    )
}

fn proven_rotator_source(before: &str, after: &str) -> String {
    format!(
        "{before}\n\
        function __STRING_ARRAY__() {{ var h = ['3', '1', '2']; __STRING_ARRAY__ = function () {{ return h; }}; return __STRING_ARRAY__(); }}\n\
        function __DECODE_0__(c, d) {{\n\
          var e = __STRING_ARRAY__();\n\
          return (__DECODE_0__ = function (f, g) {{ f = f - 0x0; var i = e[f]; return i; }}), __DECODE_0__(c, d);\n\
        }}\n\
        (function (c, d) {{\n\
          var k = __DECODE_0__, f = c();\n\
          while (!![]) {{\n\
            try {{\n\
              var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;\n\
              if (g === d) break;\n\
              else f['push'](f['shift']());\n\
            }} catch (h) {{ f['push'](f['shift']()); }}\n\
          }}\n\
        }})(__STRING_ARRAY__, 0xe);\n\
        {after}\n\
        console.log(__DECODE_0__(0x0), __DECODE_0__(0x1), __DECODE_0__(0x2));\n"
    )
}

fn refused_before_the_rotator(notes: &[String]) -> bool {
    notes
        .iter()
        .any(|note| note == untwist_deobfuscate::notes::CODE_BEFORE_ROTATOR)
}

const PUSH_PATCH: &str = "Array.prototype['pu' + 'sh'] = function (x) { return [x][0]; };";

#[test]
fn a_push_patch_before_the_rotator_refuses_the_unit_before_any_decode() {
    let (code, notes) = common::run_passes(&proven_rotator_source(PUSH_PATCH, ""));
    assert!(refused_before_the_rotator(&notes), "{notes:?}");
    assert!(code.contains("__DECODE_0__("), "{code}");
}

#[test]
fn a_push_patch_after_a_proven_rotator_does_not_reach_the_rotation() {
    let (code, notes) = common::run_passes(&proven_rotator_source("", PUSH_PATCH));
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(!code.contains("__DECODE_0__("), "{code}");
}

#[test]
fn push_shift_after_a_concatenated_computed_push_write_directly_on_array_prototype_refuses_the_unit(
) {
    let patch = "Array.prototype['pu' + 'sh'] = function (x) { return [x][0]; };";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_a_concatenated_computed_push_write_through_an_alias_refuses_the_unit() {
    let patch = "\
        var P = Array.prototype;\n\
        P['pu' + 'sh'] = function (x) { return [x][0]; };";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_a_compound_assignment_through_an_alias_refuses_the_unit() {
    let patch = "\
        var P = Array.prototype;\n\
        P['pu' + 'sh'] += 0;";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_an_update_expression_through_an_alias_refuses_the_unit() {
    let patch = "\
        var P = Array.prototype;\n\
        P.push++;";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_a_define_property_through_an_alias_refuses_the_unit() {
    let patch = "\
        var P = Array.prototype;\n\
        Object.defineProperty(P, 'push', { value: function (x) { return [x][0]; } });";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_checksum_after_a_char_code_at_patch_is_refused_before_any_decode() {
    let source = "\
        String.prototype.charCodeAt = function () { return 999; };\n\
        function __STRING_ARRAY__() { var h = ['3', '1', '2']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }\n\
        function __DECODE_0__(c, d) {\n\
          var e = __STRING_ARRAY__();\n\
          return (__DECODE_0__ = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), __DECODE_0__(c, d);\n\
        }\n\
        (function (c, d) {\n\
          var k = __DECODE_0__, f = c();\n\
          while (!![]) {\n\
            try {\n\
              var g = parseInt('0') + k(0x0)['charCodeAt'](0) - 0x30;\n\
              if (g === d) break;\n\
              else f['push'](f['shift']());\n\
            } catch (h) { f['push'](f['shift']()); }\n\
          }\n\
        })(__STRING_ARRAY__, 0x3);\n\
        console.log(__DECODE_0__(0x0), __DECODE_0__(0x1), __DECODE_0__(0x2));\n";
    let (_code, notes) = common::run_passes(source);
    assert!(refused_before_the_rotator(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_patchit_helper_writes_through_a_passed_alias_refuses_the_unit() {
    let patch = "\
        var P = Array.prototype;\n\
        function patchIt(o) {\n\
          o.push = function (x) { return [x][0]; };\n\
          o.shift = function () { return 0; };\n\
        }\n\
        patchIt(P);";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn push_shift_after_patchit_helper_with_the_alias_declared_after_the_call_refuses_the_unit() {
    let patch = "\
        function patchIt(o) {\n\
          o.push = function (x) { return [x][0]; };\n\
          o.shift = function () { return 0; };\n\
        }\n\
        (function () {\n\
          try { patchIt(P); } catch (e) {}\n\
        })();\n\
        var P = Array.prototype;";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

const TERMINATING_PUSH_PATCH: &str =
    "var orig = q.push; q.push = function (x) { orig.call(this, x); return x; };";

#[test]
fn a_direct_prototype_source_passed_to_a_member_helper_refuses_the_unit() {
    let patch = format!(
        "var o = {{ f: function (q) {{ {TERMINATING_PUSH_PATCH} }} }};\n\
        o.f(Array.prototype);"
    );
    let (_code, notes) = common::run_passes(&rotator_source(&patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_direct_prototype_source_passed_to_an_iife_refuses_the_unit() {
    let patch = format!("(function (q) {{ {TERMINATING_PUSH_PATCH} }})(Array.prototype);");
    let (_code, notes) = common::run_passes(&rotator_source(&patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_direct_prototype_source_passed_through_a_reassigned_function_alias_refuses_the_unit() {
    let patch = format!(
        "function real(q) {{ {TERMINATING_PUSH_PATCH} }}\n\
        var h = real;\n\
        h(Array.prototype);"
    );
    let (_code, notes) = common::run_passes(&rotator_source(&patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_direct_prototype_source_passed_through_call_refuses_the_unit() {
    let patch = format!(
        "function real(q) {{ {TERMINATING_PUSH_PATCH} }}\n\
        real.call(null, Array.prototype);"
    );
    let (_code, notes) = common::run_passes(&rotator_source(&patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_helper_that_copies_its_parameter_to_a_local_still_refuses_the_unit() {
    let patch = "\
        function real(q) { var r = q; var orig = r.push; r.push = function (x) { orig.call(this, x); return x; }; }\n\
        real(Array.prototype);";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn the_core_js_dom_token_list_prototype_shape_still_decodes_pinned_as_r20() {
    let patch = "\
        function documentCreateElement(tag) {\n\
          return { classList: { constructor: function () {} } };\n\
        }\n\
        var classList = documentCreateElement('span').classList;\n\
        var P = classList && classList.constructor && classList.constructor.prototype;\n\
        var exported = P === Object.prototype ? undefined : P;";
    let (code, notes) = common::run_passes(&rotator_source(patch));
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(!code.contains("__DECODE_0__("), "{code}");
}

#[test]
fn a_known_receivers_constructor_prototype_passed_to_a_helper_refuses_the_unit() {
    let patch = "\
        function helper(q) { q.push = function (x) { return [x][0]; }; }\n\
        helper([].constructor.prototype);";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_never_written_known_receivers_constructor_prototype_passed_to_a_helper_refuses_the_unit() {
    let patch = "\
        function helper(q) { q.push = function (x) { return [x][0]; }; }\n\
        var a = [];\n\
        helper(a.constructor.prototype);";
    let (_code, notes) = common::run_passes(&rotator_source(patch));
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn an_unknown_receivers_constructor_prototype_still_decodes_pinned_as_r20() {
    let patch = "\
        function classify(x) {\n\
          var P = x && x.constructor && x.constructor.prototype;\n\
          return P === Object.prototype ? undefined : P;\n\
        }\n\
        classify({});";
    let (code, notes) = common::run_passes(&rotator_source(patch));
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(!code.contains("__DECODE_0__("), "{code}");
}

fn shadow_source(second_key: &str) -> String {
    format!(
        "\
        function __STRING_ARRAY__() {{ var h = ['log', 'x']; __STRING_ARRAY__ = function () {{ return h; }}; return __STRING_ARRAY__(); }}\n\
        function __DECODE_0__(c) {{ var e = __STRING_ARRAY__(); return e[c].toUpperCase(); }}\n\
        {{ let P = Array.prototype; P.foo = 1; }}\n\
        {{ let P = String.prototype; P.{second_key} = function () {{ return 999; }}; }}\n\
        console.log(__DECODE_0__(0x0));\n"
    )
}

#[test]
fn a_same_named_shadowed_alias_writing_a_different_key_still_decodes() {
    let source = shadow_source("charCodeAt");
    let (code, notes) = common::run_passes(&source);
    assert!(!escape_refused(&notes), "{notes:?}");
    assert!(!code.contains("__DECODE_0__("), "{code}");
}

#[test]
fn a_same_named_shadowed_alias_writing_the_read_key_refuses_the_unit() {
    let alloc = Allocator::default();
    let source = shadow_source("toUpperCase");
    let (_code, notes) = common::run_ctors(&alloc, &source, &[untwist_deobfuscate::decoded::boxed]);
    assert!(escape_refused(&notes), "{notes:?}");
}

#[test]
fn a_decoder_reading_the_first_of_two_same_named_shadowed_aliases_key_refuses_the_unit() {
    let source = "\
        function __STRING_ARRAY__() { var h = ['log', 'x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }\n\
        function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return String(e[c].charCodeAt(0)); }\n\
        { let P = String.prototype; P.charCodeAt = function () { return 999; }; }\n\
        { let P = String.prototype; P.toUpperCase = function () { return 'X'; }; }\n\
        console.log(__DECODE_0__(0x0));\n";
    let alloc = Allocator::default();
    let (_code, notes) = common::run_ctors(&alloc, source, &[untwist_deobfuscate::decoded::boxed]);
    assert!(escape_refused(&notes), "{notes:?}");
}
