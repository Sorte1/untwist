use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack_inputs, UnpackOptions};

const VARIANTS: &str = "../../fixtures/webpack-variants";

fn cell(name: &str) -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(VARIANTS)
        .join(name);
    let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("{dir:?}: {err}"))
        .map(|entry| {
            let path = entry.expect("entry").path();
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .to_string();
            (name, std::fs::read_to_string(&path).expect("read"))
        })
        .collect();
    files.sort();
    files
}

struct Outcome {
    modules: Vec<(String, String)>,
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    pretty_codegen(&parse_program(&alloc, source).program)
}

fn unpacked(files: &[(String, String)]) -> Result<Outcome, Vec<String>> {
    let alloc = Allocator::default();
    let texts: Vec<String> = files.iter().map(|(_, source)| printed(source)).collect();
    let mut programs: Vec<Program<'_>> = texts
        .iter()
        .map(|source| parse_program(&alloc, source).program)
        .collect();
    let (bundle, notes) = unpack_inputs(&alloc, &mut programs, &[], UnpackOptions::default());
    let Some(bundle) = bundle else {
        return Err(notes.into_iter().map(|note| note.message).collect());
    };
    let modules = bundle
        .modules
        .values()
        .map(|module| (module.id.clone(), pretty_codegen(module.program())))
        .collect();
    Ok(Outcome { modules })
}

fn edited(
    mut files: Vec<(String, String)>,
    file: &str,
    from: &str,
    to: &str,
) -> Vec<(String, String)> {
    let (_, text) = files
        .iter_mut()
        .find(|(name, _)| name == file)
        .unwrap_or_else(|| panic!("no {file}"));
    assert!(text.contains(from), "{file} has no {from:?}");
    *text = text.replacen(from, to, 1);
    files
}

fn without(files: Vec<(String, String)>, file: &str) -> Vec<(String, String)> {
    files.into_iter().filter(|(name, _)| name != file).collect()
}

fn module<'o>(outcome: &'o Outcome, id: &str) -> Option<&'o str> {
    outcome
        .modules
        .iter()
        .find(|(module, _)| module == id)
        .map(|(_, text)| text.as_str())
}

fn assert_refused(files: &[(String, String)], why: &str) {
    let Err(notes) = unpacked(files) else {
        panic!("unpacked, expected a refusal: {why}");
    };
    assert!(
        notes.iter().any(|note| note.contains(why)),
        "no note contains {why:?}: {notes:?}"
    );
}

#[test]
fn a_node_entry_and_its_chunks_unpack_into_one_tree() {
    let outcome = unpacked(&cell("w5.112-node")).unwrap_or_else(|notes| panic!("{notes:?}"));
    for id in ["entry", "339", "542", "899", "_runtime/webpack"] {
        assert!(module(&outcome, id).is_some(), "no module {id}");
    }
    let runtime = module(&outcome, "_runtime/webpack").expect("runtime");
    assert!(runtime.contains("Promise.all([])"), "{runtime}");
    assert!(!runtime.contains("installedChunks"), "{runtime}");
}

#[test]
fn a_chunk_that_is_not_among_the_inputs_refuses_the_bundle() {
    assert_refused(
        &without(cell("w5.112-node"), "899.chunk.js"),
        "loads chunk 899, which is not among the inputs",
    );
}

#[test]
fn a_startup_chunk_that_is_not_among_the_inputs_refuses_the_bundle() {
    assert_refused(
        &without(cell("w5.112-web-split"), "814.js"),
        "814, which is not among the inputs",
    );
}

#[test]
fn a_loader_with_an_extra_statement_is_not_webpacks() {
    let files = edited(
        cell("w5.112-node"),
        "index.js",
        "// object to store loaded chunks",
        "console.log(\"loader ran\");",
    );
    assert!(unpacked(&files).is_err());
}

#[test]
fn a_jsonp_callback_that_does_more_than_install_is_not_webpacks() {
    let files = edited(
        cell("w5.112-web"),
        "index.js",
        "let [chunkIds, moreModules, runtime] = data;",
        "let [chunkIds, moreModules, runtime] = data; console.log(\"arrived\");",
    );
    assert!(unpacked(&files).is_err());
}

#[test]
fn an_ensure_helper_with_a_side_effect_is_not_replaced() {
    let files = edited(
        cell("w5.112-node"),
        "index.js",
        "const promises = [];",
        "const promises = []; console.log(chunkId);",
    );
    if let Ok(outcome) = unpacked(&files) {
        assert!(module(&outcome, "entry").is_none());
        let runtime = module(&outcome, "_runtime/webpack").unwrap_or_default();
        assert!(!runtime.contains("Promise.all([])"), "{runtime}");
    }
}

#[test]
fn a_deferred_startup_waits_on_webpacks_own_chunk_loaded_helper() {
    let files = edited(
        cell("w5.112-web-split"),
        "index.js",
        "deferred.push([chunkIds, fn]);",
        "deferred.push([chunkIds, fn]); console.log(\"deferred\");",
    );
    assert!(unpacked(&files).is_err());
}

#[test]
fn a_chunk_alone_that_uses_a_helper_has_no_runtime() {
    let files: Vec<(String, String)> = cell("w5.112-node")
        .into_iter()
        .filter(|(name, _)| name == "339.chunk.js")
        .collect();
    assert_refused(&files, "has no runtime in this input");
}

#[test]
fn chunks_are_not_written_alone_when_their_runtime_was_refused() {
    let files = edited(
        cell("w5.112-web-split"),
        "index.js",
        "// object to store loaded and loading chunks",
        "console.log(\"loader ran\");",
    );
    assert_refused(&files, "its chunks are not written on their own");
}

#[test]
fn a_webpack4_split_bundle_starts_its_deferred_entry() {
    let outcome = unpacked(&cell("w4-web-split")).unwrap_or_else(|notes| panic!("{notes:?}"));
    assert!(module(&outcome, "3").is_some());
}

#[test]
fn a_runtime_chunk_starts_the_entry_its_chunk_names() {
    let outcome =
        unpacked(&cell("w5.112-web-runtime-single")).unwrap_or_else(|notes| panic!("{notes:?}"));
    assert!(module(&outcome, "237").is_some());
}
