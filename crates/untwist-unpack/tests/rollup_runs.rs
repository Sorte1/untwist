use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack_program_with, UnpackOptions};

const ASSUMED: UnpackOptions = UnpackOptions {
    split_hoisted: true,
    assume_modules: true,
    conservative: false,
    trust_free_loader: false,
};

const VECTOR: &str = "class Vector {
  constructor(x) { this.x = x; }
  add(other) { return new Vector(this.x + other.x); }
}
function length(v) { return Math.abs(v.x); }
var vectors = /*#__PURE__*/Object.freeze({ __proto__: null, Vector: Vector, length: length });
";
const COLOR: &str = "class Color {
  constructor(hex) { this.hex = hex; }
  mix(other) { return new Color(this.hex | other.hex); }
}
function parseColor(text) { return new Color(parseInt(text, 16)); }
var colors = /*#__PURE__*/Object.freeze({ __proto__: null, Color: Color, parseColor: parseColor });
";
const MAIN: &str = "const a = new Vector(1).add(new Vector(2));
console.log(length(a), parseColor(\"ff\").mix(new Color(1)).hex);
console.log(length(new Vector(-3)), parseColor(\"10\").hex);
console.log(Object.keys(vectors), Object.keys(colors));
";

fn bundle(body: &str) -> String {
    format!("(function () {{\n\"use strict\";\n{body}}})();\n")
}

fn modules(source: &str) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let bundle = unpack_program_with(&alloc, &parsed.program, ASSUMED)?;
    Some(
        bundle
            .modules
            .iter()
            .map(|(id, module)| (id.clone(), pretty_codegen(module.program())))
            .collect(),
    )
}

fn text_of(found: &[(String, String)], id: &str) -> String {
    found
        .iter()
        .find(|(known, _)| known == id)
        .map(|(_, text)| text.clone())
        .unwrap_or_else(|| panic!("no module {id}: {found:?}"))
}

#[test]
fn independent_libraries_before_the_entry_become_modules_named_after_their_classes() {
    let found = modules(&bundle(&format!("{VECTOR}{COLOR}{MAIN}"))).expect("split");
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["Color", "Vector", "index"], "{found:?}");
    let entry = text_of(&found, "index");
    let color = entry.find("\"Color\"").expect("color import");
    let vector = entry.find("\"Vector\"").expect("vector import");
    assert!(color < vector, "{entry}");
    let color_module = text_of(&found, "Color");
    assert!(
        color_module.starts_with("import \"Vector\";"),
        "{color_module}"
    );
    assert!(
        color_module.contains("export { Color, colors, parseColor }"),
        "{color_module}"
    );
}

#[test]
fn a_write_from_another_module_goes_through_a_setter_in_the_declaring_module() {
    let state = "class Store {\n  constructor() { this.items = []; }\n}\nlet current = null;\nfunction peek() { return current; }\nvar stores = /*#__PURE__*/Object.freeze({ __proto__: null, Store: Store, peek: peek });\n";
    let main = "current = new Store();\ncurrent = function () {};\nconsole.log(peek().name, new Store().items.length, stores);\nconsole.log(typeof peek());\n";
    let found = modules(&bundle(&format!("{VECTOR}{state}{main}"))).expect("split");
    let state_module = text_of(&found, "Store");
    assert!(
        state_module.contains("function set_current(value)"),
        "{state_module}"
    );
    let entry = text_of(&found, "index");
    assert!(entry.contains("set_current(new Store())"), "{entry}");
    assert!(
        entry.contains("set_current({ current: function() {} }.current)"),
        "{entry}"
    );
    assert!(entry.contains("from \"Store\""), "{entry}");
}

#[test]
fn compound_writes_keep_the_writer_with_the_declaration() {
    let state = "let total = 0;\nfunction bump() { total += 1; return total; }\nvar counters = /*#__PURE__*/Object.freeze({ __proto__: null, bump: bump });\n";
    let main = "bump();\ntotal += 2;\nconsole.log(total, counters);\n";
    let found = modules(&bundle(&format!("{VECTOR}{state}{main}"))).expect("split");
    let entry = text_of(&found, "index");
    assert!(entry.contains("let total = 0"), "{entry}");
    assert!(!entry.contains("set_total"), "{entry}");
}

#[test]
fn a_statement_that_needs_the_wrapper_function_keeps_everything_after_it_in_the_entry() {
    let pinned = "const self = this;\n";
    let found = modules(&bundle(&format!("{VECTOR}{pinned}{COLOR}{MAIN}"))).expect("split");
    let entry = text_of(&found, "index");
    assert!(entry.contains("const self = this"), "{entry}");
    assert!(entry.contains("class Color"), "{entry}");
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["Vector", "index"], "{found:?}");
}

#[test]
fn a_bundle_without_any_boundary_signal_stays_one_file() {
    let source = bundle("const a = 1;\nconsole.log(a);\n");
    assert!(modules(&source).is_none());
}
