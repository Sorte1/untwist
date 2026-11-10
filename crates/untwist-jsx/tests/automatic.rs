use untwist_core::ast_util::rename::UNIQUE_NAME_SUFFIX_CAP;
use untwist_core::pass::{Note, PassCtor, PassGroup, PassOptions, RunReport, SPANNED_NOTE_CAP};
use untwist_testing::Allocator;

fn opts(extended: bool) -> PassOptions {
    PassOptions {
        unsafe_transforms: true,
        extended,
        max_iterations: None,
        fuel: None,
        ..PassOptions::default()
    }
}

fn transform(source: &str, extended: bool) -> (String, RunReport) {
    let alloc = Allocator::default();
    transform_with(&alloc, &[untwist_jsx::automatic::boxed], source, extended)
}

fn transform_with<'a>(
    alloc: &'a Allocator,
    ctors: &[PassCtor<'a>],
    source: &'a str,
    extended: bool,
) -> (String, RunReport) {
    let group = PassGroup::from_ctors(ctors, opts(extended));
    untwist_testing::transform_report_with(alloc, source, group)
}

fn run(source: &str) -> String {
    transform(source, false).0
}

fn run_default(source: &str) -> String {
    transform(source, true).0
}

fn notes_of(source: &str) -> Vec<Note> {
    transform(source, false).1.notes
}

fn run_both_runtimes(source: &str, extended: bool) -> String {
    let alloc = Allocator::default();
    transform_with(&alloc, &untwist_jsx::jsx_ctors(), source, extended).0
}

fn every_component_uid_bound() -> String {
    let mut source = String::from("var Component = 1;\n");
    for suffix in 2..=UNIQUE_NAME_SUFFIX_CAP {
        source.push_str(&format!("var Component_{suffix}__ = 1;\n"));
    }
    source.push_str("function a() {}\njsx(a, {});");
    source
}

#[test]
fn an_exhausted_uid_budget_leaves_the_call_and_every_component_name_as_they_were() {
    let source = every_component_uid_bound();
    let code = run_default(&source);
    assert!(code.contains("function a() {}\njsx(a, {});"), "{code}");
    assert_eq!(
        code.matches("Component").count(),
        source.matches("Component").count(),
        "a Component identifier was introduced: {code}"
    );
}

#[test]
fn tag_name_type() {
    assert_eq!(run("jsx(\"div\", {});"), "<div />;");
}

#[test]
fn component_type() {
    assert_eq!(run("jsx(TodoList, {});"), "<TodoList />;");
}

#[test]
fn deeply_nested_member_expression_type() {
    assert_eq!(
        run("jsx(components.list.TodoList, {});"),
        "<components.list.TodoList />;"
    );
}

#[test]
fn any_other_expression_type_is_hoisted() {
    assert_eq!(
        run("jsx(r ? \"a\" : \"div\", {});"),
        "const Component = r ? \"a\" : \"div\";\n<Component />;"
    );
}

#[test]
fn rename_component_with_conflicting_name_carries_babels_prefix_under_conservative() {
    assert_eq!(
        run("function a(){} jsx(a, {});"),
        "function _Component() {}\n<_Component />;"
    );
}

#[test]
fn the_default_mode_uses_the_same_uid_generator_as_the_classic_runtime() {
    assert_eq!(
        run_default("function a(){} jsx(a, {});"),
        "function Component() {}\n<Component />;"
    );
}

#[test]
fn a_lowercase_component_used_twice_is_renamed_once() {
    assert_eq!(
        run("function a(){} jsx(a, {}); jsx(a, {});"),
        "function _Component() {}\n<_Component />;\n<_Component />;"
    );
    assert_eq!(
        run_default("function a(){} jsx(a, {}); jsx(a, {});"),
        "function Component() {}\n<Component />;\n<Component />;"
    );
}

#[test]
fn two_lowercase_types_get_two_distinct_names() {
    assert_eq!(
        run("function a(){} function b(){} jsx(a, {}); jsx(b, {});"),
        "function _Component() {}\nfunction _Component_2__() {}\n<_Component />;\n<_Component_2__ />;"
    );
}

#[test]
fn a_component_whose_name_starts_with_a_non_ascii_letter_is_not_renamed() {
    assert_eq!(run("function ñ(){} jsx(ñ, {});"), "function ñ() {}\n<ñ />;");
}

#[test]
fn attributes() {
    assert_eq!(
        run("jsx(\"div\", { \"data-hover\": \"tooltip\", style: { display: \"block\" } });"),
        "<div data-hover=\"tooltip\" style={{ display: \"block\" }} />;"
    );
}

#[test]
fn spread_attributes() {
    assert_eq!(run("jsx(\"div\", {...props});"), "<div {...props} />;");
}

#[test]
fn children() {
    assert_eq!(
        run("jsx(\"div\", { children: jsxs(\"span\", { children: [\"Hello \", name] }) });"),
        "<div><span>Hello {name}</span></div>;"
    );
}

#[test]
fn text_with_special_chars() {
    assert_eq!(
        run("jsx(\"div\", { children: \".style { color: red; }\" });"),
        "<div>{\".style { color: red; }\"}</div>;"
    );
}

#[test]
fn a_string_with_an_html_entity_is_never_raw_jsx_text_or_attribute_text() {
    let source = "jsxs(\"div\", { title: \"a &amp; b\", children: [\"1 &amp; 2\", \"&#123;x\"] });";
    let expected = "<div title={\"a &amp; b\"}>{\"1 &amp; 2\"}{\"&#123;x\"}</div>;";
    assert_eq!(run(source), expected);
    assert_eq!(run_default(source), expected);
}

#[test]
fn a_string_with_a_newline_is_never_raw_attribute_text() {
    let source = "jsx(\"div\", { title: \"line1\\n   line2\", children: \"a\\nb\" });";
    let expected = "<div title={\"line1\\n   line2\"}>{\"a\\nb\"}</div>;";
    assert_eq!(run(source), expected);
    assert_eq!(run_default(source), expected);
}

#[test]
fn component_with_key() {
    assert_eq!(run("jsx(\"div\", {}, \"test\");"), "<div key=\"test\" />;");
}

#[test]
fn array_expression_child_of_a_non_plural_pragma_stays_one_child() {
    assert_eq!(run("jsx(\"div\", { children: [1] });"), "<div>{[1]}</div>;");
}

#[test]
fn fragment() {
    assert_eq!(
        run("jsxs(React.Fragment, { children: [jsx(\"span\", {}), \"test\"] });"),
        "<><span />test</>;"
    );
}

#[test]
fn a_fragment_child_prints_inline_by_default_and_in_a_container_under_conservative() {
    let source =
        "jsx(\"div\", { children: jsxs(React.Fragment, { children: [\"x\", jsx(\"b\", {})] }) });";
    assert_eq!(run_default(source), "<div><>x<b /></></div>;");
    assert_eq!(run(source), "<div>{<>x<b /></>}</div>;");
}

#[test]
fn fragment_with_key_is_an_element() {
    assert_eq!(
        run("jsx(React.Fragment, {}, o);"),
        "<React.Fragment key={o} />;"
    );
}

#[test]
fn fragment_with_spread_element() {
    assert_eq!(
        run("jsxs(React.Fragment, { children: [x, ...y] });"),
        "<>{x}{...y}</>;"
    );
}

#[test]
fn the_automatic_fragment_resolves_a_react_binding_by_default_and_stays_literal_under_conservative()
{
    let source =
        "import R from \"react\";\njsxs(R.Fragment, { children: [jsx(\"b\", {}), \"x\"] });";
    assert_eq!(
        run_default(source),
        "import R from \"react\";\n<><b />x</>;"
    );
    assert_eq!(
        run(source),
        "import R from \"react\";\n<R.Fragment><b />x</R.Fragment>;"
    );
}

#[test]
fn a_shadowed_literal_react_fragment_is_an_element_by_default_in_the_automatic_runtime() {
    let source =
        "var React = { Fragment: F };\njsxs(React.Fragment, { children: [jsx(\"b\", {}), \"x\"] });";
    assert_eq!(
        run_default(source),
        "var React = { Fragment: F };\n<React.Fragment><b />x</React.Fragment>;"
    );
    assert_eq!(run(source), "var React = { Fragment: F };\n<><b />x</>;");
}

#[test]
fn leading_comments_are_dropped() {
    assert_eq!(
        run("f(/*#__PURE__*/_jsx(\"h1\", { children: /*#__PURE__*/_jsx(\"div\", {}) }));"),
        "f(<h1><div /></h1>);"
    );
}

#[test]
fn props_with_escaped_strings() {
    let source = "_jsx(Foo, { bar: 'abc' });\n\
                  _jsx(Foo, { bar: 'a\\\\nbc' });\n\
                  _jsx(Foo, { bar: 'ab\\\\tc' });\n\
                  _jsx(Foo, { bar: 'ab\"c' });\n\
                  _jsx(Foo, { bar: \"ab'c\" });";
    assert_eq!(
        run(source),
        "<Foo bar=\"abc\" />;\n<Foo bar={\"a\\\\nbc\"} />;\n<Foo bar={\"ab\\\\tc\"} />;\n<Foo bar={\"ab\\\"c\"} />;\n<Foo bar=\"ab'c\" />;"
    );
}

#[test]
fn indirect_jsx_call() {
    assert_eq!(run("(0, r.jsx)(\"div\", {});"), "<div />;");
}

#[test]
fn object_member_jsxs_and_jsx() {
    assert_eq!(
        run("r.jsxs(Foo, { bar: 1, children: [r.jsx(Child, {})] });"),
        "<Foo bar={1}><Child /></Foo>;"
    );
}

#[test]
fn a_pragma_name_that_is_not_in_the_list_is_left_alone() {
    assert_eq!(run("jsxFoo(\"div\", {});"), "jsxFoo(\"div\", {});");
}

#[test]
fn a_non_object_props_argument_is_left_alone() {
    assert_eq!(run("jsx(\"div\", null);"), "jsx(\"div\", null);");
}

#[test]
fn a_tag_name_jsx_cannot_spell_refuses_the_element() {
    let source = "jsx(\"\", {});";
    assert_eq!(run(source), source);
    let notes = notes_of(source);
    assert!(
        notes.iter().any(|note| note
            .message
            .starts_with(untwist_jsx::notes::ELEMENT_NAME_SHAPE)),
        "{notes:?}"
    );
}

#[test]
fn a_string_tag_that_reads_as_a_component_is_refused_by_default_and_converted_under_conservative() {
    let source = "jsx(\"Foo\", {});";
    assert_eq!(run_default(source), source);
    let notes = transform(source, true).1.notes;
    assert!(
        notes.iter().any(|note| note
            .message
            .starts_with(untwist_jsx::notes::COMPONENT_LOOKING_STRING)),
        "{notes:?}"
    );
    assert_eq!(run(source), "<Foo />;");
}

#[test]
fn a_repeated_refusal_reports_three_spanned_notes_and_one_summary() {
    let call = "jsx(\"div\", {}, 1, 2);";
    let source = [call, call, call, call].join("\n");
    let reported: Vec<Note> = notes_of(&source)
        .into_iter()
        .filter(|note| note.message.starts_with(untwist_jsx::notes::KEY_SHAPE))
        .collect();
    let spanned = reported.iter().filter(|note| note.span.is_some()).count();
    let summary = reported.iter().filter(|note| note.span.is_none()).count();
    assert_eq!((spanned, summary), (SPANNED_NOTE_CAP, 1), "{reported:?}");
}

#[test]
fn two_hoisted_types_get_two_distinct_names() {
    assert_eq!(
        run("jsx(r ? \"a\" : \"div\", {}); jsx(t ? \"b\" : \"span\", {});"),
        "const Component = r ? \"a\" : \"div\";\n<Component />;\nconst Component_2__ = t ? \"b\" : \"span\";\n<Component_2__ />;"
    );
}

#[test]
fn a_hoist_from_a_braceless_branch_lands_before_the_enclosing_list_member() {
    assert_eq!(
        run("if (x) f(jsx(r ? \"a\" : \"div\", {}));"),
        "const Component = r ? \"a\" : \"div\";\nif (x) f(<Component />);"
    );
}

#[test]
fn a_hoist_inside_a_function_stays_inside_that_function() {
    assert_eq!(
        run("function g(){ if (x) f(jsx(r ? \"a\" : \"div\", {})); }"),
        "function g() {\n  const Component = r ? \"a\" : \"div\";\n  if (x) f(<Component />);\n}"
    );
}

#[test]
fn a_hoist_in_a_switch_case_stays_inside_the_case() {
    assert_eq!(
        run("switch (x) { case 1: f(jsx(r ? \"a\" : \"div\", {})); }"),
        "switch (x) {\n  case 1:\n    const Component = r ? \"a\" : \"div\";\n    f(<Component />);\n}"
    );
}

#[test]
fn a_hoist_from_a_class_field_initialiser_lands_before_the_class() {
    assert_eq!(
        run("class A { p = jsx(r ? \"a\" : \"div\", {}); }"),
        "const Component = r ? \"a\" : \"div\";\nclass A {\n  p = <Component />;\n}"
    );
}

#[test]
fn a_hoist_reports_its_note_although_it_only_fires_on_a_changing_iteration() {
    let source = "jsx(r ? \"a\" : \"div\", {});";
    assert_eq!(
        run(source),
        "const Component = r ? \"a\" : \"div\";\n<Component />;"
    );
    let notes = notes_of(source);
    assert!(
        notes
            .iter()
            .any(|note| note.message.starts_with(untwist_jsx::notes::TYPE_HOISTED)),
        "{notes:?}"
    );
}

#[test]
fn a_hoist_whose_free_identifier_is_bound_below_the_anchor_is_refused_by_default() {
    let source = "for (let i = 0; g(jsx(w[i], {})); i++) h();";
    assert_eq!(run_default(source), source);
    let notes = transform(source, true).1.notes;
    assert!(
        notes.iter().any(|note| note
            .message
            .starts_with(untwist_jsx::notes::HOIST_ANCHOR_UNRESOLVED)),
        "{notes:?}"
    );
    assert_eq!(
        run(source),
        "const Component = w[i];\nfor (let i = 0; g(<Component />); i++) h();"
    );
}

#[test]
fn a_hoist_out_of_a_default_parameter_is_refused_by_default() {
    let source = "function f(a, b = jsx(pick(a), {})) {}";
    assert_eq!(run_default(source), source);
    assert_eq!(
        run(source),
        "const Component = pick(a);\nfunction f(a, b = <Component />) {}"
    );
}

#[test]
fn a_hoist_whose_free_identifiers_are_visible_at_the_anchor_still_lands_by_default() {
    assert_eq!(
        run_default("function g(w, i) {\n  if (x) f(jsx(w[i], {}));\n}"),
        "function g(w, i) {\n  const Component = w[i];\n  if (x) f(<Component />);\n}"
    );
}

#[test]
fn a_binding_declared_inside_the_hoisted_expression_does_not_refuse_the_hoist() {
    assert_eq!(
        run_default("jsx(list.map((x) => x), {});"),
        "const Component = list.map((x) => x);\n<Component />;"
    );
}

#[test]
fn every_recorded_site_is_reported_by_a_catalogued_note() {
    let sites = [
        (
            "jsx(\"div\", { get a() { return 1; } });",
            untwist_jsx::notes::ATTRIBUTE_SHAPE,
        ),
        ("jsx(\"\", {});", untwist_jsx::notes::ELEMENT_NAME_SHAPE),
        ("jsx(a, {});", untwist_jsx::notes::LOWERCASE_UNBOUND),
        (
            "jsx(r ? \"a\" : \"div\", {});",
            untwist_jsx::notes::TYPE_HOISTED,
        ),
        ("jsx(\"div\", {}, 1, 2);", untwist_jsx::notes::KEY_SHAPE),
    ];
    for (source, message) in sites {
        let notes = notes_of(source);
        assert!(
            notes.iter().any(|note| note.message.starts_with(message)),
            "{source} did not report {message}: {notes:?}"
        );
    }
}

#[test]
fn a_classic_rename_and_an_automatic_hoist_in_one_program_draw_distinct_names() {
    let source = "function a() {}\nReact.createElement(a, null);\njsx(r ? \"a\" : \"div\", {});";
    let expected = "function Component() {}\n<Component />;\nconst Component_2__ = r ? \"a\" : \"div\";\n<Component_2__ />;";
    assert_eq!(run_both_runtimes(source, false), expected);
    assert_eq!(run_both_runtimes(source, true), expected);
}

#[test]
fn an_automatic_hoist_before_a_classic_rename_still_draws_distinct_names() {
    let source = "jsx(r ? \"a\" : \"div\", {});\nfunction a() {}\nReact.createElement(a, null);";
    let expected = "const Component = r ? \"a\" : \"div\";\n<Component />;\nfunction Component_2__() {}\n<Component_2__ />;";
    assert_eq!(run_both_runtimes(source, false), expected);
    assert_eq!(run_both_runtimes(source, true), expected);
}

#[test]
fn a_lowercase_component_used_by_both_runtimes_is_renamed_once() {
    let source = "function b() {}\nReact.createElement(b, null);\njsx(b, {});";
    let expected = "function Component() {}\n<Component />;\n<Component />;";
    assert_eq!(run_both_runtimes(source, false), expected);
    assert_eq!(run_both_runtimes(source, true), expected);
}

#[test]
fn a_hoist_out_of_a_var_loop_head_lands_in_both_modes() {
    let source = "for (var i = 0; g(jsx(w[i], {})); i++) h();";
    let expected = "const Component = w[i];\nfor (var i = 0; g(<Component />); i++) h();";
    assert_eq!(run_default(source), expected);
    assert_eq!(run(source), expected);
}

#[test]
fn a_hoist_whose_free_identifier_is_an_arrow_parameter_is_refused_by_default() {
    let source = "(x) => jsx(w[x], {});";
    assert_eq!(run_default(source), source);
    let notes = transform(source, true).1.notes;
    assert!(
        notes.iter().any(|note| note
            .message
            .starts_with(untwist_jsx::notes::HOIST_ANCHOR_UNRESOLVED)),
        "{notes:?}"
    );
}

#[test]
fn a_children_prop_that_is_not_last_once_and_without_holes_leaves_the_call() {
    let sources = [
        "jsx(\"div\", { children: f(), children: g() });",
        "jsx(\"div\", { children: a, ...p });",
        "jsx(\"div\", { children: f(), id: g() });",
        "jsxs(\"div\", { children: [x, , y] });",
    ];
    for source in sources {
        let code = run(source);
        assert!(code.starts_with("jsx"), "{source}: {code}");
        assert!(!code.contains('<'), "{source}: {code}");
        let notes = notes_of(source);
        assert!(
            notes
                .iter()
                .any(|note| note.message.starts_with(untwist_jsx::notes::CHILDREN_SHAPE)),
            "{source}: {notes:?}"
        );
    }
}

#[test]
fn a_last_children_prop_after_a_spread_is_converted() {
    assert_eq!(
        run("jsx(\"div\", { ...p, id: g(), children: f() });"),
        "<div {...p} id={g()}>{f()}</div>;"
    );
}
