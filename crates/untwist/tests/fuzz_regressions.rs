use std::path::{Path, PathBuf};

enum ArtifactKind {
    Crash,
    Slow,
    Unknown,
}

fn artifact_kind(name: &str) -> ArtifactKind {
    if name.starts_with("crash-") || name.starts_with("leak-") || name.starts_with("oom-") {
        ArtifactKind::Crash
    } else if name.starts_with("timeout-") || name.starts_with("slow-unit-") {
        ArtifactKind::Slow
    } else {
        ArtifactKind::Unknown
    }
}

fn kind_of(path: &Path) -> ArtifactKind {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        panic!("{} has a non-UTF-8 file name", path.display());
    };
    artifact_kind(name)
}

fn artifact_paths() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/artifacts");
    let Ok(targets) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for target in targets {
        let Ok(target) = target else {
            panic!("{} could not be listed", dir.display());
        };
        let target_path = target.path();
        let Ok(entries) = std::fs::read_dir(&target_path) else {
            panic!("{} could not be listed", target_path.display());
        };
        for entry in entries {
            let Ok(entry) = entry else {
                panic!("{} could not be listed", target_path.display());
            };
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                panic!("{} has no readable file type", path.display());
            };
            if !file_type.is_file() {
                panic!(
                    "{} is a directory, not a committed artifact",
                    path.display()
                );
            }
            files.push(path);
        }
    }
    files.sort();
    files
}

fn read_prefix(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        panic!("{} could not be read", path.display());
    };
    let valid_len = match std::str::from_utf8(&bytes) {
        Ok(_) => bytes.len(),
        Err(err) => err.valid_up_to(),
    };
    let Ok(text) = std::str::from_utf8(&bytes[..valid_len]) else {
        panic!("{} has no valid UTF-8 prefix", path.display());
    };
    text.to_string()
}

fn replay_pipeline(path: &Path, source: &str) {
    for unsafe_transforms in [false, true] {
        for extended in [false, true] {
            let options = untwist::Options {
                unsafe_transforms,
                extended,
                max_iterations: Some(8),
                ..untwist::Options::default()
            };
            let out = untwist::Pipeline::new(options).run(source);
            assert!(
                out.panicked.is_empty(),
                "{} still panics (unsafe_transforms={unsafe_transforms}, extended={extended})",
                path.display()
            );
        }
    }
}

fn replay_passes(_path: &Path, source: &str) {
    let alloc = oxc_allocator::Allocator::default();
    let mut parsed = untwist_core::parse::parse_program(&alloc, source);
    let sem = untwist_core::semantic::build_scoping(&parsed.program);
    let opts = untwist_core::pass::PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: Some(4),
        fuel: None,
        ..untwist_core::pass::PassOptions::default()
    };
    let ctors = untwist::all_pass_ctors();
    let mut group = untwist_core::pass::PassGroup::from_ctors(&ctors, opts);
    let _ = untwist_core::pass::run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

fn replay_evaluator(path: &Path, source: &str) {
    let alloc = oxc_allocator::Allocator::default();
    let mut parsed = untwist_core::parse::parse_program(&alloc, source);
    let sem = untwist_core::semantic::build_scoping(&parsed.program);
    let opts = untwist_core::pass::PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: Some(4),
        fuel: None,
        ..untwist_core::pass::PassOptions::default()
    };
    let (_sem, report) =
        untwist_deobfuscate::run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    assert!(
        report.iterations <= untwist_deobfuscate::DEOBFUSCATE_MAX_ROUNDS + 1,
        "{} ran the deobfuscate driver past its round cap",
        path.display()
    );
}

type Replayer = fn(&Path, &str);

const TARGETS: [(&str, Replayer); 3] = [
    ("pipeline", replay_pipeline),
    ("passes", replay_passes),
    ("evaluator", replay_evaluator),
];

fn target_of(path: &Path) -> Option<&str> {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
}

fn replayer_of(target: &str) -> Option<Replayer> {
    TARGETS
        .iter()
        .find(|(name, _)| *name == target)
        .map(|(_, replayer)| *replayer)
}

fn replay(path: &Path) {
    let source = read_prefix(path);
    let Some(replayer) = target_of(path).and_then(replayer_of) else {
        panic!(
            "{} is not under a known fuzz target directory",
            path.display()
        );
    };
    replayer(path, &source);
}

#[test]
fn every_committed_crash_artifact_still_runs_clean() {
    let files: Vec<PathBuf> = artifact_paths()
        .into_iter()
        .filter(|path| !matches!(kind_of(path), ArtifactKind::Slow))
        .collect();
    assert!(
        !files.is_empty(),
        "no crash/leak/oom artifact is committed under fuzz/artifacts/, so this test asserts \
         nothing; commit at least the finding a fuzz campaign produces"
    );
    files.iter().for_each(|path| replay(path));
}

fn slow_artifacts_of(target: &str) -> Vec<PathBuf> {
    artifact_paths()
        .into_iter()
        .filter(|path| matches!(kind_of(path), ArtifactKind::Slow))
        .filter(|path| target_of(path) == Some(target))
        .collect()
}

fn every_slow_artifact_of(target: &str) {
    let files = slow_artifacts_of(target);
    assert!(
        !files.is_empty(),
        "no timeout/slow-unit artifact is committed under fuzz/artifacts/{target}/, so this \
         test asserts nothing"
    );
    files.iter().for_each(|path| replay(path));
}

#[test]
fn every_slow_artifact_sits_under_a_known_target() {
    for path in artifact_paths() {
        if matches!(kind_of(&path), ArtifactKind::Slow) {
            assert!(
                target_of(&path).and_then(replayer_of).is_some(),
                "{} is not under a known fuzz target directory",
                path.display()
            );
        }
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the debug-build interpreter is too slow on these; run with cargo test --release"
)]
fn every_committed_slow_pipeline_artifact_still_finishes() {
    every_slow_artifact_of("pipeline");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the debug-build interpreter is too slow on these; run with cargo test --release"
)]
fn every_committed_slow_passes_artifact_still_finishes() {
    every_slow_artifact_of("passes");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the debug-build interpreter is too slow on these; run with cargo test --release"
)]
fn every_committed_slow_evaluator_artifact_still_finishes() {
    every_slow_artifact_of("evaluator");
}
