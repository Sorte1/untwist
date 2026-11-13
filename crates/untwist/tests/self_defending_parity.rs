use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, SOURCE_READER_UNGUARDED, SOURCE_READER_UNGUARDED_MODULE};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const WORKERS: usize = 4;
const ARROW_CONTROLLERS: &str = "-terser-arrows--";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/self-defending")
}

fn versions() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/obfuscator-versions")
}

fn version_rows() -> Vec<(String, String)> {
    let manifest = std::fs::read_to_string(versions().join("MANIFEST")).unwrap();
    manifest
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let (file, cell, source) = (fields.first()?, fields.get(1)?, fields.get(2)?);
            let stem = Path::new(source).file_stem()?.to_str()?;
            cell.contains("-self-")
                .then(|| (file.to_string(), format!("{stem}.stdout")))
        })
        .collect()
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn rows() -> Vec<String> {
    let manifest = std::fs::read_to_string(fixtures().join("MANIFEST")).unwrap();
    manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| line.split('\t').next().unwrap().to_string())
        .collect()
}

fn lanes() -> [(&'static str, Options); 3] {
    [
        ("default", Options::default()),
        (
            "unsafe",
            Options {
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
        (
            "no-deobfuscate no-unminify",
            Options {
                deobfuscate: false,
                unminify: false,
                ..Options::default()
            },
        ),
    ]
}

fn observed(node: &Path, path: &Path) -> String {
    match run(node, path).unwrap() {
        Some(out) => format!("{}exit {:?}", out.stdout, out.exit),
        None => "did not finish".to_string(),
    }
}

fn check(node: &Path, dir: &Path, file: &str) -> Vec<String> {
    let expected =
        std::fs::read_to_string(fixtures().join("expected").join(format!("{file}.stdout")))
            .unwrap();
    let net_allowed = file.contains(ARROW_CONTROLLERS);
    compare(node, dir, &fixtures().join(file), &expected, net_allowed)
}

fn compare(
    node: &Path,
    dir: &Path,
    input: &Path,
    expected: &str,
    net_allowed: bool,
) -> Vec<String> {
    let file = input.file_name().unwrap().to_str().unwrap();
    let source = std::fs::read_to_string(input).unwrap();
    let want = format!("{expected}exit Some(0)");
    let mut wrong = Vec::new();
    for (lane, options) in lanes() {
        let out = Pipeline::new(options).run(&source);
        let netted = out.notes.iter().any(|note| {
            note.message == SOURCE_READER_UNGUARDED
                || note.message == SOURCE_READER_UNGUARDED_MODULE
        });
        let path = dir.join(format!("{file}.{}.js", lane.replace(' ', "-")));
        std::fs::write(&path, out.code()).unwrap();
        let got = observed(node, &path);
        if got != want || (netted && !net_allowed) {
            wrong.push(format!(
                "{file} ({lane}): node printed {got:?} where the input printed {want:?}; \
                 passed through by the source-reader check: {netted}"
            ));
        }
    }
    wrong
}

fn in_parallel<T: Sync>(items: &[T], run: impl Fn(&T) -> Vec<String> + Sync) -> Vec<String> {
    let chunk = items.len().div_ceil(WORKERS).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|part| {
                let run = &run;
                scope.spawn(move || part.iter().flat_map(run).collect::<Vec<_>>())
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    })
}

#[test]
fn every_self_defending_cell_of_the_versions_matrix_runs_like_its_input_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let rows = version_rows();
    assert!(rows.len() >= 90, "{}", rows.len());
    let wrong = in_parallel(&rows, |(file, stdout)| {
        let input = versions().join(file);
        let expected = std::fs::read_to_string(versions().join("expected").join(stdout)).unwrap();
        if observed(&node, &input) != format!("{expected}exit Some(0)") {
            return Vec::new();
        }
        compare(&node, dir.path(), &input, &expected, true)
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_self_defending_combination_runs_like_its_input_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let rows = rows();
    assert!(rows.len() >= 100, "{}", rows.len());
    let wrong = in_parallel(&rows, |file| check(&node, dir.path(), file));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
