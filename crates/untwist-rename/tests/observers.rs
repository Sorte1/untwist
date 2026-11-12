mod common;

use common::{assert_scope_safe, rename_compact};
use untwist_core::semantic::{assume_observers, Observers};
use untwist_rename::{RenameConfig, RenameMode};

const MODES: [RenameMode; 3] = [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique];

fn renamed(source: &str, mode: RenameMode) -> String {
    let output = rename_compact(
        source,
        RenameConfig {
            mode,
            rename_all: true,
            extended: true,
        },
    );
    assert_scope_safe(source, &output);
    output
}

fn assert_kept(source: &str, kept: &[&str], renamed_away: &[&str]) {
    for mode in MODES {
        let output = renamed(source, mode);
        for name in kept {
            let bound = format!("{name},");
            let last = format!("{name})");
            let declared = format!("var {name}");
            assert!(
                output.contains(&bound) || output.contains(&last) || output.contains(&declared),
                "{mode:?} renamed {name}: {output}"
            );
        }
        for name in renamed_away {
            assert!(!output.contains(name), "{mode:?} kept {name}: {output}");
        }
    }
}

#[test]
fn a_closed_eval_pins_only_the_bindings_its_payload_reads() {
    assert_kept(
        "function outer(alpha, beta) { return eval('alpha') + beta; }",
        &["alpha"],
        &["beta"],
    );
}

#[test]
fn an_open_eval_pins_every_binding_on_its_scope_chain() {
    assert_kept(
        "var outerTop = 1; function outer(alpha, source) { return eval(source); }",
        &["alpha", "source", "outerTop"],
        &[],
    );
    assert_kept(
        "function outer(alpha, source) { return (eval)(source); }",
        &["alpha", "source"],
        &[],
    );
}

#[test]
fn an_open_eval_leaves_bindings_off_its_chain_alone() {
    assert_kept(
        "function outer(source) { function inner(gamma) { return gamma; } return eval(source) + inner(1); }",
        &["source"],
        &["gamma"],
    );
}

#[test]
fn an_optional_eval_call_is_indirect_and_pins_nothing() {
    assert_kept(
        "function outer(alpha, omega) { return eval?.(omega) + alpha; }",
        &[],
        &["alpha", "omega"],
    );
}

#[test]
fn a_with_body_pins_what_it_reads_from_outside() {
    assert_kept(
        "function outer(object, alpha) { with (object) { return alpha; } }",
        &["alpha"],
        &["object"],
    );
    assert_kept(
        "function outer(object, alpha) { with (object) { return function () { return alpha; }; } }",
        &["alpha"],
        &["object"],
    );
}

#[test]
fn a_var_declared_in_a_with_body_is_pinned() {
    assert_kept(
        "function outer(object) { with (object) { var delta = 1; } return delta; }",
        &["delta"],
        &["object"],
    );
}

#[test]
fn bindings_declared_inside_a_with_body_are_still_renamed() {
    assert_kept(
        "function outer(object) { with (object) { let epsilon = 1; (function (zeta) { return zeta; })(epsilon); } }",
        &[],
        &["object", "epsilon", "zeta"],
    );
}

#[test]
fn assuming_no_observers_renames_what_an_eval_could_see() {
    let _observers = assume_observers(Observers::None);
    assert_kept(
        "function outer(alpha, source) { return eval(source) + alpha; }",
        &[],
        &["alpha", "source"],
    );
}

#[test]
fn smart_keeps_what_an_eval_or_a_with_body_can_see() {
    let config = RenameConfig {
        mode: RenameMode::Smart,
        rename_all: true,
        extended: true,
    };
    let source = "function outer(_0xa, _0xb) { try { return eval('_0xa') + _0xb; } catch (_0xc) { return _0xc; } }";
    let output = rename_compact(source, config);
    assert_scope_safe(source, &output);
    assert!(output.contains("_0xa"), "{output}");
    assert!(!output.contains("_0xc"), "{output}");
    let source = "var o = {}; function w(_0xd) { with (o) { return [_0xd].map(function (_0xe) { return _0xe + _0xd; }); } }";
    let output = rename_compact(source, config);
    assert_scope_safe(source, &output);
    assert!(output.contains("_0xd"), "{output}");
}
