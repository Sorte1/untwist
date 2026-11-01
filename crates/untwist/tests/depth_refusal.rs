use untwist::{
    Isolation, Options, Output, Pipeline, TestHooks, NESTED_TOO_DEEP, PARENTHESES_TOO_DEEP,
    PIPELINE_NOTES, PRINT_TOO_LARGE, TYPESCRIPT_IN_JAVASCRIPT, UNPACK_PANICKED,
    UNPACK_TEXT_TOO_DEEP, WORKER_DIED,
};

const TYPESCRIPT: &str = "import x = require('y');\n";
const NORMAL: &str = "var a = 1;\nconsole.log(a);\n";

fn assert_refused_alone(output: &Output, source: &str, note: &str) {
    assert_eq!(output.depth_refused, [0]);
    assert_eq!(output.code(), source);
    assert!(output.panicked.is_empty());
    assert!(output.stages.is_empty());
    assert_eq!(output.parse_errors, 0);
    let messages: Vec<&str> = output
        .notes
        .iter()
        .map(|note| note.message.as_str())
        .collect();
    assert_eq!(messages, [note]);
    assert_eq!(output.depth_refusals(), [(0, note)]);
}

fn isolated(isolation: Isolation) -> Pipeline {
    Pipeline::new(Options {
        isolation,
        ..Options::default()
    })
}

#[test]
fn an_input_the_worker_died_on_is_refused_before_its_scan() {
    let output = isolated(Isolation {
        refused: vec![0],
        ..Isolation::default()
    })
    .run(NORMAL);
    assert_refused_alone(&output, NORMAL, WORKER_DIED);
}

#[test]
fn a_lost_unpack_is_recorded_as_an_unpack_panic() {
    let output = isolated(Isolation {
        unpack_lost: true,
        ..Isolation::default()
    })
    .run(NORMAL);
    assert!(output.unpack_panicked);
    assert!(output.bundle.is_none());
    assert!(output.depth_refused.is_empty());
    assert!(output
        .notes
        .iter()
        .any(|note| note.pass == "pipeline" && note.message == UNPACK_PANICKED));
}

fn with_a_panicking_admit(index: usize, jobs: usize) -> Pipeline {
    Pipeline::new(Options {
        jobs: std::num::NonZeroUsize::new(jobs),
        unpack: false,
        test_hooks: TestHooks {
            admit_panic: Some(index),
            ..TestHooks::default()
        },
        ..Options::default()
    })
}

#[test]
fn a_panic_before_the_parse_refuses_only_that_input() {
    let sources = ["var a = !0;", "javascript:var%20b%20=%20!0;", "var c = !1;"];
    for jobs in [1, 3] {
        let output = with_a_panicking_admit(1, jobs).run_many(&sources);
        assert_eq!(output.depth_refused, [1], "-j {jobs}");
        assert!(output.panicked.is_empty(), "-j {jobs}");
        assert_eq!(output.codes[1], sources[1], "-j {jobs}");
        assert_eq!(output.codes[0], "var a = true;", "-j {jobs}");
        assert_eq!(output.codes[2], "var c = false;", "-j {jobs}");
        let refusals = output.depth_refusals();
        assert_eq!(refusals.len(), 1, "-j {jobs}");
        assert!(
            PIPELINE_NOTES
                .iter()
                .any(|(_, text)| *text == refusals[0].1),
            "{refusals:?}"
        );
    }
}

#[test]
fn typescript_in_javascript_is_refused() {
    let output = Pipeline::new(Options::default()).run(TYPESCRIPT);
    assert_refused_alone(&output, TYPESCRIPT, TYPESCRIPT_IN_JAVASCRIPT);
}

#[test]
fn a_type_operator_after_a_closed_export_clause_is_refused() {
    let source = "export {}\nx satisfies y\n";
    let output = Pipeline::new(Options::default()).run(source);
    assert_refused_alone(&output, source, TYPESCRIPT_IN_JAVASCRIPT);
}

#[test]
fn a_type_node_past_the_scan_is_refused_after_the_parse() {
    let source = "var a = 1;\nx satisfies y\n";
    let output = Pipeline::new(Options {
        test_hooks: TestHooks {
            scan_off: true,
            ..TestHooks::default()
        },
        ..Options::default()
    })
    .run(source);
    assert_refused_alone(&output, source, TYPESCRIPT_IN_JAVASCRIPT);
}

#[test]
fn an_errorless_default_interface_is_refused_after_the_parse() {
    let source = "export default interface I {}\nvar a = 1;\n";
    let output = Pipeline::new(Options {
        test_hooks: TestHooks {
            scan_off: true,
            ..TestHooks::default()
        },
        ..Options::default()
    })
    .run(source);
    assert_refused_alone(&output, source, TYPESCRIPT_IN_JAVASCRIPT);
}

#[test]
fn a_nested_errorless_default_interface_is_refused_after_the_parse() {
    let sources = [
        "export {}\n{ export default interface I { m(b = J); } }\nvar J = g();\nconsole.log(1);\n",
        "export {}\nfunction f() { return () => { { export default interface I {} } }; }\n",
        "export {}\nclass C { static { export default interface I extends J {} } }\n",
    ];
    for source in sources {
        let output = Pipeline::new(Options::default()).run(source);
        assert_refused_alone(&output, source, TYPESCRIPT_IN_JAVASCRIPT);
    }
}

#[test]
fn a_class_without_type_nodes_still_runs_after_the_parse_check() {
    let source = "class A extends B {}\nexport default class C {}\nvar o = { m(a, ...b) {} };\n";
    let output = Pipeline::new(Options::default()).run(source);
    assert!(output.depth_refused.is_empty());
    assert!(!output.stages.is_empty());
}

#[test]
fn a_refused_input_takes_no_part_in_unpack_and_the_others_still_run() {
    let output = Pipeline::new(Options::default()).run_many(&[TYPESCRIPT, NORMAL]);
    assert_eq!(output.depth_refused, [0]);
    assert_eq!(output.codes[0], TYPESCRIPT);
    assert_ne!(output.codes[1], NORMAL);
    assert_eq!(output.depth_refusals(), [(0, TYPESCRIPT_IN_JAVASCRIPT)]);
}

#[test]
fn every_refusal_note_is_catalogued_and_reached() {
    let reached = [
        NESTED_TOO_DEEP,
        PARENTHESES_TOO_DEEP,
        PRINT_TOO_LARGE,
        TYPESCRIPT_IN_JAVASCRIPT,
        WORKER_DIED,
        UNPACK_TEXT_TOO_DEEP,
    ];
    for note in reached {
        assert!(PIPELINE_NOTES.iter().any(|(_, message)| *message == note));
    }
    untwist_testing::notes::every_case_names_a_catalogued_note(PIPELINE_NOTES, &reached);
}

#[test]
fn a_shallow_final_text_passes_the_scanning_guard_and_unpacks() {
    let output = Pipeline::new(Options::default()).run_with_final_text_guard_for_tests(&[NORMAL]);
    assert!(output
        .notes
        .iter()
        .all(|note| note.message != UNPACK_TEXT_TOO_DEEP));
    assert!(!output.unpack_panicked);
}

#[cfg(all(target_os = "linux", not(debug_assertions)))]
mod deep {
    use super::assert_refused_alone;
    use std::ffi::OsString;
    use untwist::{
        Options, Pipeline, NESTED_TOO_DEEP, PARENTHESES_TOO_DEEP, PRINT_TOO_LARGE,
        UNPACK_TEXT_TOO_DEEP,
    };
    use untwist_testing::capped_child::{self, Cap};
    use untwist_testing::depth_classes::{generate, Class};

    const CHILD: &str = "UNTWIST_DEPTH_REFUSAL_CHILD";

    const CASES: [(&str, Class, usize, &str); 3] = [
        ("nesting", Class::Array, 200_000, NESTED_TOO_DEEP),
        ("print", Class::Block, 20_000, PRINT_TOO_LARGE),
        ("parentheses", Class::Arrowobj, 4_000, PARENTHESES_TOO_DEEP),
    ];

    fn final_text_row() {
        let text = generate(Class::Array, 200_000);
        let output =
            Pipeline::new(Options::default()).run_with_final_text_guard_for_tests(&[&text]);
        assert!(output.bundle.is_none());
        assert!(!output.unpack_panicked);
        let messages: Vec<&str> = output
            .notes
            .iter()
            .map(|note| note.message.as_str())
            .collect();
        assert_eq!(messages, [UNPACK_TEXT_TOO_DEEP]);
        println!("refused m7");
    }

    #[test]
    fn child_entry() {
        let Ok(name) = std::env::var(CHILD) else {
            return;
        };
        if name == "m7" {
            final_text_row();
            return;
        }
        if name == "m4" {
            super::budget::child();
            return;
        }
        let (_, class, depth, note) = CASES
            .into_iter()
            .find(|case| case.0 == name)
            .expect("a known case");
        let source = generate(class, depth);
        let output = Pipeline::new(Options::default()).run(&source);
        assert_refused_alone(&output, &source, note);
        println!("refused {name}");
    }

    #[test]
    fn deep_inputs_are_refused_in_process() {
        if std::env::var_os(CHILD).is_some() {
            return;
        }
        let exe = std::env::current_exe().expect("the test binary");
        let args: Vec<OsString> = ["deep::child_entry", "--exact", "--nocapture"]
            .into_iter()
            .map(OsString::from)
            .collect();
        let mut failures = Vec::new();
        let names = CASES.iter().map(|case| case.0).chain(["m7", "m4"]);
        for name in names {
            let env = [(OsString::from(CHILD), OsString::from(name))];
            let outcome = capped_child::run(exe.as_os_str(), &args, &env, None, Cap::DEFAULT);
            let stdout = String::from_utf8_lossy(&outcome.stdout);
            if outcome.exit_code != Some(0) || !stdout.contains(&format!("refused {name}")) {
                failures.push(format!("{name}: {outcome:?}"));
            }
        }
        failures.extend(super::budget::node_agrees());
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

#[cfg(all(target_os = "linux", not(debug_assertions)))]
mod budget {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use untwist::{Options, Output, Pipeline, TestHooks};
    use untwist_testing::capped_child::{self, Cap};
    use untwist_testing::node::{node, skip_allowed, SKIP_MESSAGE};

    const SITES: usize = 1_000;
    const REPEATS: usize = 2_500;
    const HOOK: u64 = 1 << 20;
    const SPENT: &str = "heap budget is spent";
    const CACHED_RETURN: &str = "var first = e[0], key = f + first, hit = b['cache'][key];\nreturn !hit ? ((i = b['decoder'](i)), (b['cache'][key] = i)) : (i = hit), i;";
    const COPYING_RETURN: &str = "return i + '';";

    fn scratch_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("m4-decode-budget")
    }

    fn amplified(sites: usize) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/eval/base64-rot0-shift0.js");
        let fixture = std::fs::read_to_string(path).expect("the base64 fixture");
        let long = format!("'{}'", "Bg9N".repeat(REPEATS));
        let calls = "console.log(b(0));\n".repeat(sites);
        let head = fixture
            .split("b(0);\nb(1);")
            .next()
            .expect("the fixture's call sites");
        format!("{}{calls}", head.replacen("'Bg9N'", &long, 1))
    }

    fn copying(sites: usize) -> String {
        amplified(sites).replacen(CACHED_RETURN, COPYING_RETURN, 1)
    }

    fn run(source: &str, heap_budget: Option<u64>) -> Output {
        let options = Options {
            test_hooks: TestHooks {
                heap_budget,
                ..TestHooks::default()
            },
            ..Options::default()
        };
        Pipeline::new(options).run(source)
    }

    fn peak(output: &Output) -> u64 {
        output.heap_peak
    }

    fn sites(code: &str) -> Vec<bool> {
        code.lines()
            .filter(|line| line.starts_with("console.log("))
            .map(|line| line.contains("b(0)"))
            .collect()
    }

    fn expected_decodes(make: fn(usize) -> String, budget: u64) -> usize {
        let first = peak(&run(&make(1), Some(u64::MAX)));
        let call = peak(&run(&make(2), Some(u64::MAX))).saturating_sub(first);
        if first > budget {
            return 0;
        }
        if call == 0 {
            return SITES;
        }
        let fitting = 1 + (budget - first) / call;
        usize::try_from(fitting).map_or(SITES, |fitting| fitting.min(SITES))
    }

    fn spent_notes(output: &Output) -> usize {
        output
            .notes
            .iter()
            .filter(|note| note.message.contains(SPENT))
            .count()
    }

    fn check(label: &str, make: fn(usize) -> String, budget: u64) -> usize {
        let expected = expected_decodes(make, budget);
        let source = make(SITES);
        let output = run(&source, Some(budget));
        let encoded = sites(output.code());
        assert_eq!(encoded.len(), SITES, "{label}");
        let decoded = encoded.iter().position(|encoded| *encoded).unwrap_or(SITES);
        assert_eq!(decoded, expected, "{label}");
        assert!(encoded[decoded..].iter().all(|encoded| *encoded), "{label}");
        assert!(peak(&output) <= budget, "{label}");
        assert_eq!(
            spent_notes(&output),
            usize::from(decoded < SITES),
            "{label}"
        );
        decoded
    }

    fn keep(name: &str, source: &str, output: &str) {
        let dir = scratch_dir();
        std::fs::create_dir_all(&dir).expect("the node scratch dir");
        std::fs::write(dir.join(format!("{name}-input.js")), source).expect("a node input");
        std::fs::write(dir.join(format!("{name}-output.js")), output).expect("a node output");
    }

    pub fn child() {
        let cached = check("the plan's input under 1 MiB", amplified, HOOK);
        assert_eq!(cached, SITES);
        let copied = check("a copying decoder under 1 MiB", copying, HOOK);
        assert!(copied > 0 && copied < SITES);
        let source = amplified(SITES);
        let default_run = run(&source, None);
        assert!(sites(default_run.code()).iter().all(|encoded| !encoded));
        assert_eq!(spent_notes(&default_run), 0);
        keep("default", &source, default_run.code());
        let source = copying(SITES);
        keep("copying", &source, run(&source, Some(HOOK)).code());
        println!("refused m4: copying decodes {copied} of {SITES} under {HOOK} B");
    }

    fn printed(node: &Path, file: &str) -> Result<Vec<u8>, String> {
        let args = [OsString::from(scratch_dir().join(file))];
        let outcome = capped_child::run(node.as_os_str(), &args, &[], None, Cap::DEFAULT);
        if outcome.exit_code != Some(0) {
            return Err(format!("node {file}: {outcome:?}"));
        }
        Ok(outcome.stdout)
    }

    fn agrees(node: &Path, name: &str) -> Option<String> {
        let before = printed(node, &format!("{name}-input.js"));
        let after = printed(node, &format!("{name}-output.js"));
        match (before, after) {
            (Ok(before), Ok(after)) if before == after => None,
            (Ok(_), Ok(_)) => Some(format!("{name}: node prints differently after untwist")),
            (before, after) => Some(format!("{name}: {:?} {:?}", before.err(), after.err())),
        }
    }

    pub fn node_agrees() -> Vec<String> {
        let Some(node) = node() else {
            let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
            assert!(
                skip_allowed(required.as_deref()),
                "required: {SKIP_MESSAGE}"
            );
            return Vec::new();
        };
        ["default", "copying"]
            .into_iter()
            .filter_map(|name| agrees(&node, name))
            .collect()
    }
}
