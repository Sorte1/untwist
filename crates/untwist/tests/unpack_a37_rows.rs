use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const CALLER_PREMISE: &str =
    "--unsafe assumes no .caller/.arguments/getFunction() read reaches a module function";

const PREMISES: [(&str, &str); 27] = [
    (
        "unpack-a37/p2.js --unsafe --rename smart",
        "a method installed by every use and called with a function receiver: the documented --unsafe self-read premise",
    ),
    ("unpack-a37/r4.js --unsafe", CALLER_PREMISE),
    ("unpack-a37/r7.js --unsafe", CALLER_PREMISE),
    ("unpack-a37/r8.js --unsafe", CALLER_PREMISE),
    ("unpack-a37/k1.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/es0.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/es2.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/es3.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g0.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g1.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g2.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g3.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g4.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g5.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g6.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g7.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h1.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h10.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h11.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h12.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h1b.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h2.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h20.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h22.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h6.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h7.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr2/h9.js --unsafe", CALLER_PREMISE),
];

const SPLITS: [&str; 13] = [
    "unpack-a37rr2/sf1.js",
    "unpack-a37rr2/sf1.js --unsafe",
    "unpack-a37rr2/sf2.js",
    "unpack-a37rr2/sf2.js --unsafe",
    "unpack-a37rr2/sf3.js",
    "unpack-a37rr2/sf3.js --unsafe",
    "unpack-a37rr2/sf4.js",
    "unpack-a37rr2/sf4.js --unsafe",
    "unpack-a37rr2/c1.js --unsafe",
    "unpack-a37rr2/c2.js --unsafe",
    "unpack-a37rr2/f7-esbuild-ok.js --unsafe",
    "unpack-a37rr2 lib4.js mi3.js",
    "unpack-a37rr2 lib4.js mi3.js --unsafe",
];

const LINKED: [(&str, &str, &str); 4] = [
    ("unpack-a37rr", "lib1.js", "mi1.js"),
    ("unpack-a37rr2", "lib2.js", "mi1.js"),
    ("unpack-a37rr2", "lib4.js", "mi2.js"),
    ("unpack-a37rr2", "lib4.js", "mi3.js"),
];

fn linked(path: &Path) -> bool {
    LINKED.iter().any(|(dir, library, main)| {
        path.parent().is_some_and(|parent| parent.ends_with(dir))
            && path
                .file_name()
                .is_some_and(|name| name == *library || name == *main)
    })
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn probes() -> Vec<PathBuf> {
    let root = fixtures();
    let mut found = Vec::new();
    for dir in [
        root.join("unpack-a37"),
        root.join("unpack-a37").join("m6"),
        root.join("unpack-a37rr"),
        root.join("unpack-a37rr2"),
    ] {
        for entry in std::fs::read_dir(dir)
            .expect("probes")
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !linked(&path)
                && path
                    .extension()
                    .is_some_and(|ext| ext == "js" || ext == "mjs")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn lanes(path: &Path) -> [(&'static str, Options); 2] {
    let renamed = path.extension().is_some_and(|ext| ext == "mjs")
        || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('p'))
            && !path
                .parent()
                .is_some_and(|dir| dir.ends_with("unpack-a37rr") || dir.ends_with("unpack-a37rr2"));
    let options = |unsafe_transforms: bool| Options {
        unsafe_transforms,
        rename: if renamed {
            RenameMode::Smart
        } else {
            RenameMode::None
        },
        ..Options::default()
    };
    match renamed {
        true => [
            ("--rename smart", options(false)),
            ("--unsafe --rename smart", options(true)),
        ],
        false => [("", options(false)), ("--unsafe", options(true))],
    }
}

fn printed(node: &Path, file: &Path) -> String {
    let observed = run(node, file).expect("spawn").expect("terminates");
    observed.stdout
}

fn written(path: &Path, options: Options, dir: &Path) -> PathBuf {
    let code = std::fs::read_to_string(path).expect("probe");
    let out = Pipeline::new(options).run(&code);
    if path.extension().is_some_and(|ext| ext == "mjs") {
        let file = dir.join(path.file_name().expect("name"));
        std::fs::write(&file, out.code()).expect("write");
        return file;
    }
    out.write_to(dir).expect("write");
    match dir.join("index.js").exists() {
        true => dir.join("index.js"),
        false => dir.join("deobfuscated.js"),
    }
}

#[test]
fn every_review_probe_prints_the_same_after_untwist_in_both_lanes() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let mut wrong = Vec::new();
    let mut rows = 0;
    for path in probes() {
        let expected = printed(&node, &path);
        let name = path
            .strip_prefix(fixtures())
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        for (flags, options) in lanes(&path) {
            let row = format!("{name} {flags}").trim().to_string();
            let dir = tempfile::tempdir().expect("temp dir");
            let entry = written(&path, options, dir.path());
            let got = printed(&node, &entry);
            let premise = PREMISES.iter().any(|(at, _)| *at == row);
            if SPLITS.contains(&row.as_str()) && !dir.path().join("bundle.json").exists() {
                wrong.push(format!("{row}: expected a written tree"));
            }
            rows += 1;
            match (got == expected, premise) {
                (true, true) => wrong.push(format!("{row}: now agrees, drop it from PREMISES")),
                (false, false) => wrong.push(format!(
                    "{row}: node printed {expected:?}, untwist's output printed {got:?}"
                )),
                _ => {}
            }
        }
    }
    assert!(rows >= 290, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const REALM: &str = r#"const vm = require("vm"), fs = require("fs");
const [library, main] = process.argv.slice(2);
vm.runInThisContext(fs.readFileSync(library, "utf8"), { filename: library });
if (main.endsWith("index.js")) require(main);
else vm.runInThisContext(fs.readFileSync(main, "utf8"), { filename: main });
"#;

fn in_one_realm(node: &Path, runner: &Path, library: &Path, main: &Path) -> (String, String) {
    let observed =
        untwist_testing::node::run_with(node, &[runner.as_os_str(), library.as_os_str()], main)
            .expect("spawn")
            .expect("terminates");
    (observed.stdout, observed.stderr)
}

const LINKED_PREMISES: [&str; 2] = [
    "unpack-a37rr lib1.js mi1.js --unsafe",
    "unpack-a37rr2 lib2.js mi1.js --unsafe",
];

fn realm_inputs(tree: &Path, library: &Path) -> (PathBuf, PathBuf) {
    match tree.join("index.js").exists() {
        true => {
            let written = tree.join("deobfuscated.input0.js");
            let library = match written.exists() {
                true => written,
                false => library.to_path_buf(),
            };
            (library, tree.join("index.js"))
        }
        false => (
            tree.join("deobfuscated.input0.js"),
            tree.join("deobfuscated.input1.js"),
        ),
    }
}

#[test]
fn a_frame_read_in_another_input_prints_the_same_in_one_realm_in_both_lanes() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let dir = tempfile::tempdir().expect("temp dir");
    let runner = dir.path().join("realm.js");
    std::fs::write(&runner, REALM).expect("runner");
    let mut wrong = Vec::new();
    for (at, library, main) in LINKED {
        let root = fixtures().join(at);
        let (library, main) = (root.join(library), root.join(main));
        let (expected, _) = in_one_realm(&node, &runner, &library, &main);
        let inputs = [library.as_path(), main.as_path()]
            .map(|path| std::fs::read_to_string(path).expect("probe"));
        for (flags, unsafe_transforms) in [("", false), ("--unsafe", true)] {
            let row = format!("{at} {} {} {flags}", file(&library), file(&main))
                .trim()
                .to_string();
            let options = Options {
                unsafe_transforms,
                ..Options::default()
            };
            let out = Pipeline::new(options).run_many(&[inputs[0].as_str(), inputs[1].as_str()]);
            let tree = dir.path().join(row.replace(' ', "_"));
            out.write_to(&tree).expect("write");
            if SPLITS.contains(&row.as_str()) && !tree.join("bundle.json").exists() {
                wrong.push(format!("{row}: expected a written tree {:?}", out.notes));
            }
            let (first, second) = realm_inputs(&tree, &library);
            let (got, errors) = in_one_realm(&node, &runner, &first, &second);
            let premise = LINKED_PREMISES.contains(&row.as_str());
            match (got == expected, premise) {
                (true, true) => {
                    wrong.push(format!("{row}: now agrees, drop it from LINKED_PREMISES"))
                }
                (false, false) => wrong.push(format!(
                    "{row}: node printed {expected:?}, untwist's output printed {got:?} {errors}"
                )),
                _ => {}
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn file(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}
