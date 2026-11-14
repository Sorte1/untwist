use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const RECORD_MARK: &str = "get l()";
const MODULE_OBJECT: &str = "uses its module object";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-record")
}

fn probes(prefix: char) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root())
        .expect("probes")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .collect();
    found.sort();
    found
}

fn lane(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        jsx: false,
        ..Options::default()
    }
}

struct Written {
    dir: tempfile::TempDir,
    notes: Vec<String>,
}

impl Written {
    fn of(path: &Path, options: Options) -> Self {
        let code = std::fs::read_to_string(path).expect("probe");
        let out = Pipeline::new(options).run(&code);
        let dir = tempfile::tempdir().expect("temp dir");
        out.write_to(dir.path()).expect("write");
        let notes = out.notes.into_iter().map(|note| note.message).collect();
        Written { dir, notes }
    }

    fn split(&self) -> bool {
        self.dir.path().join("bundle.json").exists()
    }

    fn entry(&self) -> PathBuf {
        match self.split() {
            true => self.dir.path().join("index.js"),
            false => self.dir.path().join("deobfuscated.js"),
        }
    }

    fn tree(&self) -> String {
        std::fs::read_dir(self.dir.path())
            .expect("tree")
            .filter_map(Result::ok)
            .map(|entry| std::fs::read_to_string(entry.path()).unwrap_or_default())
            .collect()
    }

    fn refused(&self) -> bool {
        self.notes.iter().any(|note| note.contains(MODULE_OBJECT))
    }
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
    }
    found
}

fn printed(node: &Path, file: &Path) -> String {
    let observed = run(node, file).expect("spawn").expect("terminates");
    format!("{}{}", observed.stdout, observed.stderr)
}

fn same_output(node: Option<&Path>, probe: &Path, written: &Written, row: &str) {
    if let Some(node) = node {
        assert_eq!(
            printed(node, &written.entry()),
            printed(node, probe),
            "{row}"
        );
    }
}

#[test]
fn a_polyfilled_module_read_past_exports_is_written_with_webpacks_record_under_unsafe() {
    let node = node_or_skip();
    let rows = probes('r');
    assert_eq!(rows.len(), 3);
    for probe in rows {
        let name = probe
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        let written = Written::of(&probe, lane(true));
        assert!(written.split(), "{name} --unsafe: {:?}", written.notes);
        assert!(written.tree().contains(RECORD_MARK), "{name}: no record");
        same_output(
            node.as_deref(),
            &probe,
            &written,
            &format!("{name} --unsafe"),
        );
    }
}

#[test]
fn the_default_lane_keeps_a_bundle_whose_polyfilled_module_is_read_past_exports_whole() {
    let node = node_or_skip();
    for probe in probes('r') {
        let name = probe
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        let written = Written::of(&probe, lane(false));
        assert!(!written.split(), "{name}");
        assert!(written.refused(), "{name}: {:?}", written.notes);
        same_output(node.as_deref(), &probe, &written, &name);
    }
}

#[test]
fn a_polyfilled_module_that_escapes_or_is_introspected_keeps_the_bundle_whole_in_both_lanes() {
    let node = node_or_skip();
    let rows = probes('n');
    assert_eq!(rows.len(), 5);
    for probe in rows {
        let name = probe
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        for unsafe_transforms in [false, true] {
            let row = format!("{name} unsafe={unsafe_transforms}");
            let written = Written::of(&probe, lane(unsafe_transforms));
            assert!(!written.split(), "{row}");
            assert!(written.refused(), "{row}: {:?}", written.notes);
            same_output(node.as_deref(), &probe, &written, &row);
        }
    }
}
