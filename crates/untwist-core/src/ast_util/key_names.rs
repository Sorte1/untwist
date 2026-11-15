use oxc_ast::ast::{
    Argument, BinaryExpression, CallExpression, Expression, IdentifierName, ImportExpression,
    Program, StringLiteral, TemplateElement, TemplateLiteral,
};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_syntax::operator::BinaryOperator;
use rustc_hash::FxHashSet;

const REQUIRE: &str = "require";

pub fn key_names(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<String> {
    let mut keys = KeyNames {
        scoping,
        names: FxHashSet::default(),
    };
    keys.visit_program(program);
    keys.names
}

struct KeyNames<'s> {
    scoping: &'s Scoping,
    names: FxHashSet<String>,
}

fn free_require(scoping: &Scoping, callee: &Expression<'_>) -> bool {
    let Expression::Identifier(id) = callee else {
        return false;
    };
    id.name == REQUIRE
        && id
            .reference_id
            .get()
            .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

impl<'a> Visit<'a> for KeyNames<'_> {
    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.names.insert(it.name.as_str().to_string());
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if let Some(value) = it.value.as_str() {
            self.names.insert(value.to_string());
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let required = free_require(self.scoping, &it.callee)
            && matches!(it.arguments.as_slice(), [Argument::StringLiteral(_)]);
        if required {
            return;
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if matches!(&it.source, Expression::StringLiteral(_)) && it.options.is_none() {
            return;
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        for quasi in &it.quasis {
            if let Some(cooked) = quasi
                .value
                .cooked
                .as_ref()
                .and_then(|cooked| cooked.as_str())
            {
                self.names.insert(cooked.to_string());
            }
        }
        walk::walk_template_literal(self, it);
    }
}

const ESCAPE_LEVELS: usize = 4;

pub fn text_names(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<String> {
    let mut texts = TextNames {
        scoping,
        names: FxHashSet::default(),
    };
    texts.visit_program(program);
    texts.names
}

struct TextNames<'s> {
    scoping: &'s Scoping,
    names: FxHashSet<String>,
}

impl TextNames<'_> {
    fn add(&mut self, text: &str) {
        let mut current = text.to_string();
        for _ in 0..ESCAPE_LEVELS {
            insert_tokens(&current, &mut self.names);
            if !current.contains('\\') {
                return;
            }
            current = unescape_once(&current);
        }
        insert_tokens(&current, &mut self.names);
    }
}

fn is_name_char(c: char) -> bool {
    c == '_' || c == '$' || c.is_alphanumeric()
}

fn insert_tokens(text: &str, names: &mut FxHashSet<String>) {
    for token in text.split(|c: char| !is_name_char(c)) {
        if !token.is_empty() && !names.contains(token) {
            names.insert(token.to_string());
        }
    }
}

fn unescape_once(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => out.extend(unicode_escape(&mut chars)),
            Some('x') => out.extend(hex_digits(&mut chars, 2).and_then(char::from_u32)),
            Some(d @ '0'..='7') => out.extend(octal_escape(d, &mut chars)),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn unicode_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    if chars.peek() != Some(&'{') {
        return hex_digits(chars, 4).and_then(char::from_u32);
    }
    chars.next();
    let mut value: u32 = 0;
    for c in chars.by_ref() {
        if c == '}' {
            return char::from_u32(value);
        }
        value = value.checked_mul(16)?.checked_add(c.to_digit(16)?)?;
    }
    None
}

fn hex_digits(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, count: usize) -> Option<u32> {
    let mut value = 0;
    for _ in 0..count {
        let digit = chars.peek()?.to_digit(16)?;
        chars.next();
        value = value * 16 + digit;
    }
    Some(value)
}

fn octal_escape(first: char, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    let mut value = first.to_digit(8)?;
    for _ in 0..2 {
        match chars.peek().and_then(|c| c.to_digit(8)) {
            Some(digit) if value * 8 + digit <= 0o377 => {
                value = value * 8 + digit;
                chars.next();
            }
            _ => break,
        }
    }
    char::from_u32(value)
}

fn literal_text(expression: &Expression<'_>) -> Option<String> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str())
            .map(str::to_string),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            Some(literal_text(&binary.left)? + &literal_text(&binary.right)?)
        }
        _ => None,
    }
}

impl<'a> Visit<'a> for TextNames<'_> {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if let Some(value) = it.value.as_str() {
            self.add(value);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let required = free_require(self.scoping, &it.callee)
            && matches!(it.arguments.as_slice(), [Argument::StringLiteral(_)]);
        if !required {
            walk::walk_call_expression(self, it);
        }
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if matches!(&it.source, Expression::StringLiteral(_)) && it.options.is_none() {
            return;
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        if let Some(cooked) = it.value.cooked.as_ref().and_then(|cooked| cooked.as_str()) {
            self.add(cooked);
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let folded = (it.operator == BinaryOperator::Addition)
            .then(|| literal_text(&it.left).zip(literal_text(&it.right)))
            .flatten();
        let Some((left, right)) = folded else {
            walk::walk_binary_expression(self, it);
            return;
        };
        self.add(&(left + &right));
        self.visit_leaves(&it.left);
        self.visit_leaves(&it.right);
    }
}

impl TextNames<'_> {
    fn visit_leaves<'a>(&mut self, expression: &Expression<'a>) {
        match expression.without_parentheses() {
            Expression::BinaryExpression(binary) => {
                self.visit_leaves(&binary.left);
                self.visit_leaves(&binary.right);
            }
            leaf => self.visit_expression(leaf),
        }
    }
}
