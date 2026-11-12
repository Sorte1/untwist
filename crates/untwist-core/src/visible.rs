use oxc_allocator::Allocator;
use oxc_ast::ast::{
    JSXAttributeValue, JSXText, Program, RegExpLiteral, StringLiteral, TaggedTemplateExpression,
    TemplateLiteral,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;

use crate::parse::{has_error, parse_program};

mod judge;
mod scan;

pub use judge::Respelling;
use scan::Hits;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Spelling {
    String,
    Template,
    RegExp,
    Comment,
    Jsx,
}

pub fn is_hidden(ch: char) -> bool {
    matches!(
        ch,
        '\0'..='\u{8}'
            | '\u{b}'..='\u{1f}'
            | '\u{7f}'..='\u{9f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn hidden_or_tab(ch: char) -> bool {
    is_hidden(ch) || ch == '\t'
}

fn hits(text: &str, spelling: Spelling) -> Hits<'_> {
    match spelling {
        Spelling::String => Hits::new(text, true, hidden_or_tab),
        _ => Hits::new(text, false, is_hidden),
    }
}

fn holds_hidden(text: &str) -> bool {
    Hits::new(text, false, is_hidden).next().is_some()
}

pub fn visible_literals(code: String) -> String {
    if hits(&code, Spelling::String).next().is_none() {
        return code;
    }
    let edits = {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &code);
        if has_error(&parsed.diagnostics) {
            return code;
        }
        respellings(&code, &parsed.program)
    };
    applied(&code, edits)
}

pub fn respelling(program: &Program<'_>, printed: &str) -> Respelling {
    let hidden = holds_hidden(printed);
    match hidden || printed.contains('\t') {
        true => judge::judged(program, hidden),
        false => Respelling::Plain,
    }
}

pub fn respelled_as(code: String, how: Respelling) -> String {
    match how {
        Respelling::Plain => code,
        Respelling::Parse => visible_literals(code),
        Respelling::Text if code.contains(['\r', '\u{2028}', '\u{2029}']) => visible_literals(code),
        Respelling::Text => respelled(&code, Spelling::String),
    }
}

pub fn visible_string_literal(literal: &str) -> String {
    respelled(literal, Spelling::String)
}

fn respellings(code: &str, program: &Program<'_>) -> Vec<(Span, String)> {
    let mut finder = Finder {
        code,
        edits: Vec::new(),
    };
    finder.visit_program(program);
    if let Some(hashbang) = &program.hashbang {
        finder.respell(hashbang.span, Spelling::Comment);
    }
    for comment in program.comments.iter() {
        finder.respell(comment.span, Spelling::Comment);
    }
    finder.edits.sort_by_key(|(span, _)| span.start);
    finder.edits
}

fn applied(code: &str, edits: Vec<(Span, String)>) -> String {
    let mut out = String::with_capacity(code.len() + edits.len() * 8);
    let mut at = 0usize;
    for (span, text) in edits {
        let (start, end) = (span.start as usize, span.end as usize);
        if start < at {
            continue;
        }
        out.push_str(&code[at..start]);
        out.push_str(&text);
        at = end;
    }
    out.push_str(&code[at..]);
    out
}

struct Finder<'s> {
    code: &'s str,
    edits: Vec<(Span, String)>,
}

impl Finder<'_> {
    fn respell(&mut self, span: Span, spelling: Spelling) {
        let Some(text) = self.code.get(span.start as usize..span.end as usize) else {
            return;
        };
        if hits(text, spelling).next().is_none() {
            return;
        }
        self.edits.push((span, respelled(text, spelling)));
    }
}

impl<'a> Visit<'a> for Finder<'_> {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.respell(it.span, Spelling::String);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        for quasi in &it.quasis {
            self.respell(quasi.span, Spelling::Template);
        }
        self.visit_expressions(&it.expressions);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.visit_expression(&it.tag);
        self.visit_expressions(&it.quasi.expressions);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        self.respell(it.span, Spelling::RegExp);
    }

    fn visit_jsx_attribute_value(&mut self, it: &JSXAttributeValue<'a>) {
        match it {
            JSXAttributeValue::StringLiteral(literal) => self.respell(literal.span, Spelling::Jsx),
            other => walk::walk_jsx_attribute_value(self, other),
        }
    }

    fn visit_jsx_text(&mut self, it: &JSXText<'a>) {
        self.respell(it.span, Spelling::Jsx);
    }
}

fn escaped_at(text: &str, at: usize) -> bool {
    text.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|&&byte| byte == b'\\')
        .count()
        % 2
        == 1
}

fn respelled(text: &str, spelling: Spelling) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut copied = 0;
    for (at, ch) in hits(text, spelling) {
        out.push_str(&text[copied..at]);
        copied = at + ch.len_utf8();
        let escaped =
            escaped_at(text, at) && !matches!(spelling, Spelling::Comment | Spelling::Jsx);
        if matches!(ch, '\r' | '\u{2028}' | '\u{2029}') {
            let before_newline = text[copied..].starts_with('\n');
            line_break(&mut out, ch, before_newline, escaped, spelling);
            continue;
        }
        if escaped {
            out.pop();
        }
        escape(&mut out, ch, spelling);
    }
    out.push_str(&text[copied..]);
    out
}

fn line_break(out: &mut String, ch: char, before_newline: bool, escaped: bool, spelling: Spelling) {
    let kept_as_break = escaped
        || matches!(spelling, Spelling::Comment)
        || (ch == '\r' && matches!(spelling, Spelling::Template | Spelling::Jsx));
    match (kept_as_break, ch == '\r' && before_newline) {
        (true, true) => {}
        (true, false) => out.push('\n'),
        (false, _) if spelling == Spelling::String && ch == '\r' => out.push_str("\\r"),
        (false, _) => escape(out, ch, spelling),
    }
}

fn escape(out: &mut String, ch: char, spelling: Spelling) {
    let code = u32::from(ch);
    match (spelling, ch) {
        (Spelling::Jsx, _) => out.push_str(&format!("&#x{code:X};")),
        (Spelling::String, '\t') => out.push_str("\\t"),
        _ if code <= 0xff => out.push_str(&format!("\\x{code:02X}")),
        _ => out.push_str(&format!("\\u{code:04X}")),
    }
}
