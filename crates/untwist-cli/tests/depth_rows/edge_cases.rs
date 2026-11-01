use std::path::{Path, PathBuf};

use super::harness::{
    argv, codes, passed_through, refused, untwist, workspace_file, Rows, Run, Scratch,
};
use super::supervisor::{died_warning, json_run};

const MIB: u64 = 1 << 20;
const PRINT_FLOOR: u64 = 64 * MIB;
const PRINT_BYTES_PER_INPUT_BYTE: u64 = 32;
const PROBE_DEPTH: usize = 2_500;
const WRAPPER_DEPTH: usize = 400;
const WIDE_DEPTH: usize = 300;
const WIDE_TEXTS: usize = 200;
const SMALL_JSON: &str = "{\"small\":[1,2]}";
const POOL_DEPTHS: [usize; 4] = [99_000, 99_000, 40_000, 15_000];
const POOL_FILLER_LINES: usize = 60_000;
const DEEP_CODE_TEXT: usize = 7_000;
const OBFUSCATED: &str = "fixtures/reference/deobfuscate/obfuscator.io.js";
const AFTER_INPUTS: (&str, &str) = ("UNTWIST_TEST_WORKER_ABORT", "after-inputs");
const ADMIT_PANIC: (&str, &str) = ("UNTWIST_TEST_ADMIT_PANIC", "0");
const FIRST: &str = "var first = !0;\nconsole.log(first);\n";
const SECOND: &str = "var second = void 0;\nconsole.log(second);\n";

fn print_budget(input: &str) -> u64 {
    (input.len() as u64 * PRINT_BYTES_PER_INPUT_BYTE).max(PRINT_FLOOR)
}

fn chained(depth: usize) -> String {
    format!("{}1{}", "{\"a\":1,\"b\":".repeat(depth), "}".repeat(depth))
}

fn json_sites(texts: usize, depth: usize) -> String {
    let json = chained(depth);
    (0..texts)
        .map(|index| format!("var x{index} = JSON.parse('{json}');\n"))
        .collect()
}

fn kept_strings(run: &Run) -> usize {
    String::from_utf8_lossy(run.stdout())
        .matches("JSON.parse(")
        .count()
}

fn within_budget(run: &Run, source: &str) -> bool {
    run.code() == Some(0) && !run.died() && run.stdout().len() as u64 <= print_budget(source)
}

pub fn json_parse_probes_stay_inside_the_print_budget(rows: &mut Rows, scratch: &Scratch) {
    for texts in [40, 150] {
        let source = json_sites(texts, PROBE_DEPTH);
        let input = scratch.file(&format!("jmk{texts}.js"), &source);
        let run = untwist(&argv(&[], &[&input]), &[]);
        rows.check(
            &format!("K={texts} exits 0 within the print budget"),
            within_budget(&run, &source),
            || format!("{} output {} B", run.summary(), run.stdout().len()),
        );
        rows.check(
            &format!("K={texts} keeps its JSON strings"),
            kept_strings(&run) == texts,
            || format!("kept {} of {texts}", kept_strings(&run)),
        );
    }
}

fn wrapped_wide_sites() -> String {
    let open: String = (0..WRAPPER_DEPTH)
        .map(|level| format!("function w{level}() {{\n"))
        .collect();
    let close = "}\n".repeat(WRAPPER_DEPTH);
    let small = format!("var tail = JSON.parse('{SMALL_JSON}');\n");
    format!("{open}{}{small}{close}", json_sites(WIDE_TEXTS, WIDE_DEPTH))
}

pub fn inlined_json_is_charged_to_the_print_room(rows: &mut Rows, scratch: &Scratch) {
    let source = wrapped_wide_sites();
    let input = scratch.file("wide-json.js", &source);
    let run = untwist(&argv(&["--no-unpack"], &[&input]), &[]);
    rows.check(
        "wide JSON inside a deep wrapper stays inside the print budget",
        within_budget(&run, &source),
        || format!("{} output {} B", run.summary(), run.stdout().len()),
    );
    let kept = kept_strings(&run);
    rows.check(
        "the room inlines a prefix and keeps the rest as strings",
        (1..WIDE_TEXTS).contains(&kept),
        || format!("kept {kept} of {WIDE_TEXTS}"),
    );
    let printed = String::from_utf8_lossy(run.stdout()).into_owned();
    rows.check(
        "a small JSON text after the room is spent still inlines",
        printed.contains("tail = {") && !printed.contains(SMALL_JSON),
        || format!("output {} B", printed.len()),
    );
}

fn deep_with_filler(depth: usize) -> String {
    let filler: String = (0..POOL_FILLER_LINES)
        .map(|line| format!("var v{line} = f({line}) + g({line});\n"))
        .collect();
    format!("{}1;\n{filler}", "a=".repeat(depth))
}

fn decoder_beside_a_deep_code_text() -> String {
    let obfuscated = std::fs::read_to_string(workspace_file(OBFUSCATED))
        .unwrap_or_else(|error| panic!("read {OBFUSCATED}: {error}"));
    let text = format!(
        "{}{}",
        "[".repeat(DEEP_CODE_TEXT),
        "]".repeat(DEEP_CODE_TEXT)
    );
    format!("{obfuscated}\nvar __t = Function(\"return {text}\");\n")
}

fn pool_inputs(scratch: &Scratch) -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = POOL_DEPTHS
        .iter()
        .enumerate()
        .map(|(index, depth)| scratch.file(&format!("pool-{index}.js"), &deep_with_filler(*depth)))
        .collect();
    inputs.push(scratch.file("pool-4.js", &decoder_beside_a_deep_code_text()));
    inputs
}

fn batch_codes(files: &[&Path], jobs: &str) -> (Run, Vec<String>) {
    let run = untwist(&argv(&["-j", jobs, "--report", "json"], files), &[]);
    let found = codes(&run.json());
    (run, found)
}

pub fn decoding_does_not_depend_on_jobs_or_co_inputs(rows: &mut Rows, scratch: &Scratch) {
    let inputs = pool_inputs(scratch);
    let files: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
    let (serial, serial_codes) = batch_codes(&files, "1");
    let (wide, wide_codes) = batch_codes(&files, "5");
    rows.check(
        "the pool batch runs at -j 1 and -j 5",
        serial.code() == Some(0) && wide.code() == Some(0) && serial_codes.len() == 5,
        || format!("{} / {}", serial.summary(), wide.summary()),
    );
    rows.check(
        "-j 1 and -j 5 write byte-identical outputs",
        serial_codes == wide_codes,
        || {
            let differ: Vec<usize> = (0..serial_codes.len().max(wide_codes.len()))
                .filter(|index| serial_codes.get(*index) != wide_codes.get(*index))
                .collect();
            format!("inputs {differ:?} differ")
        },
    );
    let (alone, alone_codes) = batch_codes(&files[4..], "1");
    rows.check(
        "the decoder input decodes as it does alone",
        alone.code() == Some(0) && alone_codes.first() == wide_codes.get(4),
        || alone.summary(),
    );
}

fn refused_after_the_inputs(run: &Run, sources: &[&str]) -> bool {
    let value = run.json();
    let all: Vec<u64> = (0..sources.len() as u64).collect();
    let warned = (1..=sources.len()).all(|index| run.stderr().contains(&died_warning(index)));
    let passed = codes(&value)
        .iter()
        .zip(sources)
        .all(|(code, source)| code == source);
    run.code() == Some(3) && !run.died() && refused(&value) == Some(all) && warned && passed
}

pub fn a_death_after_the_last_input_refuses_the_inputs_it_serves(
    rows: &mut Rows,
    scratch: &Scratch,
) {
    let first = scratch.file("after-first.js", FIRST);
    let second = scratch.file("after-second.js", SECOND);
    for flags in [&["--no-unpack"][..], &[][..]] {
        let run = json_run(flags, std::slice::from_ref(&first), &[AFTER_INPUTS]);
        rows.check(
            &format!("{flags:?} one input: a death after it is its refusal"),
            refused_after_the_inputs(&run, &[FIRST]),
            || run.summary(),
        );
    }
    let run = json_run(&["-j", "2"], &[first.clone(), second], &[AFTER_INPUTS]);
    rows.check(
        "two inputs: a death after both refuses both",
        refused_after_the_inputs(&run, &[FIRST, SECOND]),
        || run.summary(),
    );
    let plain = untwist(&argv(&[], &[&first]), &[AFTER_INPUTS]);
    rows.check(
        "the refused input's source is its output",
        plain.code() == Some(3) && passed_through(plain.stdout(), FIRST),
        || plain.summary(),
    );
}

pub fn a_panic_before_the_parse_is_a_refusal(rows: &mut Rows, scratch: &Scratch) {
    let first = scratch.file("admit-first.js", FIRST);
    let second = scratch.file("admit-second.js", SECOND);
    let run = json_run(&[], &[first, second], &[ADMIT_PANIC]);
    let value = run.json();
    let found = codes(&value);
    rows.check(
        "a panic in the first input's admit refuses it alone, exit 3",
        run.code() == Some(3)
            && !run.died()
            && refused(&value) == Some(vec![0])
            && found.first().map(String::as_str) == Some(FIRST)
            && found.get(1).is_some_and(|code| code != SECOND),
        || run.summary(),
    );
}
