use untwist_testing::assert_transform;

#[test]
fn a_constant_readonly_array_is_inlined_element_by_element() {
    assert_transform!(
        "const arr = ['log', 'Hello, World!'];\nconsole[arr[0]](arr[1]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"console["log"]("Hello, World!");"#
    );
}

#[test]
fn a_mutated_array_is_left_alone() {
    assert_transform!(
        "const arr2 = ['log', 'Hello, World!'];\narr2[0] = 'warn';\nconsole[arr2[0]](arr2[1]);",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    const arr2 = ["log", "Hello, World!"];
    arr2[0] = "warn";
    console[arr2[0]](arr2[1]);
    "#
    );
}

#[test]
fn an_unreferenced_array_is_left_alone() {
    assert_transform!(
        "const arr4 = ['log', 'Hello, World!'];",
        [untwist_deobfuscate::string_array::boxed],
        @r#"const arr4 = ["log", "Hello, World!"];"#
    );
}

#[test]
fn a_shared_reference_blocks_inlining() {
    assert_transform!(
        "const arr = ['log'];\nfn(arr);\nconsole[arr[0]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    const arr = ["log"];
    fn(arr);
    console[arr[0]]();
    "#
    );
}

#[test]
fn an_out_of_range_index_blocks_inlining() {
    assert_transform!(
        "const arr = ['log'];\nconsole[arr[5]]();",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    const arr = ["log"];
    console[arr[5]]();
    "#
    );
}

#[test]
fn a_read_inside_a_hoisted_function_that_can_run_before_the_declaration_keeps_the_array() {
    assert_transform!(
        "function g() { return arr[0]; }\nconsole.log(g());\nvar arr = ['EARLY'];",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    function g() {
      return arr[0];
    }
    console.log(g());
    var arr = ["EARLY"];
    "#
    );
}

#[test]
fn a_read_inside_a_hoisted_function_declared_after_an_inert_prefix_is_inlined() {
    assert_transform!(
        "var x = 1;\nvar arr = ['LATE'];\nfunction g() { return arr[0]; }\nconsole.log(g());",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    var x = 1;
    function g() {
      return "LATE";
    }
    console.log(g());
    "#
    );
}

#[test]
fn a_call_before_the_declaration_refuses_every_hoisted_read() {
    assert_transform!(
        "g();\nvar arr = ['EARLY'];\nfunction g() { return arr[0]; }",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    g();
    var arr = ["EARLY"];
    function g() {
      return arr[0];
    }
    "#
    );
}

#[test]
fn a_read_in_a_statement_before_the_declaration_keeps_the_array() {
    assert_transform!(
        "console.log(arr[0]);\nvar arr = ['EARLY'];",
        [untwist_deobfuscate::string_array::boxed],
        @r#"
    console.log(arr[0]);
    var arr = ["EARLY"];
    "#
    );
}

mod common;

const FIVE_UNEXPECTED_ACCESSES: &str = "var s1 = ['a'];\ns1.push('b');\nconsole.log(s1[0]);\nvar s2 = ['a'];\ns2.indexOf('b');\nconsole.log(s2[0]);\nvar s3 = ['a'];\ns3.push('b');\nconsole.log(s3[0]);\nvar s4 = ['a'];\nconsole.log(s4[9]);\nvar s5 = ['a'];\ns5.indexOf('b');\nconsole.log(s5[0]);";

#[test]
fn more_than_three_unexpected_accesses_collapse_into_one_summary() {
    let (_code, report) = common::run_report_extended(FIVE_UNEXPECTED_ACCESSES);
    let named: Vec<&untwist_core::pass::Note> = report
        .notes
        .iter()
        .filter(|note| {
            note.message
                .contains(untwist_deobfuscate::notes::SIMPLE_ARRAY_ACCESS)
        })
        .collect();
    let spanned: Vec<&String> = named
        .iter()
        .filter(|note| note.span.is_some())
        .map(|note| &note.message)
        .collect();
    assert_eq!(
        spanned.len(),
        untwist_core::pass::SPANNED_NOTE_CAP,
        "{named:?}"
    );
    let summaries: Vec<&String> = named
        .iter()
        .filter(|note| note.span.is_none())
        .map(|note| &note.message)
        .collect();
    assert_eq!(
        summaries,
        vec![
            &"simple string array not inlined: unexpected access at 5 sites \
               (indexOf \u{d7}2, push \u{d7}2, 9 \u{d7}1)"
                .to_string()
        ],
    );
}

fn access_notes(source: &str) -> Vec<String> {
    let (_code, report) = common::run_report_extended(source);
    report
        .notes
        .iter()
        .filter(|note| {
            note.message
                .contains(untwist_deobfuscate::notes::SIMPLE_ARRAY_ACCESS)
        })
        .map(|note| note.message.clone())
        .collect()
}

#[test]
fn an_empty_array_that_is_pushed_to_is_not_a_simple_string_array() {
    let source = "var queue = [];\nfunction add(x) { queue.push(x); }\nadd('q');\nconsole.log(queue[0], queue.length);";
    assert_eq!(access_notes(source), Vec::<String>::new());
}

#[test]
fn a_string_array_that_is_pushed_to_still_reports_the_push() {
    let source = "var words = ['x', 'y'];\nwords.push('z');\nconsole.log(words[0], words[2]);";
    let notes = access_notes(source);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].ends_with("unexpected access push"), "{notes:?}");
}

#[test]
fn a_string_array_used_only_through_methods_is_not_a_simple_string_array() {
    let source = "var allowed = ['png', 'jpg'];\nconsole.log(allowed.includes('png'), allowed.indexOf('jpg'));";
    assert_eq!(access_notes(source), Vec::<String>::new());
}
