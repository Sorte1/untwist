use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/layer_conversion_rows.txt");

const PEELED: &str = "peeled whole-program layer";

struct Row {
    label: String,
    peel: bool,
    program: String,
}

fn rows() -> Vec<Row> {
    ROWS.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '|');
            let label = fields.next()?.to_string();
            let peel = fields.next()? == "peel";
            let program = fields.next()?.to_string();
            Some(Row {
                label,
                peel,
                program,
            })
        })
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn peeled(options: &Options, source: &str) -> bool {
    let output = Pipeline::new(options.clone()).run(source);
    output
        .notes
        .iter()
        .any(|note| note.message.contains(PEELED))
}

#[test]
fn whole_program_layers_fold_conversions_and_constructor_chains_only_where_the_rows_say() {
    let all = rows();
    assert!(all.len() >= 30, "found {} rows", all.len());
    let mut wrong = Vec::new();
    for unsafe_transforms in [false, true] {
        let options = options(unsafe_transforms);
        for row in &all {
            if peeled(&options, &row.program) != row.peel {
                wrong.push(format!(
                    "{} (unsafe {unsafe_transforms}): expected peel {}",
                    row.label, row.peel
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn whole_program_layers_with_conversions_and_constructor_chains_run_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .map(|row| (row.label, row.program))
        .collect();
    let safe = options(false);
    let unsafe_lane = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_lane, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
