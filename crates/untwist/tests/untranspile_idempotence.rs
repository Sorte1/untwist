use std::path::{Path, PathBuf};

use untwist::{Options, Output, Pipeline};
use untwist_unminify::untranspile::shapes;
use untwist_unminify::untranspile_ctors;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn untranspile_fixtures(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            untranspile_fixtures(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path);
        }
    }
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn untranspile_changes(out: &Output) -> Vec<(&'static str, u32)> {
    let names: Vec<&'static str> = untranspile_ctors()
        .iter()
        .map(|ctor| ctor().name())
        .collect();
    out.stages
        .iter()
        .flat_map(|stage| stage.per_pass.iter().copied())
        .filter(|(pass, changes)| *changes > 0 && names.contains(pass))
        .collect()
}

fn assert_a_second_run_fires_no_untranspile_pass(path: &Path) {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let pipeline = Pipeline::new(unsafe_options());
    let first = pipeline.run(&source);
    shapes::take();
    let second = pipeline.run(first.code());
    let fired = shapes::take();
    assert!(
        fired.is_empty(),
        "{}: a second run fired untranspile shapes: {fired:?}",
        path.display()
    );
    let changed = untranspile_changes(&second);
    assert!(
        changed.is_empty(),
        "{}: a second run changed code through untranspile passes: {changed:?}",
        path.display()
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "game.js twice; run with cargo test --release -p untwist --test untranspile_idempotence"
)]
fn running_untwist_on_its_own_output_fires_no_untranspile_pass() {
    let mut files: Vec<PathBuf> = untwist_testing::real_world_path("game.js")
        .into_iter()
        .collect();
    untranspile_fixtures(&root().join("fixtures/untranspile"), &mut files);
    for file in &files {
        assert_a_second_run_fires_no_untranspile_pass(file);
    }
}
