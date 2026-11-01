mod common;

use common::{conservative_run, default_run};
use oxc_allocator::Allocator;
use untwist_deobfuscate::control_flow_object::boxed;

const NATURAL: &str = "function Example() {\n  const Obj = {\n    prop1: \"blahblahblah\",\n    prop3: function (x, y) {\n      return x + y;\n    },\n    prop4: function (x, y) {\n      return x * y;\n    }\n  };\n  console.log(Obj.prop1, Obj.prop3(2, 4) + Obj.prop4(2, 7));\n}";

const A_SINGLE_NATURAL_KEY: &str =
    "function f() {\n  const o = { val: \"x\" };\n  return o.val;\n}";

const A_SINGLE_FIVE_LETTER_KEY: &str =
    "function f() {\n  const o = { value: \"x\" };\n  return o.value;\n}";

const AN_ALL_STRING_OBJECT: &str =
    "function f() {\n  const o = {\n    a: \"x\",\n    b: \"y\"\n  };\n  return o.a + o.b;\n}";

const A_VALUE_SHAPE_THE_PASS_DOES_NOT_MODEL: &str = "function f() {\n  const o = {\n    a: 1,\n    b: function (x, y) {\n      return x + y;\n    }\n  };\n  return o.a + o.b(1, 2);\n}";

fn left_alone(source: &str) {
    let alloc = Allocator::default();
    assert_eq!(default_run(&alloc, source, boxed).trim(), source.trim());
}

#[test]
fn issue_209_a_naturally_named_dispatch_object_is_inlined_by_default() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, NATURAL, boxed);
    assert!(
        code.contains(r#"console.log("blahblahblah", 2 + 4 + 2 * 7)"#),
        "{code}"
    );
    assert!(!code.contains("Obj"), "{code}");
}

#[test]
fn the_widening_is_inert_under_conservative() {
    let alloc = Allocator::default();
    assert_eq!(
        conservative_run(&alloc, NATURAL, boxed).trim(),
        NATURAL.trim()
    );
}

#[test]
fn a_single_property_object_with_a_natural_key_is_not_a_dispatch_object() {
    left_alone(A_SINGLE_NATURAL_KEY);
}

#[test]
fn a_single_property_object_with_a_five_letter_key_is_inlined_even_under_conservative() {
    let alloc = Allocator::default();
    for code in [
        default_run(&alloc, A_SINGLE_FIVE_LETTER_KEY, boxed),
        conservative_run(&alloc, A_SINGLE_FIVE_LETTER_KEY, boxed),
    ] {
        assert!(code.contains(r#"return "x""#), "{code}");
        assert!(!code.contains("o.value"), "{code}");
    }
}

#[test]
fn an_all_string_object_is_not_a_dispatch_object() {
    left_alone(AN_ALL_STRING_OBJECT);
}

#[test]
fn an_object_with_a_value_shape_the_pass_does_not_model_is_left_alone() {
    left_alone(A_VALUE_SHAPE_THE_PASS_DOES_NOT_MODEL);
}
