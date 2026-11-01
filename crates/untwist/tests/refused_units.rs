mod refused_rows;

use refused_rows::{check, Row};

fn counter(source: &'static str) -> Row {
    Row {
        source,
        prints: "s01\ns12\ns23\n",
        kept: "console.log(b(0))",
    }
}

const NEW_RETURNS_A_SHARED_FUNCTION_PROPERTY: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var o=new (function(){ return b.call; })(); o.n=(o.n||0)+1; return e[i]+o.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_counter_on_what_new_returns_is_refused() {
    check(&counter(NEW_RETURNS_A_SHARED_FUNCTION_PROPERTY));
}

const FRESH_FUNCTIONS_PROTOTYPE_REPLACED: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var F=function(){}; F.prototype=b.call; F.prototype.n=(F.prototype.n||0)+1; return e[i]+F.prototype.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_counter_on_a_replaced_prototype_of_a_local_function_is_refused() {
    check(&counter(FRESH_FUNCTIONS_PROTOTYPE_REPLACED));
}

const NEW_OF_A_LOCAL_FUNCTION_RETURNING_A_SHARED_OBJECT: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); function K(){ return b.call; } var o=new K(); o.n=(o.n||0)+1; return e[i]+o.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_counter_on_a_local_constructors_shared_return_is_refused() {
    check(&counter(NEW_OF_A_LOCAL_FUNCTION_RETURNING_A_SHARED_OBJECT));
}

const TEMPLATE_NEW_RETURNS_A_SHARED_OBJECT: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f,g){ var e=a(); return (b=function(f,g){ var o=new (function(){ return b.call; })(); o.n=(o.n||0)+1; return e[f]+o.n; }), b(f,g); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_counter_on_what_new_returns_inside_the_template_is_refused() {
    check(&counter(TEMPLATE_NEW_RETURNS_A_SHARED_OBJECT));
}

fn skipped_gate(source: &'static str) -> Row {
    Row {
        source,
        prints: "s0-\ns1q\ns2q\n",
        kept: "console.log(b(0))",
    }
}

const GATE_UNDER_A_BRANCH_ON_A_PARAMETER: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (i > 0) { if (b.g === undefined) { b.g = 1; b.x = 'q'; } } return e[i] + (b.x || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_gate_under_a_branch_on_a_parameter_is_refused() {
    check(&skipped_gate(GATE_UNDER_A_BRANCH_ON_A_PARAMETER));
}

const GATE_AFTER_AN_EARLY_RETURN: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (i === 0) return e[i] + (b.x || '-'); if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + b.x; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_gate_after_an_early_return_is_refused() {
    check(&skipped_gate(GATE_AFTER_AN_EARLY_RETURN));
}

const GATE_IN_A_LOOP_OVER_A_PARAMETER: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); for (var k = 0; k < i; k++) { if (b.g === undefined) { b.g = 1; b.x = 'q'; } } return e[i] + (b.x || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_gate_in_a_loop_over_a_parameter_is_refused() {
    check(&skipped_gate(GATE_IN_A_LOOP_OVER_A_PARAMETER));
}

const GATE_IN_A_TRY_AFTER_A_THROW: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); try { if (i === 0) throw 0; if (b.g === undefined) { b.g = 1; b.x = 'q'; } } catch (z) {} return e[i] + (b.x || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_gate_in_a_try_after_a_throw_is_refused() {
    check(&skipped_gate(GATE_IN_A_TRY_AFTER_A_THROW));
}

const GATE_KEY_READ_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var t=b.x; if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + (t || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_gate_key_read_before_the_gate_is_refused() {
    check(&skipped_gate(GATE_KEY_READ_BEFORE_THE_GATE));
}

fn flag_read(source: &'static str) -> Row {
    Row {
        source,
        prints: "s0-\ns11\ns21\n",
        kept: "console.log(b(0))",
    }
}

const CONDITIONAL_GATES_FLAG_READ_OUTSIDE_ITS_TEST: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (i > 0) { if (b.g === undefined) { b.g = 1; } } return e[i] + (b.g || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_conditional_gates_flag_read_outside_its_test_is_refused() {
    check(&flag_read(CONDITIONAL_GATES_FLAG_READ_OUTSIDE_ITS_TEST));
}

const GATE_FLAG_READ_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var t=b.g; if (b.g === undefined) { b.g = 1; } return e[i] + (t || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_gate_flag_read_before_the_gate_is_refused() {
    check(&flag_read(GATE_FLAG_READ_BEFORE_THE_GATE));
}

const CONDITIONAL_GATE_WRITES_A_SECOND_KEY_AT_RUN_TIME: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.c === undefined) { b.c = 1; b.x = 'p'; } if (i >= 0) { if (b.g === undefined) { (function(){ this.x = 'q'; }).call(b); b.g = true; } } return e[i] + b.x; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_conditional_gate_writing_a_second_key_at_run_time_is_refused() {
    check(&Row {
        source: CONDITIONAL_GATE_WRITES_A_SECOND_KEY_AT_RUN_TIME,
        prints: "s0q\ns1q\ns2q\n",
        kept: "console.log(b(0))",
    });
}

const HELPER_REGISTERED_BY_A_GATE_KEY_WRITE: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  if (b.g === undefined) {
    b.c = {};
    b.h = function (f2) { if (f2 === 'z') { this.g = true; return ''; } if (b.c[f2] === undefined) { b.c[f2] = a()[f2] + this.k; } return b.c[f2]; };
    b.h.call(b.h, 'z');
    b.g = true;
  }
  return b.h.call({ k: g }, f) + (e[f] ? '' : '');
}
function later() { return b(0, 'y'); }
console.log(b(0, 'x')); console.log(later()); console.log(b(1, 'x'));"#;

#[test]
fn a_helper_that_a_gate_key_write_would_register_as_the_decoder_is_refused() {
    check(&Row {
        source: HELPER_REGISTERED_BY_A_GATE_KEY_WRITE,
        prints: "s0x\ns0x\ns1x\n",
        kept: "console.log(b(0, \"x\"))",
    });
}

const TEMPLATE_GATE_AND_MEMO_ON_THE_REWRITTEN_CLOSURE: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f, g);
}
function later() { return b(1); }
console.log(b(0), later(), b(2), b(1));"#;

#[test]
fn the_templates_gate_and_memo_on_its_rewritten_closure_still_decode() {
    check(&Row {
        source: TEMPLATE_GATE_AND_MEMO_ON_THE_REWRITTEN_CLOSURE,
        prints: "s0 s1 s2 s1\n",
        kept: "console.log(\"s0\", later(), \"s2\", \"s1\")",
    });
}

const UNIT_DECLARED_IN_A_BLOCK: &str = r#"{ function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; } }
console.log(c(0), typeof a, c.name);"#;

#[test]
fn a_unit_declared_in_a_block_and_called_outside_it_is_kept() {
    check(&Row {
        source: UNIT_DECLARED_IN_A_BLOCK,
        prints: "s0 function c\n",
        kept: "console.log(c(0), typeof a, c.name)",
    });
}

const STATELESS_UNIT_DECLARED_IN_A_BLOCK: &str = r#"{ function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; } }
console.log(b(0), b(1), typeof a);"#;

#[test]
fn a_stateless_unit_declared_in_a_block_and_called_outside_it_is_kept() {
    check(&Row {
        source: STATELESS_UNIT_DECLARED_IN_A_BLOCK,
        prints: "s0 s1 function\n",
        kept: "console.log(b(0), b(1), typeof a)",
    });
}

const HOISTED_FUNCTION_READS_A_GATE_KEY_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var t=h(); if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + (t || '-'); function h(){ return b.x; } }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_hoisted_function_reading_a_gate_key_before_the_gate_is_refused() {
    check(&skipped_gate(
        HOISTED_FUNCTION_READS_A_GATE_KEY_BEFORE_THE_GATE,
    ));
}

const HAS_OWN_PROPERTY_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var t=b.hasOwnProperty('x') ? 'q' : '-'; if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + t; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn an_inherited_method_read_on_a_gate_container_is_refused() {
    check(&skipped_gate(HAS_OWN_PROPERTY_BEFORE_THE_GATE));
}

const CONDITIONAL_FLAG_READ_BY_HAS_OWN_PROPERTY: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (i > 0) { if (b.g === undefined) { b.g = 1; } } return e[i] + (b.hasOwnProperty('g') ? 1 : '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_conditional_flag_read_by_has_own_property_is_refused() {
    check(&flag_read(CONDITIONAL_FLAG_READ_BY_HAS_OWN_PROPERTY));
}

const ALIAS_READS_A_GATE_KEY_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var z=b; var t=z.x; if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + (t || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn an_alias_reading_a_gate_key_before_the_gate_is_refused() {
    check(&skipped_gate(ALIAS_READS_A_GATE_KEY_BEFORE_THE_GATE));
}

const IN_READS_A_GATE_KEY_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var t=('x' in b) ? 'q' : '-'; if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + t; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn an_in_test_of_a_gate_key_before_the_gate_is_refused() {
    check(&skipped_gate(IN_READS_A_GATE_KEY_BEFORE_THE_GATE));
}

const OBJECT_KEYS_BEFORE_THE_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var t=Object.keys(b).length; if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + t; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn object_keys_of_a_gate_container_before_the_gate_is_refused() {
    check(&Row {
        source: OBJECT_KEYS_BEFORE_THE_GATE,
        prints: "s00\ns12\ns22\n",
        kept: "console.log(b(0))",
    });
}

const CONDITIONAL_FLAG_READ_BY_IN: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (i > 0) { if (b.g === undefined) { b.g = 1; } } return e[i] + (('g' in b) ? 1 : '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_conditional_flag_read_by_in_is_refused() {
    check(&flag_read(CONDITIONAL_FLAG_READ_BY_IN));
}

const CONDITIONAL_FLAG_READ_THROUGH_AN_ALIAS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var z=b; if (i > 0) { if (b.g === undefined) { b.g = 1; } } return e[i] + (z.g || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_conditional_flag_read_through_an_alias_is_refused() {
    check(&flag_read(CONDITIONAL_FLAG_READ_THROUGH_AN_ALIAS));
}

const CONDITIONAL_FLAG_COUNTED_BY_OBJECT_KEYS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (i > 0) { if (b.g === undefined) { b.g = 1; } } return e[i] + Object.keys(b).length; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_conditional_flag_counted_by_object_keys_is_refused() {
    check(&Row {
        source: CONDITIONAL_FLAG_COUNTED_BY_OBJECT_KEYS,
        prints: "s00\ns11\ns21\n",
        kept: "console.log(b(0))",
    });
}

const ALIAS_CALLED_AND_USED_AS_A_MEMBER_OBJECT: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var z=b; var t=i === 9 ? z(0) : z.x; if (b.g === undefined) { b.g = 1; b.x = 'q'; } return e[i] + (t || '-'); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn an_alias_that_is_also_a_member_object_is_refused() {
    check(&skipped_gate(ALIAS_CALLED_AND_USED_AS_A_MEMBER_OBJECT));
}

const CONTAINER_ARGUMENT_WHILE_A_CONDITIONAL_GATE_EXISTS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; } if (i > 0) { if (b.h === undefined) { b.h = 1; } } var L=function(o){ return o.h ? 1 : '-'; }; return e[i] + L(b); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_container_argument_while_a_conditional_gate_exists_is_refused() {
    check(&flag_read(
        CONTAINER_ARGUMENT_WHILE_A_CONDITIONAL_GATE_EXISTS,
    ));
}

const CONTAINER_ARGUMENT_AFTER_THE_ONLY_GATE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.x = 'q'; } var L=function(o){ return o.x; }; return e[i] + L(b); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_container_argument_after_its_only_gate_still_decodes() {
    check(&Row {
        source: CONTAINER_ARGUMENT_AFTER_THE_ONLY_GATE,
        prints: "s0q\ns1q\ns2q\n",
        kept: "console.log(\"s0q\")",
    });
}

fn observed(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(b(0))",
    }
}

const MEMO_KEYS_OBSERVED_BY_A_LOCAL_CALLEE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ return Object.keys(o.c).length; }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn memo_keys_counted_by_a_local_callee_are_refused() {
    check(&observed(
        MEMO_KEYS_OBSERVED_BY_A_LOCAL_CALLEE,
        "s00\ns11\ns22\n",
    ));
}

const MEMO_KEY_TESTED_BY_IN_IN_A_LOCAL_CALLEE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ return ((0 in o.c) ? 'y' : 'n'); }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_memo_key_tested_by_in_in_a_local_callee_is_refused() {
    check(&observed(
        MEMO_KEY_TESTED_BY_IN_IN_A_LOCAL_CALLEE,
        "s0n\ns1y\ns2y\n",
    ));
}

const MEMO_KEY_TESTED_BY_HAS_OWN_PROPERTY_IN_A_LOCAL_CALLEE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ return (o.c.hasOwnProperty(0) ? 'y' : 'n'); }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_memo_key_tested_by_has_own_property_in_a_local_callee_is_refused() {
    check(&observed(
        MEMO_KEY_TESTED_BY_HAS_OWN_PROPERTY_IN_A_LOCAL_CALLEE,
        "s0n\ns1y\ns2y\n",
    ));
}

const MEMO_KEYS_ENUMERATED_BY_FOR_IN_IN_A_LOCAL_CALLEE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); if (b.g === undefined) { b.g = 1; b.c = {}; } var L=function(o){ return (function(){ var n=0; for (var k in o.c) n++; return n; })(); }; if (b.c[i] === undefined) { b.c[i] = e[i] + L(b); } return b.c[i]; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn memo_keys_enumerated_by_for_in_in_a_local_callee_are_refused() {
    check(&observed(
        MEMO_KEYS_ENUMERATED_BY_FOR_IN_IN_A_LOCAL_CALLEE,
        "s00\ns11\ns22\n",
    ));
}

const TEMPLATE_MEMO_READS_ARGUMENTS_LENGTH: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(c, d){ var e = a(); b = function(f, g){ if (b.g === undefined) { b.c = {}; b.g = 1; } if (b.c[f] === undefined) { b.c[f] = e[f] + (arguments.length); } return b.c[f]; }; return b(c, d); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_template_memo_reading_arguments_length_is_refused() {
    check(&Row {
        source: TEMPLATE_MEMO_READS_ARGUMENTS_LENGTH,
        prints: "s02\ns11\ns21\n",
        kept: "console.log(b(0))",
    });
}

const TEMPLATE_FORWARD_WITH_A_DEFAULT: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(c, d = 'D'){ var e = a(); b = function(f, g){ if (b.g === undefined) { b.c = {}; b.g = 1; } if (b.c[f] === undefined) { b.c[f] = e[f] + g; } return b.c[f]; }; return b(c, d); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_template_whose_outer_parameter_has_a_default_is_refused() {
    check(&Row {
        source: TEMPLATE_FORWARD_WITH_A_DEFAULT,
        prints: "s0D\ns1undefined\ns2undefined\n",
        kept: "console.log(b(0))",
    });
}

const STATELESS_TEMPLATE_READS_ARGUMENTS_LENGTH: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(c, d){ var e = a(); b = function(f, g){ return e[f] + arguments.length; }; return b(c, d); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_stateless_template_reading_arguments_length_is_refused() {
    check(&Row {
        source: STATELESS_TEMPLATE_READS_ARGUMENTS_LENGTH,
        prints: "s02\ns11\ns21\n",
        kept: "console.log(b(0))",
    });
}

const TEMPLATE_WRITES_A_PARAMETER_ARGUMENTS_MAPS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(c, d){ var e = a(); b = function(f, g){ g = 'z'; return e[f] + arguments[1]; }; return b(c, d); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

#[test]
fn a_template_writing_a_parameter_that_arguments_maps_is_refused() {
    check(&Row {
        source: TEMPLATE_WRITES_A_PARAMETER_ARGUMENTS_MAPS,
        prints: "s0z\ns1undefined\ns2undefined\n",
        kept: "console.log(b(0))",
    });
}

const TEMPLATE_READS_A_FORWARDED_ARGUMENT_BY_INDEX: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(c, d){ var e = a(); b = function(f, g){ return e[arguments[0]]; }; return b(c, d); }
console.log(b(0), b(1), b(2));"#;

#[test]
fn a_template_reading_a_forwarded_argument_by_literal_index_still_decodes() {
    check(&Row {
        source: TEMPLATE_READS_A_FORWARDED_ARGUMENT_BY_INDEX,
        prints: "s0 s1 s2\n",
        kept: "console.log(\"s0\", \"s1\", \"s2\")",
    });
}

const ANNEX_B_OVER_AN_OUTER_BINDING: &str = r#"var b = 'outer';
function f(){ { function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), b(1)); } return typeof b; }
console.log(f(), b);"#;

#[test]
fn a_block_unit_hoisted_over_an_outer_binding_is_kept() {
    check(&Row {
        source: ANNEX_B_OVER_AN_OUTER_BINDING,
        prints: "s0 s1\nfunction outer\n",
        kept: "console.log(b(0), b(1))",
    });
}

const ANNEX_B_IN_A_SWITCH_CASE: &str = r#"var b = 'outer';
function f(k){ switch (k) { case 1: function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), b(1)); } return typeof b; }
console.log(f(1), b);"#;

#[test]
fn a_switch_case_unit_hoisted_over_an_outer_binding_is_kept() {
    check(&Row {
        source: ANNEX_B_IN_A_SWITCH_CASE,
        prints: "s0 s1\nfunction outer\n",
        kept: "console.log(b(0), b(1))",
    });
}

const ANNEX_B_ARRAY_OVER_AN_OUTER_BINDING: &str = r#"var a = 'outer';
function f(){ { function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), b(1)); } return typeof a; }
console.log(f(), a);"#;

#[test]
fn a_block_array_hoisted_over_an_outer_binding_is_kept() {
    check(&Row {
        source: ANNEX_B_ARRAY_OVER_AN_OUTER_BINDING,
        prints: "s0 s1\nfunction outer\n",
        kept: "console.log(b(0), b(1))",
    });
}
