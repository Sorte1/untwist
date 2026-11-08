use std::time::{Duration, Instant};

use untwist_testing::depth_diff::{
    fixture_paths, run_class, run_depth_classes, run_fixtures, run_soup_stream, Class, Tally,
    EDGE_SEED, MIX_SEED, SOUP_SEED,
};

struct Counts {
    soup: usize,
    mix: u64,
    edges: u64,
}

const COUNTS: Counts = if cfg!(debug_assertions) {
    Counts {
        soup: 2_000,
        mix: 300,
        edges: 10_000,
    }
} else {
    Counts {
        soup: 50_000,
        mix: 10_000,
        edges: 200_000,
    }
};

const BUDGET: Duration = if cfg!(debug_assertions) {
    Duration::from_secs(180)
} else {
    Duration::from_secs(60)
};

fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, |count| count.get().min(8))
}

fn record(name: &str, tally: Tally, total: &mut Tally) {
    println!("{name}: {}", tally.summary());
    for example in &tally.examples {
        println!("{example}");
    }
    assert!(tally.programs > 0, "{name} compared nothing");
    total.merge(tally);
}

#[test]
fn the_scan_matches_the_reference_scanner_on_every_field() {
    let started = Instant::now();
    let mut total = Tally::default();
    record("soup", run_soup_stream(SOUP_SEED, COUNTS.soup), &mut total);
    for (name, class, seed, count) in [
        ("multi", Class::Multi, MIX_SEED, COUNTS.mix),
        ("grammar", Class::Grammar, MIX_SEED, COUNTS.mix),
        ("mutate", Class::Mutate, MIX_SEED, COUNTS.mix),
        ("edges", Class::Edges, EDGE_SEED, COUNTS.edges),
    ] {
        record(
            name,
            run_class(class, seed, (0, count), threads()),
            &mut total,
        );
    }
    record("classes", run_depth_classes(), &mut total);
    let paths = fixture_paths();
    let (fixtures, unreadable) = run_fixtures(&paths);
    assert_eq!(unreadable, 0);
    if untwist_testing::real_world_path("game.js").is_some() {
        assert!(paths
            .iter()
            .any(|path| path.ends_with("real-world/game.js")));
    }
    if untwist_testing::generated_bundle_present() {
        assert!(paths
            .iter()
            .any(|path| path.ends_with("generated/webpack-5mb.js")));
    }
    record("fixtures", fixtures, &mut total);
    let elapsed = started.elapsed();
    println!("total: {} in {elapsed:?}", total.summary());
    assert_eq!(total.mismatches, 0);
    assert!(elapsed < BUDGET, "the differential took {elapsed:?}");
}
