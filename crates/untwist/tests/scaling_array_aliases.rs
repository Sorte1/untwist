mod common;

use common::scaling::assert_scales_linearly;
use untwist::{Options, Pipeline};

fn alias_chain(count: usize) -> String {
    let links: String = (1..count)
        .map(|index| format!("var q{index}=q{};", index - 1))
        .collect();
    format!(
        "var h2=['x','y'];var q0=h2;{links}console.log(q{}[0]);",
        count - 1
    )
}

fn alias_fan(count: usize) -> String {
    let aliases: String = (0..count)
        .map(|index| format!("var b{index}=h2;"))
        .collect();
    let reads: String = (0..count)
        .map(|index| format!("console.log(b{index}[{}]);", index % 2))
        .collect();
    format!("var h2=['x','y'];{aliases}{reads}")
}

#[test]
fn a_chain_of_array_aliases_resolves_in_one_round() {
    assert_scales_linearly(
        "var q1=q0; var q2=q1; ...",
        150,
        alias_chain,
        Options::default,
    );
}

#[test]
fn many_aliases_of_one_array_are_scanned_together() {
    assert_scales_linearly(
        "var b0=h2; var b1=h2; ...",
        500,
        alias_fan,
        Options::default,
    );
}

#[test]
fn a_resolved_alias_chain_reads_the_array_itself() {
    let out = Pipeline::new(Options::default()).run(&alias_chain(40));
    assert!(!out.code().contains("q39"), "{}", out.code());
}
