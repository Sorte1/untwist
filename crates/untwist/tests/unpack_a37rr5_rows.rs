use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, ANNEX_B_UNMODELLED, PARSE_UNSUPPORTED, SYNTAX_ERROR_REFUSED};
use untwist_testing::node::{node, run, skip_allowed, Observed, SKIP_MESSAGE};

const SYNTAX_ERRORS: [&str; 9] = ["c1", "c2", "c3", "f4a", "f4b", "f4d", "f4e", "f4f", "f4g"];

const UNSUPPORTED: [&str; 2] = ["f3a", "f3b"];

const PREMISES: [&str; 3] = ["h1", "h1 --unsafe", "g-d30 --unsafe"];

const LINKED_OR_RUN: [&str; 4] = [
    "import { if as iff } from \"node:fs\";",
    "import { arguments as b } from \"node:fs\";",
    "export { if } from \"node:fs\";",
    "new RegExp(\"(\");",
];

const DETECTED_LATER: [&str; 3] = ["l10:12", "l10:14", "l10:28"];

const WORKERS: usize = 8;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-a37rr5")
}

fn probes() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("probes")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "js" || ext == "cjs")
        })
        .collect();
    found.sort();
    found
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn options(name: &str, unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        input_names: vec![name.to_string()],
        ..Options::default()
    }
}

fn observed(node: &Path, file: &Path) -> (String, bool) {
    let observed: Observed = run(node, file).expect("spawn").expect("terminates");
    (observed.stdout, observed.exit == Some(0))
}

fn rejected(node: &Path, file: &Path) -> bool {
    let observed: Observed = run(node, file).expect("spawn").expect("terminates");
    observed.stderr.contains("SyntaxError") && !observed.stdout.lines().any(|line| line == "ran")
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Label {
    Processed,
    Syntax,
    Unsupported,
    AnnexB,
}

struct Ran {
    label: Label,
    code: String,
}

fn label_of(notes: &str, parse_errors: usize) -> Label {
    if parse_errors > 0 && notes.contains(SYNTAX_ERROR_REFUSED) {
        return Label::Syntax;
    }
    if parse_errors > 0 && notes.contains(PARSE_UNSUPPORTED) {
        return Label::Unsupported;
    }
    if notes.contains(ANNEX_B_UNMODELLED) {
        return Label::AnnexB;
    }
    Label::Processed
}

fn ran(source: &str, options: Options) -> Ran {
    let out = Pipeline::new(options).run(source);
    let notes = format!("{:?}", out.notes);
    Ran {
        label: label_of(&notes, out.parse_errors),
        code: out.code().to_string(),
    }
}

fn written_entry(path: &Path, options: Options, dir: &Path) -> (PathBuf, Ran) {
    let source = std::fs::read_to_string(path).expect("probe");
    let out = Pipeline::new(options).run(&source);
    let notes = format!("{:?}", out.notes);
    let result = Ran {
        label: label_of(&notes, out.parse_errors),
        code: out.code().to_string(),
    };
    if path.extension().is_some_and(|ext| ext == "cjs") {
        let entry = dir.join(file_name(path));
        std::fs::write(&entry, &result.code).expect("write");
        return (entry, result);
    }
    out.write_to(dir).expect("write");
    let entry = match dir.join("index.js").exists() {
        true => dir.join("index.js"),
        false => dir.join("deobfuscated.js"),
    };
    (entry, result)
}

fn expected_label(name: &str) -> Label {
    match (SYNTAX_ERRORS.contains(&name), UNSUPPORTED.contains(&name)) {
        (true, _) => Label::Syntax,
        (_, true) => Label::Unsupported,
        _ => Label::Processed,
    }
}

fn require_node() -> Option<PathBuf> {
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

#[test]
fn every_a37rr5_probe_file_prints_and_exits_the_same_after_untwist_in_both_lanes() {
    let Some(node) = require_node() else {
        return;
    };
    let mut wrong = Vec::new();
    let mut rows = 0;
    for path in probes() {
        let expected = observed(&node, &path);
        let name = stem(&path);
        let source = std::fs::read_to_string(&path).expect("probe");
        for lane in [false, true] {
            let row = format!("{name}{}", if lane { " --unsafe" } else { "" });
            let dir = tempfile::tempdir().expect("temp dir");
            let (entry, result) =
                written_entry(&path, options(&file_name(&path), lane), dir.path());
            rows += 1;
            if result.label != expected_label(&name) {
                wrong.push(format!("{row}: label {:?}", result.label));
            }
            if result.label != Label::Processed && result.code != source {
                wrong.push(format!("{row}: a refused input is not passed through"));
            }
            let got = observed(&node, &entry);
            if got != expected && !PREMISES.contains(&row.as_str()) {
                wrong.push(format!(
                    "{row}: node gave {expected:?}, the output gave {got:?}"
                ));
            }
        }
    }
    assert!(rows >= 140, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

struct Snippet {
    name: String,
    ext: String,
    source: String,
}

fn snippets() -> Vec<Snippet> {
    let text = std::fs::read_to_string(fixtures().join("snippets.jsonl")).expect("snippets");
    text.lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("json");
            let field = |key: &str| value[key].as_str().expect("field").to_string();
            Snippet {
                name: field("name"),
                ext: field("ext"),
                source: field("source"),
            }
        })
        .collect()
}

fn snippet_rows(node: &Path, snippet: &Snippet, dir: &Path) -> Vec<String> {
    let file = format!("input.{}", snippet.ext);
    let input = dir.join(&file);
    std::fs::write(&input, &snippet.source).expect("write");
    let node_rejects = rejected(node, &input);
    let expected = observed(node, &input);
    let late = LINKED_OR_RUN
        .iter()
        .any(|line| snippet.source.starts_with(line));
    let mut wrong = Vec::new();
    if DETECTED_LATER.contains(&snippet.name.as_str()) {
        return wrong;
    }
    for lane in [false, true] {
        let row = format!("{}{}", snippet.name, if lane { " --unsafe" } else { "" });
        let result = ran(&snippet.source, options(&file, lane));
        let refused = result.label != Label::Processed;
        if refused && result.code != snippet.source {
            wrong.push(format!("{row}: a refused input is not passed through"));
        }
        match (node_rejects, result.label) {
            (true, Label::Syntax) | (false, Label::Unsupported | Label::AnnexB) => continue,
            (true, Label::Processed) if late => {}
            (true, label) | (false, label @ Label::Syntax) => {
                wrong.push(format!(
                    "{row}: node rejects it: {node_rejects}, untwist: {label:?}"
                ));
                continue;
            }
            (false, Label::Processed) => {}
        }
        let output = dir.join(format!("output.{}", snippet.ext));
        std::fs::write(&output, &result.code).expect("write");
        let after_rejects = rejected(node, &output);
        let got = observed(node, &output);
        let same = match node_rejects {
            true => after_rejects,
            false => got == expected,
        };
        if !same {
            wrong.push(format!(
                "{row}: node gave {expected:?}, the output gave {got:?}"
            ));
        }
    }
    wrong
}

#[test]
fn every_a37rr5_snippet_is_refused_exactly_when_node_rejects_it_in_both_lanes() {
    let Some(node) = require_node() else {
        return;
    };
    let all = snippets();
    assert!(all.len() >= 1600, "{}", all.len());
    let chunk = all.len().div_ceil(WORKERS);
    let wrong: Vec<String> = std::thread::scope(|scope| {
        let workers: Vec<_> = all
            .chunks(chunk)
            .map(|part| {
                let node = node.clone();
                scope.spawn(move || {
                    let dir = tempfile::tempdir().expect("temp dir");
                    part.iter()
                        .flat_map(|snippet| snippet_rows(&node, snippet, dir.path()))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker"))
            .collect()
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
