mod common;

use std::path::{Path, PathBuf};

use common::{bin, stdin_output};
use untwist_rename::fingerprint::alpha_fingerprint;
use untwist_rename::generate::title_case;
use untwist_rename::reserved::{PSEUDO_GLOBALS, RESERVED_WORDS};

const MODES: [&str; 4] = ["mangle", "simple", "unique", "smart"];
const RENAMING_FIXTURE: &str = "ternary-logical-if.js";
const MANGLE_CORPORA: [&str; 3] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
];
const IDEMPOTENCY_ALLOWLIST: &str = "tests/rename-idempotency-allowlist.txt";
const READS_OF_A_RESERVED_BASE: usize = 400;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn fixtures_dir() -> PathBuf {
    repo_root().join("fixtures/unminify")
}

fn js_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "js"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixtures found in {}", dir.display());
    files
}

fn fixtures() -> Vec<PathBuf> {
    js_files_under(&fixtures_dir())
}

fn corpora() -> Vec<PathBuf> {
    let root = repo_root();
    MANGLE_CORPORA
        .iter()
        .flat_map(|corpus| js_files_under(&root.join(corpus)))
        .collect()
}

fn relative(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn idempotency_allowlist() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(IDEMPOTENCY_ALLOWLIST);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut words = line.split_whitespace();
            let mode = words.next().unwrap_or_default().to_string();
            let file = words.next().unwrap_or_default().to_string();
            assert!(
                MODES.contains(&mode.as_str()) && !file.is_empty(),
                "malformed allowlist line: {line}"
            );
            (mode, file)
        })
        .collect()
}

fn run(path: &Path, mode: &str) -> String {
    let out = bin()
        .arg("--rename")
        .arg(mode)
        .arg("--no-unpack")
        .arg(path)
        .output()
        .expect("spawn untwist");
    assert!(
        out.status.success(),
        "untwist --rename {mode} failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "untwist --rename {mode} wrote to stderr on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn fingerprint(output: &str, path: &Path, mode: &str) -> String {
    alpha_fingerprint(output).unwrap_or_else(|| {
        panic!(
            "--rename {mode} output for {} did not re-parse cleanly",
            path.display()
        )
    })
}

fn assert_every_mode_renamed_something(renamed: [usize; MODES.len()], fixtures: usize) {
    for (index, mode) in MODES.into_iter().enumerate() {
        assert!(
            renamed[index] > 0,
            "--rename {mode} left all {fixtures} fixtures byte-identical to the --rename none baseline"
        );
    }
}

#[test]
fn every_fixture_renames_and_keeps_its_binding_structure() {
    let mut renamed = [0usize; MODES.len()];
    let paths = fixtures();
    for path in &paths {
        let baseline = run(path, "none");
        let expected = fingerprint(&baseline, path, "none");
        for (index, mode) in MODES.into_iter().enumerate() {
            let output = run(path, mode);
            assert!(
                !output.is_empty(),
                "{} produced no output in {mode}",
                path.display()
            );
            assert_eq!(
                expected,
                fingerprint(&output, path, mode),
                "--rename {mode} changed the binding structure of {}",
                path.display()
            );
            if output != baseline {
                renamed[index] += 1;
            }
        }
    }
    assert_every_mode_renamed_something(renamed, paths.len());
}

#[test]
fn mangling_a_known_fixture_actually_changes_it() {
    let path = fixtures_dir().join(RENAMING_FIXTURE);
    let baseline = run(&path, "none");
    let mangled = run(&path, "mangle");
    assert_ne!(
        baseline, mangled,
        "--rename mangle left {RENAMING_FIXTURE} byte-identical to the --rename none baseline"
    );
}

#[test]
fn renaming_is_deterministic() {
    for path in fixtures() {
        for mode in MODES {
            assert_eq!(
                run(&path, mode),
                run(&path, mode),
                "{} in {mode}",
                path.display()
            );
        }
    }
}

fn first_difference(once: &str, twice: &str) -> String {
    once.lines()
        .zip(twice.lines())
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .map(|(line, (a, b))| format!("line {}:\n  once:  {a}\n  twice: {b}", line + 1))
        .unwrap_or_else(|| "the outputs differ only in length".to_string())
}

#[test]
fn renaming_an_already_renamed_unminify_output_changes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    for path in fixtures() {
        let intermediate = dir
            .path()
            .join(path.file_name().expect("fixture file name"));
        for mode in MODES {
            let once = run(&path, mode);
            std::fs::write(&intermediate, &once).expect("write intermediate");
            let twice = run(&intermediate, mode);
            assert_eq!(
                once,
                twice,
                "re-running the whole pipeline under --rename {mode} changed {} again; \
                 the culprit may be any stage, not only the rename\n{}",
                relative(&path),
                first_difference(&once, &twice)
            );
        }
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "300 CLI runs over the three diff-mangle corpora; run with cargo test --release -p untwist-cli --test rename_acceptance"
)]
fn renaming_an_already_renamed_output_changes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let allowlist = idempotency_allowlist();
    let mut stale: Vec<(String, String)> = allowlist.clone();
    for path in corpora() {
        let intermediate = dir
            .path()
            .join(path.file_name().expect("fixture file name"));
        for mode in MODES {
            let once = run(&path, mode);
            std::fs::write(&intermediate, &once).expect("write intermediate");
            let twice = run(&intermediate, mode);
            let entry = (mode.to_string(), relative(&path));
            if once == twice {
                continue;
            }
            assert!(
                allowlist.contains(&entry),
                "re-running the whole pipeline under --rename {mode} changed {} again and \
                 {IDEMPOTENCY_ALLOWLIST} does not name it; the culprit may be any stage, not \
                 only the rename\n{}",
                relative(&path),
                first_difference(&once, &twice)
            );
            stale.retain(|listed| *listed != entry);
        }
    }
    assert!(
        stale.is_empty(),
        "{IDEMPOTENCY_ALLOWLIST} names outputs that are idempotent now; drop them: {stale:?}"
    );
}

#[test]
fn rename_none_is_identical_to_omitting_the_flag() {
    for path in fixtures() {
        let out = bin()
            .arg("--no-unpack")
            .arg(&path)
            .output()
            .expect("spawn untwist");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.stderr.is_empty(),
            "untwist wrote to stderr on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let baseline = String::from_utf8_lossy(&out.stdout).into_owned();
        let explicit = run(&path, "none");
        assert_eq!(baseline, explicit, "{}", path.display());
    }
}

fn reserved_words_reachable_as_a_base() -> Vec<(&'static str, String)> {
    RESERVED_WORDS
        .iter()
        .chain(PSEUDO_GLOBALS.iter())
        .filter_map(|&word| {
            let read = format!("\u{e9}{}", word.strip_prefix('v')?);
            (format!("v{}", title_case(&read)) == word).then_some((word, read))
        })
        .collect()
}

#[test]
fn mangling_reads_of_a_reserved_base_numbers_every_binding_from_two() {
    let reachable = reserved_words_reachable_as_a_base();
    assert!(!reachable.is_empty());
    for (word, read) in reachable {
        let source: String = (0..READS_OF_A_RESERVED_BASE)
            .map(|index| format!("var x{index} = {read};\n"))
            .collect();
        let expected: String = (0..READS_OF_A_RESERVED_BASE)
            .map(|index| format!("var {word}{} = {read};\n", index + 2))
            .collect();
        for extra in [&[][..], &["--unsafe"][..]] {
            let args = [&["--rename", "mangle"][..], extra].concat();
            let out = stdin_output(&source, &args);
            assert!(out.status.success(), "{word} {extra:?}");
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                expected,
                "{word} {extra:?}"
            );
        }
    }
}
