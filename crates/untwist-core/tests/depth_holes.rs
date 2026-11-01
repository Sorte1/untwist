use untwist_core::depth::{scan, scan_as, DepthRefusal, Goal, Limits};

const MIB: u64 = 1 << 20;
const SHALLOW: usize = 2_000;
const DEEP: usize = 60_000;

#[derive(Clone, Copy, Debug)]
enum Reading {
    Program,
    As(Goal),
}

const P: Reading = Reading::Program;
const M: Reading = Reading::As(Goal::Module);
const S: Reading = Reading::As(Goal::Script);

fn production(len: usize) -> Limits {
    Limits {
        nesting: 100_000,
        speculative_parens: 512,
        speculative_pairs: (1 << 17).max(len as u64 / 64),
        print_bytes: (32 * len as u64).clamp(64 * MIB, (1 << 31) - 1),
    }
}

fn unlimited() -> Limits {
    Limits {
        nesting: u32::MAX,
        speculative_parens: u32::MAX,
        speculative_pairs: u64::MAX,
        print_bytes: u64::MAX,
    }
}

fn read(source: &str, limits: &Limits, reading: Reading) -> Result<u32, DepthRefusal> {
    let result = match reading {
        Reading::Program => scan(source, limits),
        Reading::As(goal) => scan_as(source, limits, goal),
    };
    result.map(|found| found.nesting)
}

fn opens(n: usize) -> String {
    "(".repeat(n)
}

fn parens(n: usize) -> String {
    format!("{}x{}", "(".repeat(n), ")".repeat(n))
}

fn hides_nothing(templates: &[&str], region: fn(usize) -> String, readings: &[Reading]) {
    for template in templates {
        for &reading in readings {
            let shallow = template.replace('¤', &region(SHALLOW));
            let nesting = read(&shallow, &unlimited(), reading);
            assert!(
                nesting.is_ok_and(|n| n as usize >= SHALLOW),
                "{reading:?} {template:?}: {nesting:?}"
            );
            let deep = template.replace('¤', &region(DEEP));
            assert_eq!(
                read(&deep, &production(deep.len()), reading),
                Err(DepthRefusal::Nesting),
                "{reading:?} {template:?}"
            );
        }
    }
}

const HIDDEN: &str = "/`/;\n¤\n;/`/";

fn after(prefix: &str) -> String {
    format!("{prefix}{HIDDEN}")
}

#[test]
fn an_identifier_escape_hides_nothing_after_its_hex_digits() {
    hides_nothing(&["a\\u{¤}", "a\\u¤"], parens, &[P, M, S]);
    hides_nothing(&["\\u{¤", "x = a\\u{1F}¤"], opens, &[P, M, S]);
}

#[test]
fn a_regex_follows_a_statement_ended_by_a_newline() {
    let scripts = [
        "for(;;){ break\n/`/;\n¤\n;/`/ }",
        "for(;;){ continue\n/`/;\n¤\n;/`/ }",
        "l: for(;;) break l\n/`/;\n¤\n;/`/",
        "l: for(;;) { continue l\n/`/;\n¤\n;/`/ }",
    ];
    hides_nothing(&scripts, parens, &[P, M, S]);
    hides_nothing(&[&after("debugger\n")], parens, &[P, M, S]);
    let modules = [
        after("import a from 'b'\n"),
        after("import 'b'\n"),
        after("import {a as b} from \"c\"\n"),
        after("export * from 'y'\n"),
        after("export {a} from 'y'\n"),
        after("import a from 'b' with {type: 'json'}\n"),
    ];
    let modules: Vec<&str> = modules.iter().map(String::as_str).collect();
    hides_nothing(&modules, parens, &[P, M]);
}

#[test]
fn a_brace_after_of_and_a_newline_opens_an_object() {
    hides_nothing(&["for (x of\n{} /¤/ 1);"], parens, &[P, M, S]);
}

#[test]
fn yield_after_an_operator_is_an_identifier_reference() {
    let prefixes = [
        "-yield",
        "!yield",
        "typeof yield",
        "++yield",
        "a + yield",
        "a ** yield",
        "a in yield",
        "a ? b : -yield",
        "<a>{-yield",
    ];
    for prefix in prefixes {
        let template = format!("function* g(){{ {prefix} /¤/ }}");
        hides_nothing(&[&template], opens, &[P, M, S]);
    }
}

#[test]
fn the_callee_of_new_reads_await_and_yield_as_names() {
    hides_nothing(
        &[
            "async function f(){ new await /¤/ }",
            "function* g(){ new yield /¤/ }",
        ],
        opens,
        &[P, M, S],
    );
}

#[test]
fn yield_before_a_word_outside_a_generator_is_an_operator() {
    hides_nothing(&["for (yield of /¤/"], opens, &[P, M, S]);
}

#[test]
fn an_await_or_yield_operand_reads_nested_ones_as_operators() {
    hides_nothing(&[&after("yield yield ")], parens, &[P, M, S]);
    hides_nothing(&[&after("await await ")], parens, &[P, S]);
}

#[test]
fn an_escaped_keyword_reads_as_the_keyword() {
    hides_nothing(&[&after("\\u0061wait ")], parens, &[P, M]);
    hides_nothing(
        &["function* g(){ yi\\u0065ld /`/;\n¤\n;/`/ }"],
        parens,
        &[P, M, S],
    );
}

#[test]
fn class_heritage_reads_await_and_yield_as_names() {
    hides_nothing(
        &[
            "async function f(){ class A extends await {}\n/`/;\n¤\n;/`/ }",
            "function* g(){ class A extends yield {}\n/`/;\n¤\n;/`/ }",
        ],
        parens,
        &[P, M, S],
    );
}

#[test]
fn a_declaration_pattern_without_an_initializer_ends_at_the_newline() {
    hides_nothing(
        &[&after("let {}\n"), &after("const [a]\n")],
        parens,
        &[P, M, S],
    );
    hides_nothing(&[&after("export let {}\n")], parens, &[P, M]);
}

#[test]
fn a_decorator_is_followed_by_an_expression() {
    hides_nothing(&["@ /=x</¤"], opens, &[P, M, S]);
}

#[test]
fn each_goal_bounds_its_own_parse() {
    hides_nothing(&[&after("await await ")], parens, &[S]);
    hides_nothing(&[&after("\\u0061wait ")], parens, &[M]);
    hides_nothing(&["for (await of\n{} /¤/ 1);"], parens, &[S]);
}

#[test]
fn a_doubtful_reading_is_also_scanned_the_other_way() {
    hides_nothing(&["@ /¤/"], opens, &[P, M, S]);
    let result = scan("@ /(((x)))/", &unlimited());
    assert!(result.is_ok_and(|r| r.regexes == 1 && r.nesting >= 6));
}

#[test]
fn for_await_outside_an_async_function_keeps_its_loop_head() {
    hides_nothing(
        &[
            "(() => { for await (x of y) of\n/¤/ })",
            "function f(){ for await (x of y) of\n/¤/ }",
        ],
        parens,
        &[P, M, S],
    );
}

#[test]
fn an_html_open_comment_in_jsx_children_is_a_script_comment() {
    hides_nothing(
        &["<a>x <!-- y\"</a>\n{¤}", "<a>x <!-- `</a>\n{¤}`"],
        parens,
        &[P, S],
    );
}

#[test]
fn import_attributes_spelled_assert_end_the_declaration() {
    let modules = [
        after("import a from 'a' assert {type: 'json'}\n"),
        after("import 'a' assert {}\n"),
        after("export * from 'a' assert {}\n"),
        after("export {a} from 'a' assert {}\n"),
        after("import a from 'a'\nwith {type: 'json'}\n"),
    ];
    let modules: Vec<&str> = modules.iter().map(String::as_str).collect();
    hides_nothing(&modules, parens, &[P, M, S]);
}

#[test]
fn a_decorated_class_takes_the_reading_before_its_decorators() {
    hides_nothing(
        &[
            "x / @a class{} / ¤ / 1",
            "x = @a class{} / ¤ / 1",
            "(@a class{} / ¤ / 1)",
            "new @a class{} / ¤",
            "async function f(){ await @a class{} / ¤ }",
            "() => typeof @a\nclass{} / ¤ /",
            "for (x of @a class{} <a>¤",
            "x = @a.b(c) @d class{} / ¤ / 1",
            "x = @a export default class{} / ¤ / 1",
            "for (let of of @ /x/\texport default class{}/[¤]/;",
        ],
        parens,
        &[P, M, S],
    );
}

#[test]
fn the_second_of_after_a_declared_of_is_the_loop_keyword() {
    hides_nothing(
        &[
            "for (let of of class{} / ¤ /1);",
            "for(let of of/>/¤",
            "for (var of of class{} / ¤ /1);",
            "for (const of of class{} / ¤ /1);",
        ],
        parens,
        &[P, M, S],
    );
}

#[test]
fn an_escaped_let_or_await_site_mixed_with_another_is_read_exactly() {
    hides_nothing(&[&after("l\\u0065t\nawait")], parens, &[P, M]);
    hides_nothing(&[&after("\\u0061wait\nyield yield")], parens, &[P, S]);
    hides_nothing(
        &[
            "function* g(){ await\n\\u0079ield/`/;\n¤\n;/`/ }",
            "function* g(){ x = yield*\\u{79}ield/`/;\n¤\n;/`/ }",
            "function* g(){ x = yield*yield/`/;\n¤\n;/`/ }",
        ],
        parens,
        &[P, M, S],
    );
}

#[test]
fn a_pattern_declaration_without_an_initializer_ends_after_any_statement_end() {
    let everywhere = [
        after("yield\nlet []\n"),
        format!("function f(){{ return\nlet {{}}\n{HIDDEN} }}"),
        after("export let []\n"),
        after("export{}\nlet[]\n"),
        after("export\nlet[]\n"),
        after("\\u0065xport\u{2028}let[]\n"),
        format!("function* g(){{ yield\nlet []\n{HIDDEN} }}"),
        "<a b={()=>{for({}of b)let[]\u{2029}<a b=\"\">`</a>\n¤\n`}} />".to_owned(),
        after("if (a) let [a]\n"),
        after("l: let [a]\n"),
        after("do let [a]\nwhile(0)\n"),
        after("for (let {a} of b) let [a]\n"),
    ];
    let everywhere: Vec<&str> = everywhere.iter().map(String::as_str).collect();
    hides_nothing(&everywhere, parens, &[P, M, S]);
    hides_nothing(&[&after("await\nlet {}\n")], parens, &[P, S]);
}

#[test]
fn a_using_declaration_without_an_initializer_ends_at_the_newline() {
    let sources = [
        after("using a\n"),
        format!("{{ using a\n{HIDDEN} }}"),
        after("await using a\n"),
        "using a\n<>`</>;\n¤\n;`".to_owned(),
    ];
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    hides_nothing(&sources, parens, &[P, M, S]);
}

fn refuses_typescript(templates: &[&str]) {
    for template in templates {
        for reading in [P, M, S] {
            for (n, limits) in [(SHALLOW, unlimited()), (DEEP, production(0))] {
                let source = template.replace('¤', &parens(n));
                assert_eq!(
                    read(&source, &limits, reading),
                    Err(DepthRefusal::Typescript),
                    "{reading:?} {template:?}"
                );
            }
        }
    }
}

#[test]
fn typescript_entry_points_oxc_accepts_in_javascript_are_refused() {
    let sources = [
        after("class A extends B<C> {}\n"),
        after("class A extends B()<C> {}\n"),
        after("import x = require('y')\n"),
        after("import x = a.b\n"),
        after("import x = a\n"),
        after("export import x = a.b\n"),
        after("a < x as y\n"),
        after("a + x satisfies y\n"),
        after("a + x as {}\n"),
        after("export default a - x satisfies y\n"),
    ];
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    refuses_typescript(&sources);
}

#[test]
fn script_await_then_yield_of_reads_division() {
    hides_nothing(
        &["for(;;)await\nyield of\n/¤/", "x = await\nyield of\n/¤/"],
        parens,
        &[P, S],
    );
}

#[test]
fn every_other_typescript_entry_point_is_refused() {
    let sources = [
        after("x as A<B>\n"),
        "x as A<B>\n/¤/ 1".to_owned(),
        after("x \\u0061s A\n"),
        after("class A implements B<C> {}\n"),
        after("class A extends B\n<C> {}\n"),
        after("export default interface I { a: ¤ }\n"),
        after("class A { [k: string]: T }\n"),
        after("class A { static [k: T]: U }\n"),
        after("class A { [...a] }\n"),
        after("class A { readonly [k: T]: U }\n"),
        after("class A { x\n[a, b]: T }\n"),
        after("class A { [a?: T]: U }\n"),
        after("for (x as T of y);\n"),
        after("({}) as T\n"),
        after("async as T\n"),
        after("get as T\n"),
        "async as \u{c9}<B>\n/¤/ 1".to_owned(),
        "async satisfies \u{c9}<B>\n/¤/ 1".to_owned(),
        "async as \\u0054<B>\n/¤/ 1".to_owned(),
        "x = async as \u{c9}<B>\n/¤/ 1".to_owned(),
        after("class A { @dec x = a as T }\n"),
        after("class A { @a @b x = c satisfies T }\n"),
        after("class A { @(a as T) x(){} }\n"),
        after("class A { m(){} x = {} as T }\n"),
        after("export {}\nx satisfies y\n"),
        after("export {}\nx as T\n"),
        after("export { a as b }\nx satisfies y\n"),
        after("import { a } from 'b'\nx as T\n"),
        after("import a, { b } from 'c'\nx satisfies y\n"),
    ];
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    refuses_typescript(&sources);
}

#[test]
fn a_name_after_a_closed_clause_starts_a_new_statement() {
    let accepted = [
        "export {}\nx\nas\n¤\n",
        "export {}\nx\nsatisfies\n¤\n",
        "export { a as satisfies }\n¤\n",
        "export { satisfies as as }\n¤\n",
        "export {} from 'x'\n¤\n",
        "export {}\nfrom\n'x'\n¤\n",
        "import { a as b } from 'c'\nx.as\n¤\n",
        "export {a}\nx(as)\n¤\n",
        "export {a}\nx = as\n¤\n",
    ];
    for source in accepted {
        for reading in [P, M] {
            let text = source.replace('¤', &parens(SHALLOW));
            assert!(
                read(&text, &unlimited(), reading).is_ok(),
                "{reading:?} {source:?}"
            );
        }
    }
}

#[test]
fn a_class_head_angle_in_the_flipped_pass_is_read() {
    for source in [
        "await\nfunction((\"\"))}/class<",
        "await\nfunction((\"\"))}/class<(x)",
    ] {
        assert!(read(source, &unlimited(), S).is_ok(), "{source:?}");
        assert!(read(source, &unlimited(), P).is_err(), "{source:?}");
    }
}

#[test]
fn the_full_oracle_classes_past_index_100000_hide_nothing() {
    hides_nothing(
        &[
            "yield yield\na?d:yield/¤",
            "d:yield yield\nyield/¤",
            "class{}yield yield\u{2028}x=yield/**//¤",
            "function(){[]\u{2029}@\\u0061wait \\u0063lass{}/`/¤",
            "function f(){[]\n@await class{}/`/¤}",
            "function(){/>/\\u0069n<>`</>;¤",
            "x = /a/\\u0069n<>`</>;¤",
        ],
        opens,
        &[P, M, S],
    );
}
