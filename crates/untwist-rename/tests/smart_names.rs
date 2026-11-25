mod common;

use common::{assert_scope_safe, rename};
use untwist_rename::{RenameConfig, RenameMode};

fn smart(source: &str) -> String {
    let output = rename(
        source,
        RenameConfig {
            mode: RenameMode::Smart,
            rename_all: false,
            extended: true,
        },
    );
    assert_scope_safe(source, &output);
    output
}

fn assert_has(code: &str, expected: &[&str]) {
    for want in expected {
        assert!(code.contains(want), "missing {want:?} in:\n{code}");
    }
}

fn assert_lacks(code: &str, unwanted: &[&str]) {
    for bad in unwanted {
        assert!(!code.contains(bad), "unexpected {bad:?} in:\n{code}");
    }
}

#[test]
fn short_coordinate_properties_name_their_values() {
    let code = smart(
        "function f(_0x1) { const _0x2 = _0x1.x; const _0x3 = _0x1.y; const _0x4 = _0x1.width; return _0x2 + _0x3 + _0x4; }",
    );
    assert_has(&code, &["const x = ", "const y = ", "const width = "]);
}

#[test]
fn a_member_value_keeps_its_property_name() {
    let code = smart(
        "function f() { const _0x1 = Math.floor; const _0x2 = Math.max; return _0x1(_0x2(1, 2)); }",
    );
    assert_has(&code, &["const floor = Math.floor", "const max = Math.max"]);
}

#[test]
fn a_holder_write_still_beats_the_member_name() {
    let code = smart("const _0x1 = Math.floor; window.roundDown = _0x1;");
    assert_has(&code, &["const roundDown = Math.floor"]);
}

#[test]
fn an_element_of_a_singular_property_takes_the_property_name() {
    let code = smart("function f(_0x1) { const _0x2 = _0x1.trigCond[0]; return _0x2; }");
    assert_has(&code, &["const trigCond = "]);
}

#[test]
fn the_last_index_of_a_collection_is_named() {
    let code = smart("function f(_0x1) { const _0x2 = _0x1.length - 1; return _0x1[_0x2]; }");
    assert_has(&code, &["const lastIndex = "]);
}

#[test]
fn a_this_alias_is_named_self() {
    let code = smart("function F() { const _0x1 = this; return function () { return _0x1; }; }");
    assert_has(&code, &["const self = this"]);
}

#[test]
fn a_this_alias_does_not_capture_the_global_self() {
    let code = smart("function F() { const _0x1 = this; return self.x + _0x1.y; }");
    assert_has(&code, &["self.x"]);
    assert_lacks(&code, &["const self = this"]);
}

#[test]
fn a_binding_assigned_only_functions_is_named_fn() {
    let code = smart(
        "function f(_0x9) { let _0x1; if (_0x9) { _0x1 = function () { return 1; }; } else { _0x1 = () => 2; } return [_0x1]; }",
    );
    assert_has(&code, &["let fn;", "fn = function", "fn = () =>"]);
}

#[test]
fn an_immediately_called_function_value_is_a_result() {
    let code = smart("function f(_0x9) { const _0x1 = function (_0x2) { return _0x2 + 1; }(_0x9); return _0x1; }");
    assert_has(&code, &["const result = function"]);
}

#[test]
fn an_awaited_call_is_named_after_the_call_without_its_accessor_prefix() {
    let code = smart(
        "async function f(_0x9) { const _0x1 = await _0x9.profile.getLeaderboard(40); const _0x2 = await _0x9.fetchPingList(); return [_0x1, _0x2]; }",
    );
    assert_has(
        &code,
        &["const leaderboard = await", "const pingList = await"],
    );
}

#[test]
fn an_awaited_verb_call_does_not_take_the_verb() {
    let code = smart("async function f(_0x9) { const _0x1 = await _0x9.pushManager.subscribe({}); return _0x1; }");
    assert_lacks(&code, &["const subscribe ="]);
}

#[test]
fn a_lookup_result_is_a_value_or_an_element_of_the_receiver() {
    let code = smart(
        "function f(map, players, _0x9) { let _0x1 = map.get(_0x9); let _0x2 = players.get(_0x9); return [_0x1, _0x2]; }",
    );
    assert_has(&code, &["let value = map.get", "let player = players.get"]);
    assert_lacks(&code, &["let get"]);
}

#[test]
fn an_arithmetic_result_does_not_take_the_verb() {
    let code = smart(
        "function f(_0x9, vector) { const _0x1 = _0x9.mul(0.07); const _0x2 = vector.add(_0x1); const _0x3 = _0x9.div(2); return [_0x2, _0x3]; }",
    );
    assert_lacks(&code, &["const mul", "const add", "const div"]);
    assert_has(&code, &["const vector2 = vector.add"]);
}

#[test]
fn a_conversion_result_drops_the_to_prefix() {
    let code = smart(
        "function f(_0x9) { const _0x1 = _0x9.toJSON(); const _0x2 = _0x9.toVar(); return [_0x1, _0x2]; }",
    );
    assert_has(&code, &["const json = "]);
    assert_lacks(&code, &["const toVar", "const toJSON"]);
}

#[test]
fn a_noun_like_method_result_keeps_its_name() {
    let code = smart("function f(_0x9) { let _0x1 = Math.floor(_0x9); const _0x2 = Math.sqrt(_0x9); return _0x1 + _0x2; }");
    assert_has(&code, &["let floor = Math.floor", "const sqrt = Math.sqrt"]);
}

#[test]
fn an_alias_of_a_kept_name_takes_that_name() {
    let code = smart(
        "var surfNorm = make().toVar(); function f() { const _0x1 = surfNorm; return _0x1.cross(_0x1); }",
    );
    assert_has(&code, &["const surfNorm2 = surfNorm"]);
    assert_lacks(&code, &["const toVar"]);
}

#[test]
fn an_awaited_call_keeps_words_that_are_not_read_prefixes() {
    let code = smart(
        "async function f(_0x9) { const _0x1 = await _0x9.createOrUpdateRecruitment(1); const _0x2 = await _0x9.loadAsync(2); const _0x3 = await _0x9.loadTextureAsync(3); return [_0x1, _0x2, _0x3]; }",
    );
    assert_has(
        &code,
        &[
            "const createOrUpdateRecruitment = await",
            "const texture = await",
        ],
    );
    assert_lacks(
        &code,
        &["const orUpdateRecruitment", "const async", "const load ="],
    );
}

#[test]
fn a_reassigned_this_alias_takes_its_assigned_name() {
    let code = smart(
        "function f(_0x9) { let _0x1 = this; _0x1 = _0x9.next; while (_0x1) _0x1 = _0x1.next; return _0x1; }",
    );
    assert_has(&code, &["let next = this"]);
    assert_lacks(&code, &["self", "getSelf"]);
}

#[test]
fn a_reassigned_last_index_takes_its_assigned_name() {
    let code = smart(
        "function f(_0x9) { let _0x1 = _0x9.length - 1; _0x1--; _0x1 = _0x9.other; return _0x1; }",
    );
    assert_has(&code, &["let other = "]);
    assert_lacks(&code, &["lastIndex"]);
}

#[test]
fn a_reassigned_member_value_does_not_keep_its_first_property() {
    let code =
        smart("function f(_0x9) { var _0x1 = _0x9.x; _0x1 = _0x9.y; _0x1 = _0x9.z; return _0x1; }");
    assert_lacks(&code, &["var x = "]);
}

#[test]
fn the_property_a_value_is_stored_under_beats_its_declared_name() {
    let code = smart(
        "function f(_0x9, _0x8) { var _0x1 = this; _0x8.owner = _0x1; var _0x2 = _0x9.length - 1; _0x8.cursor = _0x2; return [_0x1, _0x2]; }",
    );
    assert_has(&code, &["var owner = this", "var cursor = "]);
}

#[test]
fn an_entry_of_caches_is_a_cache() {
    let code = smart("function f(_0x1) { const _0x2 = _0x1.caches.get('a'); return _0x2; }");
    assert_has(&code, &["const cache = "]);
}
