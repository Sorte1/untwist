use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_array_rows.txt");
const PREFIXES: [&str; 5] = ["dec-", "unsafe-dec-", "ref-", "guard-", "premise-"];
const UNSAFE_PREMISE: &str = "ref-premise-";
const SELF_DEFENDING: &str = "self-defending";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn decoded(code: &str) -> bool {
    !code.contains("arr0 = ") && !code.contains("function arr0")
}

fn unsafe_premise(label: &str) -> bool {
    label.starts_with(UNSAFE_PREMISE) || label.starts_with("premise-")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            PREFIXES.iter().any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label_in_both_lanes() {
    for lane in [false, true] {
        for (label, source) in rows() {
            if lane && unsafe_premise(&label) {
                continue;
            }
            let should_decode =
                label.starts_with("dec-") || (lane && label.starts_with("unsafe-dec-"));
            let code = transformed(&options(lane), &source);
            assert_eq!(
                decoded(&code),
                should_decode,
                "{label} (unsafe={lane}): expected decode={should_decode}:\n{code}"
            );
        }
    }
}

#[test]
fn the_unsafe_lane_decodes_the_rows_its_premises_cover() {
    let covered = [
        "ref-premise-global-key-read",
        "ref-premise-push-patched-first",
        "ref-premise-shift-patched-before-a-rotated-function",
        "ref-premise-shift-defined-before-a-rotated-function",
        "ref-premise-push-patched-by-a-called-function",
        "ref-premise-rotated-after-a-patch-in-its-function",
    ];
    for (label, source) in rows() {
        if !covered.contains(&label.as_str()) {
            continue;
        }
        let code = transformed(&options(true), &source);
        assert!(decoded(&code), "{label}:\n{code}");
    }
}

#[test]
fn a_refused_array_is_left_as_written() {
    for (label, source) in rows() {
        if !label.starts_with("ref-") {
            continue;
        }
        let code = transformed(&options(false), &source);
        let kept = ["var arr0 = [", "const arr0 = ["];
        assert!(
            kept.iter().any(|text| code.contains(text)),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn both_lanes_run_like_node_outside_the_unsafe_premises() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 15, "found {} rows", programs.len());
    let safe = options(false);
    let aggressive = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let printed_as_written: Vec<(String, String)> = programs
        .iter()
        .filter(|(label, _)| !label.contains(SELF_DEFENDING))
        .cloned()
        .collect();
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let mut wrong = node_rows::mismatches(&node, &printed_as_written, &modes);
    let covered: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| !unsafe_premise(label))
        .collect();
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    wrong.extend(node_rows::mismatches(&node, &covered, &modes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_readable_cookie_rotator_whose_guard_runs_twice_is_left_alone() {
    let source = "var arr0=['one','two','three'];(function(J,K){var L=function(M){while(--M){J['push'](J['shift']());}};var G=function(){var M={'data':{'key':'cookie','value':'timeout'},'setCookie':function(O,P,Q,R){R=R||{};var S=P+'='+Q;var T=0x0;for(var T=0x0,U=O['length'];T<U;T++){var V=O[T];S+=';\\x20'+V;var X=O[V];O['push'](X);U=O['length'];if(X!==!![]){S+='='+X;}}R['cookie']=S;},'removeCookie':function(){return'dev';},'getCookie':function(O,P){O=O||function(S){return S;};var Q=O(new RegExp('(?:^|;\\x20)'+P['replace'](/([.$?*|{}()[]\\/+^])/g,'$1')+'=([^;]*)'));var W=function(Y,Z){Y(++Z);};W(L,K);return Q?decodeURIComponent(Q[0x1]):undefined;}};var C=function(){var E=new RegExp('\\x5cw+\\x20*\\x5c(\\x5c)\\x20*{\\x5cw+\\x20*[\\x27|\\x22].+[\\x27|\\x22];?\\x20*}');return E['test'](M['removeCookie']['toString']());};M['updateCookie']=C;var K0='';var N=M['updateCookie']();if(!N){M['setCookie'](['*'],'counter',0x1);}else if(N){K0=M['getCookie'](null,'counter');}else{M['removeCookie']();}};G();G();}(arr0,0x1));var dec0=function(t,x){var e=arr0[t-=0];return e};console.log(dec0('0x0'),dec0('0x1'),dec0('0x2'));";
    for lane in [false, true] {
        let code = transformed(&options(lane), source);
        assert!(!decoded(&code), "unsafe={lane}:\n{code}");
    }
}
