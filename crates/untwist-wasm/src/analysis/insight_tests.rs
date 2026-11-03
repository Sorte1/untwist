use serde_json::Value;

use super::analyze;

fn at(text: &str, needle: &str) -> u32 {
    let byte = text
        .find(needle)
        .unwrap_or_else(|| panic!("no {needle} in the text"));
    u32::try_from(text[..byte].encode_utf16().count()).unwrap()
}

fn definition(text: &str, needle: &str) -> Value {
    let json = analyze(text)
        .definition(at(text, needle))
        .unwrap_or_else(|| panic!("no definition at {needle}"));
    serde_json::from_str(&json).unwrap()
}

fn completion(text: &str, pos: u32, label: &str) -> Value {
    let list: Value = serde_json::from_str(&analyze(text).completions(pos)).unwrap();
    list.as_array()
        .unwrap()
        .iter()
        .find(|item| item["label"] == label)
        .cloned()
        .unwrap_or_else(|| panic!("no completion {label}"))
}

fn described(text: &str, pos: u32, name: &str) -> Option<Value> {
    analyze(text)
        .describe(pos, name)
        .map(|json| serde_json::from_str(&json).unwrap())
}

fn properties(text: &str, pos: u32, name: &str) -> Vec<String> {
    described(text, pos, name)
        .map(|value| serde_json::from_value(value["properties"].clone()).unwrap())
        .unwrap_or_default()
}

fn end(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap()
}

#[test]
fn a_constant_shows_its_decoded_string_value() {
    let text = "const greeting = \"\\x68\\x69\\u0021\"; log(greeting);";
    let shown = definition(text, "greeting)");
    assert_eq!(shown["kind"], "const");
    assert_eq!(shown["value"], "\"hi!\"");
    assert_eq!(shown["decoded"], true);
    assert_eq!(shown["form"], "string");
}

#[test]
fn a_plain_string_is_not_marked_decoded_and_numbers_keep_their_source() {
    let text = "const name = 'plain'; const size = 0x1a; const yes = !0; name; size; yes;";
    assert_eq!(definition(text, "name;")["value"], "\"plain\"");
    assert_eq!(definition(text, "name;").get("decoded"), None);
    assert_eq!(definition(text, "size;")["value"], "0x1a");
    assert_eq!(definition(text, "size;")["form"], "number");
    assert_eq!(definition(text, "yes;")["value"], "true");
}

#[test]
fn a_reassigned_variable_shows_no_value() {
    let text = "var count = 1; count = 2; count;";
    let shown = definition(text, "count;");
    assert_eq!(shown["kind"], "var");
    assert_eq!(shown.get("value"), None);
    assert_eq!(shown.get("form"), None);
}

#[test]
fn a_long_string_value_is_cut_short() {
    let text = format!("const long = \"{}\"; long;", "a".repeat(500));
    let shown = definition(&text, "long;");
    let value = shown["value"].as_str().unwrap();
    assert!(value.chars().count() < 220, "{value}");
    assert!(value.ends_with("…\""), "{value}");
}

#[test]
fn functions_and_arrows_list_their_parameters_and_params_are_their_own_kind() {
    let text = "function add(a, b = 2, ...rest) { return a; }\nconst twice = (x) => x * 2;\nadd(1); twice(1);";
    let add = definition(text, "add(1)");
    assert_eq!(add["form"], "function");
    assert_eq!(add["params"], serde_json::json!(["a", "b = 2", "...rest"]));
    let twice = definition(text, "twice(1)");
    assert_eq!(twice["params"], serde_json::json!(["x"]));
    assert_eq!(definition(text, "a; }")["kind"], "param");
}

#[test]
fn a_class_lists_its_constructor_parameters() {
    let text = "class Point { constructor(x, y) { this.x = x; } }\nnew Point(1, 2);";
    let point = definition(text, "Point(1");
    assert_eq!(point["form"], "class");
    assert_eq!(point["params"], serde_json::json!(["x", "y"]));
}

#[test]
fn an_instance_names_its_constructor() {
    let text = "const seen = new Map(); seen;";
    assert_eq!(definition(text, "seen;")["form"], "new Map");
}

#[test]
fn completions_carry_function_parameters_and_the_param_kind() {
    let text = "function add(a, b) { return a; }\nfunction run(input) {\n  \n}";
    let pos = at(text, "  \n}") + 2;
    assert_eq!(
        completion(text, pos, "add")["params"],
        serde_json::json!(["a", "b"])
    );
    assert_eq!(completion(text, pos, "input")["kind"], "param");
    assert_eq!(completion(text, pos, "input").get("params"), None);
}

#[test]
fn properties_seen_on_an_object_include_its_literal_keys_and_uses() {
    let text = "const box = { width: 1, 'height': 2, nested() {} };\nbox.depth = 3;\nbox.width;\nbox[\"color\"];\n";
    assert_eq!(
        properties(text, end(text), "box"),
        vec!["color", "depth", "height", "nested", "width"]
    );
}

#[test]
fn properties_of_an_unresolved_global_are_gathered_by_name() {
    let text = "window.alpha = 1; window.beta(); other.gamma;\n";
    assert_eq!(properties(text, end(text), "window"), vec!["alpha", "beta"]);
    assert_eq!(
        described(text, end(text), "window").unwrap()["kind"],
        "global"
    );
    assert_eq!(described(text, end(text), "nothing"), None);
}

#[test]
fn properties_are_per_binding_found_from_the_scope_at_the_cursor() {
    let text = "function f() { const o = { a: 1 }; return o.a; }\nconst o = { b: 2 };\n";
    assert_eq!(properties(text, at(text, "return"), "o"), vec!["a"]);
    assert_eq!(properties(text, end(text), "o"), vec!["b"]);
}

#[test]
fn a_name_is_described_from_the_scope_at_a_position() {
    let text =
        "function add(a, b) { return a + b; }\nconst label = \"\\x41\";\nconst list = [1];\n";
    let add = described(text, end(text), "add").unwrap();
    assert_eq!(add["kind"], "function");
    assert_eq!(add["params"], serde_json::json!(["a", "b"]));
    let label = described(text, end(text), "label").unwrap();
    assert_eq!(label["value"], "\"A\"");
    assert_eq!(label["decoded"], true);
    assert_eq!(described(text, end(text), "list").unwrap()["form"], "array");
    assert_eq!(
        described(text, at(text, "return"), "a").unwrap()["kind"],
        "param"
    );
}
