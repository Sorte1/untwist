mod refused_rows;

use refused_rows::{check, Row};

const CONTROL_FLOW_DEAD_SHADOWED_UNDEFINED: &str = r#"(function(undefined){
var _0x1 = {'aB': function(a, b){ return a === b; }, 'cD': function(a, b){ return a + b; }};
if (_0x1.aB(undefined, 3)) { _0x1.cD = function(a, b){ return a * b; }; }
console.log(_0x1.cD(3, 4));
})(3);"#;

#[test]
fn control_flow_dead_shadowed_undefined() {
    check(&Row {
        source: CONTROL_FLOW_DEAD_SHADOWED_UNDEFINED,
        prints: "12\n",
        kept: r#"_0x1.cD(3, 4)"#,
    });
}

const CONTROL_FLOW_DEAD_SHADOWED_NAN: &str = r#"(function(NaN){
var _0x1 = {'aB': function(a, b){ return a === b; }, 'cD': function(a, b){ return a + b; }};
if (_0x1.aB(NaN, 3)) { _0x1.cD = function(a, b){ return a * b; }; }
console.log(_0x1.cD(3, 4));
})(3);"#;

#[test]
fn control_flow_dead_shadowed_nan() {
    check(&Row {
        source: CONTROL_FLOW_DEAD_SHADOWED_NAN,
        prints: "12\n",
        kept: r#"_0x1.cD(3, 4)"#,
    });
}

const CONTROL_FLOW_DEAD_SHADOWED_INFINITY: &str = r#"(function(Infinity){
var _0x1 = {'aB': function(a, b){ return a < b; }, 'cD': function(a, b){ return a + b; }};
if (_0x1.aB(Infinity, 4)) { _0x1.cD = function(a, b){ return a * b; }; }
console.log(_0x1.cD(3, 4));
})(3);"#;

#[test]
fn control_flow_dead_shadowed_infinity() {
    check(&Row {
        source: CONTROL_FLOW_DEAD_SHADOWED_INFINITY,
        prints: "12\n",
        kept: r#"_0x1.cD(3, 4)"#,
    });
}

const DEFAULT_PARAMETERS_SHADOWED_UNDEFINED: &str = r#"(function(undefined){ function f(a){ if (a === undefined) a = 1; return a; } console.log(f(), f(2), f(9)); })(9);"#;

#[test]
fn default_parameters_shadowed_undefined() {
    check(&Row {
        source: DEFAULT_PARAMETERS_SHADOWED_UNDEFINED,
        prints: "undefined 2 1\n",
        kept: r#"if (a === undefined)"#,
    });
}

const DEFAULT_PARAMETERS_ARGUMENTS_SHADOWED_UNDEFINED: &str = r#"(function(undefined){ function f(){ var a = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1; return a; } console.log(f(), f(2), f(9)); })(9);"#;

#[test]
fn default_parameters_arguments_shadowed_undefined() {
    check(&Row {
        source: DEFAULT_PARAMETERS_ARGUMENTS_SHADOWED_UNDEFINED,
        prints: "1 2 1\n",
        kept: r#"arguments[0] !== undefined"#,
    });
}

const EVALUATE_GLOBALS_WITH: &str =
    r#"with({atob:function(s){ return 'own:' + s; }}){ console.log(atob('aGk=')); }"#;

#[test]
fn evaluate_globals_with() {
    check(&Row {
        source: EVALUATE_GLOBALS_WITH,
        prints: "own:aGk=\n",
        kept: r#"atob("aGk=")"#,
    });
}

const FOLD_CHAIN_SHADOWED_UNDEFINED: &str =
    r#"(function(undefined){ console.log('a,b'.split(undefined).join('-')); })(',');"#;

#[test]
fn fold_chain_shadowed_undefined() {
    check(&Row {
        source: FOLD_CHAIN_SHADOWED_UNDEFINED,
        prints: "a-b\n",
        kept: r#""a,b".split(undefined).join("-")"#,
    });
}

const FOLD_CHAIN_SHADOWED_NAN: &str = r#"(function(NaN){ console.log('abc'.charAt(NaN)); })(1);"#;

#[test]
fn fold_chain_shadowed_nan() {
    check(&Row {
        source: FOLD_CHAIN_SHADOWED_NAN,
        prints: "b\n",
        kept: r#""abc".charAt(NaN)"#,
    });
}

const FOLD_CHAIN_SHADOWED_INFINITY: &str =
    r#"(function(Infinity){ console.log('abcdef'.slice(Infinity)); })(2);"#;

#[test]
fn fold_chain_shadowed_infinity() {
    check(&Row {
        source: FOLD_CHAIN_SHADOWED_INFINITY,
        prints: "cdef\n",
        kept: r#""abcdef".slice(Infinity)"#,
    });
}

const FOLD_CHAIN_WITH_NAN: &str = r#"with({NaN:2}){ console.log('abc'.charAt(NaN)); }"#;

#[test]
fn fold_chain_with_nan() {
    check(&Row {
        source: FOLD_CHAIN_WITH_NAN,
        prints: "c\n",
        kept: r#""abc".charAt(NaN)"#,
    });
}

const INFINITY_WITH: &str = r#"with({Infinity:3}){ console.log(1/0); }"#;

#[test]
fn infinity_with() {
    check(&Row {
        source: INFINITY_WITH,
        prints: "Infinity\n",
        kept: r#"1 / 0"#,
    });
}

const INFINITY_EVAL: &str = r#"(function(){ eval("var Infinity = 7"); console.log(1/0); })();"#;

#[test]
fn infinity_eval() {
    check(&Row {
        source: INFINITY_EVAL,
        prints: "Infinity\n",
        kept: r#"1 / 0"#,
    });
}

const JSON_PARSE_WITH: &str =
    r#"with({JSON:{parse:function(s){ return 'own:' + s; }}}){ console.log(JSON.parse('[1]')); }"#;

#[test]
fn json_parse_with() {
    check(&Row {
        source: JSON_PARSE_WITH,
        prints: "own:[1]\n",
        kept: r#"JSON.parse("[1]")"#,
    });
}

const NULLISH_SHADOWED_UNDEFINED: &str = r#"(function(undefined){ var a=0; var r = a !== null && a !== undefined ? a : 'd'; console.log(r); })(0);"#;

#[test]
fn nullish_shadowed_undefined() {
    check(&Row {
        source: NULLISH_SHADOWED_UNDEFINED,
        prints: "d\n",
        kept: r#"a !== undefined ? a : "d""#,
    });
}

const OPTIONAL_CHAINING_SHADOWED_UNDEFINED: &str = r#"(function(undefined){
  function f(a){ var t; return (t = a) === null || t === undefined ? undefined : t.b; }
  function g(a){ return a === null || a === undefined ? undefined : a.b; }
  console.log(f(null), f({b:1}), g(null), g({b:2}));
})(3);"#;

#[test]
fn optional_chaining_shadowed_undefined() {
    check(&Row {
        source: OPTIONAL_CHAINING_SHADOWED_UNDEFINED,
        prints: "3 1 3 2\n",
        kept: r#"(t = a) === null || t === undefined"#,
    });
}

const SIMPLE_ARRAY_SHADOWED_UNDEFINED: &str =
    r#"(function(undefined){ var a=['x', undefined]; console.log(a[0]+a[1]); })('Y');"#;

#[test]
fn simple_array_shadowed_undefined() {
    check(&Row {
        source: SIMPLE_ARRAY_SHADOWED_UNDEFINED,
        prints: "xY\n",
        kept: r#"a = ["x", undefined];"#,
    });
}

const SIMPLE_ARRAY_WITH_UNDEFINED: &str = r#"var o={undefined:'W'};
with(o){ (function(){ var a=['x', undefined]; console.log(a[0]+a[1]); })(); }"#;

#[test]
fn simple_array_with_undefined() {
    check(&Row {
        source: SIMPLE_ARRAY_WITH_UNDEFINED,
        prints: "xW\n",
        kept: r#"var a = ["x", undefined];"#,
    });
}

const SIMPLE_ARRAY_LET_UNDEFINED: &str =
    r#"{ let undefined='L'; var a=['x', undefined]; console.log(a[0]+a[1]); }"#;

#[test]
fn simple_array_let_undefined() {
    check(&Row {
        source: SIMPLE_ARRAY_LET_UNDEFINED,
        prints: "xL\n",
        kept: r#"var a = ["x", undefined];"#,
    });
}

const SIMPLE_ARRAY_EVAL_UNDEFINED: &str = r#"(function(){ eval("var undefined='E'"); var a=['x', undefined]; console.log(a[0]+a[1]); })();"#;

#[test]
fn simple_array_eval_undefined() {
    check(&Row {
        source: SIMPLE_ARRAY_EVAL_UNDEFINED,
        prints: "xE\n",
        kept: r#"var a = ["x", undefined];"#,
    });
}

const UNWRAP_COMMA_CALL_TDZ_UNDEFINED: &str = r#"function g(){ return 'g'; }
try { (function(){ (undefined, g)(); let undefined; console.log('no'); })(); } catch (e) { console.log(e.constructor.name); }"#;

#[test]
fn unwrap_comma_call_tdz_undefined() {
    check(&Row {
        source: UNWRAP_COMMA_CALL_TDZ_UNDEFINED,
        prints: "ReferenceError\n",
        kept: r#"(undefined, g)()"#,
    });
}

const UNWRAP_COMMA_CALL_TDZ_NAN: &str = r#"function g(){ return 'g'; }
try { (function(){ (NaN, g)(); let NaN; console.log('no'); })(); } catch (e) { console.log(e.constructor.name); }"#;

#[test]
fn unwrap_comma_call_tdz_nan() {
    check(&Row {
        source: UNWRAP_COMMA_CALL_TDZ_NAN,
        prints: "ReferenceError\n",
        kept: r#"(NaN, g)()"#,
    });
}

const UNWRAP_COMMA_CALL_TDZ_INFINITY: &str = r#"function g(){ return 'g'; }
try { (function(){ (-Infinity, g)(); let Infinity; console.log('no'); })(); } catch (e) { console.log(e.constructor.name); }"#;

#[test]
fn unwrap_comma_call_tdz_infinity() {
    check(&Row {
        source: UNWRAP_COMMA_CALL_TDZ_INFINITY,
        prints: "ReferenceError\n",
        kept: r#"(-Infinity, g)()"#,
    });
}

const VOID_TO_UNDEFINED_WITH: &str = r#"with({undefined:5}){ console.log(void 0); }"#;

#[test]
fn void_to_undefined_with() {
    check(&Row {
        source: VOID_TO_UNDEFINED_WITH,
        prints: "undefined\n",
        kept: r#"console.log(void 0)"#,
    });
}

const VOID_TO_UNDEFINED_EVAL: &str =
    r#"(function(){ eval("var undefined = 6"); console.log(void 0); })();"#;

#[test]
fn void_to_undefined_eval() {
    check(&Row {
        source: VOID_TO_UNDEFINED_EVAL,
        prints: "undefined\n",
        kept: r#"console.log(void 0)"#,
    });
}

const YODA_SHADOWED_UNDEFINED: &str =
    r#"(function(undefined){ console.log(undefined === (undefined = 1)); })(2);"#;

#[test]
fn yoda_shadowed_undefined() {
    check(&Row {
        source: YODA_SHADOWED_UNDEFINED,
        prints: "false\n",
        kept: r#"undefined === (undefined = 1)"#,
    });
}

const YODA_SHADOWED_NAN: &str = r#"(function(NaN){ console.log(NaN < (NaN = 0, 5)); })(9);"#;

#[test]
fn yoda_shadowed_nan() {
    check(&Row {
        source: YODA_SHADOWED_NAN,
        prints: "false\n",
        kept: r#"NaN < (NaN = 0, 5)"#,
    });
}

const YODA_SHADOWED_INFINITY: &str =
    r#"(function(Infinity){ console.log(Infinity == (Infinity = 4)); })(3);"#;

#[test]
fn yoda_shadowed_infinity() {
    check(&Row {
        source: YODA_SHADOWED_INFINITY,
        prints: "false\n",
        kept: r#"Infinity == (Infinity = 4)"#,
    });
}
