use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const READ_BY_KEY: &str = "var total = 5;
var g = this;
function pick(k) { return g[k]; }
console.log(pick(String.fromCharCode(116, 111, 116, 97, 108)), typeof pick(String.fromCharCode(112, 105, 99, 107)));";

const NOT_REACHED: &str = "var total = 5;
function pick(k) { return [total][k]; }
console.log(pick(0));";

fn renamed(
    source: &str,
    rename: RenameMode,
    unsafe_transforms: bool,
    disabled: Vec<&'static str>,
) -> String {
    Pipeline::new(Options {
        rename,
        unsafe_transforms,
        disabled_passes: disabled,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

#[test]
fn script_globals_reached_by_key_keep_their_names_in_every_safe_rename_mode() {
    for rename in [
        RenameMode::Mangle,
        RenameMode::Simple,
        RenameMode::Unique,
        RenameMode::Smart,
    ] {
        let code = renamed(READ_BY_KEY, rename, false, Vec::new());
        assert!(code.contains("var total = 5"), "{rename:?}: {code}");
        assert!(code.contains("function pick("), "{rename:?}: {code}");
    }
}

#[test]
fn globals_no_key_can_reach_are_still_renamed() {
    let code = renamed(NOT_REACHED, RenameMode::Simple, false, Vec::new());
    assert!(!code.contains("var total"), "{code}");
}

#[test]
fn node_agrees_in_every_safe_rename_mode_and_the_guard_is_load_bearing() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = vec![
        ("read by key".to_string(), READ_BY_KEY.to_string()),
        ("not reached".to_string(), NOT_REACHED.to_string()),
    ];
    let mangle = |source: &str| renamed(source, RenameMode::Mangle, false, Vec::new());
    let simple = |source: &str| renamed(source, RenameMode::Simple, false, Vec::new());
    let unique = |source: &str| renamed(source, RenameMode::Unique, false, Vec::new());
    let smart = |source: &str| renamed(source, RenameMode::Smart, false, Vec::new());
    let modes: [Mode<'_>; 4] = [
        ("mangle", &mangle),
        ("simple", &simple),
        ("unique", &unique),
        ("smart", &smart),
    ];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let unguarded = |source: &str| {
        renamed(
            source,
            RenameMode::Simple,
            false,
            vec!["rename-keeps-globals"],
        )
    };
    let modes: [Mode<'_>; 1] = [("unguarded", &unguarded)];
    assert_eq!(mismatches(&node, &programs[..1], &modes).len(), 1);
}
