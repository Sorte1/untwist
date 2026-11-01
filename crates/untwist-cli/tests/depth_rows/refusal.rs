use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use untwist::{NESTED_TOO_DEEP, PARENTHESES_TOO_DEEP, PRINT_TOO_LARGE};
use untwist_testing::capped_child::Cap;
use untwist_testing::depth_classes::{generate, repeatchain, Class, ALL};

use super::harness::{
    argv, codes, deobfuscate_fixtures, passed_through, refused, run_with, untwist, without_timings,
    workspace_file, Rows, Run, Scratch, MB,
};

const MILLION: usize = 1_000_000;
const UNTABLED_DEPTH: usize = 20_000;
const BARE: &[&str] = &[
    "--no-unpack",
    "--no-deobfuscate",
    "--no-unminify",
    "--no-jsx",
];
pub const FLAG_SETS: [&[&str]; 4] = [&[], &["--unsafe"], &["--rename", "mangle"], BARE];
const PAIR_BYTES: u64 = 372;
const CHAIN_PAIRS: u64 = 124_750;
const BASELINE_PAIRS: usize = 21;
const QUICK: Duration = Duration::from_secs(1);
const SMALL_PEAK: u64 = 200 * MB;
const DEEP_INDEX: usize = 3;
const AS_LIMIT: &str = "--as=1572864000";
const PLUS_TERMS: usize = 90_000;

fn cli_kill_depth(class: Class) -> Option<usize> {
    Some(match class {
        Class::And | Class::Nullish | Class::Plus => 29_875,
        Class::Array | Class::Paren => 9_531,
        Class::Arrow => 14_937,
        Class::Assign => 19_062,
        Class::Block | Class::Try => 30_875,
        Class::Call => 10_281,
        Class::Callchain | Class::New => 55_250,
        Class::Cond => 16_312,
        Class::Destr => 49_875,
        Class::Elseif | Class::Ifelse | Class::Exp => 45_625,
        Class::Fnexpr => 5_359,
        Class::Function => 11_031,
        Class::If | Class::While => 18_437,
        Class::Jsx => 37_375,
        Class::Label => 38_875,
        Class::Member | Class::Optchain => 87_250,
        Class::Neg | Class::Numplus | Class::Strplus | Class::Typeof | Class::Unary => 104_500,
        Class::Object => 8_218,
        Class::Template => 8_406,
        Class::Klass => 4_296,
        Class::Comma | Class::Stmts => MILLION,
        _ => return None,
    })
}

pub fn refusal_depths(class: Class) -> Vec<usize> {
    let killing = cli_kill_depth(class).map_or(UNTABLED_DEPTH, |depth| (2 * depth).min(MILLION));
    let mut depths = vec![killing, MILLION];
    depths.dedup();
    depths
}

pub fn unparsable(class: Class) -> bool {
    matches!(
        class,
        Class::Openarr | Class::Openparen | Class::Openobj | Class::Opentpl
    )
}

pub fn no_class_kills_the_process(rows: &mut Rows, scratch: &Scratch) {
    for &class in ALL {
        for depth in refusal_depths(class) {
            let input = scratch.file("d1.js", &generate(class, depth));
            for flags in FLAG_SETS {
                let run = untwist(&argv(flags, &[&input]), &[]);
                let allowed = matches!(run.code(), Some(0 | 3))
                    || (unparsable(class) && run.code() == Some(1));
                let label = format!("{} at {depth} {flags:?}", class.name());
                rows.check(&label, allowed && !run.died(), || run.summary());
            }
        }
    }
}

fn refuses_with(rows: &mut Rows, label: &str, run: &Run, input: &str, note: &str) {
    let holds = run.code() == Some(3)
        && !run.died()
        && run.stderr().contains(note)
        && passed_through(run.stdout(), input);
    rows.check(label, holds, || run.summary());
}

fn refused_alone(
    rows: &mut Rows,
    scratch: &Scratch,
    row: &str,
    case: (Class, usize),
    note: &str,
) -> Run {
    let (class, depth) = case;
    let source = generate(class, depth);
    let input = scratch.file("refused.js", &source);
    let run = untwist(&argv(&[], &[&input]), &[]);
    refuses_with(
        rows,
        &format!("{row} {} at {depth}", class.name()),
        &run,
        &source,
        note,
    );
    run
}

const PRINT_CLASSES: [(Class, usize); 7] = [
    (Class::Block, 20_000),
    (Class::If, 15_000),
    (Class::While, 15_000),
    (Class::Function, 15_000),
    (Class::Try, 15_000),
    (Class::Fnexpr, 12_000),
    (Class::Klass, 8_000),
];

pub fn print_refusals(rows: &mut Rows, scratch: &Scratch) {
    for case in PRINT_CLASSES {
        refused_alone(rows, scratch, "print too large", case, PRINT_TOO_LARGE);
        refused_alone(
            rows,
            scratch,
            "print too large at 100000",
            (case.0, 100_000),
            PRINT_TOO_LARGE,
        );
    }
}

pub fn parentheses_and_nesting(rows: &mut Rows, scratch: &Scratch) {
    for class in [Class::Arrowobj, Class::Arrowdef, Class::Asyncarrow] {
        let run = refused_alone(
            rows,
            scratch,
            "parentheses too deep",
            (class, 4_000),
            PARENTHESES_TOO_DEEP,
        );
        rows.check(
            &format!("{} is small and quick", class.name()),
            run.peak_bytes() < SMALL_PEAK && run.elapsed < QUICK,
            || run.summary(),
        );
    }
    refused_alone(
        rows,
        scratch,
        "nested too deep",
        (Class::Array, 200_000),
        NESTED_TOO_DEEP,
    );
    refused_alone(
        rows,
        scratch,
        "nested too deep",
        (Class::Openarr, MILLION),
        NESTED_TOO_DEEP,
    );
}

pub fn linear_chains_run_and_parse(rows: &mut Rows, scratch: &Scratch) {
    for class in [Class::Plus, Class::Member] {
        let input = scratch.file("d5.js", &generate(class, 99_000));
        let run = untwist(&argv(&[], &[&input]), &[]);
        let label = format!("{} at 99000", class.name());
        rows.check(&label, run.code() == Some(0) && !run.died(), || {
            run.summary()
        });
        let output = scratch.file("d5-out.js", &String::from_utf8_lossy(run.stdout()));
        let reparse = untwist(&argv(BARE, &[&output]), &[]);
        rows.check(
            &format!("{label} output parses"),
            reparse.code() == Some(0),
            || reparse.summary(),
        );
    }
}

pub fn outputs_and_reports(rows: &mut Rows, scratch: &Scratch) {
    let source = generate(Class::Array, 200_000);
    let deep = scratch.file("deep.js", &source);
    let out_dir = scratch.path("d6-out");
    let run = untwist(&argv(&["-o"], &[&out_dir, &deep]), &[]);
    let written = std::fs::read_to_string(out_dir.join("deobfuscated.js")).unwrap_or_default();
    rows.check(
        "-o writes the source",
        run.code() == Some(3) && written == source,
        || run.summary(),
    );
    let json = untwist(&argv(&["--report", "json"], &[&deep]), &[]);
    let value = json.json();
    rows.check(
        "json lists the refusal",
        refused(&value) == Some(vec![0]) && value.get("panicked").is_none(),
        || json.summary(),
    );
    let normal = scratch.file("normal.js", "var a = 1;");
    let clean = untwist(&argv(&["--report", "json"], &[&normal]), &[]).json();
    rows.check(
        "a normal input has no key",
        clean.get("depth_refused").is_none(),
        || clean.to_string(),
    );
    let text = untwist(&argv(&["--report", "text"], &[&deep]), &[]);
    let has_line = text.stderr().lines().any(|line| line == "depth_refused: 0");
    rows.check("text report line", has_line, || text.summary());
    let broken = scratch.file("broken.js", "var = ;");
    let both = untwist(&argv(&[], &[&deep, &broken]), &[]);
    rows.check(
        "parse errors outrank a refusal",
        both.code() == Some(1),
        || both.summary(),
    );
}

fn mix_with(deep: &Path, at: usize) -> Vec<PathBuf> {
    let mut inputs = deobfuscate_fixtures();
    inputs.insert(at, deep.to_path_buf());
    inputs
}

fn json_run(flags: &[&str], inputs: &[PathBuf]) -> Run {
    let files: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
    let mut all = vec!["--report", "json"];
    all.extend_from_slice(flags);
    untwist(&argv(&all, &files), &[])
}

fn alone_codes(inputs: &[PathBuf], skip: usize) -> Vec<Option<String>> {
    inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            (index != skip)
                .then(|| codes(&json_run(&[], std::slice::from_ref(input)).json()).concat())
        })
        .collect()
}

fn others_match_alone(mix: &Value, alone: &[Option<String>]) -> bool {
    let mixed = codes(mix);
    mixed.len() == alone.len()
        && mixed
            .iter()
            .zip(alone)
            .all(|(code, single)| single.as_ref().is_none_or(|single| single == code))
}

pub fn a_deep_input_among_fixtures(rows: &mut Rows, scratch: &Scratch) {
    let deep = scratch.file("deep.js", &generate(Class::Array, 200_000));
    let inputs = mix_with(&deep, DEEP_INDEX);
    let wide = json_run(&["-j", "4"], &inputs);
    let narrow = json_run(&["-j", "1"], &inputs);
    let alone = alone_codes(&inputs, DEEP_INDEX);
    let value = wide.json();
    rows.check(
        "-j 4 refuses the deep input alone",
        wide.code() == Some(3) && refused(&value) == Some(vec![DEEP_INDEX as u64]),
        || wide.summary(),
    );
    rows.check(
        "the others equal their own runs",
        others_match_alone(&value, &alone),
        || "a fixture's output differs from its single-file output".to_string(),
    );
    rows.check(
        "-j 1 and -j 4 give the same JSON",
        without_timings(value) == without_timings(narrow.json()),
        || narrow.summary(),
    );
}

pub fn a_refused_ladder_stays_silent(rows: &mut Rows, scratch: &Scratch) {
    let input = scratch.file("d11.js", &generate(Class::Plus, 5_000));
    let two = untwist(
        &argv(&[], &[&input]),
        &[("UNTWIST_TEST_LADDER_REFUSE", "2")],
    );
    rows.check(
        "two refused rungs",
        two.code() == Some(0) && two.outcome.stderr.is_empty(),
        || two.summary(),
    );
    let every = untwist(
        &argv(&[], &[&input]),
        &[("UNTWIST_TEST_LADDER_REFUSE", "4")],
    );
    rows.check(
        "every rung refused",
        matches!(every.code(), Some(0 | 3)) && !every.died(),
        || every.summary(),
    );
}

fn plus_files(scratch: &Scratch, count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|index| {
            let source = format!("var p{index} = 1;\n{}", generate(Class::Plus, PLUS_TERMS));
            scratch.file(&format!("plus{index}.js"), &source)
        })
        .collect()
}

pub fn many_deep_inputs_share_the_reserve(rows: &mut Rows, scratch: &Scratch) {
    let inputs = plus_files(scratch, 8);
    let single = json_run(&[], &inputs[..1]);
    let wide = json_run(&["-j", "8"], &inputs);
    let narrow = json_run(&["-j", "1"], &inputs);
    let bound = (1 << 30) + 8 * single.peak_bytes();
    rows.check(
        "-j 8 runs within the reserve",
        wide.code() == Some(0) && !wide.died() && wide.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", wide.summary()),
    );
    rows.check(
        "-j 8 equals -j 1",
        codes(&wide.json()) == codes(&narrow.json()),
        || narrow.summary(),
    );
}

pub fn unique_rename_cannot_deadlock(rows: &mut Rows, scratch: &Scratch) {
    let inputs = plus_files(scratch, 4);
    let narrow = json_run(&["-j", "1", "--rename", "unique"], &inputs);
    for attempt in 0..5 {
        let wide = json_run(&["-j", "4", "--rename", "unique"], &inputs);
        let holds = wide.code() == Some(0)
            && !wide.outcome.timed_out
            && codes(&wide.json()) == codes(&narrow.json());
        rows.check(&format!("attempt {attempt}"), holds, || wide.summary());
    }
    let deep = scratch.file("deep0.js", &generate(Class::Array, 200_000));
    let mut refused_first = vec![deep];
    refused_first.extend(inputs.into_iter().take(3));
    let run = json_run(&["-j", "4", "--rename", "unique"], &refused_first);
    rows.check(
        "a refused input at index 0 gives up its turn",
        run.code() == Some(3) && !run.outcome.timed_out && refused(&run.json()) == Some(vec![0]),
        || run.summary(),
    );
}

fn bundle_mix() -> Vec<PathBuf> {
    let mut inputs = deobfuscate_fixtures();
    inputs.push(workspace_file("fixtures/reference/unpack/webpack-4.js"));
    inputs
}

pub fn refused_text_is_never_parsed(rows: &mut Rows, scratch: &Scratch) {
    let cases = [
        ("openarr", generate(Class::Openarr, MILLION)),
        ("array", generate(Class::Array, MILLION)),
        ("arrowobj", generate(Class::Arrowobj, 4_000)),
        ("repeatchain(25)", repeatchain(25)),
    ];
    let base = json_run(&["-j", "4"], &bundle_mix());
    let bundle = base.json().get("bundle").cloned();
    rows.check("the mix has a bundle", bundle.is_some(), || base.summary());
    for (name, source) in cases {
        let deep = scratch.file("d13.js", &source);
        let alone = untwist(&argv(&[], &[&deep]), &[]);
        rows.check(
            &format!("{name} alone"),
            alone.code() == Some(3) && !alone.died(),
            || alone.summary(),
        );
        let mut inputs = bundle_mix();
        inputs.insert(DEEP_INDEX, deep);
        let mixed = json_run(&["-j", "4"], &inputs);
        let value = mixed.json();
        let holds = mixed.code() == Some(3)
            && !mixed.died()
            && refused(&value) == Some(vec![DEEP_INDEX as u64])
            && value.get("bundle").cloned() == bundle;
        rows.check(&format!("{name} in a -j 4 mix"), holds, || mixed.summary());
    }
}

struct Excess {
    runs: Vec<Run>,
    baseline: u64,
    subject: u64,
}

impl Excess {
    fn over(small: &Path, subject: &Path) -> Self {
        let mut runs = Vec::with_capacity(BASELINE_PAIRS);
        let mut baseline = u64::MAX;
        for _ in 0..BASELINE_PAIRS {
            baseline = baseline.min(untwist(&argv(&[], &[small]), &[]).peak_bytes());
            runs.push(untwist(&argv(&[], &[subject]), &[]));
        }
        let subject = runs.iter().map(Run::peak_bytes).min().unwrap_or(u64::MAX);
        Self {
            runs,
            baseline,
            subject,
        }
    }

    fn excess(&self) -> u64 {
        self.subject.saturating_sub(self.baseline)
    }

    fn holds(&self, bound: u64) -> bool {
        self.runs.iter().all(|run| run.code() == Some(0)) && self.excess() <= bound
    }

    fn summary(&self, bound: u64) -> String {
        let runs: Vec<String> = self.runs.iter().map(Run::summary).collect();
        format!(
            "least subject peak {} B minus least baseline peak {} B is {} B, bound {bound} B; {}",
            self.subject,
            self.baseline,
            self.excess(),
            runs.join("; ")
        )
    }
}

pub fn the_cumulative_speculative_bound(rows: &mut Rows, scratch: &Scratch) {
    let small = scratch.file("plus500.js", &generate(Class::Plus, 500));
    let one_chain = PAIR_BYTES * CHAIN_PAIRS;
    let source = repeatchain(1);
    let input = scratch.file("rc1.js", &source);
    let excess = Excess::over(&small, &input);
    rows.check("repeatchain(1) runs", excess.holds(one_chain), || {
        excess.summary(one_chain)
    });
    let input = scratch.file("rcw.js", &format!("{source}with(a){{}}"));
    let excess = Excess::over(&small, &input);
    rows.check(
        "repeatchain(1) then a script-only statement",
        excess.holds(2 * one_chain),
        || excess.summary(2 * one_chain),
    );
    let source = repeatchain(2);
    let input = scratch.file("rc2.js", &source);
    let run = untwist(&argv(&[], &[&input]), &[]);
    refuses_with(rows, "repeatchain(2)", &run, &source, PARENTHESES_TOO_DEEP);
    for k in [25, 100] {
        let source = repeatchain(k);
        let input = scratch.file("rck.js", &source);
        let run = untwist(&argv(&[], &[&input]), &[]);
        refuses_with(
            rows,
            &format!("repeatchain({k})"),
            &run,
            &source,
            PARENTHESES_TOO_DEEP,
        );
        let quick = run.peak_bytes() < SMALL_PEAK && run.elapsed < QUICK;
        rows.check(&format!("repeatchain({k}) is quick"), quick, || {
            run.summary()
        });
    }
}

fn limited_json_run(jobs: &str, inputs: &[PathBuf]) -> Run {
    let mut args: Vec<OsString> = vec![AS_LIMIT.into(), super::harness::binary().into()];
    args.extend(["-j", jobs, "--report", "json"].map(OsString::from));
    args.extend(inputs.iter().map(|path| path.as_os_str().to_owned()));
    run_with(
        Path::new("/usr/bin/prlimit"),
        &args,
        &[],
        None,
        Cap::DEFAULT,
    )
}

pub fn the_rung_does_not_depend_on_jobs(rows: &mut Rows, scratch: &Scratch) {
    let deep = scratch.file("deep.js", &generate(Class::Array, 200_000));
    let between = scratch.file("between.js", &generate(Class::Array, 30_000));
    let mut inputs = mix_with(&deep, DEEP_INDEX);
    inputs.push(between);
    let narrow = limited_json_run("1", &inputs);
    let wide = limited_json_run("4", &inputs);
    let (narrow_json, wide_json) = (narrow.json(), wide.json());
    rows.check(
        "under a 1500 MiB address space, -j 1 refuses only the deep input",
        narrow.code() == Some(3) && refused(&narrow_json) == Some(vec![DEEP_INDEX as u64]),
        || narrow.summary(),
    );
    rows.check(
        "-j 4 equals -j 1 under the same limit",
        wide.code() == narrow.code() && without_timings(wide_json) == without_timings(narrow_json),
        || wide.summary(),
    );
}
