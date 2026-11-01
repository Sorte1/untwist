use untwist::{Options, Pipeline};
use untwist_testing::node::assert_node_agrees;

const HELPERS: &str = r#"function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function show(o) { console.log(JSON.stringify(o), Object.keys(o).join()); }
"#;

const DUPLICATE_NOTE: &str = "more than once or through __proto__";

fn deobfuscated(body: &str) -> (String, String, Vec<String>) {
    let source = format!("{HELPERS}{body}");
    let out = Pipeline::new(Options::default()).run(&source);
    let notes = out.notes.iter().map(|note| note.message.clone()).collect();
    (source, out.code().to_string(), notes)
}

fn occurrences(code: &str, key: &str) -> usize {
    code.matches(&format!("{key}:")).count() + code.matches(&format!("\"{key}\":")).count()
}

#[test]
fn a_duplicate_key_with_side_effect_free_values_becomes_one_property() {
    let (source, code, notes) = deobfuscated(
        r#"var o = _defineProperty(_defineProperty(_defineProperty({ teams: true, lives: 3 }, "teams", true), "mode", "ffa"), "lives", 5);
show(o);
console.log(o.teams, o.lives);
"#,
    );
    assert_node_agrees(&source, &code);
    assert_eq!(occurrences(&code, "teams"), 1, "{code}");
    assert_eq!(occurrences(&code, "lives"), 1, "{code}");
    assert!(
        !notes.iter().any(|n| n.contains(DUPLICATE_NOTE)),
        "{notes:?}"
    );
}

#[test]
fn an_earlier_value_with_a_side_effect_keeps_both_properties() {
    let (source, code, _) = deobfuscated(
        r#"var log = [];
var o = _defineProperty(_defineProperty({ a: log.push("first") }, "b", 2), "a", 3);
show(o);
console.log(log.join());
"#,
    );
    assert_node_agrees(&source, &code);
    assert!(
        occurrences(&code, "a") != 1 || code.contains("log.push"),
        "{code}"
    );
}

#[test]
fn a_later_value_with_a_side_effect_is_not_moved_before_earlier_ones() {
    let (source, code, _) = deobfuscated(
        r#"var log = [];
var o = _defineProperty(_defineProperty({ a: 1, b: log.push("b") }, "c", 2), "a", log.push("a"));
show(o);
console.log(log.join());
"#,
    );
    assert_node_agrees(&source, &code);
}

#[test]
fn a_getter_setter_proto_or_computed_key_blocks_the_collapse() {
    for body in [
        r#"var o = _defineProperty({ get a() { return 1; } }, "a", 2); show(o);"#,
        r#"var o = _defineProperty({ __proto__: null, b: 1 }, "__proto__", 2); console.log(Object.getPrototypeOf(o) === null, Object.keys(o).join());"#,
        r#"var k = "a"; var o = _defineProperty(_defineProperty({ a: 1, [k]: 2 }, "c", 3), "a", 4); show(o);"#,
        r#"var o = _defineProperty(_defineProperty({ a: 1, a: 2, b: 0 }, "c", 3), "a", 4); show(o);"#,
        r#"var s = { a: 9 }; var o = _defineProperty(_defineProperty({ a: 1, ...s }, "c", 3), "a", 4); show(o);"#,
    ] {
        let (source, code, _) = deobfuscated(body);
        assert_node_agrees(&source, &code);
    }
}
