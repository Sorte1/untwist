use std::path::PathBuf;

use untwist::{Options, Pipeline};

const DIR: &str = "fixtures/bla";

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DIR)
}

fn lane(source: &str, unsafe_transforms: bool) -> String {
    Pipeline::new(Options {
        unsafe_transforms,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

fn predicates(code: &str) -> usize {
    code.match_indices("(Math.imul(")
        .filter(|(at, needle)| {
            let rest = &code[at + needle.len()..];
            let name = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                .unwrap_or(rest.len());
            name > 0 && (rest[name..].starts_with(" | ") || rest[name..].starts_with(" ^ "))
        })
        .count()
}

fn expected() -> Vec<(String, usize, usize, usize)> {
    let text = std::fs::read_to_string(dir().join("EXPECT")).expect("fixtures/bla/EXPECT");
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [file, input, safe, unsafe_lane] = fields.as_slice() else {
                panic!("EXPECT line {line:?} needs four fields");
            };
            let count = |field: &str| field.parse().expect("a count");
            (
                file.to_string(),
                count(input),
                count(safe),
                count(unsafe_lane),
            )
        })
        .collect()
}

#[test]
fn every_bla_sample_is_listed_in_expect() {
    let mut listed: Vec<String> = expected().into_iter().map(|row| row.0).collect();
    listed.sort();
    let mut present: Vec<String> = std::fs::read_dir(dir())
        .expect("fixtures/bla")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "cjs"))
        .filter_map(|path| Some(path.file_name()?.to_str()?.to_string()))
        .collect();
    present.sort();
    assert_eq!(listed, present);
}

#[test]
fn opaque_predicates_left_in_each_lane_match_expect() {
    let mut wrong = Vec::new();
    for (file, input, safe, unsafe_lane) in expected() {
        let source = std::fs::read_to_string(dir().join(&file)).expect("a sample");
        let found = (
            predicates(&source),
            predicates(&lane(&source, false)),
            predicates(&lane(&source, true)),
        );
        if found != (input, safe, unsafe_lane) {
            wrong.push(format!(
                "{file}: expected {input} {safe} {unsafe_lane}, found {} {} {}",
                found.0, found.1, found.2
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
