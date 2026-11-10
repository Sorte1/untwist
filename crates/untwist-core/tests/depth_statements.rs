#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_classes.rs"]
mod depth_classes;

use depth_classes::{generate, Class};
use untwist_core::depth::{scan, DepthRefusal, Limits, UNLIMITED};

const CEILING: u32 = 100_000;
const COUNTED: usize = 1_000;
const FLAT_NESTING: u32 = 64;

fn at_the_ceiling() -> Limits {
    Limits {
        nesting: CEILING,
        ..UNLIMITED
    }
}

fn nesting(source: &str) -> u32 {
    scan(source, &UNLIMITED)
        .unwrap_or_else(|refusal| panic!("refused: {refusal:?}"))
        .nesting
}

fn chain(prefix: &str, open: &str, middle: &str, close: &str, n: usize) -> String {
    format!("{prefix}{}{middle}{}", open.repeat(n), close.repeat(n))
}

fn flat_programs() -> Vec<(&'static str, String)> {
    vec![
        ("functions", "function f(){}".repeat(40_000)),
        ("calls", "a = b.c(d)\n".repeat(40_000)),
        (
            "returns",
            "function f() {\n  return 1\n}\n\n".repeat(34_000),
        ),
        ("classes", "class A{}".repeat(60_000)),
        (
            "static blocks",
            format!("class A{{{}}}", "static{}".repeat(150_000)),
        ),
        ("methods", format!("class A{{{}}}", "m(){}".repeat(150_000))),
        ("blocks", "if (a) {}\n".repeat(40_000)),
        ("templates", "x = `a`\n".repeat(40_000)),
        ("arrows", "f(() => {})\n".repeat(40_000)),
        ("declarations", "let a = 1\nconst b = [c]\n".repeat(20_000)),
        ("try", "try {} catch (e) {}\n".repeat(30_000)),
        ("else", "if (a) {} else {}\n".repeat(30_000)),
        ("strings", "x = 'a'\ny = this\n".repeat(30_000)),
        (
            "cases",
            format!("switch (a) {{{}}}", "case 1: f()\n".repeat(40_000)),
        ),
        (
            "awaits",
            format!("async function f() {{{}}}", "await g()\n".repeat(40_000)),
        ),
    ]
}

#[test]
fn flat_programs_without_semicolons_are_accepted() {
    for (name, source) in flat_programs() {
        let result = scan(&source, &at_the_ceiling());
        let Ok(found) = result else {
            panic!("{name}: {result:?}");
        };
        assert!(found.nesting < FLAT_NESTING, "{name}: {}", found.nesting);
    }
}

#[test]
fn statement_lists_stay_flat() {
    for class in [Class::Stmts, Class::Lines] {
        let found = nesting(&generate(class, 100_000));
        assert!(found < FLAT_NESTING, "{}: {found}", class.name());
    }
}

fn nesting_shapes() -> Vec<(&'static str, [&'static str; 4])> {
    vec![
        ("if lines", ["", "if (a)\n", "f()", ""]),
        ("while lines", ["", "while (a)\n", "f()", ""]),
        ("for lines", ["", "for (;;)\n", "f()", ""]),
        ("with lines", ["", "with (a)\n", "f()", ""]),
        ("else if lines", ["if (a) b\n", "else if (a) b\n", "", ""]),
        (
            "braced else if",
            ["if (a) {}\n", "else if (a) {}\n", "", ""],
        ),
        ("tight else if", ["if(a){}", "else if(a){}", "", ""]),
        ("labels", ["", "a:\n", "b", ""]),
        ("do lines", ["", "do\n", "x\n", "while (a)\n"]),
        ("blocks", ["", "{\n", "", "}\n"]),
        ("arrow conditional", ["x = ", "c ? () => {} : ", "d", ""]),
        ("conditional lines", ["x = ", "a\n? b\n: ", "c", ""]),
        ("plus before lines", ["x = a", "\n+ a", "", ""]),
        ("plus after lines", ["x = a", " +\na", "", ""]),
        ("member lines", ["x = a", "\n.b", "", ""]),
        ("call lines", ["x = a", "\n(b)", "", ""]),
        ("index lines", ["x = a", "\n[b]", "", ""]),
        ("tagged lines", ["x = a", "\n`b`", "", ""]),
        ("tagged chain", ["x = a", "``", "", ""]),
        ("in lines", ["x = a", "\nin b", "", ""]),
        ("instanceof lines", ["x = a", "\ninstanceof b", "", ""]),
        ("arrow lines", ["x = ", "a =>\n", "a", ""]),
        ("assign lines", ["x = ", "a =\n", "1", ""]),
        ("new lines", ["x = ", "new\n", "a", ""]),
        ("typeof lines", ["x = ", "typeof\n", "a", ""]),
        ("object lines", ["x = ", "{a:\n", "1", "}\n"]),
        ("array lines", ["x = ", "[\n", "1", "]\n"]),
        ("template lines", ["x = ", "`${\n", "1", "}`\n"]),
        (
            "class heritage lines",
            ["x = ", "class extends B\n{ m() { return ", "1", " } }"],
        ),
        (
            "decorated classes",
            ["x = ", "@d\nclass { m() { return ", "1", " } }"],
        ),
        (
            "function bodies",
            ["", "function f() {\nreturn a\n", "", "}\n"],
        ),
        ("awaits", ["async function f() { x = ", "await\n", "a", ""]),
    ]
}

#[test]
fn nesting_across_line_breaks_and_braces_is_still_counted() {
    for (name, [prefix, open, middle, close]) in nesting_shapes() {
        let found = nesting(&chain(prefix, open, middle, close, COUNTED));
        assert!(found as usize >= COUNTED, "{name}: {found}");
    }
}

#[test]
fn deep_nesting_across_line_breaks_and_braces_is_still_refused() {
    let deep = CEILING as usize + 1;
    for (name, [prefix, open, middle, close]) in nesting_shapes() {
        let source = chain(prefix, open, middle, close, deep);
        assert_eq!(
            scan(&source, &at_the_ceiling()),
            Err(DepthRefusal::Nesting),
            "{name}"
        );
    }
}

#[test]
fn a_tagged_template_chain_costs_a_unit_per_template() {
    let source = generate(Class::Tagged, COUNTED);
    assert!(nesting(&source) as usize >= COUNTED);
    let limits = Limits {
        nesting: COUNTED as u32,
        ..UNLIMITED
    };
    assert_eq!(
        scan(&generate(Class::Tagged, 2 * COUNTED), &limits),
        Err(DepthRefusal::Nesting)
    );
}

#[test]
fn only_a_tagged_template_costs_a_unit() {
    assert_eq!(nesting("x = `a`;"), nesting("x = 'a';"));
    assert_eq!(nesting("x = a``;"), nesting("x = `a`;") + 1);
    assert_eq!(nesting("x = a()``;"), nesting("x = a();"));
}
