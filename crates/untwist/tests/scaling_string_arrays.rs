mod common;

use common::scaling::assert_scales_linearly;
use untwist::{Options, Pipeline};

fn small_units(count: usize) -> String {
    (0..count)
        .map(|index| {
            format!(
                "function f{index}(u){{ var arr{index} = [\"a{index}\", \"b\"]; return u((function(p){{ return arr{index}[p]; }})(1)); }}\n"
            )
        })
        .collect()
}

#[test]
fn many_small_string_arrays_are_removed_without_a_walk_per_unit() {
    assert_scales_linearly(
        "function fN(u){ var arrN = [...]; return u(... arrN[p] ...); }",
        300,
        small_units,
        Options::default,
    );
}

#[test]
fn a_small_string_array_unit_still_decodes() {
    let out = Pipeline::new(Options::default()).run(&small_units(3));
    assert!(
        out.code().contains("u(\"b\")") && !out.code().contains("arr0"),
        "{}",
        out.code()
    );
}
