use untwist_core::pass::{PassCtor, PassOptions};
use untwist_testing::{transform_with, Allocator};
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

const DEFINE_HELPERS: &str = r#"function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
"#;

const SPREAD_HELPERS: &str = r#"function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
"#;

fn ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn default_lane(source: &str) -> String {
    lane(source, false)
}

fn lane(source: &str, unsafe_transforms: bool) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &ctors(),
        PassOptions {
            extended: true,
            unsafe_transforms,
            ..PassOptions::default()
        },
    )
}

fn agrees(source: &str) -> String {
    let code = default_lane(source);
    untwist_testing::node::assert_node_agrees(source, &code);
    code
}

#[test]
fn a_later_key_with_no_certain_spelling_keeps_the_redefinition_last() {
    for object in [
        r#"{ "1e+21": 1, 1e21: 2 }, "1e+21""#,
        r#"{ "1": 1, 1n: 2 }, "1""#,
        r#"{ "0.5": 1, .5: 2 }, "0.5""#,
        r#"{ "1000": 1, 1_000: 2 }, "1000""#,
        r#"{ "1": 1, 1: 2 }, "1""#,
    ] {
        let source = format!(
            "{DEFINE_HELPERS}function run() {{ var o = _defineProperty({object}, 3); return JSON.stringify(o); }}\nconsole.log(run());\n"
        );
        let code = agrees(&source);
        assert!(!code.contains("_defineProperty({"), "{object}\n{code}");
    }
}

#[test]
fn an_anonymous_function_spread_in_through_a_computed_key_keeps_its_empty_name() {
    for (value, converts) in [
        ("function () {}", true),
        ("() => {}", true),
        ("class {}", false),
    ] {
        let source = format!(
            "{SPREAD_HELPERS}function run(x, k) {{ var o = _objectSpread(_objectSpread({{}}, {{ p: x }}), {{}}, _defineProperty({{}}, k, {value})); return JSON.stringify(o[k].name); }}\nconsole.log(run(1, \"kk\"));\n"
        );
        agrees(&source);
        let code = lane(&source, true);
        untwist_testing::node::assert_node_agrees(&source, &code);
        assert_eq!(
            !code.contains("_objectSpread("),
            converts,
            "{value}\n{code}"
        );
    }
}
