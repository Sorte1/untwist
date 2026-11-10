use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const ROWS: &str = include_str!("data/function_value_rows.txt");
const MUTANT: &str = "function-values";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(mutant: bool) -> Options {
    Options {
        unpack: false,
        jsx: false,
        disabled_passes: if mutant { vec![MUTANT] } else { vec![] },
        ..Options::default()
    }
}

fn transformed(mutant: bool, source: &str) -> String {
    Pipeline::new(options(mutant))
        .run(source)
        .code()
        .to_string()
}

fn machinery_kept(code: &str) -> bool {
    code.contains("function _0x")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            label.starts_with("ref-") || label.starts_with("dec-") || label.starts_with("guard-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn a_callee_that_holds_only_user_functions_keeps_no_machinery() {
    let wrong: Vec<String> = rows()
        .into_iter()
        .filter(|(label, source)| {
            machinery_kept(&transformed(false, source)) == label.starts_with("dec-")
        })
        .map(|(label, _)| label)
        .collect();
    assert!(wrong.is_empty(), "decided against the label: {wrong:?}");
}

#[test]
fn the_default_lane_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run = |source: &str| transformed(false, source);
    let modes: [Mode<'_>; 1] = [("default", &run)];
    let wrong = mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_refusal_diverges_from_node_when_every_callee_is_taken_as_a_user_function() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let refused: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    let run = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = mismatches(&node, &refused, &modes);
    let same: Vec<&str> = refused
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| {
            !diverged
                .iter()
                .any(|line| line.starts_with(&format!("{label} ")))
        })
        .collect();
    assert!(same.is_empty(), "these still print like node: {same:?}");
}

const FAN_OUT_HEAD: &str = "function sa() { var h = ['log', 'hello', 'world']; sa = function () { return h; }; return sa(); }\nfunction dec(c, d) { var e = sa(); return (dec = function (f, g) { f = f - 0x0; var i = e[f]; return String.fromCharCode(i.charCodeAt(0)) + i.slice(1); }), dec(c, d); }\n";

fn fanned_out_parameters(width: usize) -> String {
    let mut source = String::from(FAN_OUT_HEAD);
    source.push_str("(function () {\nfunction f1(a) { a(1); }\n");
    for level in 2..=5 {
        let calls = format!("f{}(a); ", level - 1).repeat(width);
        source.push_str(&format!("function f{level}(a) {{ {calls}}}\n"));
    }
    source.push_str("f5(function () {});\n})();\nconsole.log(dec(0x1), dec(0x2));\n");
    source
}

#[test]
fn parameters_passed_down_many_call_sites_are_judged_in_bounded_time() {
    let source = fanned_out_parameters(32);
    let started = std::time::Instant::now();
    let code = Pipeline::new(Options::default())
        .run(&source)
        .code()
        .to_string();
    let elapsed = started.elapsed();
    assert!(code.contains("console.log"), "{code}");
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "a {}-byte program took {elapsed:?}",
        source.len()
    );
}
