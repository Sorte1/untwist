use std::path::PathBuf;

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/a33rr8");

const MODES: [(&str, Lane); 6] = [
    ("default", Lane::Default),
    ("unsafe", Lane::Unsafe),
    ("rename smart", Lane::Rename(RenameMode::Smart)),
    ("rename mangle", Lane::Rename(RenameMode::Mangle)),
    ("rename simple", Lane::Rename(RenameMode::Simple)),
    ("rename unique", Lane::Rename(RenameMode::Unique)),
];

const UNSAFE_PLACEMENT: [&str; 14] = [
    "705a", "705b", "705e", "725", "726", "728", "745", "746", "748", "775", "783", "786", "788",
    "791",
];

type Run = Box<dyn Fn(&str) -> String>;

#[derive(Clone, Copy)]
enum Lane {
    Default,
    Unsafe,
    Rename(RenameMode),
}

fn options(lane: Lane) -> Options {
    match lane {
        Lane::Default => Options::default(),
        Lane::Unsafe => Options {
            unsafe_transforms: true,
            ..Options::default()
        },
        Lane::Rename(rename) => Options {
            rename,
            ..Options::default()
        },
    }
}

fn transformed(lane: Lane, source: &str) -> String {
    Pipeline::new(options(lane)).run(source).code().to_string()
}

fn probes() -> Vec<(String, String)> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(DATA)
        .expect("probe dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|path| {
            let label = path.file_stem().unwrap().to_string_lossy().to_string();
            (label, std::fs::read_to_string(&path).expect("read probe"))
        })
        .collect()
}

#[test]
fn every_a33_rr8_call_site_probe_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = probes();
    assert!(programs.len() >= 75, "found {} probes", programs.len());
    let lanes: Vec<(&'static str, Run)> = MODES
        .iter()
        .map(|&(name, lane)| {
            let run: Run = Box::new(move |source| transformed(lane, source));
            (name, run)
        })
        .collect();
    let modes: Vec<node_rows::Mode<'_>> = lanes
        .iter()
        .map(|(name, run)| (*name, run.as_ref()))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let mut found: Vec<String> = wrong
        .iter()
        .map(|row| {
            row.split_once(':')
                .map_or(row.as_str(), |(head, _)| head)
                .to_string()
        })
        .collect();
    found.sort();
    let mut expected: Vec<String> = UNSAFE_PLACEMENT
        .iter()
        .map(|label| format!("{label} (unsafe)"))
        .collect();
    expected.sort();
    assert_eq!(found, expected, "{}", wrong.join("\n"));
}

#[test]
fn obfuscator_io_call_sites_are_still_removed_in_every_lane() {
    for label in [
        "700", "703", "715", "716", "731", "733", "764", "766", "784", "743", "787", "789", "790",
    ] {
        let source =
            std::fs::read_to_string(PathBuf::from(DATA).join(format!("{label}.js"))).unwrap();
        for (name, lane) in MODES {
            let code = transformed(lane, &source);
            assert!(!code.contains("apply"), "{label} ({name}): {code}");
        }
    }
}
