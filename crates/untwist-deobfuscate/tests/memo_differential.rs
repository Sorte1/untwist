#![cfg(debug_assertions)]

use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_eval::decode::{
    memo_recipe_stats_for_tests, set_recipe_mode_for_tests, take_rotation_outcomes_for_tests,
    MemoRecipeStats, RecipeMode,
};

const GROUPS: [(&str, &str); 9] = [
    ("samples", "fixtures/reference/deobfuscate"),
    ("behaviour", "fixtures/behaviour"),
    ("corpus", "fixtures/corpus-npm"),
    ("eval", "fixtures/eval"),
    ("unminify", "fixtures/unminify"),
    ("unpack", "fixtures/reference/unpack"),
    ("gaps", "fixtures/deobfuscate-gaps"),
    ("unpack-gaps", "fixtures/unpack-gaps"),
    ("rest", "fixtures/jsx"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sources(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            let name = path.to_string_lossy();
            name.ends_with(".js") || name.ends_with(".mjs") || name.ends_with(".jsx")
        })
        .collect();
    found.sort();
    found
}

fn option_sets() -> [Options; 2] {
    [
        Options::default(),
        Options {
            unpack: false,
            unsafe_transforms: true,
            extended: false,
            ..Options::default()
        },
    ]
}

fn delta(before: MemoRecipeStats, after: MemoRecipeStats) -> MemoRecipeStats {
    MemoRecipeStats {
        units: after.units - before.units,
        recipe_terms: after.recipe_terms - before.recipe_terms,
        probed_terms: after.probed_terms - before.probed_terms,
        checked_terms: after.checked_terms - before.checked_terms,
        disagreements: after.disagreements - before.disagreements,
    }
}

fn checked(files: &[PathBuf], failures: &mut Vec<String>) -> MemoRecipeStats {
    let before = memo_recipe_stats_for_tests();
    for file in files {
        let source = std::fs::read_to_string(file).expect("read an input");
        for options in option_sets() {
            let ahead = memo_recipe_stats_for_tests();
            let _ = Pipeline::new(options).run(&source);
            let seen = delta(ahead, memo_recipe_stats_for_tests());
            if seen != MemoRecipeStats::default() {
                println!("  {}: {seen:?}", file.display());
            }
            if seen.disagreements > 0 {
                failures.push(format!("{}: {seen:?}", file.display()));
            }
        }
    }
    delta(before, memo_recipe_stats_for_tests())
}

#[test]
fn every_recipe_term_agrees_with_the_probe_over_the_corpus() {
    let root = root();
    let corpus = root.join("fixtures/corpus-npm/manifest.json");
    let required = std::env::var("UNTWIST_REQUIRE_CORPUS").is_ok_and(|value| !value.is_empty());
    assert!(
        !required || corpus.is_file(),
        "required: fixtures/corpus-npm is absent"
    );
    let mut failures = Vec::new();
    let mut total = MemoRecipeStats::default();
    for (name, dir) in GROUPS {
        let stats = checked(&sources(&root.join(dir)), &mut failures);
        println!("{name}: {stats:?}");
        total.units += stats.units;
        total.checked_terms += stats.checked_terms;
        total.probed_terms += stats.probed_terms;
    }
    let real = [root.join("fixtures/real-world/game-truth.mjs")];
    println!("real-world: {:?}", checked(&real, &mut failures));
    println!("total: {total:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(total.units > 0 && total.checked_terms > 0, "{total:?}");
}

#[path = "memo_differential/generator.rs"]
mod generator;

const SEEDS: [u64; 3] = [1, 2, 3];
const PROGRAMS_PER_CLASS: usize = 30;

#[derive(Default)]
struct Tally {
    programs: usize,
    units: usize,
    recipe_terms: usize,
    probed_terms: usize,
    untouched: usize,
    committed_terms: usize,
}

struct Run {
    code: String,
    rotations: Vec<String>,
    recipe_terms: usize,
}

fn run_in(mode: RecipeMode, source: &str) -> Run {
    set_recipe_mode_for_tests(mode);
    let _ = take_rotation_outcomes_for_tests();
    let ahead = memo_recipe_stats_for_tests();
    let code = Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string();
    let recipe_terms = delta(ahead, memo_recipe_stats_for_tests()).recipe_terms;
    let rotations = take_rotation_outcomes_for_tests();
    set_recipe_mode_for_tests(RecipeMode::Default);
    Run {
        code,
        rotations,
        recipe_terms,
    }
}

fn dual(source: &str) -> Result<usize, String> {
    let committed = run_in(RecipeMode::Commit, source);
    let probed = run_in(RecipeMode::ProbeOnly, source);
    if probed.recipe_terms > 0 {
        return Err("the probe-only run committed a recipe term".to_string());
    }
    if committed.code != probed.code {
        return Err("the decoded output differs".to_string());
    }
    if committed.rotations != probed.rotations {
        return Err(format!(
            "the rotation differs: {:?} against {:?}",
            committed.rotations, probed.rotations
        ));
    }
    Ok(committed.recipe_terms)
}

fn generated(seed: u64, tallies: &mut [Tally], failures: &mut Vec<String>) {
    let mut rng = generator::Rng::new(seed);
    for round in 0..PROGRAMS_PER_CLASS {
        for (slot, class) in generator::CLASSES.iter().enumerate() {
            let program = generator::program(&mut rng, *class);
            let ahead = memo_recipe_stats_for_tests();
            let _ = Pipeline::new(Options::default()).run(&program);
            let seen = delta(ahead, memo_recipe_stats_for_tests());
            let tally = &mut tallies[slot];
            tally.programs += 1;
            tally.units += seen.units;
            tally.recipe_terms += seen.checked_terms;
            tally.probed_terms += seen.probed_terms;
            tally.untouched += usize::from(seen.checked_terms + seen.probed_terms == 0);
            let leaked = class.mutant() && seen.checked_terms > 0;
            if seen.disagreements > 0 || leaked {
                failures.push(format!(
                    "seed {seed} round {round} {class:?} {seen:?}:\n{program}"
                ));
            }
            match dual(&program) {
                Ok(committed) => tally.committed_terms += committed,
                Err(why) => failures.push(format!(
                    "seed {seed} round {round} {class:?} dual run: {why}\n{program}"
                )),
            }
        }
    }
}

#[test]
fn generated_memo_units_agree_with_the_probe() {
    let mut tallies: Vec<Tally> = generator::CLASSES
        .iter()
        .map(|_| Tally::default())
        .collect();
    let mut failures = Vec::new();
    for seed in SEEDS {
        generated(seed, &mut tallies, &mut failures);
    }
    let mut units = 0;
    for (class, tally) in generator::CLASSES.iter().zip(&tallies) {
        units += tally.units;
        println!(
            "generated {class:?}: programs {} units {} recipe {} probed {} untouched {} committed {}",
            tally.programs,
            tally.units,
            tally.recipe_terms,
            tally.probed_terms,
            tally.untouched,
            tally.committed_terms
        );
    }
    println!("generated units: {units}");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    assert!(
        units >= 300,
        "only {units} generated units reached the recipe"
    );
}

#[test]
fn recipe_and_probe_only_runs_decode_the_corpus_alike() {
    let root = root();
    let mut failures = Vec::new();
    let mut committed = 0;
    let mut files = 0;
    let groups = GROUPS.iter().map(|(_, dir)| root.join(dir));
    for dir in groups {
        for file in sources(&dir) {
            let source = std::fs::read_to_string(&file).expect("read an input");
            files += 1;
            match dual(&source) {
                Ok(terms) => committed += terms,
                Err(why) => failures.push(format!("{}: {why}", file.display())),
            }
        }
    }
    println!("corpus: {files} inputs, {committed} recipe terms committed");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(committed > 0);
}
