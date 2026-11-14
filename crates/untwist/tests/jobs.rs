mod common;
#[path = "common/panic_on_marker.rs"]
mod panic_on_marker;

use std::num::NonZeroUsize;
use std::sync::mpsc;
use std::time::Duration;

use common::{CHUNK_A, CHUNK_B};
use panic_on_marker::panicking;
use untwist::{
    BundleOutput, Note, Options, Output, Pipeline, Plugins, RenameMode, Stage, StageReport,
    StopReason,
};
use untwist_testing::read_fixture;

type ComparableModule = (String, String, bool, String);
type ComparableBundle = (String, Option<String>, Vec<String>, Vec<ComparableModule>);

#[derive(Debug, PartialEq)]
struct ComparableStage {
    stage: &'static str,
    round: u32,
    iterations: u32,
    changes: u32,
    per_pass: Vec<(&'static str, u32)>,
    fixpoint_per_pass: Vec<(&'static str, u32)>,
    stop: StopReason,
}

#[derive(Debug, PartialEq)]
struct Comparable {
    codes: Vec<String>,
    bundle: Option<ComparableBundle>,
    notes: Vec<Note>,
    stages: Vec<ComparableStage>,
    panicked: Vec<usize>,
    parse_diagnostics: Vec<String>,
    parse_errors: usize,
}

fn comparable_bundle(bundle: &BundleOutput) -> ComparableBundle {
    (
        bundle.kind.clone(),
        bundle.entry_id.clone(),
        bundle.chunk_ids.clone(),
        bundle
            .modules
            .iter()
            .map(|module| {
                (
                    module.id.clone(),
                    module.path.clone(),
                    module.is_entry,
                    module.code.clone(),
                )
            })
            .collect(),
    )
}

fn comparable_stage(stage: &StageReport) -> ComparableStage {
    ComparableStage {
        stage: stage.stage,
        round: stage.round,
        iterations: stage.iterations,
        changes: stage.changes,
        per_pass: stage.per_pass.clone(),
        fixpoint_per_pass: stage.fixpoint_per_pass.clone(),
        stop: stage.stop,
    }
}

fn comparable(out: &Output) -> Comparable {
    Comparable {
        codes: out.codes.clone(),
        bundle: out.bundle.as_ref().map(comparable_bundle),
        notes: out.notes.clone(),
        stages: out.stages.iter().map(comparable_stage).collect(),
        panicked: out.panicked.clone(),
        parse_diagnostics: out.parse_diagnostics.clone(),
        parse_errors: out.parse_errors,
    }
}

fn with_jobs(jobs: usize, base: Options) -> Options {
    Options {
        jobs: NonZeroUsize::new(jobs),
        ..base
    }
}

fn assert_jobs_agree(sources: &[&str], base: Options) {
    let sequential = Pipeline::new(with_jobs(1, base.clone())).run_many(sources);
    let parallel = Pipeline::new(with_jobs(4, base)).run_many(sources);
    assert_eq!(comparable(&sequential), comparable(&parallel));
    assert_eq!(sequential.code(), parallel.code());
}

#[test]
fn two_chunk_files_agree_between_one_and_four_jobs() {
    assert_jobs_agree(&[CHUNK_A, CHUNK_B], Options::default());
}

#[test]
fn a_webpack_bundle_and_a_jsonp_chunk_agree_between_one_and_four_jobs() {
    let bundle = read_fixture("unpack", "webpack-5.js");
    let chunk = read_fixture("unpack", "webpack-jsonp-chunk.js");
    assert_jobs_agree(&[bundle.as_str(), chunk.as_str()], Options::default());
}

#[test]
fn unique_renaming_numbers_inputs_in_order_under_four_jobs() {
    let base = Options {
        rename: RenameMode::Unique,
        rename_all: false,
        unpack: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    let sources = [
        "function _0xa() {}",
        "",
        "function _0xb() {} function _0xc() {}",
        "function _0xd() {}",
    ];
    assert_jobs_agree(&sources, base.clone());
    let out = Pipeline::new(with_jobs(4, base)).run_many(&sources);
    assert_eq!(out.codes[0], "function fn1() {}");
    assert_eq!(out.codes[2], "function fn2() {}\nfunction fn3() {}");
    assert_eq!(out.codes[3], "function fn4() {}");
}

const DEADLOCK_GRACE: Duration = Duration::from_secs(30);

fn run_many_within(options: Options, sources: &[&str], grace: Duration) -> Output {
    let owned: Vec<String> = sources.iter().map(|source| source.to_string()).collect();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
        done.send(Pipeline::new(options).run_many(&borrowed)).ok();
    });
    finished
        .recv_timeout(grace)
        .expect("the run did not finish within the grace period: the turnstile deadlocked")
}

#[test]
fn six_unique_inputs_on_two_workers_agree_with_one_and_finish() {
    let base = Options {
        rename: RenameMode::Unique,
        rename_all: false,
        unpack: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    let sources = [
        "function _0xa() {}",
        "function _0xb() {}",
        "function _0xc() {}",
        "function _0xd() {}",
        "function _0xe() {}",
        "function _0xf() {}",
    ];
    let sequential = Pipeline::new(with_jobs(1, base.clone())).run_many(&sources);
    let parallel = run_many_within(with_jobs(2, base), &sources, DEADLOCK_GRACE);
    assert_eq!(comparable(&sequential), comparable(&parallel));
    let expected: Vec<String> = (1..=6)
        .map(|number| format!("function fn{number}() {{}}"))
        .collect();
    assert_eq!(parallel.codes, expected);
}

#[test]
fn a_panicking_input_is_reported_at_the_same_index_under_four_jobs() {
    let mut plugins = Plugins::new();
    plugins.register(Stage::AfterParse, panicking as untwist::PluginCtor);
    let base = Options {
        plugins,
        unpack: false,
        rename: RenameMode::Unique,
        unsafe_transforms: true,
        ..Options::default()
    };
    let sources = [
        "f(\"ok\");",
        "g(\"boom\");",
        "h(\"fine\");",
        "function _0xa() {}",
    ];
    assert_jobs_agree(&sources, base.clone());
    let out = Pipeline::new(with_jobs(4, base)).run_many(&sources);
    assert_eq!(out.panicked, vec![1]);
    assert_eq!(out.codes[3], "function fn1() {}");
}

#[test]
fn one_input_ignores_jobs() {
    let many = Pipeline::new(with_jobs(8, Options::default())).run("var a = !0;");
    let one = Pipeline::new(with_jobs(1, Options::default())).run("var a = !0;");
    assert_eq!(comparable(&many), comparable(&one));
}
