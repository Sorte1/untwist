use untwist::{Options, Pipeline};
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");

fn label_like(label: &str) -> bool {
    !label.is_empty()
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn rows() -> Vec<(String, String)> {
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

fn transformed(points_to: bool, source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        disabled_passes: if points_to {
            vec![BUILTIN_ALIAS_BITS]
        } else {
            Vec::new()
        },
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn the_points_to_judgement_refuses_every_alias_row_the_bits_proof_refuses() {
    for (label, source) in rows() {
        if !label.starts_with("builtin_alias_rows/ref-") {
            continue;
        }
        let code = transformed(true, &source);
        assert!(
            code.contains("arr0"),
            "{label} decoded under points-to:\n{code}"
        );
    }
}

#[test]
fn rows_the_points_to_judgement_changes_still_run_like_node() {
    let all = rows();
    assert!(all.len() > 200, "found {} rows", all.len());
    let changed: Vec<(String, String)> = all
        .into_iter()
        .filter(|(_, source)| transformed(false, source) != transformed(true, source))
        .collect();
    eprintln!(
        "rows whose output changes under points-to: {}",
        changed.len()
    );
    for (label, _) in &changed {
        eprintln!("  {label}");
    }
    let Some(node) = node_or_skip() else {
        return;
    };
    let run = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 1] = [("points-to", &run)];
    let wrong = node_rows::mismatches(&node, &changed, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
