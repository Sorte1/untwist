use untwist_core::pass::{NoteKind, PassOptions};
use untwist_testing::{transform_report, transform_with, Allocator};
use untwist_unminify::unwrap_comma_call::SCOPE_HAZARD;

fn run(source: &str, extended: bool) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &[untwist_unminify::unwrap_comma_call::boxed],
        PassOptions {
            unsafe_transforms: false,
            extended,
            max_iterations: None,
            fuel: None,
            ..PassOptions::default()
        },
    )
}

#[test]
fn a_zero_prefixed_identifier_call_is_unwrapped() {
    assert_eq!(run("(0, f)(a, b);", true), "f(a, b);");
}

#[test]
fn conservative_leaves_it_alone() {
    assert_eq!(run("(0, f)(a, b);", false), "(0, f)(a, b);");
}

#[test]
fn any_pure_literal_prefix_works() {
    assert_eq!(run("(void 0, f)(a);", true), "f(a);");
    assert_eq!(run("(\"\", f)(a);", true), "f(a);");
    assert_eq!(run("(null, f)(a);", true), "f(a);");
}

#[test]
fn several_pure_prefixes_are_all_dropped() {
    assert_eq!(run("(0, 1, f)(a);", true), "f(a);");
}

#[test]
fn a_member_callee_is_left_alone_in_every_mode() {
    assert_eq!(run("(0, obj.f)(a);", true), "(0, obj.f)(a);");
    assert_eq!(run("(0, obj.f)(a);", false), "(0, obj.f)(a);");
    assert_eq!(run("(0, obj[k])(a);", true), "(0, obj[k])(a);");
}

#[test]
fn an_impure_prefix_is_left_alone() {
    assert_eq!(run("(log(), f)(a);", true), "(log(), f)(a);");
    assert_eq!(run("(i++, f)(a);", true), "(i++, f)(a);");
}

#[test]
fn a_parenthesised_function_expression_callee_is_unwrapped() {
    assert_eq!(
        run("(0, function () { return 1; })(a);", true),
        "(function () {\n  return 1;\n})(a);"
    );
}

#[test]
fn an_arrow_callee_is_unwrapped_too() {
    assert_eq!(run("(0, () => 1)(a);", true), "(() => 1)(a);");
}

#[test]
fn a_sequence_that_is_not_a_callee_is_left_alone() {
    assert_eq!(run("x = (0, f);", true), "x = (0, f);");
}

#[test]
fn a_new_expression_is_left_alone() {
    assert_eq!(run("new (0, F)(a);", true), "new (0, F)(a);");
}

#[test]
fn an_optional_call_is_left_alone() {
    assert_eq!(run("(0, f)?.(a);", true), "(0, f)?.(a);");
}

#[test]
fn a_tagged_template_is_left_alone() {
    assert_eq!(run("(0, tag)`x`;", true), "(0, tag)`x`;");
}

#[test]
fn an_indirect_eval_is_left_alone() {
    assert_eq!(run("(0, eval)(s);", true), "(0, eval)(s);");
    assert_eq!(run("(0, eval)(s);", false), "(0, eval)(s);");
}

fn run_report(source: &str) -> (String, Vec<untwist_core::pass::Note>) {
    let alloc = Allocator::default();
    let (code, report) = transform_report(
        &alloc,
        source,
        &[untwist_unminify::unwrap_comma_call::boxed],
        PassOptions {
            unsafe_transforms: false,
            extended: true,
            max_iterations: None,
            fuel: None,
            ..PassOptions::default()
        },
    );
    (code, report.notes)
}

fn assert_refused_whole(source: &str) {
    let (code, notes) = run_report(source);
    assert_eq!(code, source);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(notes[0].message, SCOPE_HAZARD);
    assert_eq!(notes[0].kind, NoteKind::Refusal);
    assert_eq!(notes[0].span, None);
    assert_eq!(notes[0].pass, untwist_unminify::unwrap_comma_call::PASS);
}

#[test]
fn a_with_block_refuses_the_whole_program() {
    assert_refused_whole(
        "var o = { f: function () {\n  return this === o;\n} };\nwith(o) {\n  console.log((0, f)());\n}",
    );
}

#[test]
fn a_direct_eval_refuses_the_whole_program() {
    assert_refused_whole("function g() {\n  eval(s);\n  return (0, f)(a);\n}\n(0, h)(b);");
}

#[test]
fn the_scope_hazard_note_never_begins_with_the_pass_name() {
    assert!(!SCOPE_HAZARD.starts_with(untwist_unminify::unwrap_comma_call::PASS));
    assert!(!SCOPE_HAZARD.starts_with("unwrap"));
}
