use untwist::{NoteKind, Options, Output, Pipeline, WasmOrigin, WasmSupply};

fn fixture_path(name: &str) -> String {
    format!("{}/tests/wasm/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).unwrap()
}

fn wasm_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

fn run(source: &str) -> Output {
    Pipeline::new(Options::default()).run(source)
}

fn settled(source: &str) -> Output {
    let mut output = run(source);
    output.settle_wasm(&WasmSupply::default(), true);
    output
}

fn wasm_notes(output: &Output) -> Vec<&str> {
    output
        .notes
        .iter()
        .filter(|note| note.pass == "wasm")
        .map(|note| note.message.as_str())
        .collect()
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (index, byte)| {
            word | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            match index <= chunk.len() {
                true => out.push(ALPHABET[(word >> (18 - 6 * index) & 63) as usize] as char),
                false => out.push('='),
            }
        }
    }
    out
}

#[test]
fn wasm_bindgen_glue_gets_its_payload_extracted_and_its_imports_and_exports_named() {
    let out = settled(&fixture("bindgen.js"));
    assert_eq!(out.wasm.len(), 1);
    let file = &out.wasm[0];
    assert_eq!(file.path, "wasm/deobfuscated.wasm");
    assert_eq!(file.origin, WasmOrigin::Embedded);
    assert_eq!(file.bytes, wasm_bytes("bindgen.wasm"));
    assert_eq!(file.renamed, 3);
    assert!(wasm_notes(&out).is_empty());
    insta::assert_snapshot!(out.code());
}

#[test]
fn emscripten_single_file_names_the_glue_from_the_wasm_name_section() {
    let out = settled(&fixture("emscripten.js"));
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].bytes, wasm_bytes("emscripten.wasm"));
    assert_eq!(out.wasm[0].renamed, 5);
    insta::assert_snapshot!(out.code());
}

#[test]
fn webpack_inline_wasm_is_extracted_per_module_and_the_hashed_async_module_is_listed() {
    let out = settled(&fixture("webpack.js"));
    let bundle = out.bundle.as_ref().expect("the webpack bundle unpacks");
    let module = |path: &str| {
        bundle
            .modules
            .iter()
            .find(|module| module.path == path)
            .map(|module| module.code.as_str())
            .unwrap()
    };
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].path, "wasm/index.wasm");
    assert_eq!(out.wasm[0].bytes, wasm_bytes("webpack.wasm"));
    assert!(module("src/inline.js").contains("\"../wasm/index.wasm\""));
    assert!(module("src/index.js").contains("function log(e)"));
    assert_eq!(
        out.wasm_unresolved
            .iter()
            .map(|reference| reference.name.as_str())
            .collect::<Vec<_>>(),
        ["3f2a91c0d1e2b3a4.module.wasm"]
    );
    insta::assert_snapshot!(module("src/index.js"));
}

#[test]
fn a_malformed_payload_is_refused_and_its_literal_kept() {
    let mut bytes = b"\0asm\x01\0\0\0".to_vec();
    bytes.extend([0x02, 0x7f, 0xff, 0xff]);
    let payload = base64(&bytes);
    let out = settled(&format!(
        "const b = \"{payload}\";\nWebAssembly.instantiate(b);"
    ));
    assert!(out.wasm.is_empty());
    assert!(out.code().contains(&payload));
    let notes = wasm_notes(&out);
    assert_eq!(notes.len(), 1);
    assert!(
        notes[0].contains("is not a well-formed wasm module"),
        "{notes:?}"
    );
    assert!(out
        .notes
        .iter()
        .any(|note| note.pass == "wasm" && note.kind == NoteKind::Refusal));
}

#[test]
fn a_payload_with_the_wrong_version_is_refused() {
    let payload = base64(b"\0asm\x02\0\0\0");
    let out = settled(&format!("WebAssembly.instantiate(\"{payload}\");"));
    assert!(out.wasm.is_empty());
    assert!(wasm_notes(&out)[0].contains("version"));
}

#[test]
fn an_oversize_payload_is_refused_without_being_decoded() {
    let mut out = run("");
    let payload = format!(
        "AGFzbQEAAAA{}",
        "A".repeat(untwist_unpack::wasm::MAX_WASM_BYTES / 3 * 4 + 4)
    );
    out.codes = vec![format!("WebAssembly.instantiate(\"{payload}\");")];
    out.settle_wasm(&WasmSupply::default(), true);
    assert!(out.wasm.is_empty());
    assert!(out.codes[0].contains(&payload));
    assert!(wasm_notes(&out)[0].contains("size limit"));
}

#[test]
fn a_module_that_does_not_cover_its_glue_gives_no_names() {
    let glue = fixture("bindgen.js");
    let start = glue.find("AGFzbQ").unwrap();
    let end = start + glue[start..].find('"').unwrap();
    let swapped = format!(
        "{}{}{}",
        &glue[..start],
        base64(&wasm_bytes("webpack.wasm")),
        &glue[end..]
    );
    let out = settled(&swapped);
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].renamed, 0);
    assert!(out.code().contains("function Q(e, t)"));
    let notes = wasm_notes(&out);
    assert!(notes[0].contains("does not match the glue"), "{notes:?}");
}

#[test]
fn remote_wasm_is_reported_only_where_it_is_loaded_and_never_fetched() {
    let source = r#"
        fetch("pkg/app_bg.wasm?v=3");
        __wbg_init(new URL("other_bg.wasm", import.meta.url));
        Module.locateFile("a.out.wasm");
        var wasmBinaryFile = "game.wasm";
        require("./src/add.wasm");
        var ids = { "./src/add.wasm": 1 };
        console.log("not.wasm");
    "#;
    let out = settled(source);
    let names: Vec<(&str, &str)> = out
        .wasm_unresolved
        .iter()
        .map(|reference| (reference.name.as_str(), reference.url.as_str()))
        .collect();
    assert_eq!(
        names,
        [
            ("app_bg.wasm", "pkg/app_bg.wasm?v=3"),
            ("other_bg.wasm", "other_bg.wasm"),
            ("a.out.wasm", "a.out.wasm"),
            ("game.wasm", "game.wasm"),
        ]
    );
    assert_eq!(out.wasm_references().len(), 4);
}

#[test]
fn without_extraction_the_glue_is_named_and_the_literal_kept() {
    let mut out = run(&fixture("bindgen.js"));
    out.settle_wasm(&WasmSupply::default(), false);
    assert!(out.code().contains("AGFzbQ"));
    assert!(out.code().contains("function __wbg_log_9a1b2c(e, t)"));
    assert_eq!(out.wasm.len(), 1);
}

#[test]
fn text_without_wasm_is_left_byte_for_byte() {
    let mut out = run("var a = [0, 1, 2];\nfetch('x.js');");
    let before = out.code().to_string();
    out.settle_wasm(&WasmSupply::default(), true);
    assert_eq!(out.code(), before);
    assert!(out.wasm.is_empty() && out.wasm_unresolved.is_empty());
}

#[test]
fn extracted_wasm_is_written_beside_the_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = settled(&fixture("bindgen.js"));
    let report = out.write_to(dir.path()).unwrap();
    assert!(report.files.contains(&"wasm/deobfuscated.wasm".to_string()));
    assert_eq!(
        std::fs::read(dir.path().join("wasm/deobfuscated.wasm")).unwrap(),
        wasm_bytes("bindgen.wasm")
    );
}

#[test]
fn a_supplied_name_matches_exactly_or_with_its_hash_segments_dropped() {
    let file = |name: &str| untwist::SuppliedWasm {
        name: name.to_string(),
        bytes: Vec::new(),
        origin: WasmOrigin::Supplied,
        url: None,
    };
    let supply = WasmSupply {
        files: vec![
            file("3f2a91.module.wasm"),
            file("app.wasm"),
            file("x.1a2b3c.wasm"),
            file("x.wasm"),
        ],
    };
    let found = |name: &str| supply.find(name).map(|file| file.name.as_str());
    assert_eq!(found("3f2a91.module.wasm"), Some("3f2a91.module.wasm"));
    assert_eq!(found("app.9e8d7c6b.wasm"), Some("app.wasm"));
    assert_eq!(
        found("x.ffffff.wasm"),
        None,
        "two files fit, so neither is taken"
    );
    assert_eq!(found("other.wasm"), None);
    assert_eq!(found("aaaaaa.module.wasm"), None);
}

fn supplied(name: &str, wasm: &str) -> WasmSupply {
    WasmSupply {
        files: vec![untwist::SuppliedWasm {
            name: name.to_string(),
            bytes: wasm_bytes(wasm),
            origin: WasmOrigin::Supplied,
            url: None,
        }],
    }
}

fn module_code<'o>(out: &'o Output, path: &str) -> &'o str {
    out.bundle
        .as_ref()
        .expect("the bundle unpacks")
        .modules
        .iter()
        .find(|module| module.path == path)
        .map(|module| module.code.as_str())
        .unwrap()
}

fn settled_with(source: &str, supply: &WasmSupply) -> Output {
    let mut out = run(source);
    out.settle_wasm(supply, true);
    out
}

#[test]
fn webpack_async_wasm_names_the_glue_in_the_modules_it_imports_from_and_exports_to() {
    let supply = supplied("3f2a91c0d1e2b3a4.module.wasm", "webpack.wasm");
    let out = settled_with(&fixture("webpack-async.js"), &supply);
    assert!(out.wasm_unresolved.is_empty());
    assert_eq!(out.wasm[0].renamed, 2);
    let env = module_code(&out, "src/env.js");
    assert!(env.contains("function log(e)"), "{env}");
    assert!(env.contains("Z: () => log"), "{env}");
    assert!(module_code(&out, "src/index.js").contains("const add = o.add;"));
}

#[test]
fn webpack_sync_wasm_names_the_glue_through_the_runtime_import_objects() {
    let supply = supplied("9c8b7a6f5e4d3c2b.module.wasm", "webpack.wasm");
    let out = settled_with(&fixture("webpack-sync.js"), &supply);
    assert!(out.wasm_unresolved.is_empty());
    assert_eq!(out.wasm[0].renamed, 2);
    assert!(module_code(&out, "src/env.js").contains("function log(e)"));
    assert!(module_code(&out, "src/index.js").contains("const add = r.add;"));
}

#[test]
fn a_bundle_module_that_does_not_cover_the_cross_module_glue_names_nothing() {
    let supply = supplied("3f2a91c0d1e2b3a4.module.wasm", "bindgen.wasm");
    let out = settled_with(&fixture("webpack-async.js"), &supply);
    assert_eq!(out.wasm[0].renamed, 0);
    assert!(module_code(&out, "src/env.js").contains("function o(e)"));
    assert!(module_code(&out, "src/index.js").contains("const a = o.add;"));
    assert!(wasm_notes(&out)
        .iter()
        .any(|note| note.contains("does not match the glue around src/add.wasm.js")));
}
