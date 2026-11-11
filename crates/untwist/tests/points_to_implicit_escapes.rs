#[path = "common/host_alias.rs"]
mod host_alias;

use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::mutants::keep_escaped_objects_for_tests;
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;
use untwist_testing::node_rows::patched_after_named_rotator;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");
const BUDGET_RAN_OUT: &str = "the points-to analysis gave up (its budget ran out)";
const SUM_KEY_WRITE: &str = "var g = this; function put(k, v) { g[k + 1] = v; } put(String.fromCharCode(98, 81), function () { return 'patched'; });";

fn label_like(label: &str) -> bool {
    !label.is_empty()
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn data_rows() -> Vec<(String, String)> {
    let mut files: Vec<_> = std::fs::read_dir(DATA)
        .expect("row data")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let name = file.file_stem().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&file).unwrap();
        for line in text.lines() {
            if let Some((label, program)) = line.split_once('|') {
                if label_like(label) {
                    found.push((format!("{name}/{label}"), program.to_string()));
                }
            }
        }
    }
    found
}

fn unknown_key_rows() -> Vec<(String, String)> {
    ["Q", "Q1"]
        .iter()
        .map(|suffix| {
            (
                format!("unknown_key/sum-key-{suffix}"),
                patched_after_named_rotator(suffix, "true", SUM_KEY_WRITE),
            )
        })
        .collect()
}

fn all_rows() -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = host_alias::rows()
        .into_iter()
        .chain(host_alias::global_routes())
        .map(|(label, source)| (format!("host_alias/{label}"), source))
        .collect();
    rows.extend(data_rows());
    rows.extend(unknown_key_rows());
    rows
}

fn points_to(source: &str, keep_escaped: bool) -> Output {
    keep_escaped_objects_for_tests(keep_escaped);
    let options = Options {
        unpack: false,
        jsx: false,
        disabled_passes: vec![BUILTIN_ALIAS_BITS],
        ..Options::default()
    };
    let out = Pipeline::new(options).run(source);
    keep_escaped_objects_for_tests(false);
    out
}

fn ran_out(out: &Output) -> bool {
    out.notes
        .iter()
        .any(|note| note.message.contains(BUDGET_RAN_OUT))
}

#[test]
fn escaped_objects_left_implicit_under_world_change_no_answer_where_the_explicit_form_settles() {
    let rows = all_rows();
    assert!(rows.len() > 300, "found {} rows", rows.len());
    let mut settled = 0;
    let mut exhausted = Vec::new();
    let mut wrong = Vec::new();
    for (label, source) in rows {
        let explicit = points_to(&source, true);
        if ran_out(&explicit) {
            exhausted.push(label);
            continue;
        }
        settled += 1;
        let implicit = points_to(&source, false);
        if implicit.code() != explicit.code() {
            wrong.push(format!(
                "{label}:\nexplicit:\n{}\nimplicit:\n{}",
                explicit.code(),
                implicit.code()
            ));
        }
    }
    eprintln!(
        "rows where the explicit form settles: {settled}; where it runs out: {} {exhausted:?}",
        exhausted.len()
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
