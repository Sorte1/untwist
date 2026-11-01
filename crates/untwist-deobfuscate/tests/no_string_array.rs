mod common;

#[test]
fn a_control_flow_switch_is_flattened_without_any_string_array() {
    let (code, _notes) = common::run_passes(
        "function hi() {\n  var d = '2|0|1'.split('|');\n  var e = 0;\n  while (!![]) {\n    switch (d[e++]) {\n      case '0':\n        a();\n        continue;\n      case '1':\n        b();\n        continue;\n      case '2':\n        c();\n        continue;\n    }\n    break;\n  }\n}",
    );
    assert_eq!(code.trim(), "function hi() {\n  c();\n  a();\n  b();\n}");
}

#[test]
fn a_control_flow_object_is_inlined_without_any_string_array() {
    let (code, _notes) = common::run_passes(
        "function hi() {\n  var obj = { aokfw: function (a, b) { return a + b; } };\n  return obj.aokfw(1, 2);\n}",
    );
    assert_eq!(code.trim(), "function hi() {\n  return 1 + 2;\n}");
}

#[test]
fn dead_code_is_removed_without_any_string_array() {
    let (code, _notes) = common::run_passes("if (\"a\" !== \"a\") {\n  a();\n} else {\n  b();\n}");
    assert_eq!(code.trim(), "b();");
}
