use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::{self, listed_unsafe_witnesses, node_or_skip, observed, Mode};

const ROWS: &str = include_str!("data/anonymous_name_rows.txt");

const UNSAFE_WITNESSES: [&str; 71] = [
    "var-function",
    "var-class",
    "let-arrow",
    "const-async-arrow",
    "assign-function",
    "for-init",
    "generator",
    "destructured-name",
    "name-read-in-eval-text",
    "name-read-in-function-text",
    "paren-assign-function",
    "paren-assign-class",
    "paren-assign-arrow",
    "paren-assign-comment",
    "paren-nested",
    "paren-in-value",
    "fold-conditional-assign",
    "fold-logical-assign",
    "fold-declarations",
    "fold-paren-target",
    "fold-into-property",
    "fold-defaults-and-fields",
    "fold-alternate-and-parens",
    "fold-void-and-null",
    "fold-string-comparison",
    "fold-chained-and",
    "fold-nested",
    "default-parameter-from-arguments",
    "default-parameter-class-arrow",
    "merge-object-member-function",
    "fold-in-eval-layer",
    "temp-nullish-strict",
    "temp-nullish-loose",
    "temp-nullish-arrow",
    "temp-optional-strict",
    "temp-optional-loose",
    "temp-logical-object",
    "cff-key-fold",
    "merge-object-computed-key",
    "code-text-a0",
    "code-text-a1",
    "code-text-a2",
    "code-text-a3",
    "code-text-a4",
    "code-text-a5",
    "code-text-a6",
    "code-text-a7",
    "code-text-a8",
    "code-text-b1",
    "code-text-b2",
    "code-text-b4",
    "code-text-b5",
    "code-text-b6",
    "code-text-b7",
    "decoder-name-and-text",
    "decoder-assigned-in-sequence",
    "decoder-arrow-in-lexical-sequence",
    "decoder-reads-its-own-name",
    "code-text-rr2-c2",
    "code-text-rr2-c5",
    "code-text-rr2-c6",
    "code-text-rr2-c7",
    "code-text-rr2-c8",
    "code-text-rr2-c9",
    "code-text-rr2-d1",
    "code-text-rr2-d2",
    "code-text-rr2-d3",
    "code-text-rr2-d7",
    "code-text-rr2-d8",
    "code-text-rr2-d9",
    "with-name-read",
];

const TRIVIA_ROWS: [(&str, &str); 8] = [
    (
        "paren-comment-cr",
        "var f; (f) //x\r= function () {};\rconsole.log(JSON.stringify(f.name));\r",
    ),
    (
        "paren-comment-line-separator",
        "var f; (f) //x\u{2028}= function () {};\nconsole.log(JSON.stringify(f.name));",
    ),
    (
        "paren-comment-paragraph-separator",
        "var f; (f) //x\u{2029}= function () {};\nconsole.log(JSON.stringify(f.name));",
    ),
    (
        "paren-html-open-comment",
        "var f; (f) <!-- x\n= function () {};\nconsole.log(JSON.stringify(f.name));",
    ),
    (
        "paren-html-close-comment",
        "var f; (f)\n--> x\n= function () {};\nconsole.log(JSON.stringify(f.name));",
    ),
    (
        "paren-html-close-after-block-comment",
        "var f; (f) /*\u{2028}*/ --> x\n= class {};\nconsole.log(JSON.stringify(f.name));",
    ),
    (
        "paren-html-close-after-cr",
        "var f; (f) //a\r /* b */ --> c\r= () => 1;\rconsole.log(JSON.stringify(f.name));",
    ),
    (
        "paren-then-plain-assignment",
        "var f; (f) //x\r; f = function () {};\rconsole.log(JSON.stringify(f.name));",
    ),
];

const MODULE_ROWS: [(&str, &str); 2] = [
    (
        "fold-export-default-function",
        "import f from \"./program.mjs\";\nexport default true ? function () {} : 0;\nconsole.log(JSON.stringify(f.name));\n",
    ),
    (
        "fold-export-default-class",
        "import C from \"./program.mjs\";\nexport default 0 || class {};\nconsole.log(JSON.stringify(C.name));\n",
    ),
];

fn module_printed(node: &std::path::Path, dir: &std::path::Path, text: &str) -> String {
    let path = dir.join("program.mjs");
    std::fs::write(&path, text).unwrap();
    run(node, &path)
        .unwrap()
        .map_or("did not finish".to_string(), |out| observed(&out))
}

fn renamed(rename: RenameMode, source: &str) -> String {
    let options = Options {
        rename,
        ..Options::default()
    };
    transformed(&options, source)
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

#[test]
fn an_anonymous_function_behind_a_comma_keeps_its_name_where_node_reads_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 12, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let smart = |source: &str| renamed(RenameMode::Smart, source);
    let mangle = |source: &str| renamed(RenameMode::Mangle, source);
    let simple = |source: &str| renamed(RenameMode::Simple, source);
    let modes: [Mode<'_>; 5] = [
        ("default", &run_safe),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
        ("unsafe", &run_unsafe),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (unexplained, witnesses) = listed_unsafe_witnesses(wrong, &UNSAFE_WITNESSES);
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert_eq!(witnesses, UNSAFE_WITNESSES);
}

#[test]
fn a_program_that_reads_no_name_has_its_sequence_split_and_keeps_the_name_unless_unsafe() {
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    for (label, source) in rows()
        .iter()
        .filter(|(label, _)| label.starts_with("no-name-read"))
    {
        let code = transformed(&safe, source);
        assert!(!code.contains("(console.log"), "{label}: {code}");
        assert!(code.contains("(0, "), "{label}: {code}");
        let code = transformed(&unsafe_transforms, source);
        assert!(!code.contains("(console.log"), "{label}: {code}");
        assert!(!code.contains("(0, "), "{label}: {code}");
    }
}

#[test]
fn a_decoder_split_from_its_rotator_stays_anonymous_and_decodes() {
    let (_, source) = rows()
        .into_iter()
        .find(|(label, _)| label == "decoder-in-sequence")
        .unwrap();
    for rename in [
        RenameMode::None,
        RenameMode::Smart,
        RenameMode::Mangle,
        RenameMode::Simple,
    ] {
        let code = renamed(rename, &source);
        assert_eq!(
            code.trim(),
            "console.log(\"beta\", \"alpha\");",
            "{rename:?}"
        );
    }
}

#[test]
fn a_parenthesised_target_behind_any_line_terminator_or_html_comment_stays_unnamed() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = TRIVIA_ROWS
        .iter()
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let smart = |source: &str| renamed(RenameMode::Smart, source);
    let mangle = |source: &str| renamed(RenameMode::Mangle, source);
    let simple = |source: &str| renamed(RenameMode::Simple, source);
    let modes: [Mode<'_>; 5] = [
        ("default", &run_safe),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
        ("unsafe", &run_unsafe),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let labels: Vec<&str> = TRIVIA_ROWS.iter().map(|(label, _)| *label).collect();
    let (unexplained, witnesses) = listed_unsafe_witnesses(wrong, &labels);
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert!(witnesses.len() <= labels.len(), "{witnesses:?}");
}

#[test]
fn a_folded_export_default_keeps_the_name_node_gives_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let lanes = [
        ("default", RenameMode::None),
        ("rename smart", RenameMode::Smart),
        ("rename mangle", RenameMode::Mangle),
        ("rename simple", RenameMode::Simple),
    ];
    let mut wrong = Vec::new();
    for (label, source) in MODULE_ROWS {
        let before = module_printed(&node, dir.path(), source);
        for (lane, rename) in lanes {
            let after = module_printed(&node, dir.path(), &renamed(rename, source));
            if after != before {
                wrong.push(format!(
                    "{label} ({lane}): node {before:?}, output {after:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const CODE_TEXT_DEFAULT_ROWS: &str = include_str!("data/code_text_default_rows.txt");

#[test]
fn a_name_read_by_a_code_text_keeps_the_function_unnamed_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = CODE_TEXT_DEFAULT_ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    assert!(programs.len() >= 2, "found {} rows", programs.len());
    let safe = Options::default();
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const NAME_READ_KEY_ROWS: &str = include_str!("data/name_read_key_rows.txt");

const NAME_READ_KEY_WITNESSES: [&str; 32] = [
    "key-concat",
    "key-variable",
    "key-call",
    "key-template",
    "key-optional",
    "key-decoded",
    "key-in-another-scope",
    "reflect-get-literal",
    "reflect-get-variable",
    "reflect-get-alias",
    "reflect-get-global",
    "reflect-apply-get",
    "reflect-descriptor",
    "descriptor-alias",
    "descriptors-all",
    "destructure-computed",
    "destructure-get",
    "names-loop",
    "own-keys-loop",
    "entries-enumerable",
    "assign-enumerable",
    "spread-enumerable",
    "for-in-enumerable",
    "values-destructured",
    "descriptor-built-method",
    "counter-reassigned",
    "counter-for-in",
    "counter-closure",
    "counter-destructured",
    "counter-redeclared",
    "counter-catch",
    "counter-plus-string",
];

fn name_read_key_rows() -> Vec<(String, String)> {
    NAME_READ_KEY_ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

#[test]
fn a_name_read_through_a_computed_key_or_reflection_keeps_names_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = name_read_key_rows();
    assert!(programs.len() >= 20, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let smart = |source: &str| renamed(RenameMode::Smart, source);
    let mangle = |source: &str| renamed(RenameMode::Mangle, source);
    let simple = |source: &str| renamed(RenameMode::Simple, source);
    let modes: [Mode<'_>; 5] = [
        ("default", &run_safe),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
        ("unsafe", &run_unsafe),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (unexplained, witnesses) = listed_unsafe_witnesses(wrong, &NAME_READ_KEY_WITNESSES);
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert_eq!(witnesses, NAME_READ_KEY_WITNESSES);
}

#[test]
fn literal_keys_other_than_name_drop_the_wrapper_only_under_unsafe() {
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    for (label, source) in name_read_key_rows()
        .iter()
        .filter(|(label, _)| label.starts_with("no-name-read"))
    {
        let code = transformed(&safe, source);
        assert!(code.contains("(0, "), "{label}: {code}");
        let code = transformed(&unsafe_transforms, source);
        assert!(!code.contains("(0, "), "{label}: {code}");
    }
}

const NX4_ROWS: &str = include_str!("data/name_read_nx4_rows.txt");

fn nx4_rows() -> Vec<(String, String)> {
    NX4_ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

const NX4_WITNESSES: [&str; 6] = [
    "nx4-d-05proto",
    "nx4-d-06protoget",
    "nx4-d-07protokey",
    "nx4-d-08arrproto",
    "nx4-d-09strproto",
    "nx4-d-10numproto",
];

#[test]
fn a_name_read_through_globals_code_text_or_a_prototype_keeps_names_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = nx4_rows();
    assert!(programs.len() >= 20, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let smart = |source: &str| renamed(RenameMode::Smart, source);
    let mangle = |source: &str| renamed(RenameMode::Mangle, source);
    let simple = |source: &str| renamed(RenameMode::Simple, source);
    let modes: [Mode<'_>; 5] = [
        ("default", &run_safe),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
        ("unsafe", &run_unsafe),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (unexplained, witnesses) = listed_unsafe_witnesses(wrong, &NX4_WITNESSES);
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert_eq!(witnesses, NX4_WITNESSES);
}

#[test]
fn a_local_loop_counter_key_still_lets_rename_rename_functions() {
    let (_, source) = nx4_rows()
        .into_iter()
        .find(|(label, _)| label == "nx4-counter-local")
        .unwrap();
    let code = renamed(RenameMode::Mangle, &source);
    assert!(!code.contains("foo"), "{code}");
    assert!(!code.contains("bar"), "{code}");
}

#[test]
fn a_top_level_counter_of_a_script_keeps_function_names() {
    let (_, source) = nx4_rows()
        .into_iter()
        .find(|(label, _)| label == "nx4-counter-top-level")
        .unwrap();
    let code = renamed(RenameMode::Mangle, &source);
    assert!(code.contains("function foo"), "{code}");
    assert!(code.contains("bar"), "{code}");
}

const RR4_ROWS: &str = include_str!("data/name_guard_rr4_rows.txt");

const RR4_RENAME_PREMISE: [&str; 2] = ["rr4-10", "rr4-r2"];

fn rename_premise_rows(wrong: Vec<String>, labels: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut unexplained = Vec::new();
    let mut premise = Vec::new();
    for row in wrong {
        let listed = labels
            .iter()
            .find(|label| row.starts_with(&format!("{label} (rename ")));
        match listed {
            Some(label) => premise.push(label.to_string()),
            None => unexplained.push(row),
        }
    }
    premise.sort();
    premise.dedup();
    (unexplained, premise)
}

#[test]
fn the_default_lane_keeps_every_name_a_built_in_or_host_may_read() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = RR4_ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    assert!(programs.len() >= 40, "found {} rows", programs.len());
    let safe = Options::default();
    let run_safe = |source: &str| transformed(&safe, source);
    let smart = |source: &str| renamed(RenameMode::Smart, source);
    let mangle = |source: &str| renamed(RenameMode::Mangle, source);
    let simple = |source: &str| renamed(RenameMode::Simple, source);
    let unique = |source: &str| renamed(RenameMode::Unique, source);
    let modes: [Mode<'_>; 5] = [
        ("default", &run_safe),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
        ("rename unique", &unique),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (unexplained, premise) = rename_premise_rows(wrong, &RR4_RENAME_PREMISE);
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
    assert_eq!(premise, RR4_RENAME_PREMISE);
}
