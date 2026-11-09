use untwist::{Options, Pipeline};

const VERSIONS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/obfuscator-versions"
);
const TARGET: &str =
    "4.2.2--plain-rot-noshuf-noshift-istr-w2f5-calls-t1-self--iife--terser--objects-1.js";
const BEFORE: [&str; 3] = [
    "0.10.2--b64-rot-t75-self--wp4-bundle--uglify--greeting-1.js",
    "2.13.0--b64-norot-shuf-shift-inum-w3f-t1-split--iife--uglify--control-1.js",
    "2.15.6--b64-norot-shuf-shift-istr-w2u-t1--iife--terser--classes-1.js",
];

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{VERSIONS}/{name}")).expect("fixture")
}

fn run(source: &str, unsafe_transforms: bool) -> String {
    Pipeline::new(Options {
        unsafe_transforms,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

#[test]
fn a_program_runs_the_same_after_another_program_on_the_same_thread() {
    let target = fixture(TARGET);
    let alone = run(&target, false);
    for before in BEFORE {
        let source = fixture(before);
        for lane in [false, true] {
            let _ = run(&source, lane);
            assert_eq!(
                run(&target, false),
                alone,
                "{TARGET} changed after {before} (unsafe={lane}) ran on the same thread"
            );
        }
    }
}
