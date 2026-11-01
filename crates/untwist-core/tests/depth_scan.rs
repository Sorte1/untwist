#[allow(dead_code)]
#[path = "../../untwist-testing/src/depth_classes.rs"]
mod depth_classes;

use std::path::{Path, PathBuf};

use depth_classes::{generate, repeatchain, Class, TABLE_ONE};
use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXElement, JSXFragment, RegExpLiteral, TemplateLiteral};
use oxc_ast_visit::{walk, Visit};
use untwist_core::depth::{
    print_estimate, scan, scan_as, DepthRefusal, DepthScan, Goal, Limits, PRINT_GROWTH,
    PRINT_LENGTH_FACTOR, UNLIMITED,
};
use untwist_core::parse::parse_program;

const MIB: u64 = 1 << 20;

fn scanned(source: &str) -> DepthScan {
    scan(source, &UNLIMITED).unwrap_or_else(|refusal| panic!("refused: {refusal:?}"))
}

fn with_nesting(nesting: u32) -> Limits {
    Limits {
        nesting,
        ..UNLIMITED
    }
}

fn estimate(source: &str) -> u64 {
    print_estimate(&scanned(source), source.len())
}

fn production_print_limit(len: usize) -> u64 {
    (32 * len as u64).clamp(64 * MIB, (1 << 31) - 1)
}

#[test]
fn a_plus_chain_counts_every_operator_and_prints_linearly() {
    let source = generate(Class::Plus, 100_000);
    let operators = source
        .bytes()
        .filter(|byte| matches!(byte, b'+' | b'='))
        .count();
    let result = scanned(&source);
    assert_eq!(result.nesting as usize, operators);
    assert!(print_estimate(&result, source.len()) <= (PRINT_GROWTH + 1) * source.len() as u64);
}

#[test]
fn flat_parentheses_still_count_their_recursion() {
    let source = format!("{}1{}", "(".repeat(9_531), ")".repeat(9_531));
    assert!(scanned(&source).nesting >= 9_531);
}

#[test]
fn every_nesting_class_of_table_one_reaches_its_depth() {
    for &class in TABLE_ONE {
        if matches!(class, Class::Comma | Class::Stmts) {
            continue;
        }
        let nesting = scanned(&generate(class, 1_000)).nesting;
        assert!(nesting >= 1_000, "{}: {nesting}", class.name());
    }
}

#[test]
fn statement_chains_count_every_link() {
    for class in [
        Class::Ifnb,
        Class::Elseif,
        Class::Ifelse,
        Class::Label,
        Class::While,
    ] {
        let nesting = scanned(&generate(class, 1_000)).nesting;
        assert!(nesting >= 1_000, "{}: {nesting}", class.name());
    }
    assert_eq!(scanned("return a?b:c?d:e;").nesting, 5);
}

#[test]
fn speculative_heads_follow_the_maybe_classification() {
    for class in [Class::Arrowdef, Class::Arrowobj, Class::Asyncarrow] {
        let result = scanned(&generate(class, 600));
        assert!(result.speculative_parens >= 600, "{}", class.name());
        assert_eq!(result.speculative_pairs, 600 * 599 / 2, "{}", class.name());
    }
    assert!(scanned(&generate(Class::Arrowcomma, 600)).speculative_parens >= 600);
    for class in [
        Class::Callarg,
        Class::Parengrp,
        Class::Andnest,
        Class::Ternparen,
    ] {
        let parens = scanned(&generate(class, 600)).speculative_parens;
        assert!(parens <= 1, "{}: {parens}", class.name());
    }
}

#[test]
fn repeated_chains_add_their_pairs() {
    for k in [1, 2, 5] {
        let result = scanned(&repeatchain(k));
        assert_eq!(result.speculative_pairs, k as u64 * 124_750);
        assert_eq!(result.speculative_parens, 500);
    }
}

#[test]
fn a_slash_after_a_statement_head_starts_a_regex() {
    assert_eq!(scanned("if (x) /re/.test(y)").regexes, 1);
    assert_eq!(scanned("a = (b) / 2 / c").regexes, 0);
}

#[test]
fn a_slash_after_a_closing_brace_follows_the_brace_kind() {
    assert_eq!(scanned("function f(){} /x/.test(y)").regexes, 1);
    assert_eq!(scanned("x = {} / 2").regexes, 0);
}

#[test]
fn jsx_text_and_attributes_are_not_code() {
    let element = scanned(r#"<a b="'">'{"["}</a>"#);
    assert_eq!(element.jsx_elements, 1);
    assert!(element.nesting <= 6, "{}", element.nesting);
    assert_eq!(scanned("x = a < b > c").jsx_elements, 0);
}

#[test]
fn nested_templates_count_each_literal() {
    let result = scanned("`${`${`${1}`}`}`");
    assert_eq!(result.templates, 3);
    assert!(result.nesting >= 3);
}

#[test]
fn unclosed_brackets_are_refused_for_nesting() {
    let source = "[".repeat(100_000);
    assert_eq!(
        scan(&source, &with_nesting(100_000)),
        Err(DepthRefusal::Nesting)
    );
}

#[test]
fn each_limit_admits_its_exact_value_and_refuses_one_more() {
    let plus = generate(Class::Plus, 1_000);
    let nesting = scanned(&plus).nesting;
    assert!(scan(&plus, &with_nesting(nesting)).is_ok());
    assert_eq!(
        scan(&plus, &with_nesting(nesting - 1)),
        Err(DepthRefusal::Nesting)
    );
    let arrows = generate(Class::Arrowdef, 600);
    let result = scanned(&arrows);
    let parens = |speculative_parens| Limits {
        speculative_parens,
        ..UNLIMITED
    };
    assert!(scan(&arrows, &parens(result.speculative_parens)).is_ok());
    let fewer = scan(&arrows, &parens(result.speculative_parens - 1));
    assert_eq!(fewer, Err(DepthRefusal::SpeculativeParens));
    let pairs = |speculative_pairs| Limits {
        speculative_pairs,
        ..UNLIMITED
    };
    assert!(scan(&arrows, &pairs(result.speculative_pairs)).is_ok());
    let fewer = scan(&arrows, &pairs(result.speculative_pairs - 1));
    assert_eq!(fewer, Err(DepthRefusal::SpeculativeParens));
    let block = generate(Class::Block, 100);
    let bound = estimate(&block);
    let print = |print_bytes| Limits {
        print_bytes,
        ..UNLIMITED
    };
    assert!(scan(&block, &print(bound)).is_ok());
    assert_eq!(
        scan(&block, &print(bound - 1)),
        Err(DepthRefusal::PrintSize)
    );
}

#[test]
fn brackets_inside_a_string_are_not_counted() {
    let source = format!("x=\"{}\";", "[".repeat(200_000));
    assert!(scanned(&source).nesting <= 3);
}

#[test]
fn brackets_inside_a_block_comment_are_not_counted() {
    let source = format!("/* {} */x;", "(".repeat(200_000));
    assert!(scanned(&source).nesting <= 3);
}

#[test]
fn brackets_inside_a_regex_are_not_counted() {
    let source = format!("x=/[(]{{0}}{}/;", "\\(".repeat(100_000));
    let result = scanned(&source);
    assert!(result.nesting <= 3, "{}", result.nesting);
    assert_eq!(result.regexes, 1);
}

#[test]
fn brackets_inside_jsx_text_are_not_counted() {
    let source = format!("x=<a>{}</a>;", "[".repeat(100_000));
    assert!(scanned(&source).nesting <= 4);
}

#[derive(Default, Debug, PartialEq)]
struct Literals {
    regexes: u32,
    templates: u32,
    jsx: u32,
}

impl<'a> Visit<'a> for Literals {
    fn visit_reg_exp_literal(&mut self, _it: &RegExpLiteral<'a>) {
        self.regexes += 1;
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.templates += 1;
        walk::walk_template_literal(self, it);
    }

    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        self.jsx += 1;
        walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
        self.jsx += 1;
        walk::walk_jsx_fragment(self, it);
    }
}

fn parsed_literals(source: &str) -> Option<Literals> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    if untwist_core::parse::has_error(&parsed.diagnostics) {
        return None;
    }
    let mut literals = Literals::default();
    literals.visit_program(&parsed.program);
    Some(literals)
}

fn scanned_literals(source: &str) -> Literals {
    let result = scanned(source);
    Literals {
        regexes: result.regexes,
        templates: result.templates,
        jsx: result.jsx_elements,
    }
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn collect_scripts(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_scripts(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path);
        }
    }
}

fn real_inputs() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_scripts(&fixtures_root(), &mut paths);
    paths.sort();
    let required = std::env::var("UNTWIST_REQUIRE_BIG_BUNDLE").is_ok_and(|value| !value.is_empty());
    let bundle = fixtures_root().join("generated/webpack-5mb.js");
    assert!(
        !required || bundle.is_file(),
        "UNTWIST_REQUIRE_BIG_BUNDLE is set but {} is missing",
        bundle.display()
    );
    paths
}

fn on_a_large_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(body)
        .expect("a 1 GiB stack thread")
        .join()
        .expect("the large-stack body completed");
}

fn literal_mismatches() -> (usize, Vec<String>) {
    let mut mismatches = Vec::new();
    let mut compared = 0;
    for path in real_inputs() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(parsed) = parsed_literals(&source) else {
            continue;
        };
        compared += 1;
        let scanned = scanned_literals(&source);
        if scanned != parsed {
            mismatches.push(format!(
                "{}: scan {scanned:?} ast {parsed:?}",
                path.display()
            ));
        }
    }
    (compared, mismatches)
}

#[test]
fn literal_counts_agree_with_the_parsed_ast() {
    on_a_large_stack(|| {
        let (compared, mismatches) = literal_mismatches();
        assert!(compared > 200, "only {compared} fixtures parsed");
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    });
}

fn assert_a_fifth_of_every_limit(path: &Path, source: &str) {
    let result = scanned(source);
    let len = source.len() as u64;
    let name = path.display();
    assert!(
        result.nesting <= 20_000,
        "{name}: nesting {}",
        result.nesting
    );
    assert!(
        result.speculative_parens <= 102,
        "{name}: parens {}",
        result.speculative_parens
    );
    let pairs_limit = (1u64 << 17).max(len / 64);
    assert!(
        result.speculative_pairs <= pairs_limit / 5,
        "{name}: pairs {}",
        result.speculative_pairs
    );
    let print = print_estimate(&result, source.len());
    assert!(
        print <= production_print_limit(source.len()) / 5,
        "{name}: print {print}"
    );
    if name.to_string().ends_with("game.js") || name.to_string().ends_with("webpack-5mb.js") {
        println!(
            "{name}: nesting {} speculative {} pairs {} estimate {print} ({:.2} % of {})",
            result.nesting,
            result.speculative_parens,
            result.speculative_pairs,
            print as f64 * 100.0 / production_print_limit(source.len()) as f64,
            production_print_limit(source.len())
        );
    }
}

#[test]
fn real_inputs_sit_under_a_fifth_of_every_limit() {
    let mut checked = 0;
    for path in real_inputs() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        assert_a_fifth_of_every_limit(&path, &source);
        checked += 1;
    }
    assert!(checked > 200, "only {checked} inputs");
}

#[test]
fn the_print_weight_tracks_what_pretty_and_lifting_can_print() {
    for n in [10_usize, 100, 1_000] {
        let weight = scanned(&generate(Class::Block, n)).print_weight;
        let n = n as u64;
        assert!(
            (n * n..=n * n + 4 * n).contains(&weight),
            "block {n}: {weight}"
        );
    }
    for class in [Class::Plus, Class::Member, Class::Callchain] {
        let source = generate(class, 99_000);
        assert!(
            estimate(&source) <= (PRINT_GROWTH + 1) * source.len() as u64,
            "{}",
            class.name()
        );
    }
    let lifting = [
        Class::Ternnest,
        Class::Andnest,
        Class::Ornest,
        Class::Arrowbody,
        Class::Ifnb,
        Class::Whilenb,
        Class::Fornb,
        Class::Seqblock,
    ];
    for class in lifting {
        let value = estimate(&generate(class, 2_000));
        assert!(value >= 2 * 2_000 * 2_000, "{}: {value}", class.name());
    }
    for class in [Class::Stmtcond, Class::Elseifnb] {
        let source = generate(class, 2_000);
        let value = estimate(&source);
        assert!(
            value <= (7 + PRINT_LENGTH_FACTOR) * source.len() as u64,
            "{}: {value} for {}",
            class.name(),
            source.len()
        );
        let doubled = estimate(&generate(class, 4_000));
        assert!(
            doubled * 10 <= value * 22,
            "{}: {value} then {doubled}",
            class.name()
        );
    }
}

#[test]
fn the_print_refusal_depths_admit_their_nesting() {
    let depths = [
        (Class::Block, 20_000),
        (Class::If, 15_000),
        (Class::While, 15_000),
        (Class::Function, 15_000),
        (Class::Try, 15_000),
        (Class::Fnexpr, 12_000),
        (Class::Klass, 8_000),
    ];
    for (class, depth) in depths {
        let source = generate(class, depth);
        let result = scanned(&source);
        assert!(
            result.nesting <= 90_000,
            "{}: nesting {}",
            class.name(),
            result.nesting
        );
        let value = print_estimate(&result, source.len());
        assert!(value > 64 * MIB, "{}: estimate {value}", class.name());
    }
}

fn production(len: usize) -> Limits {
    Limits {
        nesting: 100_000,
        speculative_parens: 512,
        speculative_pairs: (1 << 17).max(len as u64 / 64),
        print_bytes: production_print_limit(len),
    }
}

#[test]
fn the_first_limit_crossed_in_the_stream_is_reported() {
    let block = generate(Class::Block, 100_000);
    assert_eq!(
        scan(&block, &production(block.len())),
        Err(DepthRefusal::PrintSize)
    );
    let array = generate(Class::Array, 200_000);
    assert_eq!(
        scan(&array, &production(array.len())),
        Err(DepthRefusal::Nesting)
    );
}

fn html_open_mid_line(n: usize) -> String {
    format!("a <!-- `\n{}x{}\n// `\n", "(".repeat(n), ")".repeat(n))
}

#[test]
fn a_mid_line_html_open_comment_hides_nothing_from_the_script_parse() {
    let shallow = html_open_mid_line(20_000);
    assert!(scanned(&shallow).nesting >= 40_000);
    let deep = html_open_mid_line(60_000);
    assert_eq!(
        scan(&deep, &production(deep.len())),
        Err(DepthRefusal::Nesting)
    );
}

fn html_close_at_line_start(n: usize) -> String {
    format!("x\n--> `\n{}1{}\n`;\n", "(".repeat(n), ")".repeat(n))
}

#[test]
fn html_comments_are_scanned_in_both_goals() {
    let close = html_close_at_line_start(20_000);
    assert!(scanned(&close).nesting >= 40_000);
    assert!(scan_as(&close, &UNLIMITED, Goal::Script).is_ok_and(|r| r.nesting >= 40_000));
    assert!(scan_as(&close, &UNLIMITED, Goal::Module).is_ok_and(|r| r.templates == 1));
    let open = "a <!-- `\n1";
    let module = scan_as(open, &UNLIMITED, Goal::Module).map(|r| r.templates);
    let script = scan_as(open, &UNLIMITED, Goal::Script).map(|r| r.templates);
    assert_eq!((module, script), (Ok(1), Ok(0)));
    assert_eq!(scanned(open).templates, 1);
    let line_start = "<!-- `\n1";
    assert_eq!(
        scan_as(line_start, &UNLIMITED, Goal::Module).map(|r| r.templates),
        Ok(0)
    );
}

fn nesting_and_regexes(source: &str) -> (u32, u32) {
    let result = scanned(source);
    (result.nesting, result.regexes)
}

#[test]
fn a_keyword_after_a_dot_is_a_plain_name() {
    assert_eq!(nesting_and_regexes("a.if(b) / 2 / c"), (4, 0));
    assert_eq!(nesting_and_regexes("a?.return / 2 / c"), (3, 0));
}

#[test]
fn await_and_yield_count_in_the_run() {
    assert_eq!(scanned("x=await a+await b;").nesting, 4);
    assert_eq!(scanned("function*g(){x=yield a+yield b}").nesting, 9);
}

#[test]
fn an_unterminated_literal_stops_at_the_line_end() {
    assert_eq!(nesting_and_regexes("x = \"abc\n(((1)))"), (7, 0));
    assert_eq!(nesting_and_regexes("x = /abc\n(((1)))"), (7, 0));
}

#[test]
fn unicode_spaces_and_line_terminators_separate_tokens() {
    assert_eq!(nesting_and_regexes("return\u{a0}/(((1)))/"), (1, 1));
    assert_eq!(nesting_and_regexes("return\u{2028}{} /x/g"), (3, 1));
}

#[test]
fn the_context_stack_is_capped_by_the_nesting_limit() {
    let source = format!("x=[{}]", "function,".repeat(40));
    assert_eq!(scanned(&source).nesting, 4);
    assert_eq!(scan(&source, &with_nesting(10)), Err(DepthRefusal::Nesting));
}
