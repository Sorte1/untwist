mod refused_rows;

use refused_rows::{check, Row};

macro_rules! unit {
    ($decoder:literal) => {
        concat!(
            "function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }\n",
            $decoder,
            "\nfunction later(){ return b(1); }\nconsole.log(b(0)); console.log(later()); console.log(b(2));"
        )
    };
}

fn kept(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(b(0))",
    }
}

const CLOSURE_TESTS_IN_ON_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ return function(){ return (1 in Object.create(o.c)) ? 'y' : 'n'; }; }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b)(); } return b.c[i]; }"
);

#[test]
fn a_closure_testing_in_on_an_object_derived_from_the_memo_is_refused() {
    check(&kept(CLOSURE_TESTS_IN_ON_A_DERIVED_MEMO, "s0n\ns1n\ns2y\n"));
}

const CONSTRUCTOR_AFTER_THE_GATE_TESTS_IN_ON_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var K=function(o){ this.v = (1 in Object.create(o.c)) ? 'y' : 'n'; }; if (b.c[i] === undefined) { b.c[i] = e[i] + new K(b).v; } return b.c[i]; }"
);

#[test]
fn a_local_constructor_after_the_gate_testing_in_on_a_derived_memo_is_refused() {
    check(&kept(
        CONSTRUCTOR_AFTER_THE_GATE_TESTS_IN_ON_A_DERIVED_MEMO,
        "s0n\ns1n\ns2y\n",
    ));
}

const CALLEE_TESTS_IN_ON_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ return (0 in Object.create(o.c)) ? 'y' : 'n'; }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }"
);

#[test]
fn a_local_callee_testing_in_on_a_derived_memo_is_refused() {
    check(&kept(CALLEE_TESTS_IN_ON_A_DERIVED_MEMO, "s0n\ns1y\ns2y\n"));
}

const IN_ON_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } if (b.c[i] === undefined) { b.c[i] = e[i] + ((0 in Object.create(b.c)) ? 'y' : 'n'); } return b.c[i]; }"
);

#[test]
fn in_on_an_object_derived_from_the_memo_is_refused() {
    check(&kept(IN_ON_A_DERIVED_MEMO, "s0n\ns1y\ns2y\n"));
}

const MEMO_READ_THROUGH_A_PROTOTYPE: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } if (b.c[i] === undefined) { var p=Object.create(b.c); b.c[i] = e[i] + (p[0] === undefined ? 'n' : 'y'); } return b.c[i]; }"
);

#[test]
fn a_memo_key_read_through_a_derived_object_is_refused() {
    check(&kept(MEMO_READ_THROUGH_A_PROTOTYPE, "s0n\ns1y\ns2y\n"));
}

const TEMPLATE_IN_ON_A_DERIVED_MEMO: &str = unit!(
    "function b(c){ var e=a(); b=function(f){ if (b.g === undefined) { b.g = 1; b.c = {}; } if (b.c[f] === undefined) { b.c[f]=e[f]+((0 in Object.create(b.c)) ? 'y' : 'n'); } return b.c[f]; }; return b(c); }"
);

#[test]
fn a_template_testing_in_on_a_derived_memo_is_refused() {
    check(&kept(TEMPLATE_IN_ON_A_DERIVED_MEMO, "s0n\ns1y\ns2y\n"));
}

const METHOD_ENUMERATES_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var K=function(o){ this.o=o; }; K.prototype.n=function(){ var m=0; for (var k in Object.create(this.o.c)) m++; return m; }; if (b.c[i] === undefined) { b.c[i] = e[i] + new K(b).n(); } return b.c[i]; }"
);

#[test]
fn a_method_enumerating_an_object_derived_from_the_memo_is_refused() {
    check(&kept(METHOD_ENUMERATES_A_DERIVED_MEMO, "s00\ns11\ns22\n"));
}

const CALLEE_ENUMERATES_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ var p=Object.create(o.c), n=0; for (var k in p) n++; return n; }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }"
);

#[test]
fn a_local_callee_enumerating_a_derived_memo_is_refused() {
    check(&kept(CALLEE_ENUMERATES_A_DERIVED_MEMO, "s00\ns11\ns22\n"));
}

const CALLEE_ENUMERATES_A_DERIVED_MEMO_INLINE: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ var n=0; for (var k in Object.create(o.c)) n++; return n; }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }"
);

#[test]
fn a_local_callee_enumerating_a_derived_memo_inline_is_refused() {
    check(&kept(
        CALLEE_ENUMERATES_A_DERIVED_MEMO_INLINE,
        "s00\ns11\ns22\n",
    ));
}

const ENUMERATES_A_DERIVED_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } if (b.c[i] === undefined) { var p=Object.create(b.c), n=0; for (var k in p) n++; b.c[i] = e[i] + n; } return b.c[i]; }"
);

#[test]
fn enumerating_an_object_derived_from_the_memo_is_refused() {
    check(&kept(ENUMERATES_A_DERIVED_MEMO, "s00\ns11\ns22\n"));
}

const FOR_IN_OVER_AN_INHERITED_KEY: &str = unit!(
    "function b(i){ var e=a(); var p={x:1}; var o=Object.create(p), n=0; for (var k in o) n++; return e[i] + n; }"
);

#[test]
fn for_in_over_an_inherited_key_is_refused() {
    check(&kept(FOR_IN_OVER_AN_INHERITED_KEY, "s01\ns11\ns21\n"));
}

const MEMO_AS_THE_PROTO_OF_A_LITERAL: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } if (b.c[i] === undefined) { var o={__proto__: b.c}, n=0; for (var k in o) n++; b.c[i] = e[i] + n; } return b.c[i]; }"
);

#[test]
fn the_memo_as_the_proto_of_a_literal_is_refused() {
    check(&kept(MEMO_AS_THE_PROTO_OF_A_LITERAL, "s00\ns11\ns22\n"));
}

const PROTO_KEY_IN_A_LITERAL: &str =
    unit!("function b(i){ var e=a(); var o={__proto__: {x:'P'}}; return e[i] + o.x; }");

#[test]
fn a_proto_key_in_an_object_literal_is_refused() {
    check(&kept(PROTO_KEY_IN_A_LITERAL, "s0P\ns1P\ns2P\n"));
}

const FROZEN_MEMO_IN_THE_GATE: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; Object.freeze(b.c); } if (b.c[i] === undefined) { b.c[i] = e[i] + '!'; } return b.c[i]; }"
);

#[test]
fn a_memo_frozen_in_the_gate_is_refused() {
    check(&kept(
        FROZEN_MEMO_IN_THE_GATE,
        "undefined\nundefined\nundefined\n",
    ));
}

const MEMO_FROZEN_BY_A_CALLEE: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ Object.freeze(o.c); return ''; }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }"
);

#[test]
fn a_memo_frozen_by_a_local_callee_is_refused() {
    check(&kept(
        MEMO_FROZEN_BY_A_CALLEE,
        "undefined\nundefined\nundefined\n",
    ));
}

const ARRAY_FUNCTION_PASSED_TO_A_MUTATING_CALLEE: &str = unit!(
    "function b(i){ var e=a(); var L=function(o){ o().reverse(); return ''; }; return e[i] + L(a); }"
);

#[test]
fn the_array_function_passed_to_a_local_callee_is_refused() {
    check(&kept(
        ARRAY_FUNCTION_PASSED_TO_A_MUTATING_CALLEE,
        "s0\ns1\ns2\n",
    ));
}

const ARRAY_FUNCTION_IN_AN_ARRAY_LITERAL: &str = unit!(
    "function b(i){ var e=a(); var r=e[i]; [a].map(function(f){ return f(); })[0].reverse(); return r; }"
);

#[test]
fn the_array_function_inside_an_array_literal_is_refused() {
    check(&kept(ARRAY_FUNCTION_IN_AN_ARRAY_LITERAL, "s0\ns1\ns2\n"));
}

const ARRAY_FUNCTION_ALIAS_MUTATES: &str =
    unit!("function b(i){ var e=a(); var f=a; var r=e[i]; f().reverse(); return r; }");

#[test]
fn a_call_only_alias_of_the_array_function_that_mutates_is_refused() {
    check(&kept(ARRAY_FUNCTION_ALIAS_MUTATES, "s0\ns1\ns2\n"));
}

const ARRAY_FUNCTION_CALLED_THROUGH_CALL_THEN_MUTATED: &str =
    unit!("function b(i){ var e=a(); var r=e[i]; a.call().reverse(); return r; }");

#[test]
fn the_array_function_called_through_call_and_mutated_is_refused() {
    check(&kept(
        ARRAY_FUNCTION_CALLED_THROUGH_CALL_THEN_MUTATED,
        "s0\ns1\ns2\n",
    ));
}

const THIS_IN_A_PLAIN_CALL: &str = unit!("function b(i){ var e=a(); return e[i] + typeof this; }");

#[test]
fn this_in_a_sloppy_plain_call_is_refused() {
    check(&kept(
        THIS_IN_A_PLAIN_CALL,
        "s0object\ns1object\ns2object\n",
    ));
}

const THIS_IN_A_CALLBACK: &str = unit!(
    "function b(i){ var e=a(); return e[i] + [0].map(function(){ return typeof this; })[0]; }"
);

#[test]
fn this_in_a_sloppy_callback_is_refused() {
    check(&kept(THIS_IN_A_CALLBACK, "s0object\ns1object\ns2object\n"));
}

const PRIMITIVE_THIS: &str = unit!(
    "function b(i){ var e=a(); return e[i] + (function(){ return typeof this; }).call('x'); }"
);

#[test]
fn a_primitive_this_in_sloppy_code_is_refused() {
    check(&kept(PRIMITIVE_THIS, "s0object\ns1object\ns2object\n"));
}

const STRICT_THIS_IN_A_PLAIN_CALL: &str =
    unit!("function b(i){ 'use strict'; var e=a(); return e[i] + typeof this; }");

#[test]
fn this_in_a_strict_plain_call_still_decodes() {
    check(&Row {
        source: STRICT_THIS_IN_A_PLAIN_CALL,
        prints: "s0undefined\ns1undefined\ns2undefined\n",
        kept: "console.log(\"s0undefined\")",
    });
}

const THIS_UNDER_AN_ENCLOSING_STRICT_FUNCTION: &str = r#"(function(){ 'use strict';
function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i] + typeof this; }
console.log(b(0), b(1), b(2));
})();"#;

#[test]
fn this_under_an_enclosing_strict_function_still_decodes() {
    check(&Row {
        source: THIS_UNDER_AN_ENCLOSING_STRICT_FUNCTION,
        prints: "s0undefined s1undefined s2undefined\n",
        kept: "\"s0undefined\"",
    });
}

const ARGUMENTS_CALLEE: &str =
    unit!("function b(i){ var e=a(); return e[i] + typeof arguments.callee; }");

#[test]
fn arguments_callee_is_refused() {
    check(&kept(
        ARGUMENTS_CALLEE,
        "s0function\ns1function\ns2function\n",
    ));
}

const PARAMETER_WRITE_MAPPED_INTO_ARGUMENTS: &str =
    unit!("function b(i){ var e=a(); i = 1; return e[arguments[0]]; }");

#[test]
fn a_parameter_write_seen_through_arguments_is_refused() {
    check(&kept(PARAMETER_WRITE_MAPPED_INTO_ARGUMENTS, "s1\ns1\ns1\n"));
}

const CAPTURED_ARGUMENTS_READ_IN_THE_MAPPED_RANGE: &str = r#"function a() { var h = ['', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.k === undefined) { b.r = function (x, y) { return x + y; }; c = arguments; b.k = true; }
    g = g + '!';
    var n = f + e[0];
    var hit = c[n];
    var v;
    if (!hit) { v = b.r(e[f], g); c[n] = v; } else { v = hit; }
    return v;
  }), b(c, d);
}
function later() { return b(2, 'x'); }
console.log(b(1, 'x')); console.log(later()); console.log(b(2, 'x')); console.log(b(1, 'x'));"#;

#[test]
fn a_captured_arguments_index_read_in_the_mapped_range_is_refused() {
    check(&Row {
        source: CAPTURED_ARGUMENTS_READ_IN_THE_MAPPED_RANGE,
        prints: "x!\ns2x!\ns2x!\nx!\n",
        kept: "console.log(b(1, \"x\"))",
    });
}

const PROTO_ASSIGNED_THE_MEMO: &str = unit!(
    "function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } if (b.c[i] === undefined) { var o = {}; o.__proto__ = b.c; b.c[i] = e[i] + (o[0] === undefined ? 'n' : 'y'); } return b.c[i]; }"
);

#[test]
fn a_memo_key_read_through_an_object_whose_proto_key_was_assigned_the_memo_is_refused() {
    check(&kept(PROTO_ASSIGNED_THE_MEMO, "s0n\ns1y\ns2y\n"));
}

const STATIC_PROTO_ASSIGNMENT: &str =
    unit!("function b(i){ var e=a(); var o = {}; o.__proto__ = {x:'P'}; return e[i] + o.x; }");

#[test]
fn a_static_proto_assignment_is_refused() {
    check(&kept(STATIC_PROTO_ASSIGNMENT, "s0P\ns1P\ns2P\n"));
}

const COMPUTED_PROTO_ASSIGNMENT: &str =
    unit!("function b(i){ var e=a(); var o = {}; o['__proto__'] = {x:'P'}; return e[i] + o.x; }");

#[test]
fn a_computed_proto_assignment_is_refused() {
    check(&kept(COMPUTED_PROTO_ASSIGNMENT, "s0P\ns1P\ns2P\n"));
}

const VAR_ARGUMENTS_KEEPS_THE_MAPPING: &str = unit!(
    "function b(i){ var e=a(); return e[i] + '|' + (function(p){ var arguments; p = 'Z'; return arguments[0]; })('A'); }"
);

#[test]
fn a_parameter_write_seen_through_a_redeclared_arguments_is_refused() {
    check(&kept(VAR_ARGUMENTS_KEEPS_THE_MAPPING, "s0|Z\ns1|Z\ns2|Z\n"));
}

const PROTO_READ: &str = unit!(
    "function b(i){ var e=a(); return e[i] + '|' + (({}).__proto__ === Object.prototype) + ',' + ('abc'.__proto__ === String.prototype); }"
);

#[test]
fn a_proto_read_is_refused() {
    check(&kept(
        PROTO_READ,
        "s0|true,true\ns1|true,true\ns2|true,true\n",
    ));
}
