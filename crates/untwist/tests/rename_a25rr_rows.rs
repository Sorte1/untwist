use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::{mismatches, node_or_skip, observed, Mode};

const DATA: &str = "tests/data/rename_a25rr";

const RENAMES: [(&str, RenameMode); 5] = [
    ("no rename", RenameMode::None),
    ("mangle", RenameMode::Mangle),
    ("simple", RenameMode::Simple),
    ("unique", RenameMode::Unique),
    ("smart", RenameMode::Smart),
];

const UNSAFE_PREMISE_SINGLE: &str = "q-";

const UNSAFE_PREMISE_MULTI: &[(&str, RenameMode)] = &[("u2", RenameMode::Unique)];

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(DATA)
}

fn options(rename: RenameMode, unsafe_transforms: bool) -> Options {
    Options {
        rename,
        unsafe_transforms,
        unpack: false,
        ..Options::default()
    }
}

fn singles() -> Vec<(String, String)> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(data().join("single"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let label = path.file_name().unwrap().to_string_lossy().to_string();
            (label, std::fs::read_to_string(&path).unwrap())
        })
        .collect()
}

fn single_rows(unsafe_transforms: bool) -> Vec<String> {
    let Some(node) = node_or_skip() else {
        return Vec::new();
    };
    let programs: Vec<(String, String)> = singles()
        .into_iter()
        .filter(|(label, _)| !unsafe_transforms || !label.starts_with(UNSAFE_PREMISE_SINGLE))
        .collect();
    let mut wrong = Vec::new();
    for (label, rename) in RENAMES {
        let transform = move |source: &str| {
            Pipeline::new(options(rename, unsafe_transforms))
                .run(source)
                .code()
                .to_string()
        };
        let modes: [Mode<'_>; 1] = [(label, &transform)];
        wrong.extend(mismatches(&node, &programs, &modes));
    }
    wrong
}

#[test]
fn single_probes_print_the_same_in_node_in_the_default_lane() {
    let wrong = single_rows(false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn single_probes_outside_the_unsafe_premise_print_the_same_under_unsafe() {
    let wrong = single_rows(true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_safe_lane_renames_no_top_level_binding_of_a_classic_script() {
    let source = "var _0x1a = { width: Math.random() }; var _0x2b = _0x1a.width; function _0x3c() { var _0x4d = _0x2b + _0x1a.width; return _0x4d; } console.log(_0x3c(), _0x1a, _0x2b);";
    for (label, rename) in RENAMES {
        let code = Pipeline::new(options(rename, false))
            .run(source)
            .code()
            .to_string();
        for name in ["_0x1a", "_0x2b", "_0x3c"] {
            assert!(code.contains(name), "{label}: {name} renamed in\n{code}");
        }
    }
}

#[test]
fn a_new_top_level_name_is_never_a_key_the_file_uses() {
    let source = "var cfg = { debug: Math.random() < 2 ? 1 : 0 };\nvar q = cfg.debug;\nif (globalThis.debug) { console.log(\"debug on\"); }\nconsole.log(q);\n";
    let code = Pipeline::new(options(RenameMode::Smart, true))
        .run(source)
        .code()
        .to_string();
    assert!(!code.contains("var debug ="), "{code}");
}

struct Case {
    label: String,
    inputs: Vec<(String, String)>,
}

fn cases() -> Vec<Case> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(data().join("multi"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .map(|dir| {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            files.sort();
            Case {
                label: dir.file_name().unwrap().to_string_lossy().to_string(),
                inputs: files
                    .into_iter()
                    .map(|path| {
                        let name = path.file_name().unwrap().to_string_lossy().to_string();
                        (name, std::fs::read_to_string(&path).unwrap())
                    })
                    .collect(),
            }
        })
        .collect()
}

fn runner(names: &[String]) -> String {
    let mut text = String::from(
        "import fs from \"fs\"; import vm from \"vm\"; import { pathToFileURL } from \"url\"; import path from \"path\";\nconst here = path.dirname(new URL(import.meta.url).pathname);\n",
    );
    for name in names {
        let quoted = serde_json::to_string(name).unwrap();
        if name.ends_with(".mjs") {
            text.push_str(&format!(
                "await import(pathToFileURL(path.join(here, {quoted})).href);\n"
            ));
        } else {
            text.push_str(&format!(
                "vm.runInThisContext(fs.readFileSync(path.join(here, {quoted}), \"utf8\"));\n"
            ));
        }
    }
    text
}

fn printed(node: &Path, dir: &Path, files: &[(String, String)]) -> String {
    std::fs::create_dir_all(dir).unwrap();
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    let main = dir.join("run.mjs");
    std::fs::write(&main, runner(&names)).unwrap();
    match run(node, &main).unwrap() {
        Some(out) => observed(&out),
        None => "did not finish".to_string(),
    }
}

fn multi_rows(unsafe_transforms: bool) -> Vec<String> {
    let Some(node) = node_or_skip() else {
        return Vec::new();
    };
    let temp = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for case in cases() {
        let before = printed(
            &node,
            &temp.path().join(&case.label).join("in"),
            &case.inputs,
        );
        let sources: Vec<&str> = case.inputs.iter().map(|(_, text)| text.as_str()).collect();
        for (mode, rename) in RENAMES {
            let premise = UNSAFE_PREMISE_MULTI
                .iter()
                .any(|(label, mode)| *label == case.label && *mode == rename);
            if unsafe_transforms && premise {
                continue;
            }
            let codes = Pipeline::new(options(rename, unsafe_transforms))
                .run_many(&sources)
                .codes;
            let outputs: Vec<(String, String)> = case
                .inputs
                .iter()
                .zip(codes)
                .map(|((name, _), code)| (name.clone(), code))
                .collect();
            let dir = temp.path().join(&case.label).join(mode.replace(' ', "-"));
            let after = printed(&node, &dir, &outputs);
            if after != before {
                wrong.push(format!(
                    "{} ({mode}, unsafe={unsafe_transforms}): node printed {before:?}, the outputs printed {after:?}",
                    case.label
                ));
            }
        }
    }
    wrong
}

#[test]
fn multi_input_probes_print_the_same_in_node_in_the_default_lane() {
    let wrong = multi_rows(false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn multi_input_probes_outside_the_unsafe_premise_print_the_same_under_unsafe() {
    let wrong = multi_rows(true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
