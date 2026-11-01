use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_testing::{generated_bundle_present, js_files_in, real_world_fixture};
use untwist_unpack::{unpack, unpack_taking, UnpackResult};

const BUNDLE_SETS: [&str; 3] = [
    "fixtures/reference/unpack",
    "fixtures/unpack-gaps",
    "fixtures/reference/deobfuscate",
];

fn described(result: Option<UnpackResult<'_>>) -> Option<Vec<String>> {
    let result = result?;
    let bundle = &result.bundle;
    let mut lines = vec![format!(
        "{} {:?} {} {:?}",
        bundle.kind.as_str(),
        bundle.entry_id,
        bundle.entry_candidates,
        bundle.chunk_ids
    )];
    lines.extend(bundle.modules.values().map(|module| {
        format!(
            "{} {} {} {:?} {:?}\n{}",
            module.id,
            module.path.as_str(),
            module.is_entry,
            module.dependencies,
            module.unsafe_path_source,
            pretty_codegen(module.program())
        )
    }));
    lines.extend(
        result
            .notes
            .iter()
            .map(|note| format!("{} {:?} {}", note.pass, note.kind, note.message)),
    );
    Some(lines)
}

fn assert_taken_as_cloned(label: &str, source: &str) -> bool {
    let cloned_arena = Allocator::default();
    let cloned = parse_program(&cloned_arena, source);
    let expected = described(unpack(&cloned_arena, &cloned.program, &[]));
    let taken_arena = Allocator::default();
    let mut taken = parse_program(&taken_arena, source);
    let found = described(unpack_taking(&taken_arena, &mut taken.program, &[]));
    assert_eq!(found, expected, "{label}");
    found.is_some()
}

#[test]
fn the_bundle_fixtures_extract_alike_taken_and_cloned() {
    let mut bundles = 0;
    for dir in BUNDLE_SETS {
        for (label, source) in js_files_in(dir) {
            bundles += usize::from(assert_taken_as_cloned(&label, &source));
        }
    }
    assert!(bundles > 10, "only {bundles} bundles");
}

#[test]
fn the_corpora_extract_alike_taken_and_cloned() {
    for dir in [
        "fixtures/corpus-npm",
        "fixtures/behaviour",
        "fixtures/unminify",
    ] {
        for (label, source) in js_files_in(dir) {
            assert_taken_as_cloned(&label, &source);
        }
    }
    let Some(game) = real_world_fixture("game.js") else {
        return;
    };
    assert!(!assert_taken_as_cloned("game.js", &game));
}

#[test]
fn the_generated_bundle_extracts_alike_taken_and_cloned() {
    if !generated_bundle_present() {
        return;
    }
    for (label, source) in js_files_in("fixtures/generated") {
        assert!(assert_taken_as_cloned(&label, &source), "{label}");
    }
}

fn unpack_fixture(name: &str) -> String {
    js_files_in("fixtures/reference/unpack")
        .into_iter()
        .find(|(label, _)| label.ends_with(name))
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn a_bundle_inside_a_failed_browserify_call_extracts_alike_taken_and_cloned() {
    let nested = unpack_fixture("browserify-webpack-nested.js");
    let inner_start = nested.find("!(function (e)").expect("the inner bundle");
    let inner_end = nested
        .find("]);\n      },\n      {},")
        .expect("the end of the inner bundle")
        + 3;
    let source = format!(
        "{}{}{}",
        &nested[..inner_start],
        unpack_fixture("webpack-object.js"),
        &nested[inner_end..]
    )
    .replace("{ './lib': 1, vscode: undefined }", "5");
    assert!(assert_taken_as_cloned(
        "webpack-object.js in entry 1 of a browserify call whose entry 2 has no object deps",
        &source
    ));
}
