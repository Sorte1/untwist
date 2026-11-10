use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::mutants::limit_deobfuscate_rounds_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

static ROUNDS: std::sync::Mutex<()> = std::sync::Mutex::new(());

const UNITS: usize = 300;
const NESTED: usize = 40;

fn sibling_units(count: usize) -> String {
    let units: String = (0..count)
        .map(|i| {
            format!(
                "function f{i}(u) {{ var arr{i} = [\"a{i}\", \"b{i}\"]; return u((function (p) {{ return arr{i}[p]; }})(1)); }}\n"
            )
        })
        .collect();
    format!("{units}console.log(f0(String), f{}(String));", count - 1)
}

fn nested_units(depth: usize) -> String {
    let opened: String = (0..depth)
        .map(|i| {
            format!(
                "function f{i}(u) {{ var arr{i} = [\"a{i}\", \"b{i}\"]; var r = u((function (p) {{ return arr{i}[p]; }})(1)); "
            )
        })
        .collect();
    let closed = " return r; }".repeat(depth);
    format!("{opened}{closed}\nconsole.log(f0(String));")
}

fn run(source: &str) -> Output {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source)
}

fn deobfuscate_rounds(out: &Output) -> u32 {
    out.stages
        .iter()
        .filter(|stage| stage.stage == "deobfuscate")
        .map(|stage| stage.iterations)
        .max()
        .unwrap_or(0)
}

fn half_rewritten(code: &str) -> bool {
    code.contains("_decode") || code.contains("function arr")
}

#[test]
fn sibling_units_are_rewritten_in_the_same_rounds() {
    let _rounds = ROUNDS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let out = run(&sibling_units(UNITS));
    assert!(!half_rewritten(out.code()), "{}", out.code());
    assert!(!out.code().contains("arr0"), "{}", out.code());
    assert!(
        deobfuscate_rounds(&out) <= 3,
        "took {} rounds",
        deobfuscate_rounds(&out)
    );
}

#[test]
fn a_run_stopped_by_the_round_cap_keeps_the_units_as_written() {
    let _rounds = ROUNDS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let source = nested_units(NESTED);
    limit_deobfuscate_rounds_for_tests(2);
    let out = run(&source);
    limit_deobfuscate_rounds_for_tests(0);
    assert!(!half_rewritten(out.code()), "{}", out.code());
    assert!(
        out.notes
            .iter()
            .any(|note| note.message.contains("kept as it was before deobfuscation")),
        "{:?}",
        out.notes
    );
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = vec![("nested units under a round cap".to_string(), source)];
    let capped = |source: &str| {
        limit_deobfuscate_rounds_for_tests(2);
        let code = run(source).code().to_string();
        limit_deobfuscate_rounds_for_tests(0);
        code
    };
    let modes: [Mode<'_>; 1] = [("capped", &capped)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
