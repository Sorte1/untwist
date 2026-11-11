use std::time::{Duration, Instant};

use untwist::{Options, Pipeline};

const SHORT: usize = 4_000;
const LONG: usize = 4 * SHORT;
const RUNS: usize = 3;
const QUADRATIC_RATIO: u32 = 8;

fn typeof_chain(length: usize) -> String {
    format!("var y = 1;\nconsole.log({}y);\n", "typeof ".repeat(length))
}

fn not_chain_in_a_decoder(length: usize) -> String {
    format!(
        "function _0xarr() {{ var h = ['hello','world','foo','bar']; _0xarr = function () {{ return h; }}; return _0xarr(); }}\n\
         function _0xdec(c, d) {{\n  var e = _0xarr();\n  _0xdec = function (f, g) {{ f = f - 0; var h = e[f]; h = {}h; return h; }};\n  return _0xdec(c, d);\n}}\n\
         console.log(_0xdec(0));\n",
        "!".repeat(length)
    )
}

fn fastest_run(source: &str) -> Duration {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            let output = Pipeline::new(options.clone()).run(source);
            assert!(output.panicked.is_empty());
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

fn assert_linear(shape: fn(usize) -> String) {
    let short = fastest_run(&shape(SHORT));
    let long = fastest_run(&shape(LONG));
    assert!(
        long < short * QUADRATIC_RATIO,
        "a chain four times as long took {long:?} against {short:?}"
    );
}

#[test]
fn a_long_typeof_chain_costs_linear_time() {
    assert_linear(typeof_chain);
}

#[test]
fn a_long_not_chain_in_a_decoder_costs_linear_time() {
    assert_linear(not_chain_in_a_decoder);
}
