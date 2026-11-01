mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused};
use refused_rows::{check, check_with};

const TOLERATE_NEWER_NODE_UNICODE: bool = true;

const REFUSES_REGEXP_MODIFIERS_CTOR: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return RegExp('(?i:a)b').test('Ab'); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_modifiers_ctor() {
    check(&q_refused(
        REFUSES_REGEXP_MODIFIERS_CTOR,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const REFUSES_REGEXP_LETTER_CLASS_NEW_IN_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/\p{L}/u.test('꟎')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_letter_class_new_in_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_LETTER_CLASS_NEW_IN_UNICODE_17,
            "s0|false\ns1|false\ns2|false\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_UPPERCASE_NEW_IN_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/\p{Lu}/u.test('꟎') + ',' + /\p{Ll}/u.test('꟏')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_uppercase_new_in_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_UPPERCASE_NEW_IN_UNICODE_17,
            "s0|false,false\ns1|false,false\ns2|false,false\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_UI_FOLD_NEW_IN_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/꟎/ui.test('꟏')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_ui_fold_new_in_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_UI_FOLD_NEW_IN_UNICODE_17,
            "s0|false\ns1|false\ns2|false\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_SCRIPT_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return RegExp('\\p{Script=Beria_Erfe}', 'u').test('a'); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_script_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_SCRIPT_UNICODE_17,
            "s0|caught\ns1|caught\ns2|caught\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_KELVIN_UI: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/k/ui.test('K') + ',' + /[^\W]/ui.test('K')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_kelvin_ui() {
    check(&q_refused(
        REFUSES_REGEXP_KELVIN_UI,
        "s0|true,true\ns1|true,true\ns2|true,true\n",
    ));
}

const REFUSES_REGEXP_LOWERCASE_PROP_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/\p{Lowercase}/u.test('꟏')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_lowercase_prop_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_LOWERCASE_PROP_UNICODE_17,
            "s0|false\ns1|false\ns2|false\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_UNASSIGNED_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/\p{Cn}/u.test('꟎')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_unassigned_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_UNASSIGNED_UNICODE_17,
            "s0|true\ns1|true\ns2|true\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_ID_START_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/\p{ID_Start}/u.test('꟎')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_id_start_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_ID_START_UNICODE_17,
            "s0|false\ns1|false\ns2|false\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_UI_CLASS_RANGE_UNICODE_17: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (/[꟎-꟏]/ui.test('꟏') + ',' + /[꟎]/ui.test('꟏')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_ui_class_range_unicode_17() {
    check_with(
        &q_refused(
            REFUSES_REGEXP_UI_CLASS_RANGE_UNICODE_17,
            "s0|true,false\ns1|true,false\ns2|true,false\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_REGEXP_NEGATED_NONWORD_UI_ASCII_S_K: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ([/[^\W]/ui.test('s'), /[^\W]/ui.test('k'), /[^\W]/ui.test('a'), /[^\W]/ui.test('S'), /[^\W]/ui.test('K')].join()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_negated_nonword_ui_ascii_s_k() {
    check(&q_refused(
        REFUSES_REGEXP_NEGATED_NONWORD_UI_ASCII_S_K,
        "s0|true,true,true,true,true\ns1|true,true,true,true,true\ns2|true,true,true,true,true\n",
    ));
}

const REFUSES_REGEXP_ALNUM_IDIOM_UI: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('ask_K'.replace(/[^\W_]/gui, '.')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_alnum_idiom_ui() {
    check(&q_refused(
        REFUSES_REGEXP_ALNUM_IDIOM_UI,
        "s0|..._.\ns1|..._.\ns2|..._.\n",
    ));
}

const REFUSES_REGEXP_NONWORD_FORMS_UI_ASCII: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ([/\W/ui.test('s'), /[\W]/ui.test('s'), /[^\w]/ui.test('s'), /[^\W\d]/ui.test('s'), /[^\D]/ui.test('5'), /[^\S]/ui.test(' '), /[^\W]/u.test('s')].join()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_nonword_forms_ui_ascii() {
    check(&q_refused(REFUSES_REGEXP_NONWORD_FORMS_UI_ASCII, "s0|false,false,false,true,true,true,true\ns1|false,false,false,true,true,true,true\ns2|false,false,false,true,true,true,true\n"));
}

const REFUSES_REGEXP_NEGATED_NONWORD_GI_NO_U: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('ask'.replace(/[^\W]/gi, '.')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_negated_nonword_gi_no_u() {
    check(&q_refused(
        REFUSES_REGEXP_NEGATED_NONWORD_GI_NO_U,
        "s0|...\ns1|...\ns2|...\n",
    ));
}

const REFUSES_REGEXP_V_FLAG_NEGATED_NONWORD_I: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('ask'.replace(/[^\W]/gvi, '.')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_v_flag_negated_nonword_i() {
    check(&q_refused(
        REFUSES_REGEXP_V_FLAG_NEGATED_NONWORD_I,
        "s0|...\ns1|...\ns2|...\n",
    ));
}

const REFUSES_REGEXP_STRIP_NONALNUM_GI_IDIOM: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('kiss_me Sky!'.replace(/[\W_]+/gi, '')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_strip_nonalnum_gi_idiom() {
    check(&q_refused(
        REFUSES_REGEXP_STRIP_NONALNUM_GI_IDIOM,
        "s0|kissmeSky\ns1|kissmeSky\ns2|kissmeSky\n",
    ));
}

const REFUSES_REGEXP_CLASS_NONWORD_I_TEST: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ([/[\W]/i.test('s'), /[\W]/i.test('k'), /[\W]/i.test('K'), /\W/i.test('s')].join()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_class_nonword_i_test() {
    check(&q_refused(
        REFUSES_REGEXP_CLASS_NONWORD_I_TEST,
        "s0|false,false,false,false\ns1|false,false,false,false\ns2|false,false,false,false\n",
    ));
}

const REFUSES_REGEXP_LOOKAHEAD_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (('abc'.replace(/a(?=b)/, 'x'))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_lookahead_refused() {
    check(&q_refused(
        REFUSES_REGEXP_LOOKAHEAD_REFUSED,
        "s0|xbc\ns1|xbc\ns2|xbc\n",
    ));
}

const REFUSES_REGEXP_BACKREFERENCE_I_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((String(/(a)\1/i.test('aA')))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_backreference_i_refused() {
    check(&q_refused(
        REFUSES_REGEXP_BACKREFERENCE_I_REFUSED,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const REFUSES_REGEXP_Y_FLAG_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((String(/b/y.test('ab')))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_y_flag_refused() {
    check(&q_refused(
        REFUSES_REGEXP_Y_FLAG_REFUSED,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const DECODES_SELF_DEFENDING_REGEXP_DECODES: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((String(new RegExp('\\w+ *\\(\\) *{\\w+ *[\'|"].+[\'|"];? *}').test('abc() {x "y";}')))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn decodes_self_defending_regexp_decodes() {
    check(&decoded(
        DECODES_SELF_DEFENDING_REGEXP_DECODES,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const DECODES_REGEXP_ASCII_CASE_DECODES: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (('aBc'.replace(/b/i, 'x').toUpperCase())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn decodes_regexp_ascii_case_decodes() {
    check(&decoded(
        DECODES_REGEXP_ASCII_CASE_DECODES,
        "s0|AXC\ns1|AXC\ns2|AXC\n",
    ));
}

const REFUSES_REGEXP_FLAGS_U_AND_V_IN_TRY: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { new RegExp('a','uv'); return 'ok'; } catch (e) { return e instanceof SyntaxError; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_flags_u_and_v_in_try() {
    check(&q_refused(
        REFUSES_REGEXP_FLAGS_U_AND_V_IN_TRY,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const REFUSES_REGEXP_FLAGS_UV_TEST_WITHOUT_CATCH: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (new RegExp('a', 'uv').test('a')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_flags_uv_test_without_catch() {
    check(&q_refused(REFUSES_REGEXP_FLAGS_UV_TEST_WITHOUT_CATCH, ""));
}

const REFUSES_REGEXP_FLAGS_VU_REPLACE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('abc'.replace(new RegExp('b', 'vu'), 'X')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_flags_vu_replace() {
    check(&q_refused(REFUSES_REGEXP_FLAGS_VU_REPLACE, ""));
}

const REFUSES_REGEXP_FLAGS_UV_WITHOUT_NEW: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function () { try { RegExp('x', 'uv'); return 'built'; } catch (e) { return 'threw:' + (e instanceof SyntaxError); } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_flags_uv_without_new() {
    check(&q_refused(
        REFUSES_REGEXP_FLAGS_UV_WITHOUT_NEW,
        "s0|threw:true\ns1|threw:true\ns2|threw:true\n",
    ));
}

const REFUSES_REGEXP_FLAGS_VU_TEST: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((new RegExp('b', 'vu').test('abc'))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_flags_vu_test() {
    check(&q_refused(REFUSES_REGEXP_FLAGS_VU_TEST, ""));
}

const DECODES_REGEXP_FLAGS_IU_AND_SV: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ return new RegExp('a','iu').test('A') + ',' + new RegExp('.','sv').test('\n'); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn decodes_regexp_flags_iu_and_sv() {
    check(&decoded(
        DECODES_REGEXP_FLAGS_IU_AND_SV,
        "s0|true,true\ns1|true,true\ns2|true,true\n",
    ));
}

const AGREES_REGEXP_FLAGS_V_OVER_A_U_REGEXP: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ return String(new RegExp(/a/u, 'v')) + new RegExp(/a/u, 'v').test('a'); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn agrees_regexp_flags_v_over_a_u_regexp() {
    check(&agrees(
        AGREES_REGEXP_FLAGS_V_OVER_A_U_REGEXP,
        "s0|/a/vtrue\ns1|/a/vtrue\ns2|/a/vtrue\n",
    ));
}

const CATASTROPHIC_REGEXP_SUBJECT_LEN: usize = 43;
const CATASTROPHIC_REGEXP_DEADLINE: std::time::Duration = std::time::Duration::from_secs(120);

fn catastrophic_regexp_program() -> String {
    format!(
        "function qa(){{ var h=['s0','s1','s2']; qa=function(){{return h;}}; return qa(); }}\nfunction qb(i){{ var e=qa(); return e[i] + '|' + (/^(a+)+b$/.test('{}c')); }}\nfunction later(){{ return qb(1); }}\nconsole.log(qb(0)); console.log(later()); console.log(qb(2));",
        "a".repeat(CATASTROPHIC_REGEXP_SUBJECT_LEN)
    )
}

#[test]
fn refuses_catastrophic_backtracking_without_hanging() {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let source = catastrophic_regexp_program();
        let code = untwist::Pipeline::new(untwist::Options::default())
            .run(&source)
            .code()
            .to_string();
        let _ = sender.send(code);
    });
    let code = receiver
        .recv_timeout(CATASTROPHIC_REGEXP_DEADLINE)
        .expect("the pipeline must finish instead of backtracking forever");
    assert!(
        code.contains("console.log(qb(0))"),
        "the call must stay: {code}"
    );
}
