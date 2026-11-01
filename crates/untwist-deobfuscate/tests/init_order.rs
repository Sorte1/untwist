use oxc_allocator::Allocator;
use oxc_span::{SourceType, Span};
use untwist_core::parse::parse_program;
use untwist_core::pass::PassOptions;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::init_order::try_scan_init_order;

fn inlined(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::object_props::boxed])
}

fn assert_inlined(source: &str) {
    let out = inlined(source);
    assert!(!out.contains("m.alpha"), "{source}\n-> {out}");
    assert!(!out.contains(" m = {"), "{source}\n-> {out}");
}

fn assert_refused(source: &str) {
    let out = inlined(source);
    assert!(out.contains("m.alpha"), "{source}\n-> {out}");
}

fn assert_refused_as_script(source: &str) {
    let alloc = untwist_testing::Allocator::default();
    let out = untwist_testing::transform_as(
        &alloc,
        source,
        SourceType::cjs(),
        &[untwist_deobfuscate::object_props::boxed],
    );
    assert!(out.contains("m.alpha"), "{source}\n-> {out}");
}

fn in_a_function(body: &str) -> String {
    format!("function mod() {{\n{body}\n}}\nmod();")
}

fn reference_is_initialised(source: &str, statement: &str, reference: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let order = try_scan_init_order(&parsed.program, &sem.scoping).expect("the scan fits u32");
    let start = offset_of(source, statement);
    let length = u32::try_from(statement.len()).expect("statement length fits");
    let slot = order
        .declaration_slot(Span::new(start, start + length))
        .expect("the declaration statement has a slot");
    let at = offset_of(source, reference);
    order.reference_is_initialised_before_use(slot, Span::new(at, at + 1))
}

fn reference_slot(source: &str, statement: &str, reference: &str) -> Option<usize> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let order = try_scan_init_order(&parsed.program, &sem.scoping).expect("the scan fits u32");
    let start = offset_of(source, statement);
    let length = u32::try_from(statement.len()).expect("statement length fits");
    let anchor = order
        .declaration_slot(Span::new(start, start + length))
        .expect("the anchor statement has a slot");
    let at = offset_of(source, reference);
    order
        .slot_of_reference(anchor, Span::new(at, at + 1))
        .map(|slot| slot.index())
}

fn offset_of(source: &str, needle: &str) -> u32 {
    let found = source.find(needle).expect("the needle is in the source");
    u32::try_from(found).expect("offset fits")
}

#[test]
fn a_read_inside_a_function_expression_above_the_declaration_is_inlined() {
    assert_inlined(
        "var f = function () { return m.alpha; };\nvar m = { alpha: 1 };\nconsole.log(f());",
    );
}

#[test]
fn a_read_inside_an_arrow_above_the_declaration_is_inlined() {
    assert_inlined("const f = () => m.alpha;\nconst m = { alpha: 1 };\nconsole.log(f());");
}

#[test]
fn a_read_inside_a_function_value_declared_after_the_declaration_is_still_inlined() {
    assert_inlined(
        "var m = { alpha: 1 };\nvar f = function () { return m.alpha; };\nconsole.log(f());",
    );
}

#[test]
fn an_unknown_call_in_a_root_cannot_reach_a_local_that_no_root_reads() {
    assert_inlined(&in_a_function(
        "var f = function () { return m.alpha; };\nside();\nvar m = { alpha: 1 };\nreturn f();",
    ));
}

#[test]
fn an_unknown_call_in_a_root_refuses_at_the_top_level_of_a_script() {
    let body = "var f = function () { return m.alpha; };\nside();\nvar m = { alpha: 1 };\nconsole.log(f());";
    assert_refused_as_script(body);
    assert_refused(body);
    assert_inlined(&in_a_function(body));
}

#[test]
fn a_var_binding_in_a_top_level_block_of_a_script_refuses() {
    let body =
        "if (cond) { var f = function () { return m.alpha; }; side(); var m = { alpha: 1 }; f(); }";
    assert_refused_as_script(body);
    assert_refused(body);
}

#[test]
fn a_binding_read_in_a_root_refuses() {
    let body = "var f = function () { return m.alpha; };\nf();\nvar m = { alpha: 1 };";
    assert_refused(body);
    assert_refused(&in_a_function(body));
}

#[test]
fn a_binding_read_from_a_hoisted_body_that_a_root_calls_refuses() {
    let body = "var f = function () { return m.alpha; };\nfunction g() { return f(); }\ng();\nvar m = { alpha: 1 };";
    assert_refused(body);
    assert_refused(&in_a_function(body));
}

#[test]
fn a_binding_read_inside_the_declarations_own_initialiser_refuses() {
    assert_refused("var f = function () { return m.alpha; };\nvar m = { alpha: f() };");
    assert!(!reference_is_initialised(
        &in_a_function("var f = function () { return m.alpha; };\nvar m = wrap(f);"),
        "var m = wrap(f);",
        "m.alpha"
    ));
    assert!(reference_is_initialised(
        &in_a_function("var f = function () { return m.alpha; };\nvar m = wrap(other);"),
        "var m = wrap(other);",
        "m.alpha"
    ));
}

#[test]
fn an_escape_by_argument_refuses() {
    let body = "var f = function () { return m.alpha; };\nregister(f);\nvar m = { alpha: 1 };";
    assert_refused(body);
    assert_refused(&in_a_function(body));
}

#[test]
fn an_escape_by_property_store_refuses() {
    for body in [
        "var f = function () { return m.alpha; };\nvar o = { run: f };\nvar m = { alpha: 1 };",
        "var f = function () { return m.alpha; };\nexports.f = f;\nvar m = { alpha: 1 };",
    ] {
        assert_refused(body);
        assert_refused(&in_a_function(body));
    }
}

#[test]
fn an_iife_in_a_root_refuses() {
    let body =
        "var f = function () { return m.alpha; };\n(function () { f(); })();\nvar m = { alpha: 1 };";
    assert_refused(body);
    assert_refused(&in_a_function(body));
}

#[test]
fn an_accessor_in_a_root_refuses() {
    let body = "var f = function () { return m.alpha; };\nvar o = { get p() { return f(); } };\no.p;\nvar m = { alpha: 1 };";
    assert_refused(body);
    assert_refused(&in_a_function(body));
}

#[test]
fn a_coercion_method_in_a_root_refuses() {
    let body = "var f = function () { return m.alpha; };\nvar o = { toString() { return f(); } };\n\"\" + o;\nvar m = { alpha: 1 };";
    assert_refused(body);
    assert_refused(&in_a_function(body));
}

#[test]
fn an_escaping_anonymous_value_in_a_root_refuses() {
    for body in [
        "exports.f = function () { return m.alpha; };\nrequire(\"x\");\nvar m = { alpha: 1 };",
        "emitter.on(\"x\", function () { return m.alpha; });\nvar m = { alpha: 1 };",
    ] {
        assert_refused(body);
        assert_refused(&in_a_function(body));
    }
}

#[test]
fn a_rebound_function_binding_refuses() {
    assert_refused(&in_a_function(
        "var f = function () { return m.alpha; };\nside();\nvar m = { alpha: 1 };\nf = other;\nreturn f();",
    ));
}

#[test]
fn a_function_binding_that_is_also_a_parameter_refuses() {
    assert_refused(
        "function mod(f) {\nvar f = function () { return m.alpha; };\nside();\nvar m = { alpha: 1 };\nreturn f();\n}\nmod();",
    );
}

#[test]
fn a_scope_hazard_refuses() {
    for hazard in ["eval(\"side()\");", "with (side) { other(); }"] {
        assert_refused(&in_a_function(&format!(
            "var f = function () {{ return m.alpha; }};\nside();\nvar m = {{ alpha: 1 }};\n{hazard}\nreturn f();"
        )));
    }
}

#[test]
fn a_second_initialised_declarator_refuses() {
    assert_refused(&in_a_function(
        "var f = function () { return m.alpha; };\nside();\nvar m = { alpha: 1 };\nvar f = other;\nreturn f();",
    ));
}

#[test]
fn a_class_in_a_root_refuses_and_one_below_the_declaration_is_inlined() {
    assert_refused(&in_a_function(
        "class C { go() { return m.alpha; } }\nside();\nvar m = { alpha: 1 };\nreturn new C().go();",
    ));
    assert_inlined(&in_a_function(
        "side();\nvar m = { alpha: 1 };\nclass C { go() { return m.alpha; } }\nreturn new C().go();",
    ));
}

#[test]
fn a_class_header_read_above_the_declaration_refuses() {
    assert_refused(&in_a_function(
        "class C extends base(m.alpha) {}\nside();\nvar m = { alpha: 1 };\nreturn C;",
    ));
    assert_refused(&in_a_function(
        "class C { [m.alpha]() {} }\nside();\nvar m = { alpha: 1 };\nreturn C;",
    ));
}

#[test]
fn a_class_header_read_is_a_statement_level_read() {
    let extended = in_a_function(
        "side();\nvar m = { alpha: 1 };\nclass C extends base(m.alpha) {}\nreturn C;",
    );
    assert_eq!(
        reference_slot(&extended, "var m = { alpha: 1 };", "m.alpha"),
        Some(2),
        "{extended}"
    );
    let keyed =
        in_a_function("side();\nvar m = { alpha: 1 };\nclass C { [m.alpha]() {} }\nreturn C;");
    assert_eq!(
        reference_slot(&keyed, "var m = { alpha: 1 };", "m.alpha"),
        Some(2),
        "{keyed}"
    );
    let body = in_a_function(
        "side();\nvar m = { alpha: 1 };\nclass C { go() { return m.alpha; } }\nreturn C;",
    );
    assert_eq!(
        reference_slot(&body, "var m = { alpha: 1 };", "m.alpha"),
        None,
        "{body}"
    );
}

#[test]
fn only_a_class_elements_own_key_is_read_at_the_class_statement() {
    let field = in_a_function("side();\nvar v = 1;\nclass C { x = { [v]: 1 }; }\nreturn C;");
    assert_eq!(reference_slot(&field, "var v = 1;", "v]"), None, "{field}");
    let key = in_a_function("side();\nvar v = 1;\nclass C { [v] = 1; }\nreturn C;");
    assert_eq!(reference_slot(&key, "var v = 1;", "v]"), Some(2), "{key}");
}

#[test]
fn a_declaration_after_empty_bodies_that_share_its_lists_start_keeps_its_own_list() {
    let source = "function a() {}\nfunction b() {}\nfunction c() {\n  var m = { alpha: 1 };\n  return m.alpha;\n}\nc();";
    assert_eq!(
        reference_slot(source, "var m = { alpha: 1 };", "m.alpha"),
        Some(1),
        "{source}"
    );
    assert!(
        reference_is_initialised(source, "var m = { alpha: 1 };", "m.alpha"),
        "{source}"
    );
}

#[test]
fn a_declaration_in_the_last_list_behind_an_inert_prefix_still_counts_itself() {
    let source = "function f() {\n  var k = 1;\n  var m = { alpha: 1 }, C = class { x = m.alpha; };\n  return C;\n}\nf();";
    assert!(
        !reference_is_initialised(
            source,
            "var m = { alpha: 1 }, C = class { x = m.alpha; };",
            "m.alpha"
        ),
        "{source}"
    );
}

#[test]
fn a_body_inside_a_body_of_an_unread_binding_is_inlined() {
    assert_inlined(&in_a_function(
        "var f = function () { return function () { return m.alpha; }; };\nside();\nvar m = { alpha: 1 };\nreturn f()();",
    ));
}

#[test]
fn a_reachability_chain_within_the_budget_is_inlined() {
    assert_inlined(&chain(BUDGET));
}

#[test]
fn a_reachability_chain_beyond_the_budget_refuses() {
    assert_refused(&chain(BUDGET + 1));
}

#[test]
fn a_reachability_fan_out_beyond_the_budget_refuses() {
    assert_refused(&fan_out(BUDGET));
}

#[test]
fn an_exhausted_budget_is_reported() {
    let source = chain(BUDGET + 1);
    let alloc = untwist_testing::Allocator::default();
    let (_code, report) = untwist_testing::transform_report(
        &alloc,
        &source,
        &[untwist_deobfuscate::object_props::boxed],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    let messages: Vec<&str> = report
        .notes
        .iter()
        .map(|note| note.message.as_str())
        .collect();
    assert!(
        messages.contains(
            &"init-order budget exhausted: a value that reads m was treated as running first"
        ),
        "{messages:?}"
    );
}

const BUDGET: usize = 4096;

fn chain(links: usize) -> String {
    let mut body = String::from("var f0 = function () { return m.alpha; };\n");
    for index in 1..links {
        let previous = index - 1;
        body.push_str(&format!(
            "var f{index} = function () {{ return f{previous}(); }};\n"
        ));
    }
    let last = links - 1;
    body.push_str(&format!(
        "side();\nvar m = {{ alpha: 1 }};\nreturn f{last}();\n"
    ));
    in_a_function(&body)
}

fn fan_out(readers: usize) -> String {
    let mut body = String::from("var f0 = function () { return m.alpha; };\n");
    for index in 1..=readers {
        body.push_str(&format!("var g{index} = function () {{ return f0(); }};\n"));
    }
    body.push_str("side();\nvar m = { alpha: 1 };\nreturn f0();\n");
    in_a_function(&body)
}

const A_HOISTED_READ_AT_DEPTH_ZERO: &str =
    "side();\nvar m = { alpha: 1 };\nfunction f() {\n  return m.alpha;\n}\nf();";

fn printed_alone(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(&alloc, source, &[])
}

fn under_conservative(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_deobfuscate::object_props::boxed],
        PassOptions {
            unsafe_transforms: true,
            extended: false,
            ..PassOptions::default()
        },
    )
}

#[test]
fn a_hoisted_read_at_depth_zero_refuses_in_both_modes() {
    let untouched = printed_alone(A_HOISTED_READ_AT_DEPTH_ZERO);
    assert_eq!(inlined(A_HOISTED_READ_AT_DEPTH_ZERO), untouched);
    assert_eq!(under_conservative(A_HOISTED_READ_AT_DEPTH_ZERO), untouched);
    assert_refused(A_HOISTED_READ_AT_DEPTH_ZERO);
    assert_refused_as_script(A_HOISTED_READ_AT_DEPTH_ZERO);
}

#[test]
fn the_same_shape_one_function_deep_still_inlines() {
    assert_inlined(&in_a_function(
        "side();\nvar m = { alpha: 1 };\nfunction f() {\n  return m.alpha;\n}\nreturn f();",
    ));
}
