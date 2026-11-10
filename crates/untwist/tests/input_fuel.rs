use untwist::{Options, Pipeline};
use untwist_eval::budget::{input_fuel, INPUT_STEPS_PER_BYTE};

const UNIT_FUEL: u64 = 1_000_000;
const SPIN: &str =
    "for (var i = 0; i < 99999; i++) { for (var j = 0; j < 99999; j++) { h = h + 1 - 1; } }";

fn spinning_units(count: usize) -> String {
    (0..count)
        .map(|unit| {
            format!(
                "function _0xarr{unit}() {{ var h = ['hello{unit}','world','foo','bar']; _0xarr{unit} = function () {{ return h; }}; return _0xarr{unit}(); }}\n\
                 function _0xdec{unit}(c, d) {{\n  var e = _0xarr{unit}();\n  _0xdec{unit} = function (f, g) {{ f = f - 0; var h = e[f]; {SPIN} return h; }};\n  return _0xdec{unit}(c, d);\n}}\n\
                 console.log(_0xdec{unit}(0));\n"
            )
        })
        .collect()
}

fn options() -> Options {
    Options {
        fuel: Some(UNIT_FUEL),
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

#[test]
fn many_spinning_units_share_one_step_pool_per_input() {
    let source = spinning_units(64);
    let pool = input_fuel(source.len(), UNIT_FUEL);
    assert!(pool < 64 * UNIT_FUEL, "the pool {pool} would not bind");
    let out = Pipeline::new(options()).run(&source);
    let spent = out.stats.decode.steps;
    assert!(
        spent <= pool + UNIT_FUEL,
        "spent {spent} steps over a pool of {pool}"
    );
    assert!(
        out.notes
            .iter()
            .any(|note| note.message.contains("step budget is spent")),
        "{:?}",
        out.notes
    );
}

#[test]
fn the_pool_grows_with_the_input() {
    assert_eq!(input_fuel(0, UNIT_FUEL), 32 * UNIT_FUEL);
    assert_eq!(
        input_fuel(1 << 20, UNIT_FUEL),
        (1 << 20) * INPUT_STEPS_PER_BYTE
    );
}
