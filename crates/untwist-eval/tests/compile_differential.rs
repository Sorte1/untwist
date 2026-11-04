mod common;

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{heap_budget, Budget};
use untwist_eval::decode::{DecoderProgram, Evaluator, InterpEvaluator};
use untwist_eval::heap::HeapLedger;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units, record_units_charged, CompileCensus, UnitRecord,
};

const SCRIPT_FUEL: u64 = 2_000_000;

#[derive(serde::Deserialize)]
struct Case {
    file: String,
    array_fn: String,
    decoders: Vec<String>,
    rotator_index: Option<usize>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scripts_in(dir: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root().join(dir))
        .expect("a fixture directory")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    found.sort();
    found
}

#[derive(Default)]
struct Tally {
    inputs: usize,
    units: usize,
    calls: u64,
    charged: usize,
}

impl Tally {
    fn compare(&mut self, label: &str, run: impl Fn()) -> CompileCensus {
        let mut default = Vec::new();
        let census = compile_census(|| default = record_units_charged(&run));
        frozen("eval", label, &default);
        self.inputs += 1;
        self.units += default.len();
        self.calls += census.calls;
        self.charged += default.iter().filter(|unit| unit.heap_peak > 0).count();
        census
    }
}

const NODE_BYTES: u64 = 94;
const LAZINESS_K_TWO_SESSIONS: f64 = 2.86;
const LAZINESS_K_PER_SESSION: f64 = 1.43;
const HIGH_PRESET: &str = "obfuscator.io-high.js";

#[derive(Default)]
struct Laziness {
    worst: f64,
    worst_label: String,
    per_session: f64,
    outside: Vec<String>,
}

fn ratio(census: &CompileCensus, arena_bytes: usize) -> f64 {
    let nodes = census.nodes + census.statements;
    (NODE_BYTES * nodes) as f64 / arena_bytes.max(1) as f64
}

impl Laziness {
    fn check(&mut self, label: &str, census: &CompileCensus, arena_bytes: usize) {
        let inside = |node: &(u32, u32)| {
            census
                .unit_spans
                .iter()
                .any(|unit| unit.0 <= node.0 && node.1 <= unit.1)
        };
        if !census.node_spans.iter().all(inside) {
            self.outside.push(label.to_string());
        }
        let both = ratio(census, arena_bytes);
        if both > self.worst {
            self.worst = both;
            let nodes = census.nodes + census.statements;
            self.worst_label = format!("{label}: {nodes} nodes, {arena_bytes} B");
        }
    }

    fn check_session(&mut self, census: &CompileCensus, arena_bytes: usize) {
        self.per_session = self.per_session.max(ratio(census, arena_bytes));
    }
}

fn decode_both(program: &DecoderProgram<'_>, calls: &[untwist_eval::decode::Call<'_>], len: usize) {
    for heap in [false, true] {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        if heap {
            evaluator.heap = Some(HeapLedger::new(heap_budget(len)));
        }
        let _ = evaluator.decode(program, calls);
    }
}

fn corpus_units(tally: &mut Tally) {
    let dir = root().join("fixtures/eval");
    let manifest = std::fs::read_to_string(dir.join("manifest.json")).expect("the eval corpus");
    let cases: Vec<Case> = serde_json::from_str(&manifest).expect("the manifest parses");
    for case in &cases {
        let source = std::fs::read_to_string(dir.join(&case.file)).expect("a corpus file");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &source);
        let decoders: Vec<&str> = case.decoders.iter().map(String::as_str).collect();
        let program = common::decoder_program(
            &parsed.program,
            &case.array_fn,
            &decoders,
            case.rotator_index,
        );
        let calls = common::calls_for(&parsed.program, 0);
        let label = format!("fixtures/eval/{}", case.file);
        tally.compare(&label, || decode_both(&program, &calls, source.len()));
    }
}

fn sample_units(tally: &mut Tally) {
    for (file, array, decoder) in [
        ("obfuscator.io.js", "c", "d"),
        ("obfuscator.io-high.js", "_0x4d6b", "_0x4c3e"),
    ] {
        let source = common::read_fixture(file);
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &source);
        let program = common::decoder_program(&parsed.program, array, &[decoder], Some(0));
        let calls = common::decoder_calls_anywhere(&parsed.program, &[decoder]);
        let label = format!("sample {file}");
        tally.compare(&label, || decode_both(&program, &calls, source.len()));
    }
}

fn script_units(tally: &mut Tally, laziness: &mut Laziness, dir: &str) {
    for path in scripts_in(dir) {
        let source = std::fs::read_to_string(&path).expect("a script");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let program = &parsed.program;
        let label = path
            .strip_prefix(root())
            .unwrap_or(&path)
            .display()
            .to_string();
        let census = tally.compare(&label, || {
            eval_script_in(program, Budget::new(SCRIPT_FUEL), |_, _| ());
            eval_script_decoding(program, Budget::new(SCRIPT_FUEL), |_| ());
        });
        laziness.check(&label, &census, alloc.used_bytes());
        let alone = compile_census(|| {
            eval_script_in(program, Budget::new(SCRIPT_FUEL), |_, _| ());
        });
        laziness.check_session(&alone, alloc.used_bytes());
        let decoding = compile_census(|| {
            eval_script_decoding(program, Budget::new(SCRIPT_FUEL), |_| ());
        });
        laziness.check_session(&decoding, alloc.used_bytes());
        if label.ends_with(HIGH_PRESET) {
            println!(
                "laziness, the high preset: {} nodes in one decoding session, {} arena bytes, k {:.6}",
                decoding.nodes + decoding.statements,
                alloc.used_bytes(),
                ratio(&decoding, alloc.used_bytes())
            );
        }
    }
}

fn report(group: &str, tally: &Tally) {
    println!(
        "{group}: {} inputs, {} units ({} with a heap peak), {} compiled calls, all as frozen",
        tally.inputs, tally.units, tally.charged, tally.calls
    );
}

#[test]
fn the_evaluator_records_every_unit_as_frozen() {
    let mut corpus = Tally::default();
    corpus_units(&mut corpus);
    report("eval corpus", &corpus);
    let mut samples = Tally::default();
    sample_units(&mut samples);
    report("samples", &samples);
    let mut scripts = Tally::default();
    let mut laziness = Laziness::default();
    for dir in [
        "fixtures/behaviour",
        "fixtures/reference/deobfuscate",
        "fixtures/deobfuscate-gaps",
    ] {
        script_units(&mut scripts, &mut laziness, dir);
    }
    report("scripts", &scripts);
    println!(
        "laziness: k over two sessions {:.6} against the pinned {LAZINESS_K_TWO_SESSIONS} ({}); per session {:.6} against {LAZINESS_K_PER_SESSION}; {} inputs outside",
        laziness.worst,
        laziness.worst_label,
        laziness.per_session,
        laziness.outside.len()
    );
    assert!(laziness.outside.is_empty(), "{:?}", laziness.outside);
    assert!(
        laziness.worst <= LAZINESS_K_TWO_SESSIONS,
        "k {}",
        laziness.worst
    );
    assert!(
        laziness.per_session <= LAZINESS_K_PER_SESSION,
        "k per session {}",
        laziness.per_session
    );
    assert!(scripts.calls > 0, "no script call entered compiled");
    assert!(
        scripts.charged >= scripts.inputs,
        "the script units ran without a ledger"
    );
    assert!(corpus.units >= corpus.inputs && samples.units > 0 && scripts.units > 0);
}

#[test]
fn a_unit_record_holds_what_the_differential_compares() {
    let source =
        "function arr() { return ['a', 'b']; }\nfunction dec(i) { return arr()[i]; }\ndec(1);\n";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let program = common::decoder_program(&parsed.program, "arr", &["dec"], None);
    let calls = common::decoder_calls_anywhere(&parsed.program, &["dec"]);
    let records: Vec<UnitRecord> = record_units(|| {
        decode_both(&program, &calls, source.len());
    });
    assert_eq!(records.len(), 2, "{records:?}");
    assert!(records.iter().all(|record| record.values == ["string:b"]));
    assert_eq!(records[0].heap_peak, 0);
    assert!(records[1].heap_peak > 0);
    assert!(records.iter().all(|record| record.steps_used > 0));
}
