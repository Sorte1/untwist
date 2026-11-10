use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_core::visible::{
    is_hidden, respelled_as, respelling, visible_literals, visible_string_literal, Respelling,
};

fn shown(code: &str) -> String {
    visible_literals(code.to_string())
}

#[test]
fn string_literals_spell_hidden_characters_as_escapes() {
    assert_eq!(shown("var c = \"\u{9b}31mC1\";"), "var c = \"\\x9B31mC1\";");
    assert_eq!(
        shown("var d = \"a\u{7f}b\u{9c}\";"),
        "var d = \"a\\x7Fb\\x9C\";"
    );
    assert_eq!(
        shown("var r = \"\u{202e}x\u{2066}\";"),
        "var r = \"\\u202Ex\\u2066\";"
    );
    assert_eq!(
        shown("var s = \"a\u{2028}b\u{2029}\";"),
        "var s = \"a\\u2028b\\u2029\";"
    );
    assert_eq!(shown("var t = \"a\tb\u{1}\";"), "var t = \"a\\tb\\x01\";");
    assert_eq!(
        shown("var k = { \"\u{1b}\": 1 };"),
        "var k = { \"\\x1B\": 1 };"
    );
}

#[test]
fn an_escaped_hidden_character_keeps_its_value() {
    assert_eq!(shown("var e = \"\\\u{1b}\";"), "var e = \"\\x1B\";");
    assert_eq!(shown("var f = \"\\\\\u{1b}\";"), "var f = \"\\\\\\x1B\";");
    assert_eq!(shown("var g = \"a\\\u{2028}b\";"), "var g = \"a\\\nb\";");
    assert_eq!(shown("var h = `a\\\u{2028}b`;"), "var h = `a\\\nb`;");
    assert_eq!(shown("var i = `\\\u{9b}`;"), "var i = `\\x9B`;");
}

#[test]
fn untagged_templates_spell_hidden_characters_as_escapes() {
    assert_eq!(
        shown("var t = `safe\u{1b}]0;owned\u{7}\u{1b}[1A`;"),
        "var t = `safe\\x1B]0;owned\\x07\\x1B[1A`;"
    );
    assert_eq!(shown("var u = `a\tb\nc`;"), "var u = `a\tb\nc`;");
    assert_eq!(shown("var v = `a\rb\r\nc`;"), "var v = `a\nb\nc`;");
    assert_eq!(
        shown("var w = `${x}\u{202e}${y}\u{0}1`;"),
        "var w = `${x}\\u202E${y}\\x001`;"
    );
}

#[test]
fn tagged_templates_keep_their_raw_text() {
    let code = "var t = String.raw`a\u{1b}b${`c\u{1b}`}`;";
    assert_eq!(shown(code), "var t = String.raw`a\u{1b}b${`c\\x1B`}`;");
}

#[test]
fn regular_expressions_spell_hidden_characters_as_escapes() {
    assert_eq!(
        shown("var r = /a\u{7f}[\u{9b}]/g;"),
        "var r = /a\\x7F[\\x9B]/g;"
    );
    assert_eq!(shown("var s = /\\\u{1b}/;"), "var s = /\\x1B/;");
}

#[test]
fn comments_show_hidden_characters() {
    assert_eq!(
        shown("/* a\u{202e}b\u{1b} */ x;"),
        "/* a\\u202Eb\\x1B */ x;"
    );
    assert_eq!(shown("// a\u{9b}\nx;"), "// a\\x9B\nx;");
    assert_eq!(shown("/* a\u{2028}b */ x;"), "/* a\nb */ x;");
    assert_eq!(shown("#!/bin/x \u{1b}\nx;"), "#!/bin/x \\x1B\nx;");
}

#[test]
fn jsx_text_and_attributes_use_character_references() {
    assert_eq!(
        shown("<a b=\"\u{1b}\">\u{202e}</a>;"),
        "<a b=\"&#x1B;\">&#x202E;</a>;"
    );
}

#[test]
fn code_without_hidden_characters_is_returned_as_is() {
    let code = "var a = \"caf\u{e9} \u{4e2d}\u{1f600}\", b = `x\ty`;\n";
    assert_eq!(shown(code), code);
}

#[test]
fn identifiers_and_code_that_does_not_parse_are_left_alone() {
    assert_eq!(shown("var \u{1b} = 1;"), "var \u{1b} = 1;");
}

#[test]
fn a_printed_string_literal_can_be_made_visible_alone() {
    assert_eq!(
        visible_string_literal("\"./a\u{9b}.wasm\""),
        "\"./a\\x9B.wasm\""
    );
}

fn judged_and_respelled(source: &str) -> (Respelling, String, String) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let printed = pretty_codegen(&parsed.program);
    let how = respelling(&parsed.program, &printed);
    let parsed_way = visible_literals(printed.clone());
    (how, respelled_as(printed, how), parsed_way)
}

#[test]
fn respelling_the_text_alone_matches_respelling_by_parse() {
    let sources = [
        "var a = \"x\\u009b\\ty\", b = `p${a}\\x1b\\\\q`, c = /\\x7f[\\u202e]/;",
        "var d = { \"\\x1b\": `\\x00${1}\\u2066` }; f(\"\\u0085\", '\\x07');",
        "var e = `a${\"\\x9b0\"}b\u{1b}`;",
    ];
    for source in sources {
        let (how, text_way, parsed_way) = judged_and_respelled(source);
        assert_eq!(how, Respelling::Text, "{source}");
        assert_eq!(text_way, parsed_way, "{source}");
        assert!(!text_way.chars().any(is_hidden), "{source}");
    }
}

#[test]
fn tagged_templates_comments_and_raw_tabs_are_respelled_by_parse() {
    let sources = [
        "var a = \"\\x1b\"; var b = String.raw`\\x1b\u{1b}`;",
        "/* \u{1b} */ var a = \"\\x1b\";",
        "var a = \"\\x1b\", b = `\t`;",
        "var a = \"\\x1b\", b = /\t/;",
    ];
    for source in sources {
        let (how, text_way, parsed_way) = judged_and_respelled(source);
        assert_eq!(how, Respelling::Parse, "{source}");
        assert_eq!(text_way, parsed_way, "{source}");
    }
}

#[test]
fn printed_code_without_hidden_characters_needs_no_respelling() {
    let (how, _, _) = judged_and_respelled("var a = \"plain\", b = `t\tab`; /* \t */");
    assert_eq!(how, Respelling::Plain);
}
