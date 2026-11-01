use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const DECODER: &str = "a3iíîiííi";
const ARRAY: &str = "a3iíïïîiì";

fn observed_program() -> String {
    format!(
        "function {DECODER}(iîìíîiì, iiîìîîî) {{ var iíïïîiì = {ARRAY}(); return {DECODER} = function (iíîiííi, iìíiììï) {{ iíîiííi = iíîiííi - 0x0; var iiiîíîi = iíïïîiì[iíîiííi]; return iiiîíîi; }}, {DECODER}(iîìíîiì, iiîìîîî); }}
function {ARRAY}() {{ var IÏìïìiì = ['Hello World!', 'console', 'deobfuscate', 'log']; {ARRAY} = function () {{ return IÏìïìiì; }}; return {ARRAY}(); }}
(function (iÍîîìïí, iÏììïìì) {{ var iÏiíiiî = {DECODER}, iÍíiiíi = iÍîîìïí(); while (!![]) {{ try {{ if (iÏiíiiî(3) === 'log') break; else iÍíiiíi['push'](iÍíiiíi['shift']()); }} catch (iÌïìîíi) {{ iÍíiiíi['push'](iÍíiiíi['shift']()); }} }} }})({ARRAY}, 0x0);
console.log(Object.keys(this).length >= 0);
console[{DECODER}(3)]({DECODER}(0), {DECODER}(2));"
    )
}

fn run(source: &str, unsafe_transforms: bool) -> Output {
    Pipeline::new(Options {
        unsafe_transforms,
        rename: RenameMode::None,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn occurrences(code: &str, name: &str) -> usize {
    code.match_indices(name).count()
}

#[test]
fn the_unsafe_lane_removes_decoder_array_and_rotator_once_every_call_is_inlined() {
    let out = run(&observed_program(), true);
    let code = out.code();
    assert_eq!(occurrences(code, DECODER), 0, "{code}");
    assert_eq!(occurrences(code, ARRAY), 0, "{code}");
    assert!(code.contains("\"Hello World!\""), "{code}");
}

#[test]
fn the_safe_lane_keeps_the_machinery_while_names_may_be_observed() {
    let out = run(&observed_program(), false);
    let code = out.code();
    assert!(occurrences(code, DECODER) > 0, "{code}");
    assert!(
        out.notes
            .iter()
            .any(|note| note.message == notes::MACHINERY_NAME_OBSERVED),
        "{:#?}",
        out.notes
    );
}

#[test]
fn both_lanes_print_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = vec![("observed names".to_string(), observed_program())];
    let safe = |source: &str| run(source, false).code().to_string();
    let unsafe_lane = |source: &str| run(source, true).code().to_string();
    let modes: [Mode<'_>; 2] = [("safe", &safe), ("unsafe", &unsafe_lane)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
}
