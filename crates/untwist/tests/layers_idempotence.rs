use std::path::{Path, PathBuf};

use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::layers::notes::{LAYER_CAP, LAYER_CYCLE};

const LAYER_STAGES: &[&str] = &["layers", "inline-layers"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn layer_fixtures(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            layer_fixtures(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path);
        }
    }
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn layer_changes(out: &Output) -> Vec<(&'static str, u32)> {
    out.stages
        .iter()
        .filter(|stage| LAYER_STAGES.contains(&stage.stage) && stage.changes > 0)
        .map(|stage| (stage.stage, stage.changes))
        .chain(
            out.stages
                .iter()
                .flat_map(|stage| stage.per_pass.iter().copied())
                .filter(|(pass, changes)| *changes > 0 && LAYER_STAGES.contains(pass)),
        )
        .collect()
}

fn stopped_on(out: &Output, reason: &str) -> bool {
    out.notes
        .iter()
        .any(|note| note.pass == "layers" && note.message.starts_with(reason))
}

fn assert_a_second_run_peels_nothing(path: &Path, unsafe_transforms: bool) {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let pipeline = Pipeline::new(options(unsafe_transforms));
    let first = pipeline.run(&source);
    let second = pipeline.run(first.code());
    if stopped_on(&first, LAYER_CAP) {
        return;
    }
    if stopped_on(&first, LAYER_CYCLE) {
        assert_eq!(
            second.code(),
            first.code(),
            "{} (unsafe {unsafe_transforms}): a cycle's output is not stable",
            path.display()
        );
        return;
    }
    let changed = layer_changes(&second);
    assert!(
        changed.is_empty(),
        "{} (unsafe {unsafe_transforms}): a second run peeled layers: {changed:?}",
        path.display()
    );
}

#[test]
fn running_untwist_on_its_own_output_peels_no_layer() {
    let mut files = Vec::new();
    layer_fixtures(&root().join("fixtures/layers"), &mut files);
    files.sort();
    assert!(
        files.len() >= 50,
        "fixtures/layers shrank to {}",
        files.len()
    );
    for file in &files {
        assert_a_second_run_peels_nothing(file, false);
        assert_a_second_run_peels_nothing(file, true);
    }
}
