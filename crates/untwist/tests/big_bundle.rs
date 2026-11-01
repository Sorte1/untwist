use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use untwist::{Options, Pipeline};

const BUDGET: Duration = Duration::from_secs(1);

fn fixture() -> Option<PathBuf> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated/webpack-5mb.js");
    path.is_file().then_some(path)
}

fn required() -> bool {
    std::env::var("UNTWIST_REQUIRE_BIG_BUNDLE").is_ok_and(|value| !value.is_empty())
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 5 MB bundle; run with cargo test --release -p untwist --test big_bundle"
)]
fn a_five_megabyte_webpack_bundle_unpacks_inside_the_spec_budget() {
    let Some(path) = fixture() else {
        assert!(
            !required(),
            "UNTWIST_REQUIRE_BIG_BUNDLE is set but fixtures/generated/webpack-5mb.js is missing; \
             run cargo run -p xtask -- corpus-bundle"
        );
        eprintln!("skipping: run `cargo run -p xtask -- corpus-bundle` first");
        return;
    };
    let source = std::fs::read_to_string(&path).expect("read the generated bundle");
    assert!(
        source.len() >= 5_000_000,
        "the fixture shrank: {}",
        source.len()
    );
    let options = Options {
        deobfuscate: false,
        unminify: false,
        jsx: false,
        ..Options::default()
    };
    let started = Instant::now();
    let out = Pipeline::new(options).run(&source);
    let elapsed = started.elapsed();
    let bundle = out.bundle.expect("a webpack bundle was detected");
    assert_eq!(bundle.kind, "webpack");
    assert!(bundle.modules.len() >= 1_000, "{}", bundle.modules.len());
    if cfg!(not(debug_assertions)) {
        assert!(
            elapsed <= BUDGET,
            "the budget is 1 s for a 5 MB bundle; this run took {elapsed:?}"
        );
    }
}
