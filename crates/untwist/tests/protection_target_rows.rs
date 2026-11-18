use untwist::{Options, Pipeline};

const ROWS: &str = include_str!("data/protection_target.rows");
const DECODED: &str = "console.log(\"test\");";

fn lanes() -> [(&'static str, Options); 2] {
    [
        ("default", Options::default()),
        (
            "unsafe",
            Options {
                unsafe_transforms: true,
                ..Options::default()
            },
        ),
    ]
}

#[test]
fn every_target_protection_row_decodes_fully_in_both_lanes() {
    let mut wrong = Vec::new();
    for line in ROWS.lines().filter(|line| !line.is_empty()) {
        let (name, source) = line.split_once('|').unwrap();
        for (lane, options) in lanes() {
            let out = Pipeline::new(options).run(source);
            if out.code().trim() != DECODED {
                wrong.push(format!("{name} ({lane}):\n{}", out.code()));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
