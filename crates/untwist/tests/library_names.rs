use std::path::{Path, PathBuf};

use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_testing::node::assert_node_agrees;

const DIR: &str = "fixtures/libraries";
const COVERAGE: &str = "fixtures/libraries/COVERAGE";
const WRITE_COVERAGE: &str = "UNTWIST_WRITE_LIBRARY_COVERAGE";

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels below the repository")
}

fn fixtures() -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let root = repo().join(DIR);
    let mut libraries: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the library fixtures exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    libraries.sort();
    for library in libraries {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&library)
            .expect("reading a library folder")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
            .collect();
        files.sort();
        for file in files {
            let name = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .display()
                .to_string();
            found.push((name, file));
        }
    }
    found
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(repo().join(DIR).join(name)).expect("reading a fixture")
}

fn run(source: &str, unsafe_transforms: bool, rename: RenameMode, unpack: bool) -> Output {
    let options = Options {
        unsafe_transforms,
        rename,
        unpack,
        ..Options::default()
    };
    Pipeline::new(options).run(source)
}

fn library_notes(output: &Output) -> Vec<String> {
    output
        .notes
        .iter()
        .filter(|note| note.pass == "library-names")
        .map(|note| note.message.clone())
        .collect()
}

fn modules_named(output: &Output) -> usize {
    library_notes(output)
        .iter()
        .filter_map(|note| {
            note.rsplit_once(", ")?
                .1
                .split(' ')
                .next()?
                .parse::<usize>()
                .ok()
        })
        .sum()
}

fn coverage_row(name: &str, source: &str) -> String {
    let default_lane = run(source, false, RenameMode::Smart, false);
    let unsafe_lane = run(source, true, RenameMode::Smart, false);
    let unpacked = run(source, true, RenameMode::Smart, true);
    assert_node_agrees(source, default_lane.code());
    assert_node_agrees(source, unsafe_lane.code());
    format!(
        "{name}\t{}\t{}\t{}",
        modules_named(&default_lane),
        modules_named(&unsafe_lane),
        modules_named(&unpacked)
    )
}

#[test]
fn every_library_fixture_keeps_its_recorded_coverage_and_runs_like_node() {
    let rows: Vec<String> = fixtures()
        .iter()
        .map(|(name, path)| {
            let source = std::fs::read_to_string(path).expect("reading a fixture");
            coverage_row(name, &source)
        })
        .collect();
    let text = format!(
        "# fixture\tdefault lane\tunsafe lane\tunsafe lane, unpacked (modules recognised; cargo test --test library_names with {WRITE_COVERAGE}=1 rewrites this file)\n{}\n",
        rows.join("\n")
    );
    for row in &rows {
        let counts: Vec<usize> = row
            .split('\t')
            .skip(1)
            .filter_map(|count| count.parse().ok())
            .collect();
        assert!(
            counts.iter().any(|&count| count > 0),
            "nothing recognised: {row}"
        );
    }
    let path = repo().join(COVERAGE);
    if std::env::var_os(WRITE_COVERAGE).is_some() {
        std::fs::write(&path, &text).expect("writing the coverage file");
        return;
    }
    let recorded = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        recorded, text,
        "library coverage changed; set {WRITE_COVERAGE}=1 to record it"
    );
}

fn assert_contains(code: &str, expected: &[&str]) {
    for text in expected {
        assert!(code.contains(text), "missing {text:?} in\n{code}");
    }
}

#[test]
fn the_rollup_wrappers_of_get_built_in_own_keys_and_well_known_symbol_get_their_names() {
    let output = run(
        &fixture("core-js/rollup-cjs.terser.obf.js"),
        true,
        RenameMode::Smart,
        false,
    );
    assert_contains(
        output.code(),
        &[
            ": internals/get-built-in.js",
            "function requireGetBuiltIn()",
            "if (hasRequiredGetBuiltIn)",
            "getBuiltIn = function (namespace, method)",
            ": internals/own-keys.js",
            "function requireOwnKeys()",
            "getBuiltIn(\"Reflect\", \"ownKeys\")",
            ": internals/well-known-symbol.js",
            "function requireWellKnownSymbol()",
            "shared(\"wks\")",
            "createWellKnownSymbol(\"Symbol.\" + name)",
        ],
    );
}

#[test]
fn the_default_lane_names_core_js_in_a_mangled_webpack_bundle() {
    let output = run(
        &fixture("core-js/webpack5.prod.js"),
        false,
        RenameMode::Smart,
        false,
    );
    assert_contains(output.code(), &["// core-js 3.", "isCallable"]);
}

#[test]
fn library_names_take_priority_in_every_rename_mode() {
    let source = fixture("core-js/rollup-cjs.terser.js");
    for mode in [
        RenameMode::Smart,
        RenameMode::Simple,
        RenameMode::Unique,
        RenameMode::Mangle,
    ] {
        let output = run(&source, true, mode, false);
        assert_contains(
            output.code(),
            &[
                "function requireGetBuiltIn()",
                "if (hasRequiredGetBuiltIn)",
                "getBuiltIn(\"Reflect\", \"ownKeys\")",
            ],
        );
        assert_node_agrees(&source, output.code());
    }
}

#[test]
fn the_default_lane_keeps_function_names_the_program_may_read() {
    let output = run(
        &fixture("core-js/rollup-cjs.terser.js"),
        false,
        RenameMode::Smart,
        false,
    );
    assert!(output.code().contains("if (hasRequiredGetBuiltIn)"));
    assert!(!output.code().contains("function requireGetBuiltIn()"));
}

#[test]
fn rename_none_keeps_the_names_but_still_comments() {
    let output = run(
        &fixture("core-js/rollup-cjs.terser.js"),
        false,
        RenameMode::None,
        false,
    );
    assert!(!output.code().contains("function requireGetBuiltIn()"));
    assert!(output.code().contains(": internals/get-built-in.js"));
    let libraries = output.libraries();
    assert_eq!(libraries.len(), 1, "{libraries:?}");
    assert_eq!(libraries[0].library, "core-js");
    assert_eq!(libraries[0].range, "3.39");
}

#[test]
fn library_comments_are_stable_when_untwist_runs_on_its_own_output() {
    let source = fixture("core-js/rollup-cjs.terser.js");
    let once = run(&source, false, RenameMode::None, false);
    let twice = run(once.code(), false, RenameMode::None, false);
    assert_eq!(once.code(), twice.code());
}

#[test]
fn the_library_pass_can_be_disabled() {
    let options = Options {
        disabled_passes: vec!["library-names"],
        ..Options::default()
    };
    let output = Pipeline::new(options).run(&fixture("core-js/rollup-cjs.terser.js"));
    assert!(library_notes(&output).is_empty());
    assert!(!output.code().contains("// core-js"));
}

#[test]
fn a_program_without_library_code_gets_no_library_note() {
    let output = run(
        "function add(a, b) { return a + b; }\nconsole.log(add(1, 2));\n",
        false,
        RenameMode::Smart,
        false,
    );
    assert!(
        library_notes(&output).is_empty(),
        "{:?}",
        library_notes(&output)
    );
    assert!(!output.code().contains("// core-js"));
}

#[test]
fn unique_gives_one_library_name_to_one_binding_across_inputs() {
    let source = ["__extends", "__rest", "__awaiter", "__generator"]
        .iter()
        .enumerate()
        .fold(fixture("tslib/rollup-cjs.js"), |code, (index, name)| {
            code.replace(name, &format!("q{index}"))
        });
    let options = Options {
        rename: RenameMode::Unique,
        unsafe_transforms: true,
        unpack: false,
        ..Options::default()
    };
    let output = Pipeline::new(options).run_many(&[source.as_str(), source.as_str()]);
    let declared = |code: &str| -> Vec<String> {
        code.lines()
            .filter_map(|line| line.trim_start().strip_prefix("function __"))
            .filter_map(|rest| rest.split(['(', ' ']).next())
            .filter(|name| !name.is_empty())
            .map(|name| format!("__{name}"))
            .collect()
    };
    let first = declared(&output.codes[0]);
    let second = declared(&output.codes[1]);
    assert!(!first.is_empty(), "{}", output.codes[0]);
    assert!(
        first.iter().all(|name| !second.contains(name)),
        "{first:?} {second:?}"
    );
}
