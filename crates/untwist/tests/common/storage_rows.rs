use untwist::{Options, Pipeline};

const ROWS: &str = include_str!("../data/storage_call_rows.txt");

pub fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

pub fn transformed(unsafe_transforms: bool, mutant: Option<&'static str>, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: mutant.into_iter().collect(),
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

pub fn labelled(prefix: &str) -> Vec<(String, String)> {
    rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .collect()
}
