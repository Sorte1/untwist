use serde_json::Value;
use untwist::{RenameMode, WasmCheck};
use untwist_wasm::{map_flags, options_from};

const CASES: &str = include_str!("../../../web/src/settings/flags.json");
const PASSES_JSON: &str = include_str!("../../../web/src/settings/passes.json");

fn flag(settings: &Value, key: &str, default: bool) -> bool {
    settings
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn number(settings: &Value, key: &str) -> Option<u64> {
    settings.get(key).and_then(Value::as_u64)
}

fn fuel(settings: &Value) -> Option<u64> {
    match settings.get("fuel") {
        Some(Value::Null) => Some(u64::MAX),
        _ => number(settings, "fuel"),
    }
}

fn disabled(settings: &Value) -> Vec<&str> {
    settings
        .get("off")
        .and_then(Value::as_str)
        .map(|off| off.split(',').filter(|name| !name.is_empty()).collect())
        .unwrap_or_default()
}

fn rename(settings: &Value) -> RenameMode {
    match settings.get("rename").and_then(Value::as_str) {
        Some("simple") => RenameMode::Simple,
        Some("smart") => RenameMode::Smart,
        Some("unique") => RenameMode::Unique,
        Some("mangle") => RenameMode::Mangle,
        _ => RenameMode::None,
    }
}

fn wasm_check(settings: &Value) -> WasmCheck {
    settings
        .get("wasm")
        .and_then(Value::as_str)
        .and_then(WasmCheck::named)
        .unwrap_or_default()
}

#[test]
fn every_flag_string_the_site_emits_parses_to_the_intended_options() {
    let cases: Vec<Value> = serde_json::from_str(CASES).expect("flags.json parses");
    assert!(cases.len() > 10);
    for case in &cases {
        let settings = &case["settings"];
        let flags = case["flags"].as_str().expect("flags is a string");
        let options = options_from(flags).unwrap_or_else(|error| panic!("{flags}: {error}"));
        assert_eq!(options.unpack, flag(settings, "unpack", true), "{flags}");
        assert_eq!(options.jsx, flag(settings, "jsx", true), "{flags}");
        assert_eq!(
            options.unsafe_transforms,
            flag(settings, "unsafe", false),
            "{flags}"
        );
        assert_eq!(options.rename, rename(settings), "{flags}");
        assert_eq!(options.fuel, fuel(settings), "{flags}");
        assert_eq!(options.deobfuscate, flag(settings, "deob", true), "{flags}");
        assert_eq!(options.unminify, flag(settings, "unmin", true), "{flags}");
        assert_eq!(
            options.rename_all,
            !flag(settings, "human", false),
            "{flags}"
        );
        assert_eq!(
            options.max_iterations.map(u64::from),
            number(settings, "iter"),
            "{flags}"
        );
        assert_eq!(options.disabled_passes, disabled(settings), "{flags}");
        assert_eq!(
            options.single_file,
            flag(settings, "single", false),
            "{flags}"
        );
        assert_eq!(options.wasm_check, wasm_check(settings), "{flags}");
        let split = flag(settings, "split", false);
        assert_eq!(options.split_hoisted, split, "{flags}");
        assert_eq!(
            options.split_assume_modules,
            split && flag(settings, "assume", false),
            "{flags}"
        );
        let modern = settings.get("modern").and_then(Value::as_str);
        assert_eq!(
            options.modern.as_ref().map(|baseline| baseline.text()),
            modern,
            "{flags}"
        );
        let maps = map_flags(flags);
        assert_eq!(options.source_map, flag(settings, "smap", false), "{flags}");
        assert_eq!(maps.inline, flag(settings, "smap", false), "{flags}");
        assert_eq!(maps.input_inline, flag(settings, "imap", false), "{flags}");
        assert_eq!(
            options.max_memory,
            Some(number(settings, "mem").map_or(untwist_wasm::DEFAULT_MAX_MEMORY, |mib| mib << 20)),
            "{flags}"
        );
        assert_eq!(
            options.timeout,
            number(settings, "time").map(std::time::Duration::from_secs),
            "{flags}"
        );
    }
}

#[test]
fn the_sites_pass_list_is_the_engines_pass_table() {
    let site: Value = serde_json::from_str(PASSES_JSON).expect("passes.json parses");
    let engine: Value =
        serde_json::from_str(&untwist_wasm::pass_table()).expect("pass_table parses");
    assert_eq!(site, engine);
}
