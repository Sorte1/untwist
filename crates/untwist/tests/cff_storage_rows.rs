use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/cff_storage_rows.txt");

struct Row<'r> {
    label: &'r str,
    absent: Option<&'r str>,
    program: &'r str,
}

fn rows() -> Vec<Row<'static>> {
    ROWS.lines()
        .filter_map(|line| {
            let (label, rest) = line.split_once('|')?;
            let (absent, program) = rest.split_once('|')?;
            Some(Row {
                label,
                absent: (absent != "-").then_some(absent),
                program,
            })
        })
        .collect()
}

fn run(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_storage_row_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .iter()
        .map(|row| (row.label.to_string(), row.program.to_string()))
        .collect();
    assert!(programs.len() >= 11, "found {} rows", programs.len());
    let default = |source: &str| run(false, source);
    let unsafe_lane = |source: &str| run(true, source);
    let modes: [Mode<'_>; 2] = [("default", &default), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_decided_storage_object_is_inlined_in_both_lanes() {
    let mut kept = Vec::new();
    for row in rows() {
        let Some(absent) = row.absent else {
            continue;
        };
        for (lane, unsafe_transforms) in [("default", false), ("unsafe", true)] {
            let code = run(unsafe_transforms, row.program);
            if code.contains(absent) {
                kept.push(format!("{} ({lane}) kept {absent}:\n{code}", row.label));
            }
        }
    }
    assert!(kept.is_empty(), "{}", kept.join("\n"));
}
