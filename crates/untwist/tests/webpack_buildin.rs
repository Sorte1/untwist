use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, Statement};
use untwist::{Options, Pipeline};
use untwist_core::parse::parse_program;
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};
use untwist_unpack::webpack::buildin::{buildin_kind, buildin_text};
use untwist_unpack::webpack::detect4::webpack4_factories;

const POLYFILL_MARK: &str = "webpackPolyfill";
const MODULE_OBJECT: &str = "uses its module object";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/webpack-buildin")
}

fn cells(prefix: &str) -> Vec<(String, String, String)> {
    let mut found: Vec<_> = std::fs::read_dir(fixtures())
        .expect("fixtures/webpack-buildin")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .map(|path| {
            let code = std::fs::read_to_string(&path).expect("read");
            let stdout = std::fs::read_to_string(path.with_extension("stdout")).expect("stdout");
            (
                path.file_stem().unwrap().to_string_lossy().to_string(),
                code,
                stdout,
            )
        })
        .collect();
    found.sort();
    found
}

fn lane(unsafe_transforms: bool, unpack: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack,
        jsx: false,
        ..Options::default()
    }
}

fn seen_texts(code: &str) -> Vec<String> {
    let mut texts = vec![code.to_string()];
    for unsafe_transforms in [false, true] {
        texts.push(
            Pipeline::new(lane(unsafe_transforms, false))
                .run(code)
                .code()
                .to_string(),
        );
    }
    texts
}

fn polyfill_forms(text: &str) -> Vec<Option<String>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    let mut forms = Vec::new();
    for statement in &parsed.program.body {
        let Statement::ExpressionStatement(statement) = statement else {
            continue;
        };
        for (_, factory) in webpack4_factories(&statement.expression).unwrap_or_default() {
            let Some(BindingPattern::BindingIdentifier(module)) =
                factory.params.items.first().map(|item| &item.pattern)
            else {
                continue;
            };
            let statements = factory.body.statements.as_slice();
            if !factory.body.span.source_text(text).contains(POLYFILL_MARK) {
                continue;
            }
            let form = buildin_text(statements, &module.name)
                .filter(|_| buildin_kind(statements, &module.name).is_some());
            forms.push(form.or_else(|| {
                buildin_text(statements, &module.name).map(|text| format!("UNRECORDED {text}"))
            }));
        }
    }
    forms
}

#[test]
#[ignore = "prints the polyfill forms the fixtures hold, raw and in each lane"]
fn dump_buildin_forms() {
    let mut seen = std::collections::BTreeSet::new();
    for (name, code, _) in cells("w4-") {
        for text in seen_texts(&code) {
            for form in polyfill_forms(&text) {
                if seen.insert(form.clone()) {
                    println!("{name} {form:?}");
                }
            }
        }
    }
}

#[test]
fn every_fixture_polyfill_is_a_recorded_form() {
    let mut matched = 0;
    for (name, code, _) in cells("w4-") {
        for text in seen_texts(&code) {
            for form in polyfill_forms(&text) {
                let form = form.unwrap_or_default();
                assert!(
                    !form.is_empty() && !form.starts_with("UNRECORDED"),
                    "{name}: {form}"
                );
                matched += 1;
            }
        }
    }
    assert!(matched >= 36, "{matched}");
}

fn written(code: &str, options: Options) -> (tempfile::TempDir, Vec<String>) {
    let out = Pipeline::new(options).run(code);
    let dir = tempfile::tempdir().expect("temp dir");
    out.write_to(dir.path()).expect("write");
    let notes = out.notes.into_iter().map(|note| note.message).collect();
    (dir, notes)
}

fn printed(path: &std::path::Path) -> Option<String> {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return None;
    };
    let observed = run(&node, path).expect("spawn").expect("terminates");
    Some(format!("{}{}", observed.stdout, observed.stderr))
}

#[test]
fn a_webpack_4_bundle_whose_modules_go_through_the_module_polyfill_is_unpacked_and_runs_alike() {
    for (name, code, stdout) in cells("w4-index-") {
        for unsafe_transforms in [false, true] {
            let (dir, notes) = written(&code, lane(unsafe_transforms, true));
            assert!(
                dir.path().join("bundle.json").exists(),
                "{name} unsafe={unsafe_transforms}: {notes:?}"
            );
            let tree = std::fs::read_dir(dir.path())
                .expect("tree")
                .filter_map(Result::ok)
                .map(|entry| std::fs::read_to_string(entry.path()).unwrap_or_default())
                .collect::<String>();
            assert!(
                !tree.contains(".js\")(module)"),
                "{name}: a polyfill call is left"
            );
            let entry = std::fs::read_dir(dir.path())
                .expect("tree")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| path.file_name().is_some_and(|file| file == "index.js"))
                .expect("index.js");
            if let Some(out) = printed(&entry) {
                assert_eq!(out, stdout, "{name} unsafe={unsafe_transforms}");
            }
        }
    }
}

#[test]
fn a_module_that_reads_more_of_the_polyfill_than_node_gives_keeps_the_bundle_whole() {
    for (name, code, stdout) in cells("w4-negative-") {
        for unsafe_transforms in [false, true] {
            let (dir, notes) = written(&code, lane(unsafe_transforms, true));
            assert!(!dir.path().join("bundle.json").exists(), "{name}");
            assert!(
                notes.iter().any(|note| note.contains(MODULE_OBJECT)),
                "{name}: {notes:?}"
            );
            if let Some(out) = printed(&dir.path().join("deobfuscated.js")) {
                assert_eq!(out, stdout, "{name} unsafe={unsafe_transforms}");
            }
        }
    }
}
