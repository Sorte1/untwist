use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use untwist::{UNPACK_PANICKED, WORKER_DIED};
use untwist_testing::capped_child::Cap;
use untwist_testing::depth_classes::{generate, Class};

use super::harness::{
    argv, binary, codes, deobfuscate_fixtures, passed_through, refused, run_with, untwist,
    untwist_stdin, without_timings, workspace_file, Rows, Run, Scratch,
};

const SCAN_OFF: (&str, &str) = ("UNTWIST_TEST_SCAN_OFF", "1");
pub const OFF: (&str, &str) = ("UNTWIST_TEST_SUPERVISOR", "off");
const HIDDEN_DEPTH: usize = 700_000;
pub const HIGH: &str = "fixtures/reference/deobfuscate/obfuscator.io-high.js";
const MEMORY_KILL_CAP: Cap = Cap {
    memory_max: "1G",
    timeout_s: 120,
};
pub const FALLBACK: &str = "warning: could not start a worker process (";

fn hidden_nesting() -> String {
    let parens = format!("{}x{}", "(".repeat(HIDDEN_DEPTH), ")".repeat(HIDDEN_DEPTH));
    format!("import a from 'a' assert {{type: 'json'}}\n/`/;\n{parens}\n;/`/")
}

pub fn died_warning(index: usize) -> String {
    format!("warning: input {index} was not processed: {WORKER_DIED}")
}

fn refused_by_the_supervisor(rows: &mut Rows, label: &str, run: &Run, input: &str) {
    let holds = run.code() == Some(3)
        && !run.died()
        && passed_through(run.stdout(), input)
        && run.stderr().contains(&died_warning(1));
    rows.check(label, holds, || run.summary());
}

pub fn a_stack_overflow_refuses_the_input(rows: &mut Rows, scratch: &Scratch) {
    let source = hidden_nesting();
    let input = scratch.file("hidden.js", &source);
    let run = untwist(&argv(&[], &[&input]), &[SCAN_OFF]);
    refused_by_the_supervisor(rows, "stack overflow", &run, &source);
    let json = untwist(&argv(&["--report", "json"], &[&input]), &[SCAN_OFF]);
    let value = json.json();
    rows.check(
        "json",
        json.code() == Some(3)
            && refused(&value) == Some(vec![0])
            && value.get("panicked").is_none(),
        || json.summary(),
    );
    let dir = scratch.path("i1-out");
    let written = untwist(&argv(&["-o"], &[&dir, &input]), &[SCAN_OFF]);
    let file = std::fs::read_to_string(dir.join("deobfuscated.js")).unwrap_or_default();
    rows.check("-o", written.code() == Some(3) && file == source, || {
        written.summary()
    });
}

pub fn an_abort_and_a_memory_kill_refuse_the_input(rows: &mut Rows, scratch: &Scratch) {
    let high = workspace_file(HIGH);
    let source = std::fs::read_to_string(&high).unwrap_or_default();
    let run = untwist(&argv(&[], &[&high]), &[("UNTWIST_TEST_WORKER_ABORT", "0")]);
    refused_by_the_supervisor(rows, "abort", &run, &source);
    let source = generate(Class::Arrowobj, 4_000);
    let input = scratch.file("arrowobj.js", &source);
    let run = run_with(
        &binary(),
        &argv(&[], &[&input]),
        &[SCAN_OFF],
        None,
        MEMORY_KILL_CAP,
    );
    refused_by_the_supervisor(rows, "memory kill", &run, &source);
}

pub fn json_run(flags: &[&str], inputs: &[PathBuf], env: &[(&str, &str)]) -> Run {
    let files: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
    let mut all = vec!["--report", "json"];
    all.extend_from_slice(flags);
    untwist(&argv(&all, &files), env)
}

pub fn under_four_jobs_only_the_dying_input_is_refused(rows: &mut Rows, scratch: &Scratch) {
    let hidden = scratch.file("hidden.js", &hidden_nesting());
    let mut inputs = deobfuscate_fixtures();
    inputs.insert(2, hidden.clone());
    let wide = json_run(&["-j", "4"], &inputs, &[SCAN_OFF]);
    let narrow = json_run(&["-j", "1"], &inputs, &[SCAN_OFF]);
    let wide_json = wide.json();
    rows.check(
        "-j 4 refuses exactly the dying input",
        wide.code() == Some(3) && refused(&wide_json) == Some(vec![2]),
        || wide.summary(),
    );
    let alone_ok = inputs
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != 2)
        .all(|(index, input)| {
            let alone = codes(&json_run(&[], std::slice::from_ref(input), &[]).json()).concat();
            codes(&wide_json).get(index) == Some(&alone)
        });
    rows.check("the others equal their own runs", alone_ok, || {
        wide.summary()
    });
    rows.check(
        "-j 4 JSON equals -j 1",
        without_timings(wide_json) == without_timings(narrow.json()),
        || narrow.summary(),
    );
    two_deaths_and_a_unique_rename(rows, hidden);
}

fn two_deaths_and_a_unique_rename(rows: &mut Rows, hidden: PathBuf) {
    let fixtures = deobfuscate_fixtures();
    let two = json_run(
        &["-j", "4"],
        &fixtures,
        &[("UNTWIST_TEST_WORKER_ABORT", "1,3")],
    );
    rows.check(
        "two aborting inputs",
        refused(&two.json()) == Some(vec![1, 3]),
        || two.summary(),
    );
    let mut first = vec![hidden];
    first.extend(fixtures.into_iter().take(4));
    let unique = json_run(&["-j", "4", "--rename", "unique"], &first, &[SCAN_OFF]);
    rows.check(
        "a dying input at index 0 under --rename unique",
        unique.code() == Some(3)
            && !unique.outcome.timed_out
            && refused(&unique.json()) == Some(vec![0]),
        || unique.summary(),
    );
}

pub fn stdin_input_dies(rows: &mut Rows) {
    let source = hidden_nesting();
    let run = untwist_stdin(&[], &[SCAN_OFF], source.as_bytes());
    refused_by_the_supervisor(rows, "stdin", &run, &source);
    let high = workspace_file(HIGH);
    let bytes = std::fs::read(&high).unwrap_or_default();
    let piped = untwist_stdin(&[], &[], &bytes);
    let named = untwist(&argv(&[], &[&high]), &[]);
    rows.check(
        "stdin equals the path",
        same_streams(&piped, &named),
        || piped.summary(),
    );
}

pub fn same_streams(left: &Run, right: &Run) -> bool {
    left.code() == right.code()
        && left.outcome.stdout == right.outcome.stdout
        && left.outcome.stderr == right.outcome.stderr
}

pub fn without_first_line(stderr: &str) -> Option<&str> {
    stderr
        .strip_prefix(FALLBACK)?
        .split_once('\n')
        .map(|(_, rest)| rest)
}

pub fn the_fallback_runs_in_process(rows: &mut Rows) {
    let high = workspace_file(HIGH);
    let bytes = std::fs::read(&high).unwrap_or_default();
    let supervised = untwist(&argv(&[], &[&high]), &[]);
    for hook in ["fail-exe", "fail-spawn"] {
        let env = [("UNTWIST_TEST_SUPERVISOR", hook)];
        for run in [
            untwist(&argv(&[], &[&high]), &env),
            untwist_stdin(&[], &env, &bytes),
        ] {
            let stderr = run.stderr();
            let holds = run.code() == supervised.code()
                && run.outcome.stdout == supervised.outcome.stdout
                && without_first_line(&stderr) == Some(supervised.stderr().as_str());
            rows.check(&format!("{hook}"), holds, || run.summary());
        }
    }
}

pub fn normal_exits_are_not_crashes(rows: &mut Rows, scratch: &Scratch) {
    let broken = scratch.file("broken.js", "var = ;");
    let supervised = untwist(&argv(&[], &[&broken]), &[]);
    let off = untwist(&argv(&[], &[&broken]), &[OFF]);
    rows.check(
        "parse errors",
        supervised.code() == Some(1) && same_streams(&supervised, &off),
        || supervised.summary(),
    );
    let normal = scratch.file("normal.js", "var a = 1;");
    let dir = scratch.path("i7-out");
    let _ = untwist(&argv(&["-o"], &[&dir, &normal]), &[]);
    let again = untwist(&argv(&["-o"], &[&dir, &normal]), &[]);
    rows.check("an existing -o DIR", again.code() == Some(2), || {
        again.summary()
    });
    let spawnless = [("UNTWIST_TEST_SUPERVISOR", "fail-spawn")];
    for flags in [&["--no-such-flag"][..], &["--help"], &["--version"]] {
        let run = untwist(&argv(flags, &[]), &spawnless);
        let off = untwist(&argv(flags, &[]), &[OFF]);
        rows.check(
            &format!("{flags:?} starts no worker"),
            same_streams(&run, &off),
            || run.summary(),
        );
    }
}

pub fn unpack_and_emit_deaths(rows: &mut Rows) {
    let inputs = vec![
        workspace_file("fixtures/reference/unpack/webpack-4.js"),
        workspace_file("fixtures/reference/deobfuscate/obfuscator.io.js"),
    ];
    let run = json_run(&[], &inputs, &[("UNTWIST_TEST_WORKER_ABORT", "unpack")]);
    let value = run.json();
    let noted = value
        .get("notes")
        .and_then(|notes| notes.as_array())
        .is_some_and(|notes| {
            notes
                .iter()
                .any(|note| note.as_str() == Some(&format!("pipeline: {UNPACK_PANICKED}")))
        });
    let holds = run.code() == Some(1)
        && noted
        && value.get("unpack_panicked").is_some()
        && value.get("bundle").is_none()
        && codes(&value).len() == 2;
    rows.check("a death in unpack", holds, || run.summary());
    let high = workspace_file(HIGH);
    let run = untwist(
        &argv(&[], &[&high]),
        &[("UNTWIST_TEST_WORKER_ABORT", "emit")],
    );
    let holds = run.code() == Some(1)
        && run.outcome.stdout.is_empty()
        && run
            .stderr()
            .contains("error: the worker process died while writing its output (signal 6)");
    rows.check("a death while writing", holds, || run.summary());
}

fn every_fixture() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![workspace_file("fixtures")];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "js") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn files_under(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
            if path.is_dir() {
                pending.push(path);
            } else if let (Ok(bytes), Ok(relative)) = (std::fs::read(&path), path.strip_prefix(dir))
            {
                files.insert(relative.to_path_buf(), bytes);
            }
        }
    }
    files
}

fn written(
    scratch: &Scratch,
    name: &str,
    flags: &[&str],
    input: &Path,
    env: &[(&str, &str)],
) -> (Run, BTreeMap<PathBuf, Vec<u8>>) {
    let dir = scratch.path(name);
    let mut all = vec!["-f", "-o"];
    all.extend_from_slice(flags);
    let mut args: Vec<OsString> = all.iter().map(OsString::from).collect();
    args.insert(2, dir.as_os_str().to_owned());
    args.push(input.as_os_str().to_owned());
    let run = untwist(&args, env);
    (run, files_under(&dir))
}

pub fn the_supervised_run_equals_the_in_process_run(rows: &mut Rows, scratch: &Scratch) {
    for input in every_fixture() {
        for flags in [&[][..], &["--unsafe"]] {
            let label = format!("{} {flags:?}", input.display());
            let supervised = untwist(&argv(flags, &[&input]), &[]);
            let off = untwist(&argv(flags, &[&input]), &[OFF]);
            rows.check(&label, same_streams(&supervised, &off), || {
                supervised.summary()
            });
            let (left, left_files) = written(scratch, "supervised", flags, &input, &[]);
            let (right, right_files) = written(scratch, "in-process", flags, &input, &[OFF]);
            let same = same_streams(&left, &right) && left_files == right_files;
            rows.check(&format!("{label} -o"), same, || left.summary());
        }
    }
    reports_are_identical(rows, scratch);
}

fn interleaved(flags: &[&str], input: &Path, env: &[(&str, &str)]) -> Run {
    let mut args: Vec<OsString> = vec![
        "-c".into(),
        "exec \"$0\" \"$@\" 2>&1".into(),
        binary().into(),
    ];
    args.extend(flags.iter().map(OsString::from));
    args.push(input.as_os_str().to_owned());
    run_with(Path::new("/bin/sh"), &args, env, None, Cap::DEFAULT)
}

fn reports_are_identical(rows: &mut Rows, scratch: &Scratch) {
    for input in deobfuscate_fixtures() {
        let json = |env: &[(&str, &str)]| untwist(&argv(&["--report", "json"], &[&input]), env);
        let (supervised, off) = (json(&[]), json(&[OFF]));
        let same = supervised.code() == off.code()
            && supervised.outcome.stderr == off.outcome.stderr
            && without_timings(supervised.json()) == without_timings(off.json());
        rows.check(&format!("json {}", input.display()), same, || {
            supervised.summary()
        });
        let text = |env: &[(&str, &str)]| untwist(&argv(&["--report", "text"], &[&input]), env);
        let (supervised, off) = (text(&[]), text(&[OFF]));
        let same = super::channel::same_text_reports(&supervised, &off);
        rows.check(&format!("text {}", input.display()), same, || {
            supervised.summary()
        });
    }
    let broken = scratch.file("interleaved.js", "var a = 1;\nvar = ;\nvar b = !0;\n");
    let merged = interleaved(&[], &broken, &[]);
    let merged_off = interleaved(&[], &broken, &[OFF]);
    rows.check(
        "one pipe for both streams",
        merged.code() == merged_off.code() && merged.outcome.stdout == merged_off.outcome.stdout,
        || merged.summary(),
    );
}
