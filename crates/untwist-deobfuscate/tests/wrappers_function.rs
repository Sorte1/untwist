use untwist_core::pass::PassOptions;

const HEAD: &str = "function __STRING_ARRAY__() {\n  var h = ['log'];\n  __STRING_ARRAY__ = function () { return h; };\n  return __STRING_ARRAY__();\n}\nfunction __DECODE_0__(a, b) {\n  var e = __STRING_ARRAY__();\n  return e[a];\n}\n";

const W: &str = "function alias(a, b) { return __DECODE_0__(b - 938, a); }\n";

fn transform_source(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    let code = untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::wrappers::boxed]);
    let check = untwist_testing::Allocator::default();
    let reparsed = untwist_core::parse::parse_program(&check, &code);
    assert!(
        reparsed.diagnostics.is_empty(),
        "output does not re-parse: {:?}\n{code}",
        reparsed.diagnostics
    );
    code
}

fn run(body: &str) -> String {
    transform_source(&format!("{HEAD}{body}"))
}

fn printed(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(&alloc, source, &[])
}

fn notes(source: &str) -> Vec<String> {
    let alloc = untwist_testing::Allocator::default();
    let (_, report) = untwist_testing::transform_report(
        &alloc,
        source,
        &[untwist_deobfuscate::wrappers::boxed],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    report.notes.into_iter().map(|note| note.message).collect()
}

#[test]
fn a_single_wrapper_is_inlined_with_permuted_arguments() {
    let out = run("function alias(a, b) { return __DECODE_0__(b - 938, a); }\nalias(1071, 1077);");
    assert!(out.contains("__DECODE_0__(1077 - 938, 1071);"), "{out}");
    assert!(!out.contains("function alias"), "{out}");
}

#[test]
fn nested_wrappers_flatten_all_the_way_down() {
    let out = run(
        "function alias(a, b) { return __DECODE_0__(a - 625, b); }\nfunction alias2(a, b) { return alias(b - -678, a); }\nalias2(4, 5);",
    );
    assert!(out.contains("__DECODE_0__(5 - -678 - 625, 4);"), "{out}");
    assert!(!out.contains("function alias"), "{out}");
}

#[test]
fn a_function_whose_params_are_unused_in_the_call_is_not_a_wrapper() {
    let out = run(
        "function notAWrapper(c, d) { return __DECODE_0__(0x175, 0x172); }\nconsole.log(notAWrapper(foo(), bar()));",
    );
    assert!(out.contains("function notAWrapper(c, d)"), "{out}");
    assert!(out.contains("__DECODE_0__(373, 370)"), "{out}");
}

#[test]
fn a_wrapper_with_no_call_sites_is_still_removed() {
    let out = run("function k(c, d) { return __DECODE_0__(d - -0x33, c); }\nconsole.log(1);");
    assert!(!out.contains("function k"), "{out}");
}

#[test]
fn a_single_parameter_helper_is_not_a_wrapper() {
    let out = run("function one(a) { return __DECODE_0__(a, 1); }\none(2);");
    assert!(out.contains("function one(a)"), "{out}");
}

#[test]
fn a_wrapper_inside_a_function_expression_is_flattened() {
    let code = run(&format!("(function () {{\n{W}alias(4, 5);\n}})();"));
    assert!(code.contains("__DECODE_0__(5 - 938, 4)"), "{code}");
}

#[test]
fn a_wrapper_called_in_an_arrow_body_is_flattened() {
    let code = run(&format!("{W}const z = () => alias(4, 5);"));
    assert!(code.contains("__DECODE_0__(5 - 938, 4)"), "{code}");
}

#[test]
fn a_wrapper_called_optionally_is_flattened() {
    let code = run(&format!("{W}alias?.(4, 5);"));
    assert!(code.contains("__DECODE_0__(5 - 938, 4)"), "{code}");
}

#[test]
fn a_wrapper_called_in_a_template_is_flattened() {
    let code = run(&format!("{W}f(`x${{alias(4, 5)}}y`);"));
    assert!(code.contains("__DECODE_0__(5 - 938, 4)"), "{code}");
}

#[test]
fn a_wrapper_called_in_an_await_operand_is_flattened() {
    let code = run(&format!(
        "{W}async function g() {{ return await alias(4, 5); }}"
    ));
    assert!(code.contains("__DECODE_0__(5 - 938, 4)"), "{code}");
}

#[test]
fn issue_230_shape_4_the_rotated_index_wrapper_is_flattened() {
    let code = run(
        "var rotated = function (idx, unused) { return __DECODE_0__(idx - -1, unused); };\ndocument[rotated(-5)] = \"\";",
    );
    assert!(code.contains("__DECODE_0__(-5 - -1"), "{code}");
    assert!(!code.contains("rotated"), "{code}");
}

#[test]
fn a_wrapper_that_is_also_referenced_without_a_call_is_kept() {
    let source = format!("{HEAD}{W}var x = alias;\nalias(4, 5);");
    let code = transform_source(&source);
    assert!(code.contains("__DECODE_0__(5 - 938, 4)"), "{code}");
    assert!(code.contains("function alias(a, b)"), "{code}");
    assert!(code.contains("var x = alias;"), "{code}");
    let messages = notes(&source);
    assert!(
        messages
            .iter()
            .any(|note| note.contains("wrapper kept, 0 of 1 references rewritten")),
        "{messages:?}"
    );
}

#[test]
fn a_wrapper_reached_through_a_call_member_is_left_untouched() {
    let source = format!("{HEAD}{W}alias.call(null, 4, 5);");
    let code = transform_source(&source);
    assert!(code.contains("function alias(a, b)"), "{code}");
    assert!(code.contains("alias.call(null, 4, 5)"), "{code}");
    let messages = notes(&source);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].contains("wrapper kept, 0 of 1 references rewritten"),
        "{messages:?}"
    );
}

#[test]
fn a_wrapper_body_that_returns_a_nested_function_is_left_untouched() {
    let source = format!(
        "{HEAD}function w(a, b) {{ return __DECODE_0__(a, function (a) {{ return a; }}); }}\nw(1, 2);"
    );
    let code = transform_source(&source);
    assert!(code.contains("function w(a, b)"), "{code}");
    assert!(code.contains("w(1, 2)"), "{code}");
    let messages = notes(&source);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].contains("a function, arrow or class"),
        "{messages:?}"
    );
}

#[test]
fn a_call_site_passing_a_function_argument_is_left_alone() {
    let source = format!("{HEAD}{W}alias(function () {{ return 1; }}, 5);");
    assert_eq!(transform_source(&source), printed(&source));
    let messages = notes(&source);
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages
            .iter()
            .any(|note| note.contains("an argument holds a function, arrow or class")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|note| note.contains("wrapper kept, 0 of 1 references rewritten")),
        "{messages:?}"
    );
}

#[test]
fn a_wrapper_body_that_returns_a_class_expression_is_left_untouched() {
    let source =
        format!("{HEAD}function w(a, b) {{ return __DECODE_0__(a, class {{}}); }}\nw(1, 2);");
    assert_eq!(transform_source(&source), printed(&source));
    let messages = notes(&source);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].contains("a function, arrow or class"),
        "{messages:?}"
    );
}

#[test]
fn a_call_site_spreading_its_arguments_is_left_alone() {
    let source = format!("{HEAD}{W}alias(...xs);");
    assert_eq!(transform_source(&source), printed(&source));
    let messages = notes(&source);
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages
            .iter()
            .any(|note| note.contains("a wrapper call site was left alone")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|note| note.contains("wrapper kept, 0 of 1 references rewritten")),
        "{messages:?}"
    );
}

#[test]
fn a_wrapper_call_nested_in_another_wrapper_call_leaves_nothing_dangling() {
    let source = format!(
        "{HEAD}function alias(a, b) {{ return __DECODE_0__(a - 1, b); }}\nf(alias(alias(2, 3), 4));"
    );
    let code = transform_source(&source);
    assert!(
        code.contains("__DECODE_0__(__DECODE_0__(2 - 1, 3) - 1, 4)"),
        "{code}"
    );
    assert!(!code.contains("alias"), "{code}");
    let messages = notes(&source);
    assert!(
        !messages
            .iter()
            .any(|note| note.contains("a call site was deferred to the next iteration")),
        "the deferral was resolved before the run settled, so the report must not carry it \
        : {messages:?}"
    );
}

#[test]
fn a_second_run_over_the_output_changes_nothing() {
    let first = run(
        "function alias(a, b) { return __DECODE_0__(a - 625, b); }\nfunction alias2(a, b) { return alias(b - -678, a); }\nalias2(4, 5);\nconst z = () => alias2(6, 7);",
    );
    let second = transform_source(&first);
    assert_eq!(first, second, "second run changed the output");
}

fn transform_once(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    let mut parsed = untwist_core::parse::parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = untwist_core::semantic::build_scoping(&parsed.program);
    let mut group = untwist_core::pass::PassGroup::from_ctors(
        &[untwist_deobfuscate::wrappers::boxed],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    let _ = untwist_core::pass::run_once(&alloc, &mut parsed.program, sem, &mut group);
    untwist_core::print::print(&parsed.program, untwist_core::print::PrintStyle::Pretty)
}

#[test]
fn a_deferral_inside_a_nested_wrapper_argument_is_resolved_by_the_next_round() {
    let source = format!(
        "{HEAD}function alias(a, b) {{ return __DECODE_0__(a - 1, b); }}\nfunction alias2(a, b) {{ return __DECODE_0__(a, b - 2); }}\nf(alias2(alias(2, 3), 4));"
    );
    let once = transform_once(&source);
    assert!(
        once.contains("__DECODE_0__(__DECODE_0__(2 - 1, 3), 4 - 2)"),
        "{once}"
    );
    assert!(!once.contains("alias("), "{once}");
    let code = transform_source(&source);
    assert!(!code.contains("alias"), "{code}");
    let messages = notes(&source);
    assert!(
        !messages
            .iter()
            .any(|note| note.contains("a call site was deferred to the next iteration")),
        "the deferral was resolved before the run settled, so the report must not carry it \
        : {messages:?}"
    );
}

#[test]
fn a_wrapper_body_that_reads_a_free_variable_is_left_in_place() {
    let source = format!(
        "{HEAD}var s = 0x0;\nfunction w(x, y) {{ return __DECODE_0__(x + s, y); }}\nfunction caller() {{ var s = 0x1; return w(0x0, 0x0); }}\nconsole[caller()]('t');"
    );
    let out = transform_source(&source);
    assert!(out.contains("function w(x, y)"), "{out}");
    assert!(out.contains("w(0, 0)"), "{out}");
    assert!(
        notes(&source)
            .iter()
            .any(|note| note.contains("other than its own parameters")),
        "{:?}",
        notes(&source)
    );
}

#[test]
fn a_wrapper_body_that_reads_this_is_left_in_place() {
    let source = format!(
        "{HEAD}function w(x, y) {{ return __DECODE_0__(this.n + x, y); }}\nvar o = {{ n: 0x1, go: function () {{ return w(0x0, 0x0); }} }};\nconsole[o.go()]('t');"
    );
    let out = transform_source(&source);
    assert!(out.contains("function w(x, y)"), "{out}");
    assert!(out.contains("this.n"), "{out}");
}

#[test]
fn a_call_site_with_a_side_effecting_argument_is_left_alone() {
    let source = format!(
        "{HEAD}function w(x, y) {{ return __DECODE_0__(x + y, x); }}\nvar i = 0x0;\nconsole[w(i++, 0x0)]('t');"
    );
    let out = transform_source(&source);
    assert!(out.contains("function w(x, y)"), "{out}");
    assert!(out.contains("w(i++, 0)"), "{out}");
    assert!(
        notes(&source)
            .iter()
            .any(|note| note.contains("side-effect-free")),
        "{:?}",
        notes(&source)
    );
}

#[test]
fn a_call_site_with_a_call_argument_is_left_alone() {
    let source = format!("{HEAD}{W}var i = 0x0;\nconsole[alias(f(), 0x1)]('t');");
    let out = transform_source(&source);
    assert!(out.contains("function alias(a, b)"), "{out}");
    assert!(out.contains("alias(f(), 1)"), "{out}");
}
