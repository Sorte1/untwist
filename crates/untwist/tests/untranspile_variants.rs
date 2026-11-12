use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_unminify::untranspile::identify::{
    record_prints_for_tests, take_prints_for_tests, SeenPrint,
};
use untwist_unminify::untranspile::shapes::variants;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn modes() -> [Options; 2] {
    let shell = |unsafe_transforms| Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    [shell(false), shell(true)]
}

fn last_per_name(prints: Vec<SeenPrint>) -> Vec<SeenPrint> {
    let mut last: Vec<SeenPrint> = Vec::new();
    for print in prints {
        last.retain(|kept| kept.name != print.name);
        last.push(print);
    }
    last
}

fn prints_of(fixture: &str) -> Vec<SeenPrint> {
    let path = root().join(fixture);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let mut seen = Vec::new();
    for options in modes() {
        record_prints_for_tests(true);
        Pipeline::new(options).run(&source);
        seen.extend(last_per_name(take_prints_for_tests()));
        record_prints_for_tests(false);
    }
    seen
}

#[test]
fn every_known_variant_is_seen_in_the_fixture_it_names() {
    let fixtures: BTreeSet<&str> = variants().map(|variant| variant.fixture).collect();
    for fixture in fixtures {
        let seen = prints_of(fixture);
        for variant in variants().filter(|variant| variant.fixture == fixture) {
            assert!(
                seen.iter().any(|print| print.hash == variant.hash
                    && print.params == variant.params
                    && print.statements == variant.statements
                    && print.lead == variant.lead
                    && print.focus == variant.focus),
                "{:?} {:#034x} is not seen in {fixture}; seen: {seen:#?}",
                variant.kind,
                variant.hash
            );
        }
    }
}

#[test]
#[ignore = "a development aid: UNTRANSPILE_PRINTS=<fixture> prints every candidate helper's fingerprint"]
fn print_the_candidate_fingerprints_of_a_fixture() {
    let Ok(fixtures) = std::env::var("UNTRANSPILE_PRINTS") else {
        return;
    };
    for fixture in fixtures.split(',') {
        let mut lines = BTreeSet::new();
        for print in prints_of(fixture) {
            lines.insert(format!(
                "{} {:#034x} params {} statements {} lead {} focus {} tokens {} external {:?}",
                print.name,
                print.hash,
                print.params,
                print.statements,
                print.lead,
                print.focus,
                print.tokens,
                print.external
            ));
        }
        println!("== {fixture}");
        for line in lines {
            println!("{line}");
        }
    }
}
