mod common;

use common::{assert_scope_safe, rename_full};
use untwist_rename::{RenameConfig, RenameMode};

const MODES: [RenameMode; 3] = [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique];

const SHORTHAND_TARGET: &str = concat!(
    "let a = 0;\n",
    "const o = { a: 1, zz: 2 };\n",
    "({ a } = o);\n",
    "console.log(a);\n"
);

const COLON_TARGET: &str = concat!(
    "let b = 0;\n",
    "const o = { zz: 1, q: 2 };\n",
    "({ zz: b } = o);\n",
    "console.log(b);\n"
);

fn config(mode: RenameMode) -> RenameConfig {
    RenameConfig {
        mode,
        rename_all: true,
        extended: true,
    }
}

fn renamed(source: &str, mode: RenameMode) -> String {
    let output = rename_full(source, config(mode));
    assert_scope_safe(source, &output);
    output
}

#[test]
fn a_shorthand_assignment_target_keeps_its_property_name_in_every_mode() {
    for mode in MODES {
        let output = renamed(SHORTHAND_TARGET, mode);
        assert!(
            output.contains("({a}="),
            "{mode:?} renamed a shorthand assignment target, which is also the property key, \
             so the assignment now reads a different property: {output}"
        );
    }
}

#[test]
fn pinning_the_target_does_not_pin_the_object_it_reads() {
    for mode in MODES {
        let output = renamed(SHORTHAND_TARGET, mode);
        assert!(
            !output.contains("const o="),
            "{mode:?}: only the shorthand target is pinned: {output}"
        );
    }
}

#[test]
fn a_colon_form_target_keeps_its_key_and_renames_its_binding() {
    for mode in MODES {
        let output = renamed(COLON_TARGET, mode);
        assert!(
            output.contains("({zz:"),
            "{mode:?}: a colon-form target carries its own key, which must survive: {output}"
        );
        assert!(
            !output.contains("let b="),
            "{mode:?}: its binding is free to be renamed: {output}"
        );
    }
}

#[test]
fn a_colon_form_target_collapses_only_onto_its_own_key() {
    let source = concat!(
        "let b = 0;\n",
        "const o = { a: 1, zz: 2 };\n",
        "({ a: b } = o);\n",
        "console.log(b);\n"
    );
    let output = renamed(source, RenameMode::Simple);
    assert!(
        output.contains("({a}=") && !output.contains("({b"),
        "oxc omits an assignment-target key only when it equals the target name, so the \
         property read stays `a`: {output}"
    );
}
