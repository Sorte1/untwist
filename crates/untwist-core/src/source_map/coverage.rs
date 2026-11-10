use oxc_sourcemap::OwnedSourceMap;

use super::lines::line_break;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    pub tokens: usize,
    pub on_line: usize,
    pub exact: usize,
}

impl Coverage {
    pub fn share(&self) -> f64 {
        ratio(self.on_line, self.tokens)
    }

    pub fn exact_share(&self) -> f64 {
        ratio(self.exact, self.tokens)
    }
}

fn ratio(part: usize, whole: usize) -> f64 {
    match whole {
        0 => 1.0,
        _ => part as f64 / whole as f64,
    }
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte >= 0x80
}

fn token_end(bytes: &[u8], at: usize) -> usize {
    let byte = bytes[at];
    if is_word(byte) {
        return at
            + bytes[at..]
                .iter()
                .take_while(|byte| is_word(**byte))
                .count();
    }
    if byte == b'"' || byte == b'\'' || byte == b'`' {
        let mut index = at + 1;
        while index < bytes.len() && bytes[index] != byte && bytes[index] != b'\n' {
            index += if bytes[index] == b'\\' { 2 } else { 1 };
        }
        return (index + 1).min(bytes.len());
    }
    at + 1
}

fn block_comment_end(bytes: &[u8], at: usize) -> usize {
    bytes[at + 2..]
        .windows(2)
        .position(|pair| pair == b"*/")
        .map_or(bytes.len(), |found| at + 2 + found + 2)
}

fn columns(text: &str, from: usize, to: usize) -> u32 {
    let units = text
        .get(from..to)
        .map_or(0, |part| part.encode_utf16().count());
    u32::try_from(units).unwrap_or(u32::MAX)
}

struct Scan<'t> {
    text: &'t str,
    line: u32,
    column: u32,
    counted: usize,
    at: usize,
}

impl Scan<'_> {
    fn bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    fn broke(&mut self) -> bool {
        let broke = line_break(self.bytes(), self.at);
        if broke == 0 {
            return false;
        }
        self.line += 1;
        self.at += broke;
        self.column = 0;
        self.counted = self.at;
        true
    }

    fn skip_to(&mut self, end: usize) {
        while self.at < end {
            if !self.broke() {
                self.at += 1;
            }
        }
    }

    fn skip_comment(&mut self) -> bool {
        let rest = &self.bytes()[self.at..];
        if rest.starts_with(b"//") {
            while self.at < self.text.len() && line_break(self.bytes(), self.at) == 0 {
                self.at += 1;
            }
            return true;
        }
        if rest.starts_with(b"/*") {
            let end = block_comment_end(self.bytes(), self.at);
            self.skip_to(end);
            return true;
        }
        false
    }

    fn token(&mut self) -> (u32, u32) {
        self.column = self
            .column
            .saturating_add(columns(self.text, self.counted, self.at));
        self.counted = self.at;
        self.at = token_end(self.bytes(), self.at);
        while !self.text.is_char_boundary(self.at) {
            self.at += 1;
        }
        (self.line, self.column)
    }
}

fn token_starts(text: &str) -> Vec<(u32, u32)> {
    let mut scan = Scan {
        text,
        line: 0,
        column: 0,
        counted: 0,
        at: 0,
    };
    let mut starts = Vec::new();
    while scan.at < text.len() {
        if scan.broke() || scan.skip_comment() {
            continue;
        }
        if text.as_bytes()[scan.at].is_ascii_whitespace() {
            scan.at += 1;
            continue;
        }
        starts.push(scan.token());
    }
    starts
}

pub fn coverage(map: &OwnedSourceMap, text: &str) -> Coverage {
    let mut mapped: Vec<(u32, u32)> = map
        .get_tokens()
        .filter(|token| token.get_source_id().is_some())
        .map(|token| (token.get_dst_line(), token.get_dst_col()))
        .collect();
    mapped.sort_unstable();
    let mut result = Coverage::default();
    for (line, column) in token_starts(text) {
        result.tokens += 1;
        let after = mapped.partition_point(|probe| *probe <= (line, column));
        let Some(&(found_line, found_column)) = after.checked_sub(1).and_then(|i| mapped.get(i))
        else {
            continue;
        };
        if found_line == line {
            result.on_line += 1;
            result.exact += usize::from(found_column == column);
        }
    }
    result
}
