mod common;

use common::{assert_scope_safe, rename_full};
use untwist_rename::{RenameConfig, RenameMode};

const MODES: [RenameMode; 3] = [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique];

const PROTO_SHORTHAND: &str = concat!(
    "const obj = { __proto__: null, q: 9 };\n",
    "const { __proto__: k } = obj;\n",
    "let __proto__ = 7;\n",
    "console.log(Object.keys({ __proto__ }), k);\n"
);

fn renamed(source: &str, mode: RenameMode) -> String {
    let config = RenameConfig {
        mode,
        rename_all: true,
        extended: true,
    };
    let output = rename_full(source, config);
    assert_scope_safe(source, &output);
    output
}

#[test]
fn a_shorthand_proto_property_keeps_its_own_property_name_in_every_mode() {
    for mode in MODES {
        let output = renamed(PROTO_SHORTHAND, mode);
        assert!(
            output.contains("Object.keys({__proto__})"),
            "{mode:?} renamed the binding behind a `{{ __proto__ }}` shorthand, \
             which renames the own property the literal creates: {output}"
        );
    }
}

#[test]
fn the_prototype_setting_colon_form_survives_every_mode() {
    for mode in MODES {
        let output = renamed(PROTO_SHORTHAND, mode);
        assert!(
            output.contains("{__proto__:null,"),
            "{mode:?} disturbed the colon form that sets the literal's prototype: {output}"
        );
    }
}

#[test]
fn the_shorthand_stage_never_moves_a_binding_onto_a_proto_key() {
    for mode in [RenameMode::None, RenameMode::Mangle] {
        let output = renamed("var t = {};\nf({ __proto__: t });\n", mode);
        assert!(
            !output.contains("__proto__:__proto__"),
            "{mode:?} named a binding `__proto__` to chase the key: {output}"
        );
        assert!(
            output.contains("__proto__:"),
            "{mode:?} lost the colon form, which changes what the literal does: {output}"
        );
    }
}

#[test]
fn a_destructured_proto_binding_is_renamed_and_prints_with_its_key() {
    let output = renamed(
        "const { __proto__ } = obj;\nuse(__proto__);\n",
        RenameMode::Simple,
    );
    assert!(
        output.starts_with("const{__proto__:"),
        "a binding pattern has no prototype semantics, so oxc recomputes its \
         shorthand and the binding may be renamed: {output}"
    );
}
