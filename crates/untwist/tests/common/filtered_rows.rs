use untwist::{Options, Pipeline};

const ROWS: &str = include_str!("../data/filtered_enumeration_rows.txt");

pub fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

pub fn options(unsafe_transforms: bool, mutant: Option<&'static str>) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: mutant.into_iter().collect(),
        ..Options::default()
    }
}

pub fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}
