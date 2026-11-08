use std::path::{Path, PathBuf};

use untwist::{Note, Options, Pipeline, GUARD_KEPT_ACROSS_LINES, GUARD_ON_ONE_LINE};
use untwist_eval::source::satisfies_self_defending_regex;
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const SELF_DEFENDING: [&str; 6] = ["001", "003", "005", "006", "009", "010"];
const PLAIN: [&str; 6] = ["000", "002", "004", "007", "008", "011"];
const PATCHED_BUILTIN: &str =
    "var __k = 'char' + 'At'; String.prototype[__k] = String.prototype[__k];\n";

const TABLES_AROUND_THE_CONTROLLER: &str = r#"var ctrl=function(){var k={a:"0x1",b:2};var f=!![];return function(c,fn){var r=f?function(){var j={a:"0x2"};if(fn){var x=fn[j.a==="0x2"?"apply":"call"](c,arguments);fn=null;return x}}:function(){};f=![];return r}}();var g=ctrl(this,function(){return g.toString().search("(((.+)+)+)+$")});console.log(g(),k_check());function k_check(){return typeof ctrl}"#;

const ARITHMETIC_STATE_COUNTERS: &str = r#"var S=function(p){this.a=p;this.b=[-1385*-1+-7397+6013,-3482+6593+-51*61,1*-9127+2712+6415];this.c=function(){return'newState';};this.d='\\w+ *\\(\\) *{\\w+ *';this.e='[\'|"].+[\'|"];? *}'};var s=new S(1);console.log(new RegExp(s.d+s.e).test(s.c.toString()),s.b.join());"#;

const ARITHMETIC_SELF_READ_OFFSET: &str = r#"var F=function(s){var A='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var a='',b='',c=a+F;for(var q=0,t,n,i=0;n=s.charAt(i++);~n&&(t=q%4?t*64+n:n,q++%4)?a+=c.charCodeAt(i+(-8737+-7673+16420))-(-38*232+35*-58+10856)!==-3613+3613?String.fromCharCode(255&t>>(-2*q&6)):q:0){n=A.indexOf(n)}return a};console.log(F('aGVsbG8gd29ybGQ='));"#;

const SPLIT_SELF_READ_DECLARATIONS: &str = r#"var F=function(s){var A='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var a='';var b='';var c=a+F;for(var q=0,t,n,i=0;n=s['charAt'](i++);~n&&(t=q%4?t*64+n:n,q++%4)?a+=c['charCodeAt'](i+10)-10!==0?String['fromCharCode'](255&t>>(-2*q&6)):q:0){n=A['indexOf'](n)}return a};console.log(F('aGVsbG8gd29ybGQ='));"#;

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn case(number: &str) -> String {
    let path = format!(
        "{}/../../fixtures/corpus-npm/case-{number}.js",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn without_deobfuscation() -> [(&'static str, Options); 2] {
    [
        (
            "no-deobfuscate",
            Options {
                deobfuscate: false,
                ..Options::default()
            },
        ),
        (
            "no-deobfuscate no-unminify",
            Options {
                deobfuscate: false,
                unminify: false,
                ..Options::default()
            },
        ),
    ]
}

fn decoding() -> [(&'static str, Options); 2] {
    [
        ("default", Options::default()),
        (
            "conservative",
            Options {
                extended: false,
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
    ]
}

fn observed(node: &Path, dir: &Path, text: &str) -> String {
    let path = dir.join("program.js");
    std::fs::write(&path, text).unwrap();
    match run(node, &path).unwrap() {
        Some(out) => format!("{}exit {:?}", out.stdout, out.exit),
        None => "did not finish".to_string(),
    }
}

fn holds(notes: &[Note], message: &str) -> bool {
    notes.iter().any(|note| note.message.starts_with(message))
}

fn mismatches(
    node: &Path,
    sources: &[(String, String)],
    modes: &[(&'static str, Options)],
    expect_guard_note: bool,
) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for (label, source) in sources {
        let before = observed(node, dir.path(), source);
        for (mode, options) in modes {
            let out = Pipeline::new(options.clone()).run(source);
            let after = observed(node, dir.path(), out.code());
            let noted = holds(&out.notes, GUARD_ON_ONE_LINE);
            if after != before || noted != expect_guard_note {
                wrong.push(format!(
                    "{label} ({mode}): node printed {before:?}, untwist's output printed \
                     {after:?}, guard note {noted}"
                ));
            }
            assert!(!holds(&out.notes, GUARD_KEPT_ACROSS_LINES), "{label}");
        }
    }
    wrong
}

fn cases(numbers: &[&str], prefix: &str) -> Vec<(String, String)> {
    numbers
        .iter()
        .map(|number| {
            (
                format!("case-{number}"),
                format!("{prefix}{}", case(number)),
            )
        })
        .collect()
}

#[test]
fn every_self_defending_npm_case_runs_like_its_input_without_deobfuscation() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let wrong = mismatches(
        &node,
        &cases(&SELF_DEFENDING, ""),
        &without_deobfuscation(),
        true,
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_npm_cases_without_self_defending_print_as_before() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let wrong = mismatches(&node, &cases(&PLAIN, ""), &without_deobfuscation(), false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_refused_unit_keeps_its_guards_running() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sources = cases(&["006", "001"], PATCHED_BUILTIN);
    let wrong = mismatches(&node, &sources, &decoding(), true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    for (label, source) in &sources {
        for (mode, options) in decoding() {
            let out = Pipeline::new(options).run(source);
            assert!(
                out.notes
                    .iter()
                    .any(|note| note.kind == untwist_core::pass::NoteKind::Refusal),
                "{label} ({mode}): {:?}",
                out.notes
            );
        }
    }
}

#[test]
fn a_decoded_unit_leaves_no_guard_to_print() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let wrong = mismatches(&node, &cases(&SELF_DEFENDING, ""), &decoding(), false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_compacted_newstate_function_satisfies_its_own_regex() {
    let out = Pipeline::new(Options {
        deobfuscate: false,
        ..Options::default()
    })
    .run(&case("006"));
    let start = out
        .code()
        .find("function () {return")
        .unwrap_or(out.code().len());
    let compact = out.code()[start..]
        .split_inclusive('}')
        .next()
        .unwrap_or_default();
    assert_eq!(compact, "function () {return \"newState\";}");
    assert!(satisfies_self_defending_regex(compact));
    assert!(satisfies_self_defending_regex(
        "function(){return'newState';}"
    ));
}

#[test]
fn every_widened_template_shape_runs_like_its_input_without_deobfuscation() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sources: Vec<(String, String)> = [
        ("tables around the controller", TABLES_AROUND_THE_CONTROLLER),
        ("arithmetic state counters", ARITHMETIC_STATE_COUNTERS),
        ("arithmetic self-read offset", ARITHMETIC_SELF_READ_OFFSET),
        ("split self-read declarations", SPLIT_SELF_READ_DECLARATIONS),
    ]
    .into_iter()
    .map(|(label, source)| (label.to_string(), source.to_string()))
    .collect();
    let wrong = mismatches(&node, &sources, &without_deobfuscation(), true);
    assert!(
        wrong.is_empty(),
        "{}",
        wrong.join(
            "
"
        )
    );
}

const SPACED_STATE_FUNCTION: &str = r#"var S=function(p){this.a=p;this.b=[1,0,0];this.c=function(){ return 'newState'; };this.d='\\w+ *\\(\\) *{\\w+ *';this.e='[\'|"].+[\'|"];? *}'};var s=new S(1);console.log(new RegExp(s.d+s.e).test(s.c.toString()));"#;

#[test]
fn a_single_line_state_function_its_regex_rejects_is_the_documented_premise() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let before = observed(&node, dir.path(), SPACED_STATE_FUNCTION);
    assert_eq!(before, "false\nexit Some(0)");
    for (mode, options) in without_deobfuscation() {
        let out = Pipeline::new(options).run(SPACED_STATE_FUNCTION);
        assert!(
            out.code().contains("function () {return \"newState\";}"),
            "{mode}: {}",
            out.code()
        );
        assert!(holds(&out.notes, GUARD_ON_ONE_LINE), "{mode}");
        let after = observed(&node, dir.path(), out.code());
        assert_eq!(after, "true\nexit Some(0)", "{mode}");
    }
}

const LET_CONTROLLER: &str = r#"let ctrl=function(){var k={a:"0x1",b:2};var f=!![];return function(c,fn){var r=f?function(){var j={a:"0x2"};if(fn){var x=fn[j.a==="0x2"?"apply":"call"](c,arguments);fn=null;return x}}:function(){};f=![];return r}}();var g=ctrl(this,function(){return g.toString().search("(((.+)+)+)+$")});console.log(g(),typeof ctrl);"#;

const CONST_STATE_COUNTERS: &str = r#"const S=function(p){this.a=p;this.b=[-1385*-1+-7397+6013,-3482+6593+-51*61,1*-9127+2712+6415];this.c=function(){return'newState';};this.d='\\w+ *\\(\\) *{\\w+ *';this.e='[\'|"].+[\'|"];? *}'};var s=new S(1);console.log(new RegExp(s.d+s.e).test(s.c.toString()),s.b.join());"#;

const ASSIGNED_SELF_READ: &str = r#"var F;F=function(s){var A='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var a='';var b='';var c=a+F;for(var q=0,t,n,i=0;n=s['charAt'](i++);~n&&(t=q%4?t*64+n:n,q++%4)?a+=c['charCodeAt'](i+10)-10!==0?String['fromCharCode'](255&t>>(-2*q&6)):q:0){n=A['indexOf'](n)}return a};console.log(F('aGVsbG8gd29ybGQ='));"#;

#[test]
fn a_let_a_const_and_an_assigned_guard_run_like_their_input_without_deobfuscation() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sources: Vec<(String, String)> = [
        ("let g = <G1>", LET_CONTROLLER),
        ("const g = <G2>", CONST_STATE_COUNTERS),
        ("g = <G3>;", ASSIGNED_SELF_READ),
    ]
    .into_iter()
    .map(|(label, source)| (label.to_string(), source.to_string()))
    .collect();
    let wrong = mismatches(&node, &sources, &without_deobfuscation(), true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const BLOCK_SCOPED_CONTROLLER: &str = r#"let ctrl=function(){let f=!![];return function(c,fn){const r=f?function(){const w=d;if(fn){const x=fn[w(0)](c,arguments);return fn=null,x}}:function(){};return f=![],r}}();function d(){return"apply"}const g=ctrl(this,function(){return g.toString().search("(((.+)+)+)+$").toString().constructor(g).search("(((.+)+)+)+$")});g();console.log("ran");"#;

#[test]
fn a_controller_written_with_let_and_const_inside_runs_like_its_input() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sources = vec![(
        "let/const inside <G1>".to_string(),
        BLOCK_SCOPED_CONTROLLER.to_string(),
    )];
    let wrong = mismatches(&node, &sources, &without_deobfuscation(), true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const DEAD_CODE_CONTROLLER: &str = r#"let ctrl=function(){const p={eq:function(a,b){return a===b},k:"x"};let f=!![];return function(c,fn){const q={ne:function(a,b){return p.eq(a,b)},z:p["k"]};const r=f?function(){if(q.ne(q.z,"y")){return 1}else if(fn){const x=fn[d()](c,arguments);fn=null;return x}}:function(){};f=![];return r}}();function d(){return"apply"}const g=ctrl(this,function(){return g.toString().search("(((.+)+)+)+$").toString().constructor(g).search("(((.+)+)+)+$")});g();console.log("ran");"#;

#[test]
fn a_controller_with_injected_dead_code_and_proxy_tables_runs_like_its_input() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sources = vec![(
        "dead code in <G1>".to_string(),
        DEAD_CODE_CONTROLLER.to_string(),
    )];
    let wrong = mismatches(&node, &sources, &without_deobfuscation(), true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_site_memo_reset_sample_loses_its_self_defending_check_under_unsafe() {
    let path = format!(
        "{}/../../fixtures/reference/deobfuscate/obfuscator.io-site-memo-reset.js",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(path).unwrap();
    let default = Pipeline::new(Options::default()).run(&source);
    assert!(
        default.notes.iter().any(|note| note
            .message
            .starts_with(untwist_deobfuscate::notes::BUILTIN_ALIAS_PATCHED)),
        "{:?}",
        default.notes
    );
    let out = Pipeline::new(Options {
        unsafe_transforms: true,
        ..Options::default()
    })
    .run(&source);
    let code = out.code();
    assert!(!code.contains("(((.+)+)+)+$"), "{code}");
    assert!(code.contains("console.log(\"\")"), "{code}");
    assert!(
        !out.notes
            .iter()
            .any(|note| note.message.contains(GUARD_ON_ONE_LINE)
                || note.message.contains(GUARD_KEPT_ACROSS_LINES)),
        "{:?}",
        out.notes
    );
}

const COOKIE_ROTATOR: &str = include_str!("data/guards/cookie-rotator.js");

const INLINE_CONTROLLER: &str = include_str!("data/guards/inline-controller.js");

const SEQUENCE_CONTROLLER: &str = include_str!("data/guards/sequence-controller.js");

#[test]
fn a_cookie_rotator_and_controllers_built_where_they_are_called_run_like_their_input() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sources: Vec<(String, String)> = [
        ("cookie rotator", COOKIE_ROTATOR),
        ("controller called where it is built", INLINE_CONTROLLER),
        ("controller built in a sequence", SEQUENCE_CONTROLLER),
    ]
    .into_iter()
    .map(|(label, source)| (label.to_string(), source.to_string()))
    .collect();
    let modes: Vec<(&'static str, Options)> = without_deobfuscation()
        .into_iter()
        .chain(decoding())
        .collect();
    let wrong = mismatches(&node, &sources, &modes, true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
