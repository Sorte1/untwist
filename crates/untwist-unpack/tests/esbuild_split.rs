use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::{unpack_named_inputs, UnpackOptions};

const CHUNK: &str = r#"var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
export {
  __commonJS
};
"#;

const ENTRY: &str = r#"import {
  __commonJS
} from "./chunk-ABC.js";
var require_a = __commonJS({
  "src/a.js"(exports) {
    exports.value = 1;
  }
});
var require_b = __commonJS({
  "src/b.js"(exports) {
    exports.value = require_a().value + 1;
  }
});
console.log(require_b().value);
"#;

fn modules(sources: &[&str], names: &[&str]) -> Option<Vec<String>> {
    let alloc = Allocator::default();
    let mut programs: Vec<_> = sources
        .iter()
        .map(|source| parse_program(&alloc, source).program)
        .collect();
    let names: Vec<String> = names.iter().map(|name| (*name).to_string()).collect();
    let (bundle, _) =
        unpack_named_inputs(&alloc, &mut programs, &[], UnpackOptions::default(), &names);
    let bundle = bundle?;
    Some(
        bundle
            .modules
            .values()
            .map(|module| module.path.as_str().to_string())
            .collect(),
    )
}

#[test]
fn an_entry_whose_helpers_come_from_a_split_chunk_is_unpacked() {
    let found = modules(&[ENTRY, CHUNK], &["index.js", "chunk-ABC.js"]).expect("unpacked");
    assert!(found.iter().any(|path| path == "src/a.js"), "{found:?}");
    assert!(found.iter().any(|path| path == "src/b.js"), "{found:?}");
}

#[test]
fn a_chunk_under_another_name_is_not_trusted() {
    assert_eq!(
        modules(&[ENTRY, CHUNK], &["index.js", "chunk-XYZ.js"]),
        None
    );
}

#[test]
fn a_chunk_whose_helper_is_not_the_commonjs_shape_is_not_trusted() {
    let other = CHUNK.replace("return mod ||", "return mod &&");
    assert_eq!(
        modules(&[ENTRY, other.as_str()], &["index.js", "chunk-ABC.js"]),
        None
    );
}

#[test]
fn a_chunk_that_reassigns_its_helper_is_not_trusted() {
    let other = format!("{CHUNK}__commonJS = (cb) => cb;\n");
    assert_eq!(
        modules(&[ENTRY, other.as_str()], &["index.js", "chunk-ABC.js"]),
        None
    );
}

#[test]
fn without_names_nothing_is_linked() {
    assert_eq!(modules(&[ENTRY, CHUNK], &[]), None);
}

fn cell(app: &str, name: &str) -> (Vec<String>, Vec<String>) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bundle-matrix")
        .join(app)
        .join(name);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|file| file.ends_with(".js"))
        .collect();
    files.sort();
    let texts = files
        .iter()
        .map(|file| std::fs::read_to_string(dir.join(file)).unwrap())
        .collect();
    (texts, files)
}

#[test]
fn esbuild_splitting_output_is_unpacked_from_its_entry() {
    for (app, name) in [
        ("mini", "esbuild0.28-esm-split"),
        ("base", "esbuild0.14-esm-split"),
    ] {
        let (texts, files) = cell(app, name);
        let sources: Vec<&str> = texts.iter().map(String::as_str).collect();
        let names: Vec<&str> = files.iter().map(String::as_str).collect();
        assert!(modules(&sources, &names).is_some(), "{app}/{name}");
    }
}
