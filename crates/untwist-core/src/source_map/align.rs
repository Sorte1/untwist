use oxc_sourcemap::{OwnedSourceMap, Token};

use super::lines::{char_len, line_break, utf16_len};

const PLACE_MARKER: &str = "\u{E000}";
const LOOKAHEAD: usize = 4;

fn comment_end(text: &[u8], at: usize) -> Option<usize> {
    match text.get(at..at + 2)? {
        b"//" => Some(
            text[at..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(text.len(), |offset| at + offset),
        ),
        b"/*" => Some(
            text[at + 2..]
                .windows(2)
                .position(|pair| pair == b"*/")
                .map_or(text.len(), |offset| at + 2 + offset + 2),
        ),
        _ => None,
    }
}

struct Cursor<'t> {
    text: &'t [u8],
    at: usize,
}

impl Cursor<'_> {
    fn skip_space(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn current(&self) -> Option<&[u8]> {
        let byte = *self.text.get(self.at)?;
        self.text.get(self.at..self.at + char_len(byte))
    }

    fn step(&mut self) {
        let len = self.text.get(self.at).map_or(1, |byte| char_len(*byte));
        self.at = (self.at + len).min(self.text.len());
    }

    fn ahead(&self, from: usize) -> &[u8] {
        let start = from.min(self.text.len());
        &self.text[start..(start + LOOKAHEAD).min(self.text.len())]
    }
}

struct Aligner<'t> {
    from: Cursor<'t>,
    to: Cursor<'t>,
}

impl Aligner<'_> {
    fn skip_noise(&mut self) -> bool {
        if let Some(end) = comment_end(self.to.text, self.to.at) {
            self.to.at = end;
            return true;
        }
        if self.from.text[self.from.at..].starts_with(PLACE_MARKER.as_bytes()) {
            self.from.at += PLACE_MARKER.len();
            return true;
        }
        false
    }

    fn resync(&mut self) {
        let next_from = self.from.at + self.from.current().map_or(1, <[u8]>::len);
        let next_to = self.to.at + self.to.current().map_or(1, <[u8]>::len);
        let here_from = self.from.ahead(self.from.at);
        let here_to = self.to.ahead(self.to.at);
        if self.to.ahead(next_to) == here_from {
            self.to.step();
        } else if self.from.ahead(next_from) == here_to {
            self.from.step();
        } else {
            self.from.step();
            self.to.step();
        }
    }

    fn advance(&mut self) {
        self.from.skip_space();
        self.to.skip_space();
        match (self.from.current(), self.to.current()) {
            (Some(left), Some(right)) if left == right => {
                self.from.step();
                self.to.step();
            }
            (Some(_), Some(_)) => {
                if !self.skip_noise() {
                    self.resync();
                }
            }
            (Some(_), None) => self.from.at = self.from.text.len(),
            _ => {}
        }
    }

    fn settle_on(&mut self) {
        loop {
            self.to.skip_space();
            let matched = self.from.current() == self.to.current();
            if matched || comment_end(self.to.text, self.to.at).is_none() {
                return;
            }
            self.skip_noise();
        }
    }

    fn position_of(&mut self, target: usize) -> usize {
        loop {
            self.from.skip_space();
            if self.from.at >= target || self.from.at >= self.from.text.len() {
                self.settle_on();
                return self.to.at.min(self.to.text.len());
            }
            self.advance();
        }
    }
}

struct LineWalk<'t> {
    text: &'t [u8],
    line: u32,
    line_start: usize,
    at: usize,
    column: u32,
}

impl<'t> LineWalk<'t> {
    fn new(text: &'t [u8]) -> Self {
        Self {
            text,
            line: 0,
            line_start: 0,
            at: 0,
            column: 0,
        }
    }

    fn step(&mut self) {
        let broke = line_break(self.text, self.at);
        if broke > 0 {
            self.line += 1;
            self.at += broke;
            self.line_start = self.at;
            self.column = 0;
            return;
        }
        let byte = self.text.get(self.at).copied().unwrap_or_default();
        self.column += utf16_len(byte);
        self.at += 1;
    }

    fn rewind(&mut self, line_start: usize) {
        if line_start < self.line_start {
            *self = Self::new(self.text);
        }
        self.at = self.line_start;
        self.column = 0;
    }

    fn offset_of(&mut self, line: u32, column: u32) -> Option<usize> {
        if line < self.line {
            self.rewind(0);
        }
        while self.line < line {
            if self.at >= self.text.len() {
                return None;
            }
            self.step();
        }
        if self.column > column {
            self.rewind(self.line_start);
        }
        while self.column < column {
            if self.at >= self.text.len() || line_break(self.text, self.at) > 0 {
                return None;
            }
            self.step();
        }
        while self
            .text
            .get(self.at)
            .is_some_and(|byte| utf16_len(*byte) == 0)
        {
            self.at += 1;
        }
        Some(self.at)
    }

    fn position_at(&mut self, offset: usize) -> (u32, u32) {
        if offset < self.at {
            self.rewind(offset.min(self.line_start));
        }
        while self.at < offset && self.at < self.text.len() {
            self.step();
        }
        (self.line, self.column)
    }
}

pub fn retarget(map: OwnedSourceMap, printed: &str, finished: &str) -> OwnedSourceMap {
    let mut parts = map.into_parts();
    let mut aligner = Aligner {
        from: Cursor {
            text: printed.as_bytes(),
            at: 0,
        },
        to: Cursor {
            text: finished.as_bytes(),
            at: 0,
        },
    };
    let mut lines_from = LineWalk::new(printed.as_bytes());
    let mut lines_to = LineWalk::new(finished.as_bytes());
    let mut tokens: Vec<Token> = Vec::with_capacity(parts.tokens.len());
    let mut last = None;
    for token in parts.tokens.iter() {
        let Some(offset) = lines_from.offset_of(token.get_dst_line(), token.get_dst_col()) else {
            continue;
        };
        let target = aligner.position_of(offset);
        let (line, column) = lines_to.position_at(target);
        if last == Some((line, column)) {
            tokens.pop();
        }
        last = Some((line, column));
        tokens.push(Token::new(
            line,
            column,
            token.get_src_line(),
            token.get_src_col(),
            token.get_source_id(),
            token.get_name_id(),
        ));
    }
    parts.tokens = tokens.into_boxed_slice();
    parts.token_chunks = None;
    OwnedSourceMap::from_parts(parts)
}
