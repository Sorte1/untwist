use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use sha2::{Digest, Sha256};
use untwist::{Options, Output, Pipeline};
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::table::{exact_key, loose_key};
use untwist_rename::library::{unit_records, SHAPE_EPOCH};

const DIR: &str = "fixtures/libraries";
const SHAPES: &str = "fixtures/libraries/SHAPES";
const WRITE_SHAPES: &str = "UNTWIST_WRITE_SHAPE_EPOCH";
const EPOCH_TAG: &str = "#shape-epoch\t";
const MIN_UNIT_TOKENS: u32 = 12;

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels below the repository")
}

fn fixtures() -> Vec<(String, PathBuf)> {
    let root = repo().join(DIR);
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(&root)
        .expect("the library fixtures exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .flat_map(|library| {
            std::fs::read_dir(&library)
                .expect("reading a library folder")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
                .collect::<Vec<_>>()
        })
        .map(|file| {
            let name = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .display()
                .to_string();
            (name, file)
        })
        .collect();
    found.sort();
    found
}

fn texts(output: &Output) -> Vec<&str> {
    match &output.bundle {
        Some(bundle) if !bundle.modules.is_empty() => bundle
            .modules
            .iter()
            .map(|module| module.code.as_str())
            .collect(),
        _ => output.codes.iter().map(String::as_str).collect(),
    }
}

fn keys_of(text: &str, into: &mut BTreeSet<(u32, u64)>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return;
    }
    let sem = build_scoping(&parsed.program);
    for record in unit_records(&parsed.program, &sem.scoping) {
        if record.loose.0 >= MIN_UNIT_TOKENS {
            into.insert((loose_key(record.loose), exact_key(record.exact)));
        }
    }
}

fn lane_digest(source: &str, unsafe_transforms: bool) -> String {
    let output = Pipeline::new(Options {
        unsafe_transforms,
        ..Options::default()
    })
    .run(source);
    let mut keys = BTreeSet::new();
    for text in texts(&output) {
        keys_of(text, &mut keys);
    }
    let mut hasher = Sha256::new();
    for (loose, exact) in &keys {
        hasher.update(loose.to_le_bytes());
        hasher.update(exact.to_le_bytes());
    }
    let digest = hasher.finalize();
    let mut hex = String::new();
    for byte in &digest[..12] {
        let _ = write!(hex, "{byte:02x}");
    }
    format!("{}:{hex}", keys.len())
}

fn rows() -> String {
    fixtures()
        .iter()
        .map(|(name, path)| {
            let source = std::fs::read_to_string(path).expect("reading a fixture");
            format!(
                "{name}\t{}\t{}\n",
                lane_digest(&source, false),
                lane_digest(&source, true)
            )
        })
        .collect()
}

fn split(recorded: &str) -> (Option<u32>, &str) {
    let Some(rest) = recorded.strip_prefix(EPOCH_TAG) else {
        return (None, recorded);
    };
    let (epoch, body) = rest.split_once('\n').unwrap_or((rest, ""));
    let body = body
        .split_once('\n')
        .filter(|(header, _)| header.starts_with("# "))
        .map_or(body, |(_, rows)| rows);
    (epoch.trim().parse().ok(), body)
}

#[test]
fn library_fingerprints_change_only_with_a_new_shape_epoch() {
    let computed = rows();
    let text = format!(
        "{EPOCH_TAG}{SHAPE_EPOCH}\n# fixture\tdefault lane\tunsafe lane (units of 12+ tokens: count:sha256; when this moves, bump untwist_rename::library::SHAPE_EPOCH and rerun with {WRITE_SHAPES}=1)\n{computed}"
    );
    let path = repo().join(SHAPES);
    let recorded = std::fs::read_to_string(&path).unwrap_or_default();
    let (epoch, body) = split(&recorded);
    let moved = body != computed;
    if std::env::var_os(WRITE_SHAPES).is_some() {
        assert!(
            !moved || epoch != Some(SHAPE_EPOCH),
            "library fingerprints moved under shape epoch {SHAPE_EPOCH}: bump SHAPE_EPOCH in crates/untwist-rename/src/library/mod.rs, then write {SHAPES} again"
        );
        std::fs::write(&path, &text).expect("writing the shape file");
        return;
    }
    assert_eq!(
        epoch,
        Some(SHAPE_EPOCH),
        "{SHAPES} records another shape epoch; rerun with {WRITE_SHAPES}=1"
    );
    assert!(
        !moved,
        "library fingerprints moved: untwist's output on library code changed, so every published collection loses matches. Bump SHAPE_EPOCH in crates/untwist-rename/src/library/mod.rs and rerun with {WRITE_SHAPES}=1"
    );
}
