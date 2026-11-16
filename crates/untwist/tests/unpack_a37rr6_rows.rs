use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, ANNEX_B_UNMODELLED, PARSE_UNSUPPORTED, SYNTAX_ERROR_REFUSED};
use untwist_testing::node::{node, run, skip_allowed, Observed, SKIP_MESSAGE};

const EXTENSIONS: [&str; 3] = ["js", "cjs", "mjs"];

const WORKERS: usize = 16;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-a37rr6")
}

fn probes() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("probes")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| EXTENSIONS.contains(&ext))
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

fn options(name: &str, unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        input_names: vec![name.to_string()],
        ..Options::default()
    }
}

struct Seen {
    printed: (String, bool),
    rejected: bool,
}

fn located_in(stderr: &str, file: &Path) -> bool {
    let path = file.display().to_string();
    stderr.lines().any(|line| {
        let line = line.strip_prefix("file://").unwrap_or(line);
        line.strip_prefix(path.as_str())
            .and_then(|rest| rest.strip_prefix(':'))
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    })
}

fn seen(node: &Path, file: &Path) -> Seen {
    let observed: Observed = run(node, file).expect("spawn").expect("terminates");
    let rejected = observed.stderr.contains("SyntaxError")
        && observed.stdout.is_empty()
        && located_in(&observed.stderr, file);
    Seen {
        printed: (observed.stdout, observed.exit == Some(0)),
        rejected,
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Label {
    Processed,
    Syntax,
    Unsupported,
    AnnexB,
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

fn parallel<T: Sync>(items: &[T], check: impl Fn(&T, &Path) -> Vec<String> + Sync) -> Vec<String> {
    let chunk = items.len().div_ceil(WORKERS).max(1);
    std::thread::scope(|scope| {
        let workers: Vec<_> = items
            .chunks(chunk)
            .map(|part| {
                let check = &check;
                scope.spawn(move || {
                    let dir = tempfile::tempdir().expect("temp dir");
                    part.iter()
                        .flat_map(|item| check(item, dir.path()))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker"))
            .collect()
    })
}

fn verdict(row: &str, rejected: bool, label: Label) -> Option<String> {
    match (rejected, label) {
        (true, Label::Syntax) | (false, Label::Processed | Label::Unsupported | Label::AnnexB) => {
            None
        }
        (true, label) | (false, label @ Label::Syntax) => Some(format!(
            "{row}: node rejects it: {rejected}, untwist: {label:?}"
        )),
    }
}

fn probe_rows(node: &Path, path: &PathBuf, scratch: &Path) -> Vec<String> {
    let expected = seen(node, path);
    let name = file_name(path);
    let source = std::fs::read_to_string(path).expect("probe");
    let mut wrong = Vec::new();
    for lane in [false, true] {
        let row = format!("{name}{}", if lane { " --unsafe" } else { "" });
        let out = Pipeline::new(options(&name, lane)).run(&source);
        let label = label_of(&format!("{:?}", out.notes), out.parse_errors);
        wrong.extend(verdict(&row, expected.rejected, label));
        if label != Label::Processed && out.code() != source {
            wrong.push(format!("{row}: a refused input is not passed through"));
        }
        let dir = scratch.join(format!("{name}-{lane}"));
        let report = out
            .write_to_with(&dir, std::slice::from_ref(path), true)
            .expect("write");
        let entry = match dir.join("index.js").exists() {
            true => dir.join("index.js"),
            false => report.first.clone().expect("written"),
        };
        let got = seen(node, &entry);
        if got.printed != expected.printed || got.rejected != expected.rejected {
            wrong.push(format!(
                "{row}: node gave {:?}, the output gave {:?}",
                expected.printed, got.printed
            ));
        }
    }
    wrong
}

#[test]
fn every_a37rr6_probe_file_prints_and_exits_the_same_after_untwist_in_both_lanes() {
    let Some(node) = require_node() else {
        return;
    };
    let all = probes();
    assert!(all.len() >= 200, "{}", all.len());
    let wrong = parallel(&all, |path, dir| probe_rows(&node, path, dir));
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

fn snippet_seen(node: &Path, file: &Path) -> Option<Seen> {
    let observed: Observed = run(node, file).expect("spawn")?;
    let rejected = observed.stderr.contains("SyntaxError")
        && !observed.stdout.lines().any(|line| line == "ran");
    Some(Seen {
        printed: (observed.stdout, observed.exit == Some(0)),
        rejected,
    })
}

fn snippet_rows(node: &Path, snippet: &Snippet, dir: &Path) -> Vec<String> {
    let file = format!("input.{}", snippet.ext);
    let input = dir.join(&file);
    std::fs::write(&input, &snippet.source).expect("write");
    let expected = snippet_seen(node, &input);
    let mut wrong = Vec::new();
    let Some(expected) = expected else {
        for lane in [false, true] {
            let out = Pipeline::new(options(&file, lane)).run(&snippet.source);
            let label = label_of(&format!("{:?}", out.notes), out.parse_errors);
            if label == Label::Syntax {
                wrong.push(format!("{}: node runs it, untwist: {label:?}", snippet.name));
            }
        }
        return wrong;
    };
    for lane in [false, true] {
        let row = format!("{}{}", snippet.name, if lane { " --unsafe" } else { "" });
        let out = Pipeline::new(options(&file, lane)).run(&snippet.source);
        let label = label_of(&format!("{:?}", out.notes), out.parse_errors);
        if label != Label::Processed && out.code() != snippet.source {
            wrong.push(format!("{row}: a refused input is not passed through"));
        }
        if let Some(problem) = verdict(&row, expected.rejected, label) {
            wrong.push(problem);
            continue;
        }
        if label != Label::Processed {
            continue;
        }
        let output = dir.join(format!("output.{}", snippet.ext));
        std::fs::write(&output, out.code()).expect("write");
        let Some(got) = snippet_seen(node, &output) else {
            wrong.push(format!("{row}: the output does not terminate"));
            continue;
        };
        let same = match expected.rejected {
            true => got.rejected,
            false => got.printed == expected.printed,
        };
        if !same {
            wrong.push(format!(
                "{row}: node gave {:?}, the output gave {:?}",
                expected.printed, got.printed
            ));
        }
    }
    wrong
}

#[test]
fn every_a37rr6_snippet_is_refused_exactly_when_node_rejects_it_in_both_lanes() {
    let Some(node) = require_node() else {
        return;
    };
    let all = snippets();
    assert!(all.len() >= 6000, "{}", all.len());
    let wrong = parallel(&all, |snippet, dir| snippet_rows(&node, snippet, dir));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
