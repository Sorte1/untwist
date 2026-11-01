use std::time::Duration;

use untwist_testing::depth::limits_for;

use super::harness::{argv, untwist, Rows, Run, Scratch};
use super::memory::{budget_notes, code_of, json_run, unit};

const MIB: u64 = 1 << 20;
const BUDGET: u64 = 64 * MIB;
const A_GIB_OF_ROOM: usize = 1 << 30;
const AMPLIFIED_SITES: usize = 1_000;
const CONSEQUENT_LEVELS: usize = 3_000;
const LOOP_FUEL: &str = "500000000";
const RECURSION_DEPTH: usize = 900;
const CONSEQUENT_WALL: Duration = Duration::from_secs(2);

fn cached_decoder(sites: usize) -> String {
    let head = "function a() { var h = ['Bg9N', 'sgvSBg8Gv29YBgqH', 'y29UC29Szq', \
                'zgvVyMz1C2nHDgu']; a = function () { return h; }; return a(); }\n\
                function b(c, d) {\nvar e = a();\nreturn (b = function (f, g) {\nf = f - 0;\n\
                var i = e[f];\nif (b['init'] === undefined) {\nvar atobPoly = function (input) \
                { return input['repeat'](1000)['repeat'](1000); };\nb['decoder'] = atobPoly;\n\
                b['cache'] = {};\nb['init'] = true;\n}\nvar first = e[0], key = f + first, \
                hit = b['cache'][key];\nreturn !hit ? ((i = b['decoder'](i)), \
                (b['cache'][key] = i)) : (i = hit), i;\n}), b(c, d);\n}\n";
    format!("{head}{}", "console.log(b(0).length);\n".repeat(sites))
}

fn decoded_prefix(code: &str) -> (usize, usize, bool) {
    let sites: Vec<bool> = code
        .lines()
        .filter(|line| line.starts_with("console.log("))
        .map(|line| line.contains("b(0)"))
        .collect();
    let encoded = sites.iter().filter(|encoded| **encoded).count();
    let first = sites.iter().position(|encoded| *encoded);
    let suffix = first.is_none_or(|at| sites[at..].iter().all(|encoded| *encoded));
    (sites.len() - encoded, encoded, suffix)
}

pub fn inlined_text_stays_inside_the_budget(rows: &mut Rows, scratch: &Scratch) {
    let one = scratch.file("amp-1.js", &cached_decoder(1));
    let baseline = untwist(&argv(&[], &[&one]), &[]);
    let source = cached_decoder(AMPLIFIED_SITES);
    let run = json_run(scratch, "amp-1000.js", &source, &[]);
    let code = code_of(&run);
    let (decoded, encoded, suffix) = decoded_prefix(&code);
    let bound = BUDGET + baseline.peak_bytes();
    rows.check(
        "a cached value inlined at 1000 sites stays inside the budget",
        run.code() == Some(0) && !run.died() && run.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", run.summary()),
    );
    rows.check(
        "the first sites decode, the rest stay encoded, with one note",
        decoded >= 1 && decoded + encoded == AMPLIFIED_SITES && suffix && {
            budget_notes(&run.json()) == 1
        },
        || format!("decoded {decoded}, encoded {encoded}, suffix {suffix}"),
    );
    let print_bytes = limits_for(A_GIB_OF_ROOM, source.len()).print_bytes;
    rows.check(
        "after deobfuscate: the printed text fits the input's print limit",
        (code.len() as u64) <= print_bytes && run.code() == Some(0),
        || format!("printed {} B, limit {print_bytes} B", code.len()),
    );
}

fn kept_items(outer: usize, inner: usize, item: &str) -> String {
    let body = format!(
        "var kept = []; for (var a = 0; a < {outer}; a++) {{ var inner = []; kept.push(inner); \
         for (var b = 0; b < {inner}; b++) {{ var k = a * {inner} + b; inner.push({item}); }} }} \
         return 'done' + i;"
    );
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

fn function_object() -> String {
    let members: Vec<String> = (0..100).map(|n| format!("p{n}:function(){{}}")).collect();
    format!("{{{}}}", members.join(","))
}

const CLOSURE_FACTORY: &str = "(function (q) { return function () { return q; }; })(k)";

fn bails_inside_the_budget(
    rows: &mut Rows,
    scratch: &Scratch,
    label: &str,
    (small, large): (&str, &str),
) {
    let baseline = json_run(scratch, &format!("{label}-small.js"), small, &[]);
    rows.check(
        &format!("{label}: the small input decodes"),
        baseline.code() == Some(0) && !code_of(&baseline).contains("dec(0)"),
        || baseline.summary(),
    );
    let run = json_run(scratch, &format!("{label}.js"), large, &[]);
    let bound = BUDGET + baseline.peak_bytes();
    rows.check(
        &format!("{label}: bails at the heap budget with peak RSS inside it"),
        run.code() == Some(0)
            && !run.died()
            && code_of(&run).contains("dec(0)")
            && budget_notes(&run.json()) == 1
            && run.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", run.summary()),
    );
}

pub fn objects_closures_and_scopes_are_charged(rows: &mut Rows, scratch: &Scratch) {
    let object = function_object();
    let pairs = [
        (
            "g4-objfn 1000x10000",
            kept_items(1, 10, &object),
            kept_items(1_000, 10_000, &object),
        ),
        (
            "g3-nest 1M",
            kept_items(1, 10, CLOSURE_FACTORY),
            kept_items(100, 10_000, CLOSURE_FACTORY),
        ),
    ];
    for (label, small, large) in &pairs {
        bails_inside_the_budget(rows, scratch, label, (small, large));
    }
}

fn kept_calls(count: usize, prelude: &str, kept: &str) -> String {
    let body = format!(
        "{prelude} var kept = []; for (var k = 0; k < {count}; k++) {{ kept.push({kept}); }} \
         return 'done' + i;"
    );
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

pub fn parsed_arrays_and_arguments_are_charged(rows: &mut Rows, scratch: &Scratch) {
    let json_array = "var t = '[' + '0,'['repeat'](1000000) + '0]';";
    let json_nest = |count: usize| format!("var t = '[' + '[],'['repeat']({count}) + '[]]';");
    let args = "var big = []; for (var j = 0; j < 90000; j++) big.push(j); \
                function g() { return arguments; }";
    let pairs = [
        (
            "jsonarr",
            kept_calls(1, json_array, "JSON.parse(t)"),
            kept_calls(40, json_array, "JSON.parse(t)"),
        ),
        (
            "jsonnest",
            kept_calls(1, &json_nest(3_000), "JSON.parse(t)"),
            kept_calls(40, &json_nest(300_000), "JSON.parse(t)"),
        ),
        (
            "args3k",
            kept_calls(1, args, "g.apply(null, big)"),
            kept_calls(3_000, args, "g.apply(null, big)"),
        ),
    ];
    for (label, small, large) in &pairs {
        bails_inside_the_budget(rows, scratch, label, (small, large));
    }
}

fn inside_the_budget(
    rows: &mut Rows,
    scratch: &Scratch,
    label: &str,
    (small, large): (&str, &str),
) {
    let baseline = json_run(scratch, &format!("{label}-small.js"), small, &[]);
    let run = json_run(scratch, &format!("{label}.js"), large, &[]);
    let bound = BUDGET + baseline.peak_bytes();
    rows.check(
        &format!("{label}: peak RSS stays inside the heap budget"),
        run.code() == Some(0) && !run.died() && run.peak_bytes() <= bound,
        || format!("{} (bound {bound} B)", run.summary()),
    );
}

pub fn small_strings_carry_their_header(rows: &mut Rows, scratch: &Scratch) {
    let split = "var s = i['repeat'](1000000);";
    let chars = |outer: usize| {
        let body = format!(
            "var s = i['repeat'](1000), kept = [], inner; for (var a = 0; a < {outer}; a++) \
             {{ kept.push(inner = []); for (var b = 0; b < 10000; b++) {{ \
             inner.push(s[b % 1000]); }} }} return 'done' + i;"
        );
        unit("", "['x']", &body) + "console.log(dec(0));\n"
    };
    inside_the_budget(
        rows,
        scratch,
        "splitc",
        (
            &kept_calls(1, split, "s.split('')"),
            &kept_calls(40, split, "s.split('')"),
        ),
    );
    inside_the_budget(rows, scratch, "charsk", (&chars(1), &chars(400)));
}

fn decodes(rows: &mut Rows, scratch: &Scratch, label: &str, source: &str, flags: &[&str]) {
    let run = json_run(scratch, &format!("{label}.js"), source, flags);
    rows.check(
        &format!("{label} decodes"),
        run.code() == Some(0)
            && !run.died()
            && !code_of(&run).contains("dec(0)")
            && budget_notes(&run.json()) == 0,
        || run.summary(),
    );
}

fn looped(body: &str) -> String {
    let body = format!(
        "var q = []; for (var a = 0; a < 1000; a++) for (var b = 0; b < 10000; b++) {{ {body} }} \
         return 'done' + i;"
    );
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

pub fn shrinks_and_projections_are_credited(rows: &mut Rows, scratch: &Scratch) {
    let fuel = ["--fuel", LOOP_FUEL];
    let zeroed = looped("q.push(b); q.length = 0;");
    decodes(
        rows,
        scratch,
        "a 10M push and length-zero loop",
        &zeroed,
        &fuel,
    );
    let popped = looped("q.push(b); q.pop();");
    decodes(rows, scratch, "a 10M push and pop loop", &popped, &fuel);
    let body = "var s = i['repeat'](1500000); var a = encodeURIComponent(s), \
                b = encodeURIComponent(s + 'y'), c = encodeURIComponent(s + 'z'); \
                return 'donex' + a.length + b.length + c.length;";
    decodes(
        rows,
        scratch,
        "enc",
        &(unit("", "['x']", body) + "console.log(dec(0));\n"),
        &[],
    );
}

fn consequent_chain(levels: usize) -> String {
    format!("{}b{};\n", "a ? ".repeat(levels), " : c".repeat(levels))
}

fn lifted_whole(run: &Run, levels: usize) -> bool {
    let code = String::from_utf8_lossy(run.stdout());
    code.matches("if (a)").count() == levels && !code.contains('?')
}

pub fn a_consequent_chain_lifts_in_one_visit(rows: &mut Rows, scratch: &Scratch) {
    let input = scratch.file("cons3k.js", &consequent_chain(CONSEQUENT_LEVELS));
    let run = untwist(&argv(&[], &[&input]), &[]);
    rows.check(
        "a 3000-level consequent chain lifts whole in under 2 s",
        run.code() == Some(0)
            && !run.died()
            && run.elapsed < CONSEQUENT_WALL
            && lifted_whole(&run, CONSEQUENT_LEVELS),
        || run.summary(),
    );
}

fn deep_vars(vars: usize, depth: usize) -> String {
    let names: Vec<String> = (0..vars).map(|n| format!("v{n}")).collect();
    let body = format!(
        "function r(n) {{ var {}; if (n > 0) return r(n - 1); return 0; }} r({depth}); \
         return 'done' + i;",
        names.join(",")
    );
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

fn parsed_objects(count: usize) -> String {
    let body = format!(
        "var t = '[' + '{{}},'['repeat']({}) + '{{}}]'; var v = JSON.parse(t); \
         return 'done' + i + v.length;",
        count - 1
    );
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

pub fn frames_and_native_loops_are_bounded(rows: &mut Rows, scratch: &Scratch) {
    for vars in [20_000, 100_000] {
        let label = format!("recursive var-{vars}");
        bails_inside_the_budget(
            rows,
            scratch,
            &label,
            (&deep_vars(vars, 0), &deep_vars(vars, RECURSION_DEPTH)),
        );
    }
    bails_inside_the_budget(
        rows,
        scratch,
        "JSON.parse of 1M objects",
        (&parsed_objects(1_000), &parsed_objects(1_000_000)),
    );
}

fn nested_arrows(count: usize) -> String {
    let body = format!(
        "var big = []; for (var k = 0; k < {count}; k++) big.push(k); var o = null; \
         big.forEach(x => big.forEach(y => {{ o = {{a: o}}; }})); return 'done' + i;"
    );
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

fn wide_names(vars: usize) -> String {
    (0..vars)
        .map(|n| format!("v{n}"))
        .collect::<Vec<String>>()
        .join(",")
}

fn recursion(frame: Frame, vars: usize, depth: usize) -> String {
    let names = wide_names(vars);
    let function = match frame {
        Frame::ArrowVars => {
            format!("var r = (n) => {{ var {names}; return n > 0 ? r(n - 1) : 0; }};")
        }
        Frame::ArrowLets => {
            format!("var r = (n) => {{ let {names}; if (n > 0) return r(n - 1); return 0; }};")
        }
        Frame::ArrowParams => {
            format!("var r = (n, {names}) => {{ if (n > 0) return r(n - 1); return 0; }};")
        }
        Frame::ForLet => format!(
            "function r(n) {{ for (let {names}; ;) {{ if (n > 0) return r(n - 1); break; }} \
             return 0; }}"
        ),
    };
    let body = format!("{function} r({depth}); return 'done' + i;");
    unit("", "['x']", &body) + "console.log(dec(0));\n"
}

#[derive(Clone, Copy)]
enum Frame {
    ArrowVars,
    ArrowLets,
    ArrowParams,
    ForLet,
}

pub fn every_closure_call_checks_the_heap(rows: &mut Rows, scratch: &Scratch) {
    for count in [3_000, 4_000] {
        let label = format!("nested arrow forEach {count}");
        bails_inside_the_budget(
            rows,
            scratch,
            &label,
            (&nested_arrows(10), &nested_arrows(count)),
        );
    }
    let frames = [
        ("arrow recursion, 20k vars", Frame::ArrowVars),
        ("arrow recursion, 20k lets", Frame::ArrowLets),
        ("arrow recursion, 20k params", Frame::ArrowParams),
        ("for-let copies, 20k", Frame::ForLet),
    ];
    for (label, frame) in frames {
        bails_inside_the_budget(
            rows,
            scratch,
            label,
            (
                &recursion(frame, 20_000, 0),
                &recursion(frame, 20_000, RECURSION_DEPTH),
            ),
        );
    }
}
