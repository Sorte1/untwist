use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use untwist::{Options, Pipeline};

const LAYER_PASSES: &[&str] = &["layers", "inline-layers"];

const GAME: &str = "fixtures/real-world/game.js";
const HIGH: &str = "fixtures/reference/deobfuscate/obfuscator.io-high.js";

const BASE_PINS: [(&str, &str, Mode, &str); 10] = [
    ("g-d", GAME, Mode::Safe, "116fc842"),
    ("g-p", GAME, Mode::Conservative, "5e3dabe1"),
    ("g-u", GAME, Mode::Unsafe, "d077b1d0"),
    ("g-up", GAME, Mode::UnsafeParity, "9ce29007"),
    ("h-d", HIGH, Mode::Safe, "44ea11f5"),
    ("h-p", HIGH, Mode::Conservative, "44ea11f5"),
    ("h-u", HIGH, Mode::Unsafe, "44ea11f5"),
    ("h-up", HIGH, Mode::UnsafeParity, "44ea11f5"),
    ("high u", HIGH, Mode::HighUnsafe, "26697d8e9aede2e7"),
    ("high up", HIGH, Mode::HighUnsafeParity, "26697d8e9aede2e7"),
];

const J_DIR: &str = "fixtures/reference/deobfuscate";

const J_FIXTURES_AT_THE_BASE: &[(&str, &str, &str)] = &[
    ("obfuscator.io-calls-transform", "", "cde9d4979790b72a"),
    ("obfuscator.io-control-flow", "", "02edb4c176e22d27"),
    ("obfuscator.io-control-flow-keys", "", "c3a60ff914db5461"),
    (
        "obfuscator.io-control-flow-partial-keys",
        "",
        "c3a60ff914db5461",
    ),
    (
        "obfuscator.io-control-flow-split-strings",
        "",
        "d390ca8b61ed0e9e",
    ),
    ("obfuscator.io-control-flow-spread", "", "707bfef81beec535"),
    (
        "obfuscator.io-control-flow-switch-return",
        "",
        "75c97bfa4b7f5631",
    ),
    ("obfuscator.io-decode-external-vars", "", "6a70878e2f3fc12f"),
    ("obfuscator.io-function-wrapper", "", "4b125cd52c7fb2fe"),
    ("obfuscator.io-high", "", "44ea11f5b742001f"),
    ("obfuscator.io", "", "69c0935147538771"),
    ("obfuscator.io-multi-encoders", "", "44ea11f5b742001f"),
    ("obfuscator.io-rotator-unary", "", "44ea11f5b742001f"),
    (
        "obfuscator.io-string-array-undefined",
        "",
        "04c41db9a9b9f91d",
    ),
    ("obfuscator.io-var-functions", "", "7b78dae3be3091f0"),
    ("simple-string-array", "", "a3727ad57aba5330"),
    (
        "obfuscator.io-calls-transform",
        "unsafe",
        "cde9d4979790b72a",
    ),
    ("obfuscator.io-control-flow", "unsafe", "02edb4c176e22d27"),
    (
        "obfuscator.io-control-flow-keys",
        "unsafe",
        "c3a60ff914db5461",
    ),
    (
        "obfuscator.io-control-flow-partial-keys",
        "unsafe",
        "c3a60ff914db5461",
    ),
    (
        "obfuscator.io-control-flow-split-strings",
        "unsafe",
        "d390ca8b61ed0e9e",
    ),
    (
        "obfuscator.io-control-flow-spread",
        "unsafe",
        "707bfef81beec535",
    ),
    (
        "obfuscator.io-control-flow-switch-return",
        "unsafe",
        "75c97bfa4b7f5631",
    ),
    (
        "obfuscator.io-decode-external-vars",
        "unsafe",
        "6a70878e2f3fc12f",
    ),
    (
        "obfuscator.io-function-wrapper",
        "unsafe",
        "4b125cd52c7fb2fe",
    ),
    ("obfuscator.io-high", "unsafe", "44ea11f5b742001f"),
    ("obfuscator.io", "unsafe", "69c0935147538771"),
    ("obfuscator.io-multi-encoders", "unsafe", "44ea11f5b742001f"),
    ("obfuscator.io-rotator-unary", "unsafe", "44ea11f5b742001f"),
    (
        "obfuscator.io-string-array-undefined",
        "unsafe",
        "04c41db9a9b9f91d",
    ),
    ("obfuscator.io-var-functions", "unsafe", "7b78dae3be3091f0"),
    ("simple-string-array", "unsafe", "a3727ad57aba5330"),
];

#[derive(Clone, Copy, Debug)]
enum Mode {
    Safe,
    Conservative,
    Unsafe,
    UnsafeParity,
    HighUnsafe,
    HighUnsafeParity,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn options(mode: Mode) -> Options {
    let (unsafe_transforms, extended, shell) = match mode {
        Mode::Safe => (false, true, true),
        Mode::Conservative => (false, false, true),
        Mode::Unsafe => (true, true, true),
        Mode::UnsafeParity => (true, false, true),
        Mode::HighUnsafe => (true, true, false),
        Mode::HighUnsafeParity => (true, false, false),
    };
    Options {
        unsafe_transforms,
        extended,
        unpack: shell,
        jsx: shell,
        disabled_passes: LAYER_PASSES.to_vec(),
        ..Options::default()
    }
}

fn hash_of(file: &str, mode: Mode) -> String {
    let path = root().join(file);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let out = Pipeline::new(options(mode)).run(&source);
    let mut hasher = Sha256::new();
    hasher.update(out.code().as_bytes());
    if matches!(mode, Mode::HighUnsafe | Mode::HighUnsafeParity) {
        hasher.update(b"\n");
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn j_mode(label: &str) -> Mode {
    if label == "unsafe" {
        Mode::Unsafe
    } else {
        Mode::Safe
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "game.js in four modes; run with cargo test --release -p untwist --test layers_attribution"
)]
fn with_both_layer_passes_disabled_the_pins_equal_the_base() {
    let mut moved = Vec::new();
    for (pin, file, mode, expected) in BASE_PINS {
        if file == GAME && untwist_testing::real_world_path("game.js").is_none() {
            continue;
        }
        let actual = hash_of(file, mode);
        if !actual.starts_with(expected) {
            moved.push(format!(
                "{pin} {} (base {expected})",
                &actual[..expected.len()]
            ));
        }
    }
    assert!(
        moved.is_empty(),
        "a pin moved with both layer passes disabled: {moved:?}"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the deobfuscate fixtures in two modes; run with cargo test --release -p untwist --test layers_attribution"
)]
fn with_both_layer_passes_disabled_the_j_fixtures_equal_the_base() {
    let mut moved = Vec::new();
    for (file, label, expected) in J_FIXTURES_AT_THE_BASE {
        let actual = hash_of(&format!("{J_DIR}/{file}.js"), j_mode(label));
        if !actual.starts_with(expected) {
            moved.push(format!("{file} [{label}] {}", &actual[..expected.len()]));
        }
    }
    assert_eq!(J_FIXTURES_AT_THE_BASE.len(), 32, "the j table lost rows");
    assert!(
        moved.is_empty(),
        "a j fixture moved with both layer passes disabled: {moved:?}"
    );
}
