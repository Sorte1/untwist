use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::eval_script_decoding;
use untwist_eval::value::AbruptKind;

fn bails_while_decoding(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    eval_script_decoding(&parsed.program, Budget::default(), |result| {
        matches!(
            result.map_err(|abrupt| abrupt.into_kind()),
            Err(AbruptKind::Bail(_))
        )
    })
}

#[test]
fn an_arguments_length_write_bails() {
    assert!(bails_while_decoding(
        "(function () { var a = (function () { return arguments; })(1, 2); a.length = 1; return [a.length, a[1]]; })()"
    ));
}

#[test]
fn an_arguments_callee_write_bails() {
    assert!(bails_while_decoding(
        "(function () { var a = (function () { return arguments; })(1); a.callee = 9; return a.callee; })()"
    ));
}

#[test]
fn a_new_key_on_an_arguments_object_is_written() {
    assert!(!bails_while_decoding(
        "(function () { var a = (function () { return arguments; })(1); a.k = 'v'; return a.k; })()"
    ));
}

#[test]
fn a_placeholder_constructor_read_bails() {
    assert!(bails_while_decoding(
        "(function () { return [[].constructor]; })()"
    ));
    assert!(bails_while_decoding(
        "(function () { return [({}).constructor]; })()"
    ));
}

#[test]
fn an_async_function_constructor_read_bails() {
    assert!(bails_while_decoding(
        "(function () { return [(async function () {}).constructor]; })()"
    ));
}

#[test]
fn an_intrinsic_prototype_bails() {
    assert!(bails_while_decoding(
        "(function () { return [Object.create(Math)]; })()"
    ));
}

#[test]
fn a_regexp_accessor_write_bails() {
    assert!(bails_while_decoding(
        "(function () { var r = /a/g; r.source = 'z'; return [r.source]; })()"
    ));
}

#[test]
fn a_for_of_head_that_reads_its_own_binding_bails() {
    assert!(bails_while_decoding(
        "(function () { var x = 'o'; for (const x of (typeof x, [1])) {} return ['no']; })()"
    ));
}

#[test]
fn regexp_flags_u_and_v_together_bail_while_decoding() {
    assert!(bails_while_decoding(
        "(function () { try { return [new RegExp('a', 'uv')]; } catch (e) { return [e instanceof SyntaxError]; } })()"
    ));
}

#[test]
fn catastrophic_backtracking_bails_while_decoding() {
    assert!(bails_while_decoding(&format!(
        "(function () {{ return [/^(a+)+b$/.test('{}c')]; }})()",
        "a".repeat(43)
    )));
}

#[test]
fn deleting_an_arguments_callee_bails_while_decoding() {
    assert!(bails_while_decoding(
        "(function () { 'use strict'; var o = (function () { return arguments; })(1, 2); delete o['callee']; return [o]; })()"
    ));
}

#[test]
fn deleting_a_regexp_last_index_bails_while_decoding() {
    assert!(bails_while_decoding(
        "(function () { 'use strict'; var r = /a/g; delete r.lastIndex; return [r.lastIndex]; })()"
    ));
}
