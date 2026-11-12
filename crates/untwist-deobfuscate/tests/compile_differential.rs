use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units, widest_frame, CompileCensus, UnitKind, UnitRecord,
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
    ("jsx", "fixtures/jsx"),
];

const HIGH: &str = "fixtures/reference/deobfuscate/obfuscator.io-high.js";

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

fn option_sets() -> [(&'static str, Options); 4] {
    let with = |unsafe_transforms, extended| Options {
        unsafe_transforms,
        extended,
        ..Options::default()
    };
    [
        ("default", with(false, true)),
        ("conservative", with(false, false)),
        ("unsafe", with(true, true)),
        ("unsafe conservative", with(true, false)),
    ]
}

struct Run {
    code: String,
    units: Vec<UnitRecord>,
}

fn run(source: &str, options: &Options) -> Run {
    let mut code = String::new();
    let units = record_units(|| {
        code = Pipeline::new(options.clone())
            .run(source)
            .code()
            .to_string();
    });
    Run { code, units }
}

fn freeze(input: &str, set: &str, run: &Run) {
    let mut parts: Vec<String> = run.units.iter().map(|unit| format!("{unit:?}")).collect();
    parts.push(run.code.clone());
    frozen("pipeline units", &format!("{input} [{set}]"), &parts);
}

fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[derive(Default)]
struct Tally {
    runs: usize,
    units: usize,
    calls: u64,
    checked_units: u64,
    unchecked_units: u64,
    overruns: u64,
}

fn compiled_run(source: &str, options: &Options) -> (Run, CompileCensus) {
    let mut default = None;
    let census = compile_census(|| default = Some(run(source, options)));
    let default = default.unwrap_or(Run {
        code: String::new(),
        units: Vec::new(),
    });
    (default, census)
}

fn compared(path: &Path, input: &str, tally: &mut Tally) {
    let source = std::fs::read_to_string(path).expect("read an input");
    for (label, options) in option_sets() {
        let (default, census) = compiled_run(&source, &options);
        freeze(input, label, &default);
        tally.runs += 1;
        tally.units += default.units.len();
        tally.calls += census.calls;
        tally.checked_units += census.checked_units;
        tally.unchecked_units += census.unchecked_units;
        tally.overruns += census.depth_overruns;
    }
}

fn report(name: &str, tally: &Tally) {
    println!(
        "{name}: {} runs, {} units, {} compiled calls, {} checked and {} unchecked unit runs, {} overruns, all as frozen",
        tally.runs,
        tally.units,
        tally.calls,
        tally.checked_units,
        tally.unchecked_units,
        tally.overruns
    );
}

fn group_tallies() -> Vec<(&'static str, Tally)> {
    let root = root();
    let corpus = root.join("fixtures/corpus-npm/manifest.json");
    let required = std::env::var("UNTWIST_REQUIRE_CORPUS").is_ok_and(|value| !value.is_empty());
    assert!(
        !required || corpus.is_file(),
        "required: fixtures/corpus-npm is absent"
    );
    let mut tallies = Vec::new();
    for (name, dir) in GROUPS {
        let mut tally = Tally::default();
        for path in sources(&root.join(dir)) {
            compared(&path, &relative(&path), &mut tally);
        }
        report(name, &tally);
        tallies.push((name, tally));
    }
    if let Some(game) = untwist_testing::real_world_path("game.js") {
        let mut real = Tally::default();
        compared(&game, GAME, &mut real);
        report("real-world", &real);
        tallies.push(("real-world", real));
    }
    tallies
}

fn assert_no_overrun(tallies: &[(&str, Tally)]) {
    let overruns: u64 = tallies.iter().map(|(_, tally)| tally.overruns).sum();
    assert_eq!(
        overruns, 0,
        "an unchecked unit ran past the reach it was given"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: a debug build checks every memo recipe against its probe"
)]
fn every_decode_unit_records_as_frozen() {
    let tallies = group_tallies();
    let runs: usize = tallies.iter().map(|(_, tally)| tally.runs).sum();
    let units: usize = tallies.iter().map(|(_, tally)| tally.units).sum();
    let calls: u64 = tallies.iter().map(|(_, tally)| tally.calls).sum();
    println!("total: {runs} runs, {units} units, {calls} compiled calls");
    assert_no_overrun(&tallies);
    let real = tallies
        .iter()
        .find(|(name, _)| *name == "real-world")
        .map(|(_, tally)| tally);
    assert!(real.is_none_or(|real| real.units > 0 && real.calls > 0));
    assert!(
        units > 0 && calls > 0,
        "the decode units ran no compiled call"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: a debug build checks every memo recipe against its probe"
)]
fn the_high_presets_pipeline_units_keep_their_steps_and_heap_totals() {
    let source = std::fs::read_to_string(root().join(HIGH)).expect("the high preset");
    for (label, options) in option_sets() {
        let units = run(&source, &options).units;
        let decode: Vec<(usize, u64, u64, u64, usize)> = units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Decode)
            .map(|unit| {
                (
                    unit.values.len(),
                    unit.steps_used,
                    unit.heap_peak,
                    unit.heap_final,
                    unit.memo_inserts,
                )
            })
            .collect();
        let decoding = (577, 39_333_508, 495_620, 125_534, 2_024);
        let bailed = (0, 0, 0, 0, 0);
        assert!(
            decode.contains(&decoding),
            "high preset [{label}]: {units:?}"
        );
        assert!(
            decode
                .iter()
                .all(|unit| *unit == decoding || *unit == bailed),
            "high preset [{label}]: {decode:?}"
        );
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset through the pipeline is slow in debug"
)]
fn the_high_presets_prototype_reads_are_the_same_on_every_run() {
    let source = std::fs::read_to_string(root().join(HIGH)).expect("the high preset");
    let reads = || {
        run(&source, &Options::default())
            .units
            .into_iter()
            .map(|unit| unit.proto_reads)
            .collect::<Vec<_>>()
    };
    let first = reads();
    assert!(first.iter().any(|unit| !unit.is_empty()));
    for _ in 0..3 {
        assert_eq!(reads(), first);
    }
}

const INLINE_DECLARED_BITS: usize = 64;

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset through the pipeline is slow in debug"
)]
fn every_frame_of_the_high_preset_fits_the_inline_declared_bits() {
    let source = std::fs::read_to_string(root().join(HIGH)).expect("the high preset");
    for (label, options) in option_sets() {
        let widest = widest_frame(|| {
            run(&source, &options);
        });
        println!("frame width [{label}]: widest {widest}");
        assert!(widest > 0, "[{label}]: no frame declared a binding");
        assert!(widest <= INLINE_DECLARED_BITS, "[{label}]: {widest}");
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset through the pipeline is slow in debug"
)]
fn the_high_presets_compiled_code_holds_no_maybe_node() {
    let source = std::fs::read_to_string(root().join(HIGH)).expect("the high preset");
    for (label, options) in option_sets() {
        let (_, census) = compiled_run(&source, &options);
        println!(
            "high preset compiled [{label}]: {} calls, {} nodes, {} maybe, checksum terms {} of which {} direct",
            census.calls, census.nodes, census.maybe, census.checksum_terms, census.direct_terms
        );
        assert!(census.checksum_terms > 0, "[{label}]: {census:?}");
        assert_eq!(
            census.direct_terms, census.checksum_terms,
            "[{label}]: {census:?}"
        );
        assert_eq!(census.maybe, 0, "[{label}]: {census:?}");
    }
}

const GAME: &str = "fixtures/real-world/game.js";

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: game.js through the pipeline is slow in debug"
)]
fn on_game_js_a_literal_call_argument_has_no_cache_entry_and_every_site_its_frozen_steps() {
    let Some(source) = untwist_testing::real_world_fixture("game.js") else {
        return;
    };
    let options = Options::default();
    let (default, census) = compiled_run(&source, &options);
    freeze(GAME, "default", &default);
    println!(
        "game.js call arguments: {} direct, {} compiled, {} cache entries; checksum terms {} of which {} direct",
        census.direct_arguments,
        census.compiled_arguments,
        census.argument_entries,
        census.checksum_terms,
        census.direct_terms
    );
    assert!(census.checksum_terms > 0, "{census:?}");
    assert_eq!(census.direct_terms, census.checksum_terms, "{census:?}");
    assert_eq!(census.argument_entries, 0, "{census:?}");
    assert_eq!(census.compiled_arguments, 0, "{census:?}");
    assert!(census.direct_arguments > 100_000, "{census:?}");
    let sites: usize = default.units.iter().map(|unit| unit.site_steps.len()).sum();
    assert!(sites > 100_000, "{sites} call sites");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset and game.js through the pipeline are slow in debug"
)]
fn the_string_arrays_run_as_templates_with_their_frozen_records() {
    for (input, set, options) in [
        (HIGH, "default", Options::default()),
        (
            HIGH,
            "conservative",
            Options {
                extended: false,
                ..Options::default()
            },
        ),
        (GAME, "default", Options::default()),
    ] {
        if input == GAME && untwist_testing::real_world_path("game.js").is_none() {
            continue;
        }
        let source = std::fs::read_to_string(root().join(input)).expect("an input");
        let (default, census) = compiled_run(&source, &options);
        freeze(input, set, &default);
        println!(
            "templates {input}: {} templates holding {} constants, {} batched runs, {} per-element runs",
            census.templates,
            census.template_constants,
            census.template_batches,
            census.template_fallbacks
        );
        assert!(census.templates > 0, "templates {input}: {census:?}");
        assert!(census.template_batches > 0, "templates {input}: {census:?}");
    }
}
