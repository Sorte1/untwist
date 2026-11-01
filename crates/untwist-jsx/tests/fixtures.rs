use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> Vec<PathBuf> {
    let dir = root().join("fixtures/jsx");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/jsx")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixtures under {}", dir.display());
    files
}

fn conservative_options() -> Options {
    Options {
        unpack: false,
        deobfuscate: false,
        unsafe_transforms: true,
        extended: false,
        ..Options::default()
    }
}

#[test]
fn every_jsx_fixture_reparses_and_is_idempotent() {
    let pipeline = Pipeline::new(conservative_options());
    for file in fixtures() {
        let source = std::fs::read_to_string(&file).expect("read fixture");
        let first = pipeline.run(&source);
        assert_eq!(
            first.parse_errors,
            0,
            "{}: {:?}",
            file.display(),
            first.parse_diagnostics
        );
        assert!(
            first.code().contains("/>") || first.code().contains("</"),
            "{} produced no JSX",
            file.display()
        );
        let second = pipeline.run(first.code());
        assert_eq!(second.parse_errors, 0, "{}", file.display());
        assert_eq!(
            second.code(),
            first.code(),
            "{} is not idempotent",
            file.display()
        );
    }
}
