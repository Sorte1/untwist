use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Output, Pipeline};
use untwist_core::pass::{full_rounds_for_tests, Note};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const CHAINED_WRAPPERS: &[&str] = &[
    "fixtures/obfuscator-versions/2.15.6--b64-rot-noshuf-noshift-inum-w3f-t75--iife--uglify--mixed-1.js",
    "fixtures/obfuscator-versions/2.18.1--b64-norot-noshuf-noshift-inum-w2f5-t75-cff--wp4-module--uglify--strings-1.js",
    "fixtures/obfuscator-versions/2.14.0--rc4-norot-noshuf-noshift-istr-w2f5-t1-mangled--iife--none--strings-1.js",
];

type StageRow = (&'static str, u32, u32, u32, Vec<(&'static str, u32)>);

#[derive(Debug, PartialEq)]
struct Observed {
    codes: Vec<String>,
    notes: Vec<Note>,
    stages: Vec<StageRow>,
}

fn observed(out: &Output) -> Observed {
    Observed {
        codes: out.codes.clone(),
        notes: out.notes.clone(),
        stages: out
            .stages
            .iter()
            .map(|stage| {
                (
                    stage.stage,
                    stage.round,
                    stage.iterations,
                    stage.changes,
                    stage.per_pass.clone(),
                )
            })
            .collect(),
    }
}

fn deobfuscate_runs(out: &Output) -> usize {
    out.stages
        .iter()
        .filter(|stage| stage.stage == "deobfuscate")
        .map(|stage| stage.iteration_elapsed.len())
        .sum()
}

fn run(source: &str, unsafe_transforms: bool, full: bool) -> Output {
    full_rounds_for_tests(full);
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    let out = Pipeline::new(options).run(source);
    full_rounds_for_tests(false);
    out
}

fn fixture(path: &str) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    std::fs::read_to_string(format!("{root}/{path}")).unwrap_or_else(|err| panic!("{path}: {err}"))
}

#[test]
fn chained_wrapper_rounds_skip_idle_stages_and_end_as_full_rounds_do() {
    let _serial = serial();
    for path in CHAINED_WRAPPERS {
        let source = fixture(path);
        for unsafe_transforms in [false, true] {
            let shortened = run(&source, unsafe_transforms, false);
            let full = run(&source, unsafe_transforms, true);
            assert!(
                observed(&shortened) == observed(&full),
                "{path} unsafe={unsafe_transforms}"
            );
            let (runs, full_runs) = (deobfuscate_runs(&shortened), deobfuscate_runs(&full));
            let saved = if unsafe_transforms {
                runs < full_runs
            } else {
                runs <= full_runs
            };
            assert!(
                saved,
                "{path} unsafe={unsafe_transforms}: {runs} vs {full_runs}"
            );
        }
    }
}
