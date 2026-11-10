use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span, SPAN};
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_core::source_map::{
    self, coverage, inline_map, last_map_url, retarget, Origins, OwnedSourceMap, Positions,
    Recording, Through,
};

const CALLS: &str = "console.log(f(1), g(2));";

fn column_of(text: &str, needle: &str) -> u32 {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is in the text"));
    let start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
    u32::try_from(text[start..at].encode_utf16().count()).expect("a small column")
}

fn line_of(text: &str, needle: &str) -> u32 {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is in the text"));
    u32::try_from(text[..at].matches('\n').count()).expect("a small line")
}

fn source_column(map: &OwnedSourceMap, text: &str, needle: &str) -> u32 {
    let table = map.generate_lookup_table();
    let token = map
        .lookup_token(&table, line_of(text, needle), column_of(text, needle))
        .expect("a mapping covers the position");
    assert_eq!(token.get_dst_line(), line_of(text, needle));
    token.get_src_col()
}

fn argument_spans(program: &oxc_ast::ast::Program<'_>) -> Vec<Span> {
    let Statement::ExpressionStatement(statement) = &program.body[0] else {
        panic!("expression statement");
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("call");
    };
    call.arguments
        .iter()
        .filter_map(|argument| argument.as_expression().map(GetSpan::span))
        .collect()
}

#[test]
fn a_replacement_without_a_span_maps_to_the_node_it_replaced() {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut parsed = parse_program(&alloc, CALLS);
    let mut sem = build_scoping(&parsed.program);
    let spans = argument_spans(&parsed.program);
    let recording = Recording::start();
    let mut edits = ExprEdits::new();
    let value = Expression::new_string_literal(
        SPAN,
        oxc_str::Str::from_str_in("decoded", &builder),
        None,
        &builder,
    );
    assert!(edits.insert(spans[1], value));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    let mapped = source_map::mapped(&mut parsed.program, &recording.finish()).expect("a map");

    assert_eq!(argument_spans(&parsed.program)[1], spans[1]);
    assert_eq!(
        source_column(&mapped.map, &mapped.printed, "\"decoded\""),
        column_of(CALLS, "g(2)")
    );
}

#[test]
fn a_node_moved_in_from_elsewhere_maps_to_the_site_it_landed_on() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, CALLS);
    let mut sem = build_scoping(&parsed.program);
    let spans = argument_spans(&parsed.program);
    let moved = {
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expression statement");
        };
        let Expression::CallExpression(call) = &statement.expression else {
            panic!("call");
        };
        call.arguments[0]
            .as_expression()
            .expect("an expression")
            .clone_in(&alloc)
    };
    let recording = Recording::start();
    let mut edits = ExprEdits::new();
    assert!(edits.insert(spans[1], moved));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    let mapped = source_map::mapped(&mut parsed.program, &recording.finish()).expect("a map");

    assert_eq!(mapped.printed, "console.log(f(1), f(1));");
    let second = mapped.printed.rfind("f(1)").expect("the moved call");
    let table = mapped.map.generate_lookup_table();
    let token = mapped
        .map
        .lookup_token(&table, 0, u32::try_from(second).expect("a small column"))
        .expect("a mapping");
    assert_eq!(token.get_src_col(), column_of(CALLS, "g(2)"));
}

#[test]
fn statements_put_in_place_of_one_map_to_it() {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, "a();\nb();\nc();");
    let mut sem = build_scoping(&parsed.program);
    let spans: Vec<Span> = parsed.program.body.iter().map(GetSpan::span).collect();
    let extra = {
        let inner = parse_program(&alloc, "first();\nsecond();");
        inner.program.body.into_iter().collect::<Vec<_>>()
    };
    let recording = Recording::start();
    let mut edits = StmtEdits::new();
    assert!(edits.replace(spans[2], extra));
    assert_eq!(
        edits.apply(&mut parsed.program, &alloc, &mut sem.scoping),
        1
    );

    let mapped = source_map::mapped(&mut parsed.program, &recording.finish()).expect("a map");

    let table = mapped.map.generate_lookup_table();
    for needle in ["first", "second"] {
        let token = mapped
            .map
            .lookup_token(&table, line_of(&mapped.printed, needle), 0)
            .expect("a mapping");
        assert_eq!(
            (token.get_src_line(), token.get_src_col()),
            (2, 0),
            "{needle}"
        );
    }
}

#[test]
fn a_map_follows_the_text_into_another_layout() {
    let source = "function f(){return 1}f();";
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let mapped = source_map::mapped(&mut parsed.program, &Origins::default()).expect("a map");
    let finished = "// note\nfunction f() { return 1; }\n/* why */ f();";

    let map = retarget(mapped.map, &mapped.printed, finished);

    assert_eq!(
        source_column(&map, finished, "return"),
        column_of(source, "return")
    );
    assert_eq!(
        source_column(&map, finished, "f();"),
        column_of(source, "f();")
    );
    let lines: Vec<&str> = finished.split('\n').collect();
    for token in map.get_tokens() {
        let line = lines[token.get_dst_line() as usize];
        assert!(token.get_dst_col() as usize <= line.len());
    }
}

#[test]
fn a_map_counts_columns_in_utf16_units() {
    let source = "var s=\"\u{1F600}\";f();";
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let mapped = source_map::mapped(&mut parsed.program, &Origins::default()).expect("a map");
    let finished = "var s = \"\u{1F600}\"; f();";

    let map = retarget(mapped.map, &mapped.printed, finished);

    assert_eq!(
        source_column(&map, finished, "f();"),
        column_of(source, "f();")
    );
}

#[test]
fn coverage_counts_the_tokens_a_mapping_reaches() {
    let source = "a();\nb();";
    let map = source_map::identity(source);
    let all = coverage(&map, source);
    assert_eq!((all.tokens, all.on_line, all.exact), (8, 8, 2));

    let shifted = "/* note\n*/ a();\n// gone\n\nb(); c();";
    let partly = coverage(&map, shifted);
    assert_eq!(partly.tokens, 12);
    assert_eq!(partly.on_line, 4);
    assert!(partly.share() < 0.5);

    assert!((coverage(&source_map::unmapped(source), "").share() - 1.0).abs() < f64::EPSILON);
    assert_eq!(coverage(&source_map::unmapped(source), source).on_line, 0);
}

#[test]
fn an_inline_map_is_read_from_the_last_link() {
    let json = r#"{"version":3,"sources":["a.ts"],"names":[],"mappings":"AAAA"}"#;
    let based =
        "eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbImEudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQUEifQ==";
    let source = format!(
        "//# sourceMappingURL=first.js.map\na();\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,{based}\n"
    );
    assert_eq!(inline_map(&source).as_deref(), Some(json));

    let escaped = "a();\n//@ sourceMappingURL=data:application/json,%7B%22version%22%3A3%7D";
    assert_eq!(inline_map(escaped).as_deref(), Some(r#"{"version":3}"#));

    assert_eq!(
        last_map_url("a();\n//# sourceMappingURL=a.js.map\n"),
        Some("a.js.map")
    );
    assert_eq!(inline_map("a();\n//# sourceMappingURL=a.js.map"), None);
    assert_eq!(
        inline_map("a();\n//# sourceMappingURL=data:application/json;base64,!!!"),
        None
    );
    assert_eq!(last_map_url("a();"), None);
}

#[test]
fn a_map_composes_through_the_map_of_its_input() {
    let built = "var a=1;\nconsole.log(a);";
    let earlier = source_map::parse(
        r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["let a = 1\n\nconsole.log(a)\n"],"names":[],"mappings":"AAAA;AAEA"}"#,
    )
    .expect("the earlier map parses");

    let map = source_map::compose(source_map::identity(built), earlier);

    assert_eq!(map.get_source(0), Some("src/app.ts"));
    let table = map.generate_lookup_table();
    let token = map.lookup_token(&table, 1, 0).expect("a mapping");
    assert_eq!(token.get_src_line(), 2);
    assert!(source_map::parse("not a map").is_none());
}

#[test]
fn a_map_of_a_part_goes_through_the_map_of_the_whole() {
    let original = "skip();keep(\"\u{1F600}\",inner());";
    let alloc = Allocator::default();
    let mut first = parse_program(&alloc, original);
    let mapped = source_map::mapped(&mut first.program, &Origins::default()).expect("a map");
    let whole = "skip(); keep(\"\u{1F600}\", inner());";
    let outer = retarget(mapped.map, &mapped.printed, whole);
    let mut part = parse_program(&alloc, whole);
    part.program.body.remove(0);
    let (inner, start) =
        source_map::mapped_window(&mut part.program, &Origins::default()).expect("a map");

    let map = Through::new(&outer, whole).rebased(inner.map, start);

    assert_eq!(Some(start), whole.find("keep"));
    for needle in ["keep", "inner"] {
        assert_eq!(
            source_column(&map, &inner.printed, needle),
            column_of(original, needle),
            "{needle}"
        );
    }
    assert_eq!(map.get_source(0), Some(source_map::SOURCE_NAME));
    assert_eq!(map.get_source_content(0), None);
}

#[test]
fn positions_count_lines_and_utf16_columns() {
    let text = "ab\n\u{1F600}c\r\nd\u{2028}e";
    let positions = Positions::new(text);
    let at = |needle: char| positions.of(text.find(needle).expect("in the text"));
    assert_eq!(at('c'), (1, 2));
    assert_eq!(at('d'), (2, 0));
    assert_eq!(at('e'), (3, 0));
    assert_eq!(positions.of(text.len() + 9), (3, 1));

    let long = format!("{}\n{}z", "x".repeat(5000), "\u{e9}".repeat(3000));
    let positions = Positions::new(&long);
    assert_eq!(positions.of(long.len() - 1), (1, 3000));
}

#[test]
fn inputs_are_named_by_their_place() {
    assert_eq!(source_map::input_name(0, 1), "input.js");
    assert_eq!(source_map::input_name(1, 3), "input2.js");
    assert_eq!(source_map::input_index("input2.js", 3), Some(1));
    assert_eq!(source_map::input_index("input.js", 1), Some(0));
    assert_eq!(source_map::input_index("other.js", 3), None);
    let map = source_map::renamed(source_map::identity("a();"), "input2.js");
    assert_eq!(map.get_source(0), Some("input2.js"));
    let map = source_map::named(source_map::identity("a();"), Some("out.js"), "../in.js");
    assert_eq!(
        (map.get_file(), map.get_source(0)),
        (Some("out.js"), Some("../in.js"))
    );
}
