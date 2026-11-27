use rustc_hash::FxHashMap;

pub const FIXED: [&str; 14] = [
    "Math",
    "globalThis",
    "RegExp",
    "global",
    "window",
    "Function",
    "Object",
    "String",
    "Array",
    "TextDecoder",
    "Uint8Array",
    "Buffer",
    "undefined",
    "arguments",
];

const KEYWORDS: [&str; 41] = [
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "async",
    "await",
    "of",
    "static",
];

const NO_LINE_BREAK_AFTER: [&str; 6] = ["return", "throw", "break", "continue", "yield", "async"];

const NO_LINE_BREAK_BEFORE: [&str; 3] = ["++", "--", "=>"];

const PUNCTUATORS: [&str; 52] = [
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "<<", ">>", "**", "{", "}", "(", ")", "[", "]", ";", ",", "<", ">", "+", "-", "*", "/", "%",
    "&", "|", "^", "!",
];

const SINGLE: [char; 5] = ['~', '?', ':', '=', '.'];

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Name(String),
    Word(&'static str),
    Punct(String),
    Str(Vec<u16>),
    Num(u64),
}

impl Token {
    fn render(&self, out: &mut String) {
        match self {
            Token::Name(name) | Token::Punct(name) => out.push_str(name),
            Token::Word(word) => out.push_str(word),
            Token::Str(units) => {
                out.push('"');
                for unit in units {
                    match char::from_u32(u32::from(*unit)) {
                        Some(c) if c.is_ascii_graphic() && c != '"' && c != '\\' => out.push(c),
                        Some(' ') => out.push(' '),
                        _ => out.push_str(&format!("\\u{unit:04x}")),
                    }
                }
                out.push('"');
            }
            Token::Num(bits) => out.push_str(&format!("{}", f64::from_bits(*bits))),
        }
    }
}

struct Lexer {
    chars: Vec<char>,
    offsets: Vec<u32>,
    at: usize,
}

fn is_name_start(c: char) -> bool {
    c == '$' || c == '_' || c.is_ascii_alphabetic() || (!c.is_ascii() && !c.is_whitespace())
}

fn is_name_part(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit()
}

impl Lexer {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn skip_space(&mut self) -> Option<bool> {
        let mut crossed = false;
        loop {
            match (self.peek(0), self.peek(1)) {
                (Some(c), _) if c.is_whitespace() || c == '\u{feff}' => {
                    crossed |= is_line_terminator(c);
                    self.at += 1;
                }
                (Some('/'), Some('/')) => {
                    while self.peek(0).is_some_and(|c| c != '\n' && c != '\r') {
                        self.at += 1;
                    }
                }
                (Some('/'), Some('*')) => {
                    self.at += 2;
                    while !(self.peek(0)? == '*' && self.peek(1) == Some('/')) {
                        crossed |= is_line_terminator(self.peek(0)?);
                        self.at += 1;
                    }
                    self.at += 2;
                }
                _ => return Some(crossed),
            }
        }
    }

    fn name(&mut self) -> Token {
        let start = self.at;
        while self.peek(0).is_some_and(is_name_part) {
            self.at += 1;
        }
        let name: String = self.chars[start..self.at].iter().collect();
        match KEYWORDS.iter().find(|word| **word == name) {
            Some(word) => Token::Word(word),
            None => Token::Name(name),
        }
    }

    fn number(&mut self) -> Option<Token> {
        let start = self.at;
        let radix = match (self.peek(0), self.peek(1).map(|c| c.to_ascii_lowercase())) {
            (Some('0'), Some('x')) => 16,
            (Some('0'), Some('o')) => 8,
            (Some('0'), Some('b')) => 2,
            _ => 10,
        };
        if radix != 10 {
            self.at += 2;
            let digits_start = self.at;
            while self.peek(0).is_some_and(|c| c.is_digit(radix)) {
                self.at += 1;
            }
            let digits: String = self.chars[digits_start..self.at].iter().collect();
            if digits.is_empty() || self.peek(0).is_some_and(is_name_part) {
                return None;
            }
            let value = u128::from_str_radix(&digits, radix).ok()?;
            return Some(Token::Num((value as f64).to_bits()));
        }
        while self.peek(0).is_some_and(|c| c.is_ascii_digit() || c == '.') {
            self.at += 1;
        }
        if self.peek(0).is_some_and(|c| c == 'e' || c == 'E') {
            self.at += 1;
            if self.peek(0).is_some_and(|c| c == '+' || c == '-') {
                self.at += 1;
            }
            while self.peek(0).is_some_and(|c| c.is_ascii_digit()) {
                self.at += 1;
            }
        }
        if self.peek(0).is_some_and(|c| is_name_part(c) || c == '.') {
            return None;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        if text.len() > 1 && text.starts_with('0') && !text.starts_with("0.") {
            return None;
        }
        Some(Token::Num(text.parse::<f64>().ok()?.to_bits()))
    }

    fn hex(&mut self, count: usize) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = value * 16 + self.peek(0)?.to_digit(16)?;
            self.at += 1;
        }
        Some(value)
    }

    fn escape(&mut self, units: &mut Vec<u16>) -> Option<()> {
        let c = self.peek(0)?;
        self.at += 1;
        let simple = match c {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            'b' => Some('\u{8}'),
            'f' => Some('\u{c}'),
            'v' => Some('\u{b}'),
            '0' if !self.peek(0).is_some_and(|d| d.is_ascii_digit()) => Some('\0'),
            '\n' | '\u{2028}' | '\u{2029}' => return Some(()),
            '\r' => {
                if self.peek(0) == Some('\n') {
                    self.at += 1;
                }
                return Some(());
            }
            'x' => char::from_u32(self.hex(2)?),
            'u' if self.peek(0) == Some('{') => {
                self.at += 1;
                let mut value = 0u32;
                while self.peek(0)? != '}' {
                    value = value.checked_mul(16)? + self.peek(0)?.to_digit(16)?;
                    self.at += 1;
                }
                self.at += 1;
                char::from_u32(value)
            }
            'u' => {
                units.push(u16::try_from(self.hex(4)?).ok()?);
                return Some(());
            }
            c if c.is_ascii_digit() => return None,
            other => Some(other),
        };
        let mut buffer = [0u16; 2];
        units.extend_from_slice(simple?.encode_utf16(&mut buffer));
        Some(())
    }

    fn string(&mut self, quote: char) -> Option<Token> {
        self.at += 1;
        let mut units = Vec::new();
        loop {
            let c = self.peek(0)?;
            self.at += 1;
            match c {
                c if c == quote => return Some(Token::Str(units)),
                '\\' => self.escape(&mut units)?,
                '\n' | '\r' => return None,
                c => {
                    let mut buffer = [0u16; 2];
                    units.extend_from_slice(c.encode_utf16(&mut buffer));
                }
            }
        }
    }

    fn punct(&mut self) -> Option<Token> {
        let rest: String = self.chars[self.at..self.chars.len().min(self.at + 4)]
            .iter()
            .collect();
        let found = PUNCTUATORS
            .iter()
            .copied()
            .find(|p| rest.starts_with(p))
            .map(str::to_string)
            .or_else(|| {
                let c = self.peek(0)?;
                SINGLE.contains(&c).then(|| c.to_string())
            })?;
        if found == "?." && self.peek(2).is_some_and(|c| c.is_ascii_digit()) {
            self.at += 1;
            return Some(Token::Punct("?".to_string()));
        }
        self.at += found.chars().count();
        Some(Token::Punct(found))
    }

    fn next(&mut self) -> Option<Option<(Token, bool, u32)>> {
        let crossed = self.skip_space()?;
        let Some(c) = self.peek(0) else {
            return Some(None);
        };
        let start = self.offsets[self.at];
        let token = match c {
            '\\' | '`' | '#' | '@' => return None,
            '"' | '\'' => self.string(c)?,
            c if c.is_ascii_digit() => self.number()?,
            '.' if self.peek(1).is_some_and(|d| d.is_ascii_digit()) => self.number()?,
            c if is_name_start(c) => self.name(),
            _ => self.punct()?,
        };
        Some(Some((token, crossed, start)))
    }
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn ends_at_line_break(before: Option<&Token>, token: &Token) -> bool {
    matches!(before, Some(Token::Word(word)) if NO_LINE_BREAK_AFTER.contains(word))
        || matches!(token, Token::Punct(p) if NO_LINE_BREAK_BEFORE.contains(&p.as_str()))
}

pub fn lex(text: &str) -> Option<Vec<(Token, u32)>> {
    let mut chars = Vec::new();
    let mut offsets = Vec::new();
    for (offset, c) in text.char_indices() {
        chars.push(c);
        offsets.push(u32::try_from(offset).ok()?);
    }
    offsets.push(u32::try_from(text.len()).ok()?);
    let mut lexer = Lexer {
        chars,
        offsets,
        at: 0,
    };
    let mut out: Vec<(Token, u32)> = Vec::new();
    while let Some((token, crossed, start)) = lexer.next()? {
        if crossed && ends_at_line_break(out.last().map(|(t, _)| t), &token) {
            return None;
        }
        out.push((token, start));
    }
    Some(out)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Binding(u32),
    Label,
    Free,
}

pub struct Literals<'k> {
    pub key: Key,
    pub values: &'k [Option<Vec<Token>>],
}

fn ends_operand(token: Option<&(Token, u32)>) -> bool {
    matches!(token, Some((Token::Name(_), _)))
        || matches!(token, Some((Token::Punct(p), _)) if p == ")" || p == "]")
}

fn property_name(tokens: &[Token]) -> Option<String> {
    let [Token::Str(units)] = tokens else {
        return None;
    };
    let name = String::from_utf16(units).ok()?;
    let mut chars = name.chars();
    let named = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    (named && !KEYWORDS.contains(&name.as_str())).then_some(name)
}

fn inline_literals(
    raw: Vec<(Token, u32)>,
    base: u32,
    resolve: &dyn Fn(u32) -> Option<Key>,
    literals: &Literals<'_>,
) -> Vec<(Token, u32)> {
    let mut out: Vec<(Token, u32)> = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        let value = match (&raw[at], raw.get(at + 1), raw.get(at + 2), raw.get(at + 3)) {
            (
                (Token::Name(_), start),
                Some((Token::Punct(open), _)),
                Some((Token::Num(bits), _)),
                Some((Token::Punct(close), _)),
            ) if open == "["
                && close == "]"
                && resolve(base + start) == Some(literals.key)
                && !matches!(out.last(), Some((Token::Punct(p), _)) if p == "." || p == "?.") =>
            {
                let index = f64::from_bits(*bits);
                (index.fract() == 0.0 && index >= 0.0)
                    .then(|| literals.values.get(index as usize).cloned().flatten())
                    .flatten()
            }
            _ => None,
        };
        let Some(value) = value else {
            out.push(raw[at].clone());
            at += 1;
            continue;
        };
        let start = raw[at].1;
        let member = matches!(out.last(), Some((Token::Punct(p), _)) if p == "[")
            && ends_operand(out.len().checked_sub(2).and_then(|i| out.get(i)))
            && matches!(raw.get(at + 4), Some((Token::Punct(p), _)) if p == "]");
        match property_name(&value).filter(|_| member) {
            Some(name) => {
                out.pop();
                out.push((Token::Punct(".".to_string()), start));
                out.push((Token::Name(name), start));
                at += 5;
            }
            None => {
                out.extend(value.into_iter().map(|token| (token, start)));
                at += 4;
            }
        }
    }
    out
}

pub fn dotted(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut at = 0;
    while at < tokens.len() {
        let operand = matches!(out.last(), Some(Token::Name(_)))
            || matches!(out.last(), Some(Token::Punct(p)) if p == ")" || p == "]");
        let name = match (&tokens[at], tokens.get(at + 1), tokens.get(at + 2)) {
            (Token::Punct(open), Some(key @ Token::Str(_)), Some(Token::Punct(close)))
                if operand && open == "[" && close == "]" =>
            {
                property_name(std::slice::from_ref(key))
            }
            _ => None,
        };
        match name {
            Some(name) => {
                out.push(Token::Punct(".".to_string()));
                out.push(Token::Name(name));
                at += 3;
            }
            None => {
                out.push(tokens[at].clone());
                at += 1;
            }
        }
    }
    out
}

pub fn canonical_in(
    text: &str,
    base: u32,
    resolve: &dyn Fn(u32) -> Option<Key>,
    literals: Option<&Literals<'_>>,
) -> Option<Vec<Token>> {
    let raw = match literals {
        Some(literals) => inline_literals(lex(text)?, base, resolve, literals),
        None => lex(text)?,
    };
    let mut bindings: FxHashMap<u32, usize> = FxHashMap::default();
    let mut labels: FxHashMap<String, usize> = FxHashMap::default();
    let mut tokens: Vec<Token> = Vec::with_capacity(raw.len());
    for (token, start) in raw {
        let after_dot = matches!(tokens.last(), Some(Token::Punct(p)) if p == "." || p == "?.");
        let token = match token {
            Token::Name(name) if !after_dot => match resolve(base + start) {
                Some(Key::Binding(id)) => {
                    let next = bindings.len();
                    Token::Name(format!("${}", bindings.entry(id).or_insert(next)))
                }
                Some(Key::Label) => {
                    let next = labels.len();
                    Token::Name(format!("@{}", labels.entry(name).or_insert(next)))
                }
                Some(Key::Free) | None => Token::Name(name),
            },
            other => other,
        };
        tokens.push(token);
    }
    Some(tokens)
}

pub fn render(tokens: &[Token]) -> String {
    let mut out = String::new();
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        token.render(&mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_name(text: &str) -> Vec<Token> {
        let names: FxHashMap<u32, String> = lex(text)
            .unwrap()
            .into_iter()
            .filter_map(|(token, start)| match token {
                Token::Name(name) => Some((start, name)),
                _ => None,
            })
            .collect();
        let resolve = |start: u32| {
            let name = names.get(&start)?;
            match FIXED.contains(&name.as_str()) {
                true => Some(Key::Free),
                false => {
                    Some(Key::Binding(name.bytes().fold(7u32, |h, b| {
                        h.wrapping_mul(31).wrapping_add(u32::from(b))
                    })))
                }
            }
        };
        canonical_in(text, 0, &resolve, None).unwrap()
    }

    #[test]
    fn layout_quotes_numbers_and_names_do_not_change_the_canonical_text() {
        let a = by_name("function f(a){return a[\"x\"]+0x10+'b'}");
        let b = by_name("function  g ( q ) {\n  return q[\"x\"] + 16 + \"b\";\n}");
        assert_eq!(
            render(&a),
            "function $0 ( $1 ) { return $1 [ \"x\" ] + 16 + \"b\" }"
        );
        assert_eq!(
            render(&b),
            "function $0 ( $1 ) { return $1 [ \"x\" ] + 16 + \"b\" ; }"
        );
    }

    #[test]
    fn property_names_and_free_names_are_kept() {
        let c = by_name("var o = globalThis.String.length; o.push(Object)");
        assert_eq!(
            render(&c),
            "var $0 = globalThis . String . length ; $0 . push ( Object )"
        );
    }

    #[test]
    fn unresolved_positions_keep_their_text() {
        let tokens = canonical_in("({ key: 1 })", 0, &|_| None, None).unwrap();
        assert_eq!(render(&tokens), "( { key : 1 } )");
    }

    #[test]
    fn line_breaks_where_asi_ends_a_statement_are_refused() {
        assert!(lex("return\nx").is_none());
        assert!(lex("return/*\n*/x").is_none());
        assert!(lex("return\u{2028}x").is_none());
        assert!(lex("throw\r\nx").is_none());
        assert!(lex("a\n++b").is_none());
        assert!(lex("return x\n+ y").is_some());
        assert!(lex("return/* */x").is_some());
    }

    #[test]
    fn templates_and_escaped_names_are_refused() {
        assert!(lex("var a = `x`").is_none());
        assert!(lex("var \\u0061 = 1").is_none());
        assert!(lex("var a = 08").is_none());
    }

    #[test]
    fn string_escapes_decode_to_units() {
        let tokens: Vec<Token> = lex("'\\x41\\u0042\\u{43}\\n\\\\'")
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(tokens, [Token::Str("ABC\n\\".encode_utf16().collect())]);
    }
}
