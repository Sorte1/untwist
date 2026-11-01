use untwist_core::parse::{error_count, parse_program};
use untwist_core::pass::PassOptions;
use untwist_testing::{transform_report, Allocator};

fn opts() -> PassOptions {
    PassOptions {
        unsafe_transforms: true,
        extended: false,
        max_iterations: None,
        fuel: None,
        ..PassOptions::default()
    }
}

fn run(source: &str) -> String {
    run_with(source, opts())
}

fn run_default(source: &str) -> String {
    run_with(
        source,
        PassOptions {
            extended: true,
            ..opts()
        },
    )
}

fn run_with(source: &str, opts: PassOptions) -> String {
    let alloc = Allocator::default();
    let code = transform_report(&alloc, source, &[untwist_jsx::classic::boxed], opts).0;
    let errors = {
        let arena = Allocator::default();
        let parsed = parse_program(&arena, &code);
        error_count(&parsed.diagnostics)
    };
    assert_eq!(errors, 0, "output does not re-parse: {code}");
    code
}

fn notes_of(source: &str) -> Vec<String> {
    notes_of_with(source, opts())
}

fn notes_of_with(source: &str, opts: PassOptions) -> Vec<String> {
    let alloc = Allocator::default();
    transform_report(&alloc, source, &[untwist_jsx::classic::boxed], opts)
        .1
        .notes
        .into_iter()
        .map(|note| note.message)
        .collect()
}

#[test]
fn tag_name_type() {
    assert_eq!(run("React.createElement(\"div\", null);"), "<div />;");
}

#[test]
fn component_type() {
    assert_eq!(run("React.createElement(TodoList, null);"), "<TodoList />;");
}

#[test]
fn deeply_nested_member_expression_type() {
    assert_eq!(
        run("React.createElement(components.list.TodoList, null);"),
        "<components.list.TodoList />;"
    );
}

#[test]
fn computed_member_access_on_the_pragma_is_accepted() {
    assert_eq!(run("React[\"createElement\"](\"div\", null);"), "<div />;");
}

#[test]
fn rename_component_with_conflicting_name() {
    assert_eq!(
        run("function a(){} React.createElement(a, null);"),
        "function Component() {}\n<Component />;"
    );
}

#[test]
fn a_lowercase_component_used_twice_is_renamed_once() {
    assert_eq!(
        run("function a(){} React.createElement(a, null); React.createElement(a, null);"),
        "function Component() {}\n<Component />;\n<Component />;"
    );
}

#[test]
fn two_lowercase_types_get_two_distinct_names() {
    assert_eq!(
        run("function a(){} function b(){} React.createElement(a, null); React.createElement(b, null);"),
        "function Component() {}\nfunction Component_2__() {}\n<Component />;\n<Component_2__ />;"
    );
}

#[test]
fn a_component_whose_name_starts_with_a_non_ascii_letter_is_not_renamed() {
    assert_eq!(
        run("function ñ(){} React.createElement(ñ, null);"),
        "function ñ() {}\n<ñ />;"
    );
}

#[test]
fn a_lowercase_type_with_no_binding_is_left_alone() {
    assert_eq!(
        run("React.createElement(a, null);"),
        "React.createElement(a, null);"
    );
}

#[test]
fn attributes() {
    assert_eq!(
        run("React.createElement(\"div\", { \"data-hover\": \"tooltip\", style: { display: \"block\" } });"),
        "<div data-hover=\"tooltip\" style={{ display: \"block\" }} />;"
    );
}

#[test]
fn spread_attributes() {
    assert_eq!(
        run("React.createElement(\"div\", {...props});"),
        "<div {...props} />;"
    );
}

#[test]
fn children() {
    assert_eq!(
        run("React.createElement(\"div\", null, React.createElement(\"span\", null, \"Hello \", name));"),
        "<div><span>Hello {name}</span></div>;"
    );
}

#[test]
fn spread_children() {
    assert_eq!(
        run("React.createElement(\"div\", null, ...children);"),
        "<div>{...children}</div>;"
    );
}

#[test]
fn text_with_special_chars() {
    assert_eq!(
        run("React.createElement(\"div\", null, \".style { color: red; }\");"),
        "<div>{\".style { color: red; }\"}</div>;"
    );
}

#[test]
fn a_string_with_an_html_entity_is_never_raw_jsx_text_or_attribute_text() {
    let source =
        "React.createElement(\"div\", { title: \"a &amp; b\" }, \"1 &amp; 2\", \"&#123;x\");";
    let expected = "<div title={\"a &amp; b\"}>{\"1 &amp; 2\"}{\"&#123;x\"}</div>;";
    assert_eq!(run(source), expected);
    assert_eq!(run_default(source), expected);
}

#[test]
fn a_string_with_a_newline_is_never_raw_attribute_text() {
    let source = "React.createElement(\"div\", { title: \"line1\\n   line2\" }, \"a\\nb\");";
    let expected = "<div title={\"line1\\n   line2\"}>{\"a\\nb\"}</div>;";
    assert_eq!(run(source), expected);
    assert_eq!(run_default(source), expected);
}

#[test]
fn fragment() {
    assert_eq!(
        run("React.createElement(React.Fragment, null, React.createElement(\"span\", null), \"test\");"),
        "<><span />test</>;"
    );
}

#[test]
fn a_fragment_child_prints_inline_by_default_and_in_a_container_under_conservative() {
    let source =
        "React.createElement(\"div\", null, React.createElement(React.Fragment, null, \"x\"));";
    assert_eq!(run_default(source), "<div><>x</></div>;");
    assert_eq!(run(source), "<div>{<>x</>}</div>;");
}

#[test]
fn fragment_with_key_is_an_element_not_a_fragment() {
    assert_eq!(
        run("React.createElement(React.Fragment, { key: o });"),
        "<React.Fragment key={o} />;"
    );
}

#[test]
fn fragment_with_children() {
    assert_eq!(
        run("React.createElement(React.Fragment, null, \"test\", ...children);"),
        "<>test{...children}</>;"
    );
}

#[test]
fn leading_comments_are_dropped() {
    assert_eq!(
        run("f(/*#__PURE__*/React.createElement(\"h1\", null, /*#__PURE__*/React.createElement(\"div\", null)));"),
        "f(<h1><div /></h1>);"
    );
}

#[test]
fn props_with_escaped_strings() {
    let source = "React.createElement(Foo, {bar: 'abc'});\n\
                  React.createElement(Foo, {bar: 'a\\\\nbc'});\n\
                  React.createElement(Foo, {bar: 'ab\\\\tc'});\n\
                  React.createElement(Foo, {bar: 'ab\"c'});\n\
                  React.createElement(Foo, {bar: \"ab'c\"});";
    assert_eq!(
        run(source),
        "<Foo bar=\"abc\" />;\n<Foo bar={\"a\\\\nbc\"} />;\n<Foo bar={\"ab\\\\tc\"} />;\n<Foo bar={\"ab\\\"c\"} />;\n<Foo bar=\"ab'c\" />;"
    );
}

#[test]
fn a_property_shape_jsx_cannot_express_refuses_the_whole_element() {
    assert_eq!(
        run("React.createElement(\"div\", { [k]: 1 });"),
        "React.createElement(\"div\", { [k]: 1 });"
    );
}

#[test]
fn a_lone_surrogate_prop_key_refuses_the_element_rather_than_dropping_its_props() {
    let source = "React.createElement(\"div\", { \"\\ud800\": 1, id: \"a\" });";
    let out = run(source);
    assert!(out.contains("createElement"), "{out}");
    assert!(out.contains("id"), "{out}");
}

#[test]
fn a_user_component_named_fragment_is_an_element_not_a_fragment() {
    assert_eq!(
        run("React.createElement(Modal.Fragment, null, \"x\");"),
        "<Modal.Fragment>x</Modal.Fragment>;"
    );
}

#[test]
fn a_computed_fragment_member_is_still_a_fragment() {
    assert_eq!(
        run("React.createElement(React[\"Fragment\"], null, \"x\");"),
        "<>x</>;"
    );
}

#[test]
fn a_prop_key_with_a_space_refuses_the_element() {
    let source = "React.createElement(\"div\", { \"a b\": 1 });";
    assert_eq!(run(source), source);
}

#[test]
fn an_empty_prop_key_refuses_the_element() {
    let source = "React.createElement(\"div\", { \"\": 1 });";
    assert_eq!(run(source), source);
}

#[test]
fn a_prop_key_that_looks_like_an_attribute_refuses_the_element() {
    let source = "React.createElement(\"div\", { \"a=b\": 1 });";
    assert_eq!(run(source), source);
}

#[test]
fn a_tag_name_jsx_cannot_spell_refuses_the_element() {
    let source = "React.createElement(\"\", null);";
    assert_eq!(run(source), source);
    let notes = notes_of(source);
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with(untwist_jsx::notes::ELEMENT_NAME_SHAPE)),
        "{notes:?}"
    );
}

#[test]
fn a_string_tag_that_reads_as_a_component_is_refused_by_default_and_converted_under_conservative() {
    let source = "React.createElement(\"Foo\", null);";
    assert_eq!(run_default(source), source);
    let default_opts = PassOptions {
        extended: true,
        ..opts()
    };
    let notes = notes_of_with(source, default_opts);
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with(untwist_jsx::notes::COMPONENT_LOOKING_STRING)),
        "{notes:?}"
    );
    assert_eq!(run(source), "<Foo />;");
}

#[test]
fn every_refused_shape_is_reported_by_a_catalogued_note() {
    let refusals = [
        (
            "React.createElement(\"div\", { \"a b\": 1 });",
            untwist_jsx::notes::ATTRIBUTE_SHAPE,
        ),
        (
            "React.createElement(a, null);",
            untwist_jsx::notes::LOWERCASE_UNBOUND,
        ),
    ];
    for (source, message) in refusals {
        let notes = notes_of(source);
        assert!(
            notes.iter().any(|note| note.starts_with(message)),
            "{source} did not report {message}: {notes:?}"
        );
    }
}
