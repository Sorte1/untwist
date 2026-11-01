use untwist::{Options, Pipeline};
use untwist_deobfuscate::calls::agnostic::comparing_text_scopings;
use untwist_testing::js_files_in;

const REACH_ROWS: &str = include_str!("../../untwist/tests/data/string_array_reach_rows.txt");
const SHELL_ROWS: &str = include_str!("../../untwist/tests/data/string_array_shell_rows.txt");

const KEPT_TEXTS: [&str; 21] = [
    "Object = x; Object[y]",
    "({ Object } = x); y in Object",
    "f = x; f(this)",
    "f++; f(this)",
    "f += 1; f(this)",
    "[f] = x; f(this)",
    "({ f } = x); f(this)",
    "({ k: f } = x); f(this)",
    "for (f in x) f(this);",
    "for (f of x) f(this);",
    "var f = x; f(this)",
    "function f() {} f(this)",
    "(function (f) { f(this); })(x)",
    "try { y(); } catch (f) { f(this); }",
    "let f = x; f(this)",
    "class f {} f(this)",
    "f = 1; f()",
    "f++; f()",
    "q = 0; q(y)",
    "f = 1; new f(y)",
    "g = 1; g(Object)",
];

fn modes() -> [Options; 2] {
    [
        Options::default(),
        Options {
            unsafe_transforms: true,
            ..Options::default()
        },
    ]
}

fn rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn kept_programs() -> Vec<(String, String)> {
    KEPT_TEXTS
        .iter()
        .map(|text| {
            let program = format!(
                "var arr = [\"a\", \"b\"];\neval({text:?});\nconsole.log(arr[0], arr[1]);\n"
            );
            (format!("kept {text}"), program)
        })
        .collect()
}

fn compared_over(programs: &[(String, String)]) -> (usize, Vec<String>) {
    let mut compared = 0;
    let mut differing = Vec::new();
    for (label, source) in programs {
        for options in modes() {
            let unsafe_transforms = options.unsafe_transforms;
            let pipeline = Pipeline::new(options);
            let (_, seen, differs) = comparing_text_scopings(|| pipeline.run(source));
            compared += seen;
            if differs > 0 {
                differing.push(format!("{label} (unsafe {unsafe_transforms}): {differs}"));
            }
        }
    }
    (compared, differing)
}

#[test]
fn a_binding_free_text_is_judged_alike_with_and_without_its_scoping() {
    let mut programs = rows(REACH_ROWS);
    programs.extend(rows(SHELL_ROWS));
    programs.extend(js_files_in("fixtures/behaviour"));
    programs.extend(js_files_in("fixtures/corpus-npm"));
    programs.extend(js_files_in("fixtures/reference/deobfuscate"));
    let (compared, differing) = compared_over(&programs);
    assert!(compared >= 250, "only {compared} texts were compared");
    assert!(differing.is_empty(), "{}", differing.join("\n"));
}

#[test]
fn a_text_that_binds_or_writes_a_name_keeps_its_scoping() {
    let programs = kept_programs();
    let (compared, differing) = compared_over(&programs);
    assert!(differing.is_empty(), "{}", differing.join("\n"));
    assert_eq!(
        compared, 0,
        "a text that binds or writes a name was judged without its scoping"
    );
}
