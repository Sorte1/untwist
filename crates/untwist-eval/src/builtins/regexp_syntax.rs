const ADMITTED_FLAGS: &str = "gimsuv";
const QUANTIFIER_BOUND_CAP: u32 = 1000;
const SYNTAX_CHARS: &str = "^$\\.*+?()[]{}|/";
const CONTROL_ESCAPES: [(char, char); 5] = [
    ('n', '\n'),
    ('r', '\r'),
    ('t', '\t'),
    ('f', '\u{c}'),
    ('v', '\u{b}'),
];
const CLASS_ESCAPES: &str = "dDsSwW";

#[derive(Clone, Copy)]
struct Mode {
    ignore_case: bool,
    unicode: bool,
    sets: bool,
}

struct Scan {
    chars: Vec<char>,
    at: usize,
    groups: u32,
    mode: Mode,
}

enum ClassAtom {
    Char(char),
    Escape,
}

pub fn admitted(pattern: &str, flags: &str) -> bool {
    if !flags.chars().all(|flag| ADMITTED_FLAGS.contains(flag)) {
        return false;
    }
    let mode = Mode {
        ignore_case: flags.contains('i'),
        unicode: flags.contains('u') || flags.contains('v'),
        sets: flags.contains('v'),
    };
    let mut scan = Scan {
        chars: pattern.chars().collect(),
        at: 0,
        groups: count_groups(pattern),
        mode,
    };
    scan.disjunction() && scan.at == scan.chars.len()
}

fn count_groups(pattern: &str) -> u32 {
    let bytes = pattern.as_bytes();
    let mut count = 0;
    let mut index = 0;
    let mut in_class = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'[' => in_class = true,
            b']' => in_class = false,
            b'(' if !in_class && bytes.get(index + 1) != Some(&b'?') => count += 1,
            _ => {}
        }
        index += 1;
    }
    count
}

fn literal_ok(ch: char, mode: Mode) -> bool {
    if (ch as u32) > 0xFFFF || (0xD800..=0xDFFF).contains(&(ch as u32)) {
        return false;
    }
    ch.is_ascii() || !mode.unicode
}

impl Scan {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn eat(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.at += 1;
            return true;
        }
        false
    }

    fn disjunction(&mut self) -> bool {
        loop {
            if !self.alternative() {
                return false;
            }
            if !self.eat('|') {
                return true;
            }
        }
    }

    fn alternative(&mut self) -> bool {
        while let Some(ch) = self.peek() {
            if ch == '|' || ch == ')' {
                return true;
            }
            let quantifiable = match self.term() {
                Some(quantifiable) => quantifiable,
                None => return false,
            };
            if self.at_quantifier() && (!quantifiable || !self.quantifier()) {
                return false;
            }
        }
        true
    }

    fn at_quantifier(&self) -> bool {
        match self.peek() {
            Some('*' | '+' | '?') => true,
            Some('{') => self.mode.unicode || self.braces_quantify(),
            _ => false,
        }
    }

    fn braces_quantify(&self) -> bool {
        let rest = &self.chars[self.at + 1..];
        let digits = |from: usize| {
            rest[from..]
                .iter()
                .take_while(|ch| ch.is_ascii_digit())
                .count()
        };
        let low = digits(0);
        if low == 0 {
            return false;
        }
        match rest.get(low) {
            Some('}') => true,
            Some(',') => {
                let high = digits(low + 1);
                rest.get(low + 1 + high) == Some(&'}')
            }
            _ => false,
        }
    }

    fn term(&mut self) -> Option<bool> {
        let ch = self.peek()?;
        self.at += 1;
        match ch {
            '^' | '$' => Some(false),
            '.' => Some(true),
            '(' => self.group().then_some(true),
            '[' => self.class().then_some(true),
            '\\' => self.atom_escape(),
            '{' | '}' | ']' if !self.mode.unicode => {
                self.at -= 1;
                let literal = !(ch == '{' && self.braces_quantify());
                self.at += 1;
                literal.then_some(true)
            }
            '*' | '+' | '?' | '{' | '}' | ']' => None,
            other => literal_ok(other, self.mode).then_some(true),
        }
    }

    fn group(&mut self) -> bool {
        if self.eat('?') && !self.eat(':') {
            return false;
        }
        self.disjunction() && self.eat(')')
    }

    fn quantifier(&mut self) -> bool {
        let bounded = match self.peek() {
            Some('{') => {
                self.at += 1;
                self.bounds()
            }
            Some(_) => {
                self.at += 1;
                true
            }
            None => false,
        };
        if bounded {
            self.eat('?');
        }
        bounded && !self.at_quantifier()
    }

    fn bounds(&mut self) -> bool {
        let Some(low) = self.number() else {
            return false;
        };
        if self.eat('}') {
            return true;
        }
        if !self.eat(',') {
            return false;
        }
        if self.eat('}') {
            return true;
        }
        let Some(high) = self.number() else {
            return false;
        };
        low <= high && self.eat('}')
    }

    fn number(&mut self) -> Option<u32> {
        let start = self.at;
        let mut value: u32 = 0;
        while let Some(digit) = self.peek().and_then(|ch| ch.to_digit(10)) {
            value = value.checked_mul(10)?.checked_add(digit)?;
            self.at += 1;
        }
        (self.at > start && value <= QUANTIFIER_BOUND_CAP).then_some(value)
    }

    fn atom_escape(&mut self) -> Option<bool> {
        let ch = self.peek()?;
        self.at += 1;
        match ch {
            'b' | 'B' => Some(false),
            '1'..='9' => self.backreference(ch).then_some(true),
            other if CLASS_ESCAPES.contains(other) => (!self.mode.ignore_case).then_some(true),
            '-' => None,
            _ => {
                self.at -= 1;
                self.char_escape().map(|_| true)
            }
        }
    }

    fn backreference(&mut self, first: char) -> bool {
        let mut value = first.to_digit(10).unwrap_or(0);
        while let Some(digit) = self.peek().and_then(|ch| ch.to_digit(10)) {
            value = value.saturating_mul(10).saturating_add(digit);
            self.at += 1;
        }
        !self.mode.ignore_case && value <= self.groups
    }

    fn char_escape(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.at += 1;
        let decoded = match ch {
            'x' => self.hex(2)?,
            'u' => self.hex(4)?,
            other if SYNTAX_CHARS.contains(other) => other,
            other => match CONTROL_ESCAPES.iter().find(|(name, _)| *name == other) {
                Some((_, value)) => *value,
                None if self.is_identity_escape(other) => other,
                None => return None,
            },
        };
        literal_ok(decoded, self.mode).then_some(decoded)
    }

    fn is_identity_escape(&self, ch: char) -> bool {
        !self.mode.unicode && !ch.is_ascii_alphanumeric() && ch != '_'
    }

    fn hex(&mut self, digits: usize) -> Option<char> {
        let mut value = 0u32;
        for _ in 0..digits {
            let digit = self.peek()?.to_digit(16)?;
            value = value * 16 + digit;
            self.at += 1;
        }
        char::from_u32(value)
    }

    fn class(&mut self) -> bool {
        if self.mode.sets {
            return false;
        }
        let negated = self.eat('^');
        if negated && self.mode.ignore_case {
            return false;
        }
        let first = self.at;
        loop {
            match self.peek() {
                None => return false,
                Some(']') if self.at == first => return false,
                Some(']') => {
                    self.at += 1;
                    return true;
                }
                Some(_) => {
                    if !self.class_range(first) {
                        return false;
                    }
                }
            }
        }
    }

    fn class_range(&mut self, first: usize) -> bool {
        let Some(low) = self.class_atom(first) else {
            return false;
        };
        let dash =
            self.peek() == Some('-') && !matches!(self.chars.get(self.at + 1), Some(']') | None);
        if !dash {
            return true;
        }
        self.at += 1;
        let Some(high) = self.class_atom(first) else {
            return false;
        };
        match (low, high) {
            (ClassAtom::Char(low), ClassAtom::Char(high)) => low <= high,
            _ => false,
        }
    }

    fn class_atom(&mut self, first: usize) -> Option<ClassAtom> {
        let ch = self.peek()?;
        let at_edge = self.at == first || self.chars.get(self.at + 1) == Some(&']');
        self.at += 1;
        let atom = match ch {
            '\\' => self.class_escape()?,
            '[' => return None,
            '-' if !at_edge => return None,
            other => ClassAtom::Char(other),
        };
        if let ClassAtom::Char(ch) = atom {
            let letter = ch.is_ascii_alphabetic();
            if !literal_ok(ch, self.mode) || (letter && self.mode.ignore_case) {
                return None;
            }
        }
        Some(atom)
    }

    fn class_escape(&mut self) -> Option<ClassAtom> {
        let ch = self.peek()?;
        if CLASS_ESCAPES.contains(ch) {
            self.at += 1;
            return (!self.mode.ignore_case).then_some(ClassAtom::Escape);
        }
        if ch == '-' {
            self.at += 1;
            return Some(ClassAtom::Char('-'));
        }
        self.char_escape().map(ClassAtom::Char)
    }
}
