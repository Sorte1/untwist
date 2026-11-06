use untwist_core::pass::{Pass, PassGroup, PassOptions};
use untwist_testing::{js_files_in, real_world_fixture, transform_report_with, Allocator};
use untwist_unminify::{ternary_to_if, transpile_ctors, unminify_ctors};

const DEEPEST: usize = 40;
const CORPORA: [&str; 3] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
];

fn stage_passes<'a>(one_level: bool) -> Vec<Box<dyn Pass<'a> + 'a>> {
    transpile_ctors()
        .into_iter()
        .chain(unminify_ctors())
        .map(|ctor| {
            let pass = ctor();
            if one_level && pass.name() == "ternary-to-if" {
                ternary_to_if::one_level_boxed_for_tests()
            } else {
                pass
            }
        })
        .collect()
}

fn unminified(source: &str, one_level: bool, extended: bool) -> (String, u32) {
    let alloc = Allocator::default();
    let opts = PassOptions {
        unsafe_transforms: true,
        extended,
        max_iterations: None,
        fuel: None,
        ..PassOptions::default()
    };
    let group = PassGroup::new(stage_passes(one_level), opts);
    let (code, report) = transform_report_with(&alloc, source, group);
    (code, report.iterations)
}

fn else_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!("t{level} ? a{level}() : "));
    }
    text.push_str("z();");
    text
}

fn returned_chain(depth: usize) -> String {
    let mut text = String::from("function f() { return ");
    for level in 0..depth {
        text.push_str(&format!("t{level} ? a{level}() : "));
    }
    text.push_str("z(); }");
    text
}

fn consequent_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!("t{level} ? ("));
    }
    text.push_str("a()");
    for level in (0..depth).rev() {
        text.push_str(&format!(") : c{level}()"));
    }
    text.push(';');
    text
}

fn negated_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            "!t{level} ? (u{level} ? p{level}() : q{level}()) : "
        ));
    }
    text.push_str("z();");
    text
}

fn busy_chain(depth: usize) -> String {
    let mut text = String::from("function g(x) { if (x) { ");
    for level in 0..depth {
        let branch = match level % 3 {
            0 => format!("(p{level}(), q{level}())"),
            1 => format!("u{level} && v{level}()"),
            _ => format!("!!w{level}"),
        };
        text.push_str(&format!("t{level} ? {branch} : "));
    }
    text.push_str("void 0; } }");
    text
}

fn revisited_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            "t{level} ? f{level}() : !!(a{level} != null) ? a{level} : "
        ));
    }
    text.push_str("z();");
    text
}

fn revisited_optional_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            "t{level} ? !!(o{level} === null || o{level} === undefined) ? undefined : o{level}.p : "
        ));
    }
    text.push_str("z();");
    text
}

fn revisited_double_not_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            "t{level} ? f{level}() : !!!!(a{level} != null) ? a{level} : "
        ));
    }
    text.push_str("z();");
    text
}

fn alternate_optional_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!("t{level} ? f{level}() : "));
    }
    text.push_str("!!(o === null || o === undefined) ? undefined : o.p;");
    text
}

fn alternate_double_not_nullish_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            "t{level} ? f{level}() : !!(a{level} !== null && a{level} !== undefined) ? a{level} : "
        ));
    }
    text.push_str("z();");
    text
}

fn alternate_negated_nullish_chain(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            "t{level} ? f{level}() : !(a{level} === null || a{level} === undefined) ? a{level} : "
        ));
    }
    text.push_str("z();");
    text
}

fn mixed_admitted_links_before_a_settling_claimant_expr(depth: usize) -> String {
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&match level % 2 {
            0 => format!("t{level} ? p{level}() : "),
            _ => format!("u{level} ? (q{level}(), r{level}()) : "),
        });
    }
    text.push_str(match depth % 3 {
        0 => "!!(o === null || o === undefined) ? undefined : o.p",
        1 => "!!(a !== null && a !== undefined) ? a : z()",
        _ => "!(a === null || a === undefined) ? a : z()",
    });
    text
}

fn mixed_admitted_links_before_a_settling_claimant(depth: usize) -> String {
    format!(
        "{};",
        mixed_admitted_links_before_a_settling_claimant_expr(depth)
    )
}

fn mixed_admitted_links_before_a_settling_claimant_returned(depth: usize) -> String {
    format!(
        "function f() {{ return {}; }}",
        mixed_admitted_links_before_a_settling_claimant_expr(depth)
    )
}

type Shape = (&'static str, fn(usize) -> String);

const SHAPES: [Shape; 13] = [
    ("else chain", else_chain),
    ("returned chain", returned_chain),
    ("consequent chain", consequent_chain),
    ("negated chain", negated_chain),
    ("busy chain", busy_chain),
    ("revisited chain", revisited_chain),
    ("revisited optional chain", revisited_optional_chain),
    ("revisited double-not chain", revisited_double_not_chain),
    ("alternate optional chain", alternate_optional_chain),
    (
        "alternate double-not nullish chain",
        alternate_double_not_nullish_chain,
    ),
    (
        "alternate negated nullish chain",
        alternate_negated_nullish_chain,
    ),
    (
        "mixed admitted links before a settling claimant",
        mixed_admitted_links_before_a_settling_claimant,
    ),
    (
        "mixed admitted links before a settling claimant, returned",
        mixed_admitted_links_before_a_settling_claimant_returned,
    ),
];

#[test]
fn a_nested_chain_reaches_the_same_bytes_in_one_visit_as_one_level_per_iteration() {
    for (shape, build) in SHAPES {
        for depth in 1..=DEEPEST {
            let source = build(depth);
            for extended in [false, true] {
                let (whole, whole_iterations) = unminified(&source, false, extended);
                let (peeled, peeled_iterations) = unminified(&source, true, extended);
                assert_eq!(whole, peeled, "{shape}, depth {depth}, extended {extended}");
                assert!(
                    whole_iterations <= peeled_iterations,
                    "{shape}, depth {depth}: {whole_iterations} > {peeled_iterations}"
                );
            }
        }
    }
}

#[test]
fn a_forty_link_else_chain_settles_in_two_iterations() {
    let (_, iterations) = unminified(&else_chain(DEEPEST), false, true);
    assert_eq!(iterations, 2);
    let (_, peeled) = unminified(&else_chain(DEEPEST), true, true);
    assert!(peeled > iterations, "{peeled}");
}

#[test]
fn a_consequent_chain_past_the_lift_depth_lifts_whole_in_a_few_iterations() {
    let depth = 4 * untwist_core::depth::LIFT_DEPTH_MAX;
    let source = consequent_chain(depth);
    let (whole, iterations) = unminified(&source, false, true);
    let (peeled, peeled_iterations) = unminified(&source, true, true);
    assert_eq!(whole, peeled);
    assert!(
        iterations <= 8,
        "{iterations} iterations, {peeled_iterations} peeled"
    );
}

#[test]
fn every_corpus_file_unminifies_to_the_same_bytes_either_way() {
    for corpus in CORPORA {
        for (name, source) in js_files_in(corpus) {
            for extended in [false, true] {
                assert_eq!(
                    unminified(&source, false, extended).0,
                    unminified(&source, true, extended).0,
                    "{corpus}/{name}, extended {extended}"
                );
            }
        }
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-unminify --test ternary_chain"
)]
fn game_js_unminifies_to_the_same_bytes_either_way() {
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    for extended in [false, true] {
        let (whole, whole_iterations) = unminified(&source, false, extended);
        let (peeled, peeled_iterations) = unminified(&source, true, extended);
        assert!(whole == peeled, "extended {extended}: the bytes differ");
        assert!(whole_iterations < peeled_iterations);
    }
}
