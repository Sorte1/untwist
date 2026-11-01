use std::path::{Path, PathBuf};

use serde_json::Value;
use untwist::NESTED_TOO_DEEP;
use untwist_testing::depth::{limits_for, scan_as, Goal};
use untwist_testing::depth_classes::{generate, Class};

use super::harness::{argv, codes, passed_through, untwist, Rows, Run, Scratch, MB};

const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;
const BUDGET_SPENT: &str = "heap budget is spent";
const AMPLIFIED_SITES: usize = 1_000;
const A_GIB_OF_ROOM: usize = 1 << 30;

pub fn unit(suffix: &str, array: &str, body: &str) -> String {
    format!(
        "function arr{suffix}() {{ var h = {array}; arr{suffix} = function () {{ return h; }}; \
         return arr{suffix}(); }}\n\
         function dec{suffix}(c, d) {{\nvar e = arr{suffix}();\n\
         return (dec{suffix} = function (f, g) {{\nf = f - 0;\nvar i = e[f];\n{body}\n}}), \
         dec{suffix}(c, d);\n}}\n"
    )
}

pub fn code_of(run: &Run) -> String {
    codes(&run.json()).concat()
}

pub fn budget_notes(json: &Value) -> usize {
    json.get("notes")
        .and_then(Value::as_array)
        .map_or(0, |notes| {
            notes
                .iter()
                .filter_map(Value::as_str)
                .filter(|note| note.contains(BUDGET_SPENT))
                .count()
        })
}

pub fn json_run(scratch: &Scratch, name: &str, source: &str, flags: &[&str]) -> Run {
    let input = scratch.file(name, source);
    let mut all = vec!["--report", "json"];
    all.extend_from_slice(flags);
    untwist(&argv(&all, &[&input]), &[])
}

pub fn string_chains(rows: &mut Rows, scratch: &Scratch) {
    let input = scratch.file("m1.js", &generate(Class::Strplus, 40_000));
    let bare = ["--no-unpack", "--no-deobfuscate", "--no-jsx"];
    let run = untwist(&argv(&bare, &[&input]), &[]);
    rows.check(
        "strplus 40000 without deobfuscate stays under 100 MB",
        run.code() == Some(0) && !run.died() && run.peak_bytes() < 100 * MB,
        || run.summary(),
    );
    let input = scratch.file("m2.js", &generate(Class::Strplus, 90_000));
    let run = untwist(&argv(&[], &[&input]), &[]);
    rows.check(
        "strplus 90000 runs under 1 GB",
        run.code() == Some(0) && !run.died() && run.peak_bytes() < 1_000 * MB,
        || run.summary(),
    );
    let input = scratch.file("m2b.js", &generate(Class::Strplus, 100_000));
    let run = untwist(&argv(&[], &[&input]), &[]);
    let refused = run.code() == Some(3) && run.stderr().contains(NESTED_TOO_DEEP);
    rows.check(
        "strplus 100000 runs or is refused, never killed",
        (run.code() == Some(0) || refused) && !run.died(),
        || run.summary(),
    );
    let source = generate(Class::Strplus, 1_000_000);
    let input = scratch.file("m2c.js", &source);
    let run = untwist(&argv(&[], &[&input]), &[]);
    rows.check(
        "strplus 1000000 is refused as too deep",
        run.code() == Some(3)
            && !run.died()
            && run.stderr().contains(NESTED_TOO_DEEP)
            && passed_through(run.stdout(), &source),
        || run.summary(),
    );
}

fn deep_sink_unit(text_body: &str) -> String {
    let body = format!("return f === 0 ? String({text_body}) : i;");
    unit("", "['[', ']']", &body) + "Function(dec(0));\nconsole.log(dec(1));\n"
}

pub fn a_deep_decoded_text_refuses_its_unit(rows: &mut Rows, scratch: &Scratch) {
    let source = deep_sink_unit("i['repeat'](200000) + e[1]['repeat'](200000)");
    let run = json_run(scratch, "m6.js", &source, &[]);
    rows.check(
        "a decoded text of array-200000 refuses its unit",
        run.code() == Some(0) && !run.died() && code_of(&run).contains("dec(0)"),
        || run.summary(),
    );
}

fn amplified(sites: usize) -> String {
    let body = "return i['repeat'](4096)['repeat'](4000);";
    unit("", "['x']", body) + &"console.log(dec(0).length);\n".repeat(sites)
}

pub fn encoded_sites_form_a_suffix(code: &str) -> (usize, usize, bool) {
    let sites: Vec<bool> = code
        .lines()
        .filter(|line| line.starts_with("console.log("))
        .map(|line| line.contains("dec(0)"))
        .collect();
    let encoded = sites.iter().filter(|encoded| **encoded).count();
    let first_encoded = sites.iter().position(|encoded| *encoded);
    let suffix = first_encoded.is_none_or(|at| sites[at..].iter().all(|encoded| *encoded));
    (sites.len() - encoded, encoded, suffix)
}

pub fn one_heap_budget_ends_decoding_partway(rows: &mut Rows, scratch: &Scratch) {
    let one = scratch.file("m8-one.js", &amplified(1));
    let baseline = untwist(&argv(&[], &[&one]), &[]);
    let run = json_run(scratch, "m8.js", &amplified(AMPLIFIED_SITES), &[]);
    let json = run.json();
    let (decoded, encoded, suffix) = encoded_sites_form_a_suffix(&code_of(&run));
    let bound = 64 * MIB + 32 * MB + baseline.peak_bytes();
    rows.check(
        "1000 amplified sites stay inside the heap budget",
        run.code() == Some(0) && !run.died() && run.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", run.summary()),
    );
    rows.check(
        "at most two sites decode and the rest stay encoded",
        (1..=2).contains(&decoded)
            && decoded + encoded == AMPLIFIED_SITES
            && suffix
            && budget_notes(&json) == 1,
        || format!("decoded {decoded}, encoded {encoded}, suffix {suffix}"),
    );
}

pub fn two_units_share_one_budget(rows: &mut Rows, scratch: &Scratch) {
    let body = "var t = i['repeat'](4096)['repeat'](4000); return t + '';";
    let first = unit("1", "['x']", body);
    let second = unit("2", "['y']", body);
    let calls = "console.log(dec1(0).length);\nconsole.log(dec2(0).length);\n";
    let run = json_run(scratch, "m9.js", &format!("{first}{second}{calls}"), &[]);
    let code = code_of(&run);
    rows.check(
        "the first unit's retained value leaves too little for the second",
        run.code() == Some(0)
            && !run.died()
            && !code.contains("dec1(0)")
            && code.contains("dec2(0)")
            && budget_notes(&run.json()) == 1,
        || run.summary(),
    );
}

fn shared_reserve_inputs(scratch: &Scratch) -> Vec<PathBuf> {
    let source = deep_sink_unit("i['repeat'](45000) + e[1]['repeat'](45000)");
    (0..4)
        .map(|index| scratch.file(&format!("m10-{index}.js"), &source))
        .collect()
}

pub fn four_decoded_texts_share_the_reserve(rows: &mut Rows, scratch: &Scratch) {
    let inputs = shared_reserve_inputs(scratch);
    let baselines: u64 = inputs
        .iter()
        .map(|input| untwist(&argv(&[], &[input.as_path()]), &[]).peak_bytes())
        .sum();
    let files: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
    let run = untwist(&argv(&["-j", "4", "--report", "json"], &files), &[]);
    let bound = GIB + baselines;
    rows.check(
        "four decoded texts of array-45000 under -j 4",
        run.code() == Some(0) && !run.died() && run.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", run.summary()),
    );
    rows.check(
        "every input is written",
        codes(&run.json()).len() == 4,
        || run.summary(),
    );
}

pub fn the_scan_goal_matches_each_site(rows: &mut Rows, scratch: &Scratch) {
    let deep = format!("zz <!-- `\n{}1{}", "(".repeat(200_000), ")".repeat(200_000));
    let limits = limits_for(A_GIB_OF_ROOM, deep.len());
    rows.check(
        "the text is deep only as a script",
        scan_as(&deep, &limits, Goal::Module).is_ok()
            && scan_as(&deep, &limits, Goal::Script).is_err(),
        || "the goals do not tell the text apart".to_string(),
    );
    let body = "return f === 0 ? String(i + e[1]['repeat'](200000) + e[2] + \
                e[3]['repeat'](200000)) : i;";
    let source = unit("", "['zz <!-- `\\n', '(', '1', ')']", body)
        + "Function(dec(0));\nconsole.log(dec(1));\n";
    let run = json_run(scratch, "m11.js", &source, &[]);
    rows.check(
        "the escape site scans a decoded text as a script",
        run.code() == Some(0) && !run.died() && code_of(&run).contains("dec(0)"),
        || run.summary(),
    );
}

fn prefix_keeper(iterations: usize) -> String {
    let piece = "x".repeat(1_000);
    let body = format!(
        "var s = '', kept = []; for (var k = 0; k < {iterations}; k++) {{ s += i; \
         kept.push(s); }} return kept.length > 0 ? 'kept' : i;"
    );
    unit("", &format!("['{piece}']"), &body) + "console.log(dec(0));\n"
}

pub fn live_prefixes_bail_at_the_heap_budget(rows: &mut Rows, scratch: &Scratch) {
    let small = scratch.file("prefix-small.js", &prefix_keeper(10));
    let baseline = untwist(&argv(&[], &[&small]), &[]);
    rows.check(
        "live: ten kept prefixes decode",
        code_of(&json_run(
            scratch,
            "prefix-small.js",
            &prefix_keeper(10),
            &[],
        ))
        .contains("console.log(\"kept\")"),
        || baseline.summary(),
    );
    let run = json_run(scratch, "prefix.js", &prefix_keeper(100_000), &[]);
    let bound = 64 * MIB + baseline.peak_bytes();
    rows.check(
        "live: kept prefixes bail at the budget with peak RSS inside it",
        run.code() == Some(0)
            && !run.died()
            && code_of(&run).contains("dec(0)")
            && budget_notes(&run.json()) == 1
            && run.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", run.summary()),
    );
}
