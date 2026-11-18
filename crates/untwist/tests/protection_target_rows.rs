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

const LAYERED_SERVICE_WORKER: &str = include_str!("data/protection_target_pw007.rows");

#[test]
fn layered_service_worker_blocks_are_kept_together_in_the_default_lane() {
    let [(_, default_lane), (_, unsafe_lane)] = lanes();
    let kept = Pipeline::new(default_lane).run(LAYERED_SERVICE_WORKER);
    assert!(
        kept.notes
            .iter()
            .any(|note| note.message == untwist_deobfuscate::notes::PROTECTIONS_KEPT_TOGETHER),
        "{:?}",
        kept.notes
    );
    assert!(kept.code().contains("typeof global"), "{}", kept.code());
    let decoded = Pipeline::new(unsafe_lane).run(LAYERED_SERVICE_WORKER);
    assert!(
        !decoded.code().contains("typeof global"),
        "{}",
        decoded.code()
    );
}
