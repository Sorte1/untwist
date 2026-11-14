use std::path::Path;

use untwist::{Options, Output, Pipeline, RenameMode, PASSES};
use untwist_deobfuscate::unstamp::UNSTAMP_REMAINING_PASS;
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{node_or_skip, printed};

const INLINE_DECODED: &str = "inline-decoded-strings";

const BEHAVIOUR_FILES: usize = 40;

const READ_STAMPED_NAMES: [&str; 4] = [
    "18-array-element-names-array.js",
    "19-function-constructor-names-array.js",
    "20-global-object-member.js",
    "21-global-object-computed-key.js",
];

fn reads_stamped_names(name: &str, label: &str) -> bool {
    name == UNSTAMP_REMAINING_PASS && READ_STAMPED_NAMES.iter().any(|file| label.ends_with(file))
}

const EVERY_UNMINIFY_SHAPE: &str = r#"
var React = { createElement: function (t, p) { var c = [].slice.call(arguments, 2); return [typeof t === "string" ? t : t.name, p && Object.keys(p).join(","), c.length].join("|"); } };
function Box(p) { return p; }
var log = function () { console.log([].slice.call(arguments).join(" ")); };
var a = !0, b = !1, u = void 0, big = 1 / 0;
if (typeof missing === "undefined") log("undefined", a, b, u, big);
if (1 === a + 0) log("yoda");
var o = { "key": 1, ["x" + "y"]: 2 }; log(o["key"], o["xy"]);
var s = "a" + "b" + "c"; log(s, "n=" + 1 + "", `t${"lit"}`);
function g(x) { return x ? 1 : 2; } log(g(1), g(0));
function h(x) { x && log("and"); x || log("or"); return x ? log("t") : log("f"); } h(1); h(0);
function seq() { return log("s1"), log("s2"), 3; } log(seq());
for (var i = 0, j = 2; i < j; i++) log("i", i);
for (; j > 0;) j--;
var n = 0x10 + 1e3 * 2 - -5, w = 1.50000000000000001; log(n, w);
var k = JSON.parse('{"q":[1,2]}'); log(k.q.length);
if (a) { log("if"); } else { if (b) { log("else-if"); } else { log("else"); } }
log(!!a, !(a && b), !(a || b), -(-n), +"3");
var p = null, q = p == null ? void 0 : p.r; log(q);
var r = p != null ? p : "dflt"; log(r);
var t1 = 0; t1 || (t1 = 5); var t2 = null; t2 != null ? t2 : t2 = 6; log(t1, t2);
function dp(x) { var y = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 4; return x + y; } log(dp(1), dp(1, 2));
var arrow = function (x) { return x * 2; }; log(arrow(4));
var lv = 1; lv = lv + 1; var cv = 9; log(lv, cv);
(0, log)("comma-call");
log(React.createElement("div", { id: "x" }, "y", React.createElement(Box, null)));
"#;

fn rename_for(name: &str) -> RenameMode {
    match name {
        "mangle" => RenameMode::Mangle,
        "rename-unique" => RenameMode::Unique,
        "rename-simple" | "rename-shorthand" => RenameMode::Simple,
        _ => RenameMode::None,
    }
}

fn options(rename: RenameMode, disabled: Vec<&'static str>) -> Options {
    Options {
        unsafe_transforms: true,
        rename,
        disabled_passes: disabled,
        ..Options::default()
    }
}

fn programs() -> Vec<(String, String)> {
    let mut programs = vec![(
        "every-unminify-shape".to_string(),
        EVERY_UNMINIFY_SHAPE.to_string(),
    )];
    programs.extend(
        js_files_in("fixtures/behaviour")
            .into_iter()
            .take(BEHAVIOUR_FILES),
    );
    programs.extend(js_files_in("fixtures/reference/deobfuscate"));
    programs
}

fn changes_by(output: &Output, name: &str) -> u32 {
    output
        .stages
        .iter()
        .flat_map(|stage| stage.per_pass.iter().chain(stage.fixpoint_per_pass.iter()))
        .filter(|(pass, _)| *pass == name)
        .map(|(_, count)| count)
        .sum()
}

fn ran_while_disabled(output: &Output, source: &str, mode: RenameMode, name: &'static str) -> bool {
    if changes_by(output, name) == 0 {
        return false;
    }
    if name != INLINE_DECODED {
        return true;
    }
    let without_unstamp = options(mode, vec![name, UNSTAMP_REMAINING_PASS]);
    changes_by(&Pipeline::new(without_unstamp).run(source), name) > 0
}

struct Program {
    label: String,
    source: String,
    printed: String,
    baselines: Vec<(RenameMode, String)>,
}

fn node_equal_programs(node: &Path, dir: &Path) -> Vec<Program> {
    let modes = [
        RenameMode::None,
        RenameMode::Simple,
        RenameMode::Unique,
        RenameMode::Mangle,
    ];
    programs()
        .into_iter()
        .filter_map(|(label, source)| {
            let printed_by_input = printed(node, dir, &source).ok()?;
            let mut baselines = Vec::with_capacity(modes.len());
            for mode in modes {
                let code = Pipeline::new(options(mode, Vec::new()))
                    .run(&source)
                    .code()
                    .to_string();
                if printed(node, dir, &code).ok()? != printed_by_input {
                    return None;
                }
                baselines.push((mode, code));
            }
            Some(Program {
                label,
                source,
                printed: printed_by_input,
                baselines,
            })
        })
        .collect()
}

fn row(node: &Path, dir: &Path, programs: &[Program], name: &'static str) -> (Vec<String>, usize) {
    let mode = rename_for(name);
    let pipeline = Pipeline::new(options(mode, vec![name]));
    let mut wrong = Vec::new();
    let mut differed = 0;
    for program in programs {
        let output = pipeline.run(&program.source);
        if ran_while_disabled(&output, &program.source, mode, name) {
            wrong.push(format!(
                "{name} on {}: the disabled pass still ran",
                program.label
            ));
        }
        let baseline = program.baselines.iter().find(|(seen, _)| *seen == mode);
        if baseline.is_some_and(|(_, code)| output.code() == code) {
            continue;
        }
        differed += 1;
        let after = printed(node, dir, output.code());
        if after.as_deref() != Ok(program.printed.as_str())
            && !reads_stamped_names(name, &program.label)
        {
            wrong.push(format!(
                "{name} on {}: node printed {:?}, the output printed {after:?}",
                program.label, program.printed
            ));
        }
    }
    (wrong, differed)
}

#[test]
fn every_pass_can_be_disabled_and_the_output_stays_node_equal() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let programs = node_equal_programs(&node, dir.path());
    assert!(
        programs.len() >= 30,
        "only {} programs are node-equal with every pass on",
        programs.len()
    );
    assert!(programs
        .iter()
        .any(|program| program.label == "every-unminify-shape"));
    let mut wrong = Vec::new();
    let mut silent = Vec::new();
    for pass in PASSES {
        let (row_wrong, differed) = row(&node, dir.path(), &programs, pass.name);
        wrong.extend(row_wrong);
        if differed == 0 {
            silent.push(pass.name);
        }
    }
    eprintln!("passes whose removal changed no output: {silent:?}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
