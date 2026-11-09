#[path = "common/wasm_module.rs"]
mod wasm_module;

use untwist::{NoteKind, Options, Output, Pipeline, WasmCheck, WasmOrigin, WasmSupply};
use wasm_module::base64;

fn fixture_path(name: &str) -> String {
    format!("{}/tests/wasm/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).unwrap()
}

fn wasm_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

fn run_checked(source: &str, wasm_check: WasmCheck) -> Output {
    Pipeline::new(Options {
        wasm_check,
        ..Options::default()
    })
    .run(source)
}

fn run(source: &str) -> Output {
    run_checked(source, WasmCheck::Auto)
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
    assert_eq!(out.wasm[0].renamed, 6);
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
    let payload = base64(b"\0asm\x02\0\0\0\x01\0");
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
    assert!(out.code().contains("function log(e)"), "{}", out.code());
    assert!(out.code().contains("Z: () => log"));
    assert!(out.code().contains("const add = o.add;"));
}

#[test]
fn webpack_sync_wasm_names_the_glue_through_the_runtime_import_objects() {
    let supply = supplied("9c8b7a6f5e4d3c2b.module.wasm", "webpack.wasm");
    let out = settled_with(&fixture("webpack-sync.js"), &supply);
    assert!(out.wasm_unresolved.is_empty());
    assert_eq!(out.wasm[0].renamed, 2);
    assert!(module_code(&out, "src/env.js").contains("function log(e)"));
    assert!(module_code(&out, "src/index.js").contains("const add = r.add;"));
    assert!(out.code().contains("function log(e)"), "{}", out.code());
    assert!(out.code().contains("const add = r.add;"));
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
}

#[test]
fn the_combined_output_keeps_its_names_when_its_copy_of_a_module_differs() {
    let supply = supplied("9c8b7a6f5e4d3c2b.module.wasm", "webpack.wasm");
    let mut out = run(&fixture("webpack-sync.js"));
    let combined = out.codes[0]
        .replace("function o(e)", "function q(e)")
        .replace("() => o", "() => q");
    out.codes[0] = combined.clone();
    out.settle_wasm(&supply, true);
    assert!(module_code(&out, "src/env.js").contains("function log(e)"));
    assert_eq!(out.code(), combined);
    assert!(wasm_notes(&out)
        .iter()
        .any(|note| note.contains("were not applied to deobfuscated.js: o is not declared once")));
}

#[test]
fn a_module_rename_in_the_combined_output_stays_inside_its_module_function() {
    let supply = supplied("9c8b7a6f5e4d3c2b.module.wasm", "webpack.wasm");
    let source = fixture("webpack-sync.js").replace(
        "      function o(e) {",
        "      const c = 5;\n      console.log(c);\n      function o(e) {",
    );
    let out = settled_with(&source, &supply);
    let code = out.code();
    assert!(code.contains("const add = r.add;"), "{code}");
    assert!(code.contains("console.log(add(1, 2)"), "{code}");
    assert!(
        code.contains("const c = 5;\n      console.log(c);"),
        "{code}"
    );
    assert!(code.contains("function log(e)"), "{code}");
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

#[test]
fn a_header_only_feature_detect_is_not_a_payload() {
    let source = "const a = WebAssembly.validate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));\nconst b = WebAssembly.validate(Uint8Array.from(atob(\"AGFzbQEAAAA=\")));\nconsole.log(a, b);";
    assert!(!untwist_unpack::wasm::may_hold_wasm(source));
    let mut out = run(source);
    let before = out.code().to_string();
    out.settle_wasm(&WasmSupply::default(), true);
    assert_eq!(out.code(), before);
    assert!(out.wasm.is_empty() && out.wasm_unresolved.is_empty());
    assert!(wasm_notes(&out).is_empty());
}

#[test]
fn a_feature_detect_beside_a_real_payload_stays_in_place() {
    let glue = fixture("bindgen.js");
    let source =
        format!("WebAssembly.validate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));\n{glue}");
    let out = settled(&source);
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].bytes, wasm_bytes("bindgen.wasm"));
    assert!(out.code().contains("0, 97, 115, 109, 1, 0, 0, 0"));
}

#[test]
fn with_the_check_off_an_embedded_module_is_only_flagged() {
    let mut out = run_checked(&fixture("bindgen.js"), WasmCheck::Off);
    let before = out.code().to_string();
    out.settle_wasm(&WasmSupply::default(), true);
    assert_eq!(out.code(), before);
    assert!(out.wasm.is_empty());
    assert!(out.wasm_unchecked);
    assert!(wasm_notes(&out).is_empty());
}

#[test]
fn with_the_check_off_text_without_a_module_is_not_flagged() {
    let mut out = run_checked(
        "WebAssembly.validate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));",
        WasmCheck::Off,
    );
    out.settle_wasm(&WasmSupply::default(), true);
    assert!(!out.wasm_unchecked);
}

#[test]
fn with_the_check_off_a_supplied_module_still_names_its_glue() {
    let supply = WasmSupply {
        files: vec![untwist::SuppliedWasm {
            name: "app_bg.wasm".to_string(),
            bytes: wasm_bytes("bindgen.wasm"),
            origin: WasmOrigin::Supplied,
            url: None,
        }],
    };
    let mut out = run_checked(&fixture("bindgen-remote.js"), WasmCheck::Off);
    out.settle_wasm(&supply, true);
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].origin, WasmOrigin::Supplied);
    assert!(out.code().contains("function __wbg_log_9a1b2c(e, t)"));
}

#[test]
fn with_the_check_on_a_module_the_text_trigger_misses_is_extracted() {
    let bytes = wasm_bytes("webpack.wasm");
    let hex: Vec<String> = bytes.iter().map(|byte| format!("0x{byte:x}")).collect();
    let code = format!(
        "WebAssembly.instantiate(new Uint8Array([{}]));",
        hex.join(", ")
    );
    assert!(!untwist_unpack::wasm::may_hold_wasm(&code));
    for (check, found) in [(WasmCheck::Auto, 0), (WasmCheck::On, 1)] {
        let mut out = run_checked("", check);
        out.codes = vec![code.clone()];
        out.settle_wasm(&WasmSupply::default(), false);
        assert_eq!(out.wasm.len(), found, "{check:?}");
    }
}

#[test]
fn auto_leaves_a_module_with_no_imports_or_exports_alone_and_on_extracts_it() {
    let probe =
        "WebAssembly.validate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0]));";
    let source = format!("{probe}\n{}", fixture("bindgen.js"));
    let auto = settled(&source);
    assert_eq!(auto.wasm.len(), 1);
    assert_eq!(auto.wasm[0].bytes, wasm_bytes("bindgen.wasm"));
    assert!(auto.code().contains("0, 97, 115, 109, 1, 0, 0, 0, 1, 4"));
    let mut on = run_checked(probe, WasmCheck::On);
    on.settle_wasm(&WasmSupply::default(), true);
    assert_eq!(on.wasm.len(), 1);
    assert_eq!(on.wasm[0].byte_length, 14);
}

#[test]
fn emscripten_glue_without_a_name_section_is_named_from_its_export_wrappers_and_kinds() {
    let out = settled(&fixture("emscripten-nameless.js"));
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].bytes, wasm_bytes("emscripten-nameless.wasm"));
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm[0].renamed, 5);
    insta::assert_snapshot!(out.code());
}

#[test]
fn an_export_wrapper_whose_key_reads_a_missing_export_names_nothing() {
    let source = fixture("emscripten-nameless.js").replace("Z.e)(e)", "Z.q)(e)");
    let out = settled(&source);
    assert_eq!(out.wasm[0].renamed, 0);
    assert!(out.code().contains("Yb(4)"));
    assert!(wasm_notes(&out)
        .iter()
        .any(|note| note.contains("export q")));
}

#[test]
fn an_export_wrapper_key_on_a_memory_export_is_not_a_function_name() {
    let source = fixture("emscripten-nameless.js").replace("Z.e)(e)", "Z.c)(e)");
    let out = settled(&source);
    let code = out.code();
    assert!(!code.contains("_malloc(4)"), "{code}");
    assert!(!code.contains("wasmMemory"), "{code}");
}

#[test]
fn minified_emscripten_two_glue_from_a_published_package_is_named_without_a_name_section() {
    let out = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| settled(&fixture("real/argon2-bundled.min.js")))
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(out.wasm.len(), 1);
    assert!(out.wasm[0]
        .summary
        .exports
        .iter()
        .all(|export| export.function_name.is_none()));
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    let code = out.code();
    for named in [
        "Q.asm = wasmExports;",
        "U((wasmMemory = Q.asm.c).buffer);",
        "wasmTable = Q.asm.k;",
        "(_malloc = Q._malloc = Q.asm.f)",
        "(stackAlloc = Q.stackAlloc = Q.asm.n)",
        "var wasmImports = {",
    ] {
        assert!(code.contains(named), "{named} missing\n{code}");
    }
}

#[test]
fn emscripten_three_glue_from_a_published_package_matches_its_module_and_keeps_its_names() {
    let mut out = run(&fixture("real/wasm-lz4.js"));
    let before = out.code().to_string();
    out.settle_wasm(&supplied("wasm-lz4.wasm", "real/wasm-lz4.wasm"), true);
    assert_eq!(out.wasm.len(), 1);
    assert!(out.wasm_unresolved.is_empty());
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm[0].renamed, 0);
    assert_eq!(out.code(), before);
}

#[test]
fn emscripten_three_glue_given_another_module_reports_the_mismatch_and_names_nothing() {
    let mut out = run(&fixture("real/wasm-lz4.js"));
    let before = out.code().to_string();
    out.settle_wasm(&supplied("wasm-lz4.wasm", "emscripten-nameless.wasm"), true);
    assert_eq!(out.wasm[0].renamed, 0);
    assert_eq!(out.code(), before);
    assert!(wasm_notes(&out)
        .iter()
        .any(|note| note.contains("does not match the glue")));
}

fn webpack_bundle(modules: &[(&str, &str)]) -> String {
    let table: Vec<String> = modules
        .iter()
        .map(|(path, code)| format!("\"./src/{path}\":(e,t,n)=>{{{code}}}"))
        .collect();
    let requires: String = modules
        .iter()
        .map(|(path, _)| format!("n(\"./src/{path}\");"))
        .collect();
    format!(
        "(()=>{{var e={{{}}},t={{}};function n(r){{var o=t[r];if(void 0!==o)return o.exports;var s=t[r]={{id:r,exports:{{}}}};return e[r](s,s.exports,n),s.exports}}{requires}}})();",
        table.join(",")
    )
}

const OTHER_MODULE: (&str, &str) = ("other.js", "var o={a:{z:1}};console.log(o)");

#[test]
fn glue_names_found_in_a_bundle_module_also_apply_in_the_combined_output() {
    let source = webpack_bundle(&[("em.js", &fixture("emscripten-nameless.js")), OTHER_MODULE]);
    let out = settled(&source);
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm.len(), 1);
    assert_eq!(out.wasm[0].renamed, 5);
    let module = module_code(&out, "src/em.js");
    let code = out.code();
    for named in [
        "function _emscripten_memcpy_big(e, t, n)",
        "wasmMemory = wasmExports.c;",
        "var wasmImports = {",
        "const e = { a: wasmImports };",
        "_malloc(4);",
    ] {
        assert!(module.contains(named), "{named} missing\n{module}");
        assert!(code.contains(named), "{named} missing\n{code}");
    }
    assert!(code.contains("console.log({ a: { z: 1 } });"), "{code}");
}

#[test]
fn two_bundle_modules_with_their_own_wasm_name_their_glue_in_the_combined_output() {
    let source = webpack_bundle(&[
        ("em.js", &fixture("emscripten-nameless.js")),
        ("named.js", &fixture("emscripten.js")),
    ]);
    let out = settled(&source);
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm.len(), 2);
    let code = out.code();
    for named in [
        "wasmMemory = wasmExports.c;",
        "_malloc(4);",
        "function emscripten_memcpy_big(e, t, n)",
        "Module._main = wasmExports.d;",
    ] {
        assert!(code.contains(named), "{named} missing\n{code}");
    }
    assert_eq!(code.matches("var wasmImports = {").count(), 2, "{code}");
}

#[test]
fn a_bundle_module_whose_wasm_does_not_cover_its_glue_names_nothing_in_the_combined_output() {
    let glue = fixture("emscripten-nameless.js").replace("Z.e)(e)", "Z.q)(e)");
    let source = webpack_bundle(&[("em.js", &glue), OTHER_MODULE]);
    let out = settled(&source);
    assert_eq!(out.wasm[0].renamed, 0);
    let code = out.code();
    assert!(code.contains("Yb(4);"), "{code}");
    assert!(!code.contains("wasmExports"), "{code}");
    let notes = wasm_notes(&out);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].contains("does not match the glue in src/em.js"),
        "{notes:?}"
    );
}
