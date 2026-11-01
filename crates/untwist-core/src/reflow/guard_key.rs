pub(super) type GuardKey = u64;

const RENAMED: u64 = 0x100;

const MULTIPLIER: u64 = 0x9E37_79B9_7F4A_7C15;

fn is_keyword(word: &[u8]) -> bool {
    matches!(
        word,
        b"arguments"
            | b"async"
            | b"await"
            | b"break"
            | b"case"
            | b"catch"
            | b"class"
            | b"const"
            | b"continue"
            | b"debugger"
            | b"default"
            | b"delete"
            | b"do"
            | b"else"
            | b"eval"
            | b"export"
            | b"extends"
            | b"false"
            | b"finally"
            | b"for"
            | b"function"
            | b"get"
            | b"if"
            | b"import"
            | b"in"
            | b"instanceof"
            | b"let"
            | b"new"
            | b"null"
            | b"of"
            | b"return"
            | b"set"
            | b"static"
            | b"super"
            | b"switch"
            | b"this"
            | b"throw"
            | b"true"
            | b"try"
            | b"typeof"
            | b"var"
            | b"void"
            | b"while"
            | b"yield"
    )
}

const OTHER: u8 = 0;
const START: u8 = 1;
const DIGIT: u8 = 2;
const QUOTE: u8 = 4;

const CLASSES: [u8; 256] = {
    let mut table = [OTHER; 256];
    let mut byte = 0;
    while byte < 256 {
        let b = byte as u8;
        table[byte] = if b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80 {
            START
        } else if b.is_ascii_digit() {
            DIGIT
        } else if b == b'\'' || b == b'"' || b == b'`' {
            QUOTE
        } else {
            OTHER
        };
        byte += 1;
    }
    table
};

fn class(byte: u8) -> u8 {
    CLASSES[usize::from(byte)]
}

fn names_a_property(bytes: &[u8], start: usize) -> bool {
    let dot = |at: usize| bytes.get(at).copied() == Some(b'.');
    start >= 1 && dot(start - 1) && !(start >= 2 && dot(start - 2))
}

fn template_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    while at < bytes.len() && bytes[at] != b'`' {
        if bytes[at] == b'$' && bytes.get(at + 1) == Some(&b'{') {
            at = substitution_end(bytes, at + 2);
            continue;
        }
        at += if bytes[at] == b'\\' { 2 } else { 1 };
    }
    (at + 1).min(bytes.len())
}

fn substitution_end(bytes: &[u8], mut at: usize) -> usize {
    let mut depth = 1usize;
    while at < bytes.len() {
        match bytes[at] {
            b'`' => {
                at = template_end(bytes, at);
                continue;
            }
            b'\'' | b'"' => {
                at = quoted_end(bytes, at);
                continue;
            }
            b'{' => depth += 1,
            b'}' if depth == 1 => return at + 1,
            b'}' => depth -= 1,
            _ => {}
        }
        at += 1;
    }
    at
}

fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    if quote == b'`' {
        return template_end(bytes, start);
    }
    let mut at = start + 1;
    while at < bytes.len() && bytes[at] != quote {
        at += if bytes[at] == b'\\' { 2 } else { 1 };
    }
    (at + 1).min(bytes.len())
}

fn other_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    while at < bytes.len() && class(bytes[at]) == OTHER && bytes[at] != b'/' {
        at += 1;
    }
    at
}

fn word_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    while at < bytes.len() && class(bytes[at]) & (START | DIGIT) != 0 {
        at += 1;
    }
    at
}

fn packed(word: &[u8]) -> u64 {
    if word.len() >= 8 {
        return 0;
    }
    word.iter().fold(word.len() as u64, |packed, byte| {
        (packed << 8) | u64::from(*byte)
    })
}

struct Normalizer<'n> {
    renamed: Vec<(u64, &'n [u8])>,
    single: [u16; 128],
    hash: u64,
}

impl<'n> Normalizer<'n> {
    fn mix(&mut self, value: u64) {
        self.hash = (self.hash ^ value).wrapping_mul(MULTIPLIER).rotate_left(29);
    }

    fn mix_all(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for chunk in chunks {
            self.mix(u64::from_le_bytes(*chunk));
        }
        let mut word = [0; 8];
        word[..rest.len()].copy_from_slice(rest);
        self.mix(u64::from_le_bytes(word) ^ (rest.len() as u64) << 60);
    }

    fn index_of(&mut self, word: &'n [u8]) -> usize {
        if let [byte @ 0..=127] = word {
            return self.single_index(usize::from(*byte), word);
        }
        let key = packed(word);
        let found = self
            .renamed
            .iter()
            .position(|(seen, text)| *seen == key && (key != 0 || *text == word));
        found.unwrap_or_else(|| {
            self.renamed.push((key, word));
            self.renamed.len() - 1
        })
    }

    fn single_index(&mut self, byte: usize, word: &'n [u8]) -> usize {
        match self.single[byte] {
            0 => {
                self.renamed.push((packed(word), word));
                self.single[byte] = u16::try_from(self.renamed.len()).unwrap_or(u16::MAX);
                self.renamed.len() - 1
            }
            index => usize::from(index) - 1,
        }
    }

    fn word(&mut self, word: &'n [u8], property: bool) {
        if property || is_keyword(word) {
            self.mix_all(word);
            return;
        }
        let index = self.index_of(word) as u64;
        self.mix(RENAMED + index);
    }

    fn finish(mut self, bytes: &'n [u8]) -> GuardKey {
        self.tokens(bytes, 0, bytes.len());
        self.hash
    }

    fn tokens(&mut self, bytes: &'n [u8], mut at: usize, stop: usize) {
        let mut previous = Previous::Operator;
        while at < stop {
            let (end, next) = self.token(&bytes[..stop], at, previous);
            previous = next;
            at = end;
        }
    }

    fn token(&mut self, bytes: &'n [u8], at: usize, previous: Previous) -> (usize, Previous) {
        match class(bytes[at]) {
            START => {
                let end = word_end(bytes, at);
                let kept = names_a_property(bytes, at) || names_a_key(bytes, end, previous);
                self.word(&bytes[at..end], kept);
                (end, after_word(&bytes[at..end]))
            }
            QUOTE if bytes[at] == b'`' => (self.template(bytes, at), Previous::Value),
            kind => {
                let (end, next) = match kind {
                    DIGIT => (number_end(bytes, at), Previous::Value),
                    QUOTE => (quoted_end(bytes, at), Previous::Value),
                    _ if bytes[at] == b'/' && previous != Previous::Value => {
                        (regex_end(bytes, at), Previous::Value)
                    }
                    _ => {
                        let end = other_end(bytes, at);
                        (end, after_other(&bytes[at..end], previous))
                    }
                };
                self.mix_all(&bytes[at..end]);
                (end, next)
            }
        }
    }

    fn template(&mut self, bytes: &'n [u8], start: usize) -> usize {
        let mut at = start + 1;
        let mut piece = start;
        while at < bytes.len() && bytes[at] != b'`' {
            if bytes[at] == b'$' && bytes.get(at + 1) == Some(&b'{') {
                self.mix_all(&bytes[piece..at + 2]);
                let end = substitution_end(bytes, at + 2);
                self.tokens(bytes, at + 2, end.saturating_sub(1).max(at + 2));
                piece = end.saturating_sub(1);
                at = end;
                continue;
            }
            at += if bytes[at] == b'\\' { 2 } else { 1 };
        }
        let end = (at + 1).min(bytes.len());
        self.mix_all(&bytes[piece..end]);
        end
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Previous {
    Operator,
    Opener,
    Value,
}

fn after_word(word: &[u8]) -> Previous {
    let precedes_an_expression = matches!(
        word,
        b"return"
            | b"typeof"
            | b"case"
            | b"do"
            | b"else"
            | b"in"
            | b"instanceof"
            | b"new"
            | b"void"
            | b"delete"
            | b"throw"
            | b"yield"
            | b"await"
    );
    match word {
        b"static" | b"get" | b"set" | b"async" => Previous::Opener,
        _ if precedes_an_expression => Previous::Operator,
        _ => Previous::Value,
    }
}

fn after_other(token: &[u8], previous: Previous) -> Previous {
    match token.iter().rev().find(|byte| !byte.is_ascii_whitespace()) {
        None => previous,
        Some(b')' | b']') => Previous::Value,
        Some(b'{' | b',' | b'}' | b';') => Previous::Opener,
        Some(_) => Previous::Operator,
    }
}

fn number_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    while let Some(&byte) = bytes.get(at) {
        let fraction = byte == b'.' && bytes.get(at + 1).is_some_and(u8::is_ascii_digit);
        if class(byte) & (START | DIGIT) == 0 && !fraction {
            break;
        }
        at += 1;
    }
    at
}

fn regex_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    let mut in_class = false;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'\\' => at += 1,
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => return word_end(bytes, at),
            _ => {}
        }
        at += 1;
    }
    bytes.len()
}

fn next_significant(bytes: &[u8], from: usize) -> Option<(usize, u8)> {
    (from..bytes.len())
        .map(|at| (at, bytes[at]))
        .find(|(_, byte)| !byte.is_ascii_whitespace())
}

fn closing_paren(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, &byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' if depth == 1 => return Some(at),
            b')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn names_a_key(bytes: &[u8], end: usize, previous: Previous) -> bool {
    if previous != Previous::Opener {
        return false;
    }
    match next_significant(bytes, end) {
        Some((_, b':')) => true,
        Some((open, b'(')) => closing_paren(bytes, open)
            .and_then(|close| next_significant(bytes, close + 1))
            .is_some_and(|(_, byte)| byte == b'{'),
        _ => false,
    }
}

pub(super) fn guard_key(compact: &str) -> GuardKey {
    Normalizer {
        renamed: Vec::with_capacity(16),
        single: [0; 128],
        hash: 0,
    }
    .finish(compact.as_bytes())
}
