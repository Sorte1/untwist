use untwist_core::pass::PassOptions;
use untwist_core::semantic::{assume_observers, Observers};
use untwist_deobfuscate::pure_calls;
use untwist_testing::{transform_report, transform_with, Allocator};

fn lane(source: &str, unsafe_transforms: bool) -> String {
    let observers = if unsafe_transforms {
        Observers::None
    } else {
        Observers::Open
    };
    let _observers = assume_observers(observers);
    let alloc = Allocator::default();
    let opts = PassOptions {
        unsafe_transforms,
        ..PassOptions::default()
    };
    transform_with(&alloc, source, &[pure_calls::boxed], opts)
        .trim()
        .to_string()
}

fn safe(source: &str) -> String {
    lane(source, false)
}

fn unsafe_lane(source: &str) -> String {
    lane(source, true)
}

fn both(source: &str) -> String {
    let folded = safe(source);
    assert_eq!(folded, unsafe_lane(source), "the lanes differ on {source}");
    folded
}

fn kept_in_both(source: &str) {
    let expected = safe_identity(source);
    assert_eq!(safe(source), expected, "the safe lane changed {source}");
    assert_eq!(
        unsafe_lane(source),
        expected,
        "the unsafe lane changed {source}"
    );
}

fn safe_identity(source: &str) -> String {
    let alloc = Allocator::default();
    transform_with(&alloc, source, &[], PassOptions::default())
        .trim()
        .to_string()
}

#[test]
fn the_reported_sample_folds_and_drops_the_declaration() {
    let source = "let dec = (a) => {\n  return [\"hello\", \"worold\", \"log\"][a]\n}\nconsole[dec(2)](dec(0), dec(1))";
    assert_eq!(both(source), "console[\"log\"](\"hello\", \"worold\");");
}

#[test]
fn an_expression_bodied_const_arrow_folds() {
    let source = "const dec = (a) => [\"x\", \"y\"][a];\nconsole.log(dec(1));";
    assert_eq!(both(source), "console.log(\"y\");");
}

#[test]
fn a_function_declaration_folds() {
    let source = "function dec(a) { return [\"x\", \"y\"][a]; }\nconsole.log(dec(0));";
    assert_eq!(both(source), "console.log(\"x\");");
}

#[test]
fn a_named_function_expression_folds() {
    let source = "var dec = function d(a) { return [\"x\", \"y\"][a]; };\nconsole.log(dec(0));";
    assert_eq!(both(source), "console.log(\"x\");");
}

#[test]
fn a_hoisted_constant_array_folds_and_both_declarations_go() {
    let source =
        "const t = [\"x\", \"y\"];\nfunction dec(a) { return t[a]; }\nconsole.log(dec(1));";
    assert_eq!(both(source), "console.log(\"y\");");
}

#[test]
fn arithmetic_on_the_parameter_before_indexing_folds() {
    let source = "const t = [\"x\", \"y\"];\nconst dec = (a) => t[a - 0x5];\nconsole.log(dec(6));";
    assert_eq!(both(source), "console.log(\"y\");");
}

#[test]
fn a_ternary_and_concatenation_fold() {
    let source = "const pick = (a, b) => a > 1 ? \"x\" + b : `y${b}!`;\nconsole.log(pick(2, \"a\"), pick(0, 3));";
    assert_eq!(both(source), "console.log(\"xa\", \"y3!\");");
}

#[test]
fn string_indexing_length_and_typeof_fold() {
    let source = "function wrap() {\n  const f = (s, i) => s[i] + s.length + typeof i;\n  console.log(f(\"abc\", 1));\n}";
    assert_eq!(
        both(source),
        "function wrap() {\n  console.log(\"b3number\");\n}"
    );
}

#[test]
fn a_missing_argument_is_undefined() {
    let source = "const f = (a, b) => b === undefined ? \"none\" : b;\nconsole.log(f(1));";
    assert_eq!(both(source), "console.log(\"none\");");
}

#[test]
fn a_primitive_constant_and_an_object_literal_fold() {
    let source = "const off = 2;\nconst o = { a: \"x\", 1: \"y\" };\nconst f = (k) => o[k - off];\nconsole.log(f(3));";
    assert_eq!(both(source), "console.log(\"y\");");
}

#[test]
fn negative_zero_reads_index_zero() {
    let source = "const f = (a) => [\"x\"][-a];\nconsole.log(f(0));";
    assert_eq!(both(source), "console.log(\"x\");");
}

#[test]
fn nested_calls_fold_over_two_iterations() {
    let source = "function wrap() {\n  const a = (x) => [\"p\", \"q\"][x];\n  const b = (y) => y + \"!\";\n  console.log(b(a(1)));\n}";
    assert_eq!(both(source), "function wrap() {\n  console.log(\"q!\");\n}");
}

#[test]
fn a_call_inside_a_later_arrow_folds_and_then_the_arrow_folds() {
    let source = "function wrap() {\n  let dec = (a) => [\"x\", \"y\"][a];\n  const later = () => dec(1);\n  return later();\n}";
    assert_eq!(both(source), "function wrap() {\n  return \"y\";\n}");
}

#[test]
fn an_out_of_range_index_is_left_alone() {
    let source = "function wrap() {\n  const dec = (a) => [\"x\", \"y\"][a];\n  console.log(dec(5), dec(0));\n}";
    assert_eq!(
        both(source),
        "function wrap() {\n  const dec = (a) => [\"x\", \"y\"][a];\n  console.log(dec(5), \"x\");\n}"
    );
}

#[test]
fn a_reassigned_binding_is_left_alone() {
    kept_in_both("let dec = (a) => [\"x\"][a];\nconsole.log(dec(0));\ndec = (a) => \"z\";");
}

#[test]
fn a_redeclared_function_is_left_alone() {
    kept_in_both("function dec(a) { return [\"x\"][a]; }\nvar dec;\nconsole.log(dec(0));");
}

#[test]
fn a_body_that_mutates_is_left_alone() {
    kept_in_both("let n = 0;\nconst dec = (a) => (n++, [\"x\"][a]);\nconsole.log(dec(0));");
    kept_in_both("const t = [\"x\"];\nconst dec = (a) => (t[0] = \"z\");\nconsole.log(dec(0));");
}

#[test]
fn a_body_that_reads_a_global_is_left_alone() {
    kept_in_both("const dec = (a) => prefix + [\"x\"][a];\nconsole.log(dec(0));");
    kept_in_both("let p = \"q\";\np = \"r\";\nconst dec = (a) => p + a;\nconsole.log(dec(0));");
}

#[test]
fn a_body_that_calls_is_left_alone() {
    kept_in_both("const dec = (a) => String(a);\nconsole.log(dec(0));");
    kept_in_both("const dec = (a) => \"ab\".charAt(a);\nconsole.log(dec(0));");
}

#[test]
fn this_arguments_and_complex_parameters_are_left_alone() {
    kept_in_both("function dec(a) { return this; }\nconsole.log(dec(0));");
    kept_in_both("function dec(a) { return arguments.length; }\nconsole.log(dec(0));");
    kept_in_both("const dec = (a = 1) => a;\nconsole.log(dec(0));");
    kept_in_both("const dec = ({ a }) => a;\nconsole.log(dec(0));");
    kept_in_both("const dec = (...a) => a.length;\nconsole.log(dec(0));");
}

#[test]
fn async_and_generator_functions_are_left_alone() {
    kept_in_both("async function dec(a) { return [\"x\"][a]; }\nconsole.log(dec(0));");
    kept_in_both("function* dec(a) { return [\"x\"][a]; }\nconsole.log(dec(0));");
    kept_in_both("const dec = async (a) => [\"x\"][a];\nconsole.log(dec(0));");
}

#[test]
fn a_non_literal_argument_is_left_alone() {
    kept_in_both("const dec = (a) => [\"x\", \"y\"][a];\nconsole.log(dec(i));");
    kept_in_both("const dec = (a) => [\"x\", \"y\"][a];\nconsole.log(dec(0, f()));");
    kept_in_both("const dec = (a) => [\"x\", \"y\"][a];\nconsole.log(dec(...[0]));");
}

#[test]
fn a_call_before_the_let_declaration_is_left_alone() {
    let source = "function wrap() {\n  console.log(dec(0));\n  let dec = (a) => [\"x\", \"y\"][a];\n  console.log(dec(1));\n}";
    assert_eq!(
        both(source),
        "function wrap() {\n  console.log(dec(0));\n  let dec = (a) => [\"x\", \"y\"][a];\n  console.log(\"y\");\n}"
    );
}

#[test]
fn a_call_before_the_constant_array_declaration_is_left_alone() {
    let source = "function dec(a) { return t[a]; }\nconsole.log(dec(0));\nconst t = [\"x\"];";
    kept_in_both(source);
}

#[test]
fn a_call_inside_a_hoisted_function_is_left_alone() {
    kept_in_both("show();\nlet dec = (a) => [\"x\"][a];\nfunction show() { return dec(0); }");
    kept_in_both("const t = [\"x\"];\nfunction show() { const dec = (a) => t[a]; return dec(0); }");
}

#[test]
fn a_call_inside_its_own_initialiser_is_left_alone() {
    kept_in_both("const dec = (a) => [\"x\"][a], v = (() => dec(0))();");
}

#[test]
fn a_declaration_in_a_switch_case_is_left_alone() {
    kept_in_both(
        "switch (k) {\n  case 0:\n    let dec = (a) => [\"x\"][a];\n  case 1:\n    console.log(dec(0));\n}",
    );
}

#[test]
fn a_var_in_a_nested_block_is_left_alone() {
    kept_in_both(
        "function f(c) {\n  if (c) {\n    var dec = (a) => [\"x\"][a];\n  }\n  return dec(0);\n}",
    );
}

#[test]
fn a_block_function_is_left_alone() {
    kept_in_both("{\n  function dec(a) { return [\"x\"][a]; }\n}\nconsole.log(dec(0));");
}

#[test]
fn a_name_a_block_function_shares_is_left_alone() {
    kept_in_both(
        "var f = function () { return \"v\"; };\n(function () {\n  console.log(f());\n  {\n    function f() { return \"w\"; }\n  }\n})();",
    );
    kept_in_both(
        "const t = [\"x\"];\nconst dec = (a) => t[a];\nconsole.log(dec(0));\n{\n  function t() {}\n}",
    );
}

#[test]
fn an_array_that_escapes_or_is_mutated_is_left_alone() {
    kept_in_both("const t = [\"x\"];\nconst dec = (a) => t[a];\nmutate(t);\nconsole.log(dec(0));");
    kept_in_both(
        "const t = [\"x\"];\nconst dec = (a) => t[a];\nt.push(\"y\");\nconsole.log(dec(0));",
    );
    kept_in_both(
        "const t = [\"x\"];\nconst dec = (a) => t[a];\nt[0] = \"y\";\nconsole.log(dec(0));",
    );
    kept_in_both("const t = [[\"x\"]];\nconst dec = (a) => t[a][0];\nconsole.log(dec(0));");
}

#[test]
fn inherited_and_unusual_keys_are_left_alone() {
    kept_in_both("const f = (k) => ({ a: \"x\" })[k];\nconsole.log(f(\"constructor\"));");
    kept_in_both("const f = (k) => [\"x\"][k];\nconsole.log(f(\"01\"));");
    kept_in_both(
        "const o = { __proto__: { k: \"x\" } };\nconst f = (k) => o[k];\nconsole.log(f(\"k\"));",
    );
    kept_in_both("const f = (i) => \"\\u{1F600}\"[i];\nconsole.log(f(0));");
    kept_in_both("const f = (a) => [a, \"x\"][1];\nconsole.log(f(0));");
}

#[test]
fn an_object_operand_is_left_alone() {
    kept_in_both("const f = (a) => [\"x\"] + a;\nconsole.log(f(0));");
}

#[test]
fn new_and_tagged_calls_are_left_alone() {
    kept_in_both("const dec = (a) => [\"x\"][a];\nconsole.log(new dec(0));");
}

#[test]
fn a_function_passed_elsewhere_is_left_alone_only_in_the_safe_lane() {
    let source = "const dec = (a) => [\"x\", \"y\"][a];\nregister(dec);\nconsole.log(dec(1));";
    assert_eq!(safe(source), safe_identity(source));
    assert_eq!(
        unsafe_lane(source),
        "const dec = (a) => [\"x\", \"y\"][a];\nregister(dec);\nconsole.log(\"y\");"
    );
}

#[test]
fn a_script_global_the_program_may_rebind_by_key_is_left_alone_only_in_the_safe_lane() {
    let source = "function dec(a) { return [\"x\", \"y\"][a]; }\nglobalThis[Object.keys({ dec: 1 })[0]] = function () { return \"z\"; };\nconsole.log(dec(1));";
    assert!(safe(source).contains("dec(1)"), "{}", safe(source));
    assert!(
        unsafe_lane(source).contains("console.log(\"y\")"),
        "{}",
        unsafe_lane(source)
    );
}

#[test]
fn a_script_global_the_program_rebinds_by_a_known_key_is_left_alone_in_both_lanes() {
    kept_in_both(
        "function dec(a) { return [\"x\", \"y\"][a]; }\nglobalThis[\"de\" + \"c\"] = function () { return \"z\"; };\nconsole.log(dec(1));",
    );
}

#[test]
fn a_script_global_the_program_observes_keeps_its_declaration_in_the_unsafe_lane() {
    let source =
        "var t = [\"x\"];\nfunction dec(a) { return t[a]; }\nconsole.log(\"dec\" in this, dec(0));";
    assert_eq!(safe(source), safe_identity(source));
    assert_eq!(
        unsafe_lane(source),
        "var t = [\"x\"];\nfunction dec(a) {\n  return t[a];\n}\nconsole.log(\"dec\" in this, \"x\");"
    );
}

#[test]
fn direct_eval_disables_the_pass() {
    kept_in_both("const dec = (a) => [\"x\"][a];\neval(input);\nconsole.log(dec(0));");
}

#[test]
fn a_program_without_a_pure_function_is_unchanged() {
    let alloc = Allocator::default();
    let source = "console.log(f(1), g(2));\nfunction f(a) { a++; return a; }\nvar g = (a) => h(a);";
    let (_, report) =
        transform_report(&alloc, source, &[pure_calls::boxed], PassOptions::default());
    assert_eq!(report.total_changes, 0);
    assert_eq!(report.iterations, 1);
}
