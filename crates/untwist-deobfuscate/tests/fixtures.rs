use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn differential_options() -> Options {
    Options {
        unpack: false,
        jsx: false,
        unsafe_transforms: true,
        extended: false,
        ..Options::default()
    }
}

fn normalize(code: &str) -> String {
    let alloc = oxc_allocator::Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, code);
    untwist_core::print::print(&parsed.program, untwist_core::print::PrintStyle::Compact)
}

fn allowlisted() -> Vec<String> {
    let path = root().join("xtask/diff-deobfuscate-allowlist.txt");
    std::fs::read_to_string(&path)
        .expect("read the differential allowlist")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("fixtures/"))
        .map(str::to_string)
        .collect()
}

fn samples() -> Vec<PathBuf> {
    let dir = root().join("fixtures/reference/deobfuscate");
    let entries = |suffix: &'static str| {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("fixtures")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.to_string_lossy().ends_with(suffix))
            .collect();
        found.sort();
        found
    };
    let mut files = entries(".js");
    assert!(!files.is_empty(), "no fixtures under {}", dir.display());
    assert_eq!(
        files.len(),
        entries(".js.snap").len(),
        "every .js needs a .js.snap"
    );
    if untwist_testing::real_world_path("game.js").is_some() {
        files.push(root().join("fixtures/real-world/game.js"));
    }
    files
}

fn relative(file: &Path) -> String {
    file.strip_prefix(root())
        .unwrap_or(file)
        .display()
        .to_string()
}

fn matches_reference(file: &Path) -> bool {
    let source = std::fs::read_to_string(file).expect("read fixture");
    let expected = std::fs::read_to_string(file.with_extension("js.snap")).expect("snapshot");
    let actual = Pipeline::new(differential_options()).run(&source);
    normalize(&expected) == normalize(actual.code())
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-deobfuscate --test fixtures"
)]
fn every_deobfuscate_sample_matches_reference_except_the_allowlist() {
    let allowlist = allowlisted();
    let mut failures = Vec::new();
    let mut stale = Vec::new();
    for file in samples() {
        let name = relative(&file);
        let identical = matches_reference(&file);
        match (allowlist.contains(&name), identical) {
            (false, false) => failures.push(name),
            (true, true) => stale.push(name),
            _ => {}
        }
    }
    assert!(
        failures.is_empty(),
        "differ from the reference deobfuscator with no allowlist entry to own the difference: {failures:?}"
    );
    assert!(
        stale.is_empty(),
        "match the reference deobfuscator, so their allowlist entry is stale: {stale:?}"
    );
}

#[test]
fn the_allowlist_names_only_fixtures_that_exist() {
    for name in allowlisted() {
        if let Some(sample) = name.strip_prefix("fixtures/real-world/") {
            if untwist_testing::real_world_path(sample).is_none() {
                continue;
            }
        }
        assert!(
            root().join(&name).is_file(),
            "{name} is allowlisted but does not exist"
        );
    }
}

const RETRACTED: &[&str] = &[
    "control-flow object kept",
    "decoder machinery kept",
    "left in place",
];

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset decodes hundreds of call sites; run with cargo test --release -p untwist-deobfuscate --test fixtures"
)]
fn the_high_preset_reports_no_refusal_its_own_output_contradicts() {
    let source = untwist_testing::read_fixture("deobfuscate", "obfuscator.io-high.js");
    let out = Pipeline::new(differential_options()).run(&source);
    assert_eq!(
        out.code().trim(),
        "function hi() {\n  console.log(\"Hello World!\");\n}\nhi();"
    );
    let stale: Vec<&String> = out
        .notes
        .iter()
        .map(|note| &note.message)
        .filter(|message| RETRACTED.iter().any(|text| message.contains(text)))
        .collect();
    assert!(
        stale.is_empty(),
        "the report describes objects, machinery and wrappers the output does not contain \
        : {stale:?}"
    );
}
