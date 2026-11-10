use std::time::{Duration, Instant};

use untwist::{Options, Pipeline};

const RUNS: usize = 3;
const FACTOR: usize = 4;
const RATIO_BOUND: f64 = 8.0;
const FLOOR: Duration = Duration::from_millis(1);

pub fn fastest_run(source: &str, options: fn() -> Options) -> Duration {
    (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            Pipeline::new(options()).run(source);
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

pub fn assert_scales_linearly(
    shape: &str,
    small: usize,
    make: impl Fn(usize) -> String,
    options: fn() -> Options,
) {
    let small_cost = fastest_run(&make(small), options).max(FLOOR);
    let large = small * FACTOR;
    let large_cost = fastest_run(&make(large), options);
    let ratio = large_cost.as_secs_f64() / small_cost.as_secs_f64();
    assert!(
        ratio < RATIO_BOUND,
        "{shape}: {large} copies took {large_cost:?}, {small} took {small_cost:?}: \
         x{ratio:.1} for x{FACTOR} the input, the bound is x{RATIO_BOUND}"
    );
}
