use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::{self, listed_unsafe_witnesses, node_or_skip, observed, Mode};

const ROWS: &str = include_str!("data/anonymous_name_rows.txt");

const UNSAFE_WITNESSES: [&str; 70] = [
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
fn a_program_that_reads_no_name_still_has_its_sequence_split() {
    let safe = Options::default();
    for (label, source) in rows()
        .iter()
        .filter(|(label, _)| label.starts_with("no-name-read"))
    {
        let code = transformed(&safe, source);
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
