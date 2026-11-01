mod common;

use common::eval;

#[test]
fn decode_uri_component_round_trips_and_throws_on_garbage() {
    assert_eq!(
        eval("decodeURIComponent('%48%65%6c%6c%6f')"),
        Ok("string:Hello".to_string())
    );
    assert_eq!(
        eval("decodeURIComponent('%E2%9C%93')"),
        Ok("string:\u{2713}".to_string())
    );
    assert_eq!(
        eval("decodeURIComponent('%FF')"),
        Err("throw error:URIError".to_string())
    );
    assert_eq!(
        eval("decodeURIComponent('%')"),
        Err("throw error:URIError".to_string())
    );
}

#[test]
fn the_uri_error_is_catchable_which_is_how_the_rotator_advances() {
    assert_eq!(
        eval("var hit = 0; try { decodeURIComponent('%FF'); } catch (e) { hit = 1; } hit"),
        Ok("number:1".to_string())
    );
}

#[test]
fn encode_uri_component_matches_js() {
    assert_eq!(
        eval("encodeURIComponent('a b/c')"),
        Ok("string:a%20b%2Fc".to_string())
    );
    assert_eq!(eval("encodeURI('a b/c')"), Ok("string:a%20b/c".to_string()));
}

#[test]
fn escape_unescape_and_base64_are_available() {
    assert_eq!(eval("escape('a b')"), Ok("string:a%20b".to_string()));
    assert_eq!(eval("unescape('a%20b')"), Ok("string:a b".to_string()));
    assert_eq!(eval("btoa('Hi')"), Ok("string:SGk=".to_string()));
    assert_eq!(eval("atob('SGk=')"), Ok("string:Hi".to_string()));
}

#[test]
fn decode_uri_component_passes_through_literal_astral_characters() {
    assert_eq!(
        eval("decodeURIComponent('\u{1F600}')"),
        Ok("string:\u{1F600}".to_string())
    );
    assert_eq!(
        eval("decodeURIComponent('%F0%9F%98%80')"),
        Ok("string:\u{1F600}".to_string())
    );
    assert_eq!(
        eval("decodeURIComponent('%E0%A4%A')"),
        Err("throw error:URIError".to_string())
    );
}

#[test]
fn decode_uri_preserves_reserved_octets_but_component_decodes_them() {
    assert_eq!(eval("decodeURI('%3B%41')"), Ok("string:%3BA".to_string()));
    assert_eq!(
        eval("decodeURIComponent('%3B%41')"),
        Ok("string:;A".to_string())
    );
}

#[test]
fn atob_skips_ascii_whitespace_and_still_rejects_garbage() {
    assert_eq!(eval("atob('aGVs\\nbG8=')"), Ok("string:hello".to_string()));
    assert_eq!(eval("atob('aGVs bG8=')"), Ok("string:hello".to_string()));
    assert_eq!(
        eval("atob('a!b=')"),
        Err("throw error:TypeError".to_string())
    );
}

#[test]
fn atob_is_forgiving_base64_not_break_on_first_equals() {
    assert_eq!(eval("atob('')"), Ok("string:".to_string()));
    assert_eq!(eval("atob('aGVsbG8=')"), Ok("string:hello".to_string()));
    assert_eq!(
        eval("atob(' aG Vs\\nbG8= ')"),
        Ok("string:hello".to_string())
    );
    assert_eq!(eval("atob('aGVsbG8')"), Ok("string:hello".to_string()));
    assert_eq!(eval("atob('aGVsbA==')"), Ok("string:hell".to_string()));
    assert_eq!(eval("atob('a')"), Err("throw error:TypeError".to_string()));
    assert_eq!(eval("atob('a=')"), Err("throw error:TypeError".to_string()));
    assert_eq!(
        eval("atob('abcde')"),
        Err("throw error:TypeError".to_string())
    );
    assert_eq!(
        eval("atob('aGVsbG8=aaaa')"),
        Err("throw error:TypeError".to_string())
    );
    assert_eq!(
        eval("atob('aGVsbA=')"),
        Err("throw error:TypeError".to_string())
    );
}

#[test]
fn decode_uri_component_rejects_a_non_hex_percent_pair() {
    assert_eq!(
        eval("decodeURIComponent('%zz')"),
        Err("throw error:URIError".to_string())
    );
}

#[test]
fn decode_uri_rejects_malformed_input_the_same_way_as_the_component_form() {
    assert_eq!(
        eval("decodeURI('%')"),
        Err("throw error:URIError".to_string())
    );
    assert_eq!(
        eval("decodeURI('%FF')"),
        Err("throw error:URIError".to_string())
    );
}

#[test]
fn unescape_leaves_a_malformed_sequence_as_is() {
    assert_eq!(eval("unescape('%')"), Ok("string:%".to_string()));
    assert_eq!(eval("unescape('%u')"), Ok("string:%u".to_string()));
    assert_eq!(eval("unescape('%zz')"), Ok("string:%zz".to_string()));
}
