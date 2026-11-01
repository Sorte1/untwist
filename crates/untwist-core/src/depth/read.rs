use super::context::Ctx;
use super::keywords;
use super::lexer::{self, Tok, CLASS, HIGH, ID_START, LINE, SPACE};
use super::{Goal, Mode, Scanner};

pub(super) type Read = Option<(Tok, usize, bool)>;

impl Scanner<'_> {
    pub(super) fn mode_token(&mut self) -> Read {
        match self.mode {
            Mode::Template => self.template_token(),
            Mode::Children => self.jsx_child_token(),
            Mode::Tag => self.jsx_tag_token(),
            Mode::Code => self.code_token_at(false),
        }
    }

    #[inline(always)]
    pub(super) fn read_code(
        &mut self,
        byte: u8,
        class: u8,
        start: usize,
        newline: &mut bool,
    ) -> Option<Tok> {
        if class & ID_START != 0 {
            return Some(self.word(start, start + 1));
        }
        if class & HIGH != 0 {
            return self.high(start, newline);
        }
        let source = self.source;
        match byte {
            b'0'..=b'9' => self.number(start),
            b'.' if source.get(start + 1).is_some_and(u8::is_ascii_digit) => self.number(start),
            b'"' | b'\'' => {
                self.position = lexer::string_end(source, start + 1, byte);
                Some(Tok::Str)
            }
            b'`' => self.punct(Tok::BackQuote, start, 1),
            b'/' => self.slash(start, newline),
            b'<' | b'-' | b'#' => self.rare(byte, start, newline),
            b'\\' => Some(self.word(start, start)),
            _ => {
                let (tok, length) = lexer::punctuator(source, start);
                self.punct(tok, start, length)
            }
        }
    }

    #[inline(always)]
    fn punct(&mut self, tok: Tok, start: usize, length: usize) -> Option<Tok> {
        self.position = start + length;
        Some(tok)
    }

    #[inline(always)]
    fn number(&mut self, start: usize) -> Option<Tok> {
        self.position = lexer::number_end(self.source, start);
        Some(Tok::Num)
    }

    #[inline(always)]
    fn word(&mut self, start: usize, from: usize) -> Tok {
        let (end, rare) = lexer::word_span(self.source, from);
        let end = end.max(start + 1);
        self.position = end;
        if matches!(self.prev, Tok::Dot | Tok::QuestionDot) {
            return Tok::Name;
        }
        let word = &self.source[start..end];
        if rare {
            return self.escaped_word(word);
        }
        keywords::classify_word(word)
    }

    #[cold]
    #[inline(never)]
    pub(super) fn escaped_word(&mut self, word: &[u8]) -> Tok {
        let tok = keywords::classify_escaped(word);
        if tok == Tok::Name || !word.contains(&b'\\') {
            return tok;
        }
        let exact = tok != Tok::NameLet || self.contexts.top() == Ctx::PStat;
        if self.contexts.doubt(exact) {
            tok
        } else {
            Tok::Name
        }
    }

    fn high(&mut self, start: usize, newline: &mut bool) -> Option<Tok> {
        let (code, width) = lexer::decode(self.source, start);
        if lexer::is_unicode_line(code) {
            *newline = true;
        } else if !lexer::is_unicode_space(code) {
            return Some(self.word(start, start + width));
        }
        self.position = start + width;
        None
    }

    fn slash(&mut self, start: usize, newline: &mut bool) -> Option<Tok> {
        let source = self.source;
        match source.get(start + 1) {
            Some(&b'/') => {
                self.position = lexer::line_end(source, start + 2);
                None
            }
            Some(&b'*') => {
                let (end, crossed) = lexer::block_comment_end(source, start + 2);
                *newline |= crossed;
                self.position = end;
                None
            }
            _ if self.expression_follows(*newline) => Some(self.regex(start)),
            _ if self.restart => self.stop(),
            _ => {
                let (tok, length) = lexer::punctuator(source, start);
                self.punct(tok, start, length)
            }
        }
    }

    fn regex(&mut self, start: usize) -> Tok {
        self.position = match lexer::regex_end(self.source, start + 1) {
            Some(end) => {
                self.regexes = self.regexes.saturating_add(1);
                end
            }
            None => lexer::line_end(self.source, start + 1),
        };
        Tok::Regex
    }

    fn rare(&mut self, byte: u8, start: usize, newline: &mut bool) -> Option<Tok> {
        let source = self.source;
        let rest = &source[start..];
        let line_start = *newline || self.prev == Tok::Start;
        let comment = match byte {
            b'<' => rest.starts_with(b"<!--") && self.html_open_is_a_comment(line_start),
            b'-' => line_start && rest.starts_with(b"-->") && self.html_close_is_a_comment(),
            _ => start == 0 && rest.starts_with(b"#!"),
        };
        if comment {
            self.position = lexer::line_end(source, start + 1);
            return None;
        }
        if byte == b'<' && self.angle_in_class_head() {
            return None;
        }
        if byte == b'<' && self.expression_follows(*newline) && source.get(start + 1) != Some(&b'!')
        {
            return self.punct(Tok::JsxTagStart, start, 1);
        }
        if self.restart {
            return self.stop();
        }
        if byte == b'#' {
            return Some(self.private(start));
        }
        let (tok, length) = lexer::punctuator(source, start);
        self.punct(tok, start, length)
    }

    fn stop(&mut self) -> Option<Tok> {
        self.position = self.source.len();
        None
    }

    #[inline(always)]
    pub(super) fn binds(&self, prev: Tok) -> bool {
        prev.declares() && (prev != Tok::NameLet || self.contexts.let_binds())
    }

    pub(super) fn binds_pattern(&self, prev: Tok, tok: Tok) -> bool {
        prev.declares()
            && (prev != Tok::NameLet
                || (self.contexts.let_declares
                    && (!self.contexts.let_bracket_only || tok == Tok::BracketL)))
    }

    fn expression_follows(&mut self, newline: bool) -> bool {
        if self.contexts.expr_allowed {
            if self.prev == Tok::At {
                return self.contexts.doubt(true);
            }
            return true;
        }
        if !newline {
            return false;
        }
        if self.statement_ended() {
            return self.contexts.doubt(true);
        }
        match self.prev {
            Tok::NameBinding => true,
            Tok::NameAwait | Tok::NameYield => false,
            prev => {
                if prev.is_name() && !self.precise {
                    self.restart_precise();
                }
                false
            }
        }
    }

    fn html_open_is_a_comment(&mut self, line_start: bool) -> bool {
        if self.goal == Goal::Script {
            return true;
        }
        self.divergent |= !line_start;
        line_start
    }

    fn html_close_is_a_comment(&mut self) -> bool {
        self.divergent = true;
        self.goal == Goal::Script
    }

    fn private(&mut self, start: usize) -> Tok {
        let next = self.source.get(start + 1).copied().unwrap_or(0);
        if CLASS[usize::from(next)] & (ID_START | HIGH) == 0 {
            self.position = start + 1;
            return Tok::Op;
        }
        self.position = lexer::word_end(self.source, start + 2);
        Tok::PrivateName
    }

    fn template_token(&mut self) -> Read {
        let start = self.position;
        let source = self.source;
        let tok = match source.get(start)? {
            b'`' => self.punct(Tok::BackQuote, start, 1),
            b'$' if source.get(start + 1) == Some(&b'{') => self.punct(Tok::DollarBraceL, start, 2),
            _ => {
                self.position = lexer::template_chunk_end(source, start);
                Some(Tok::Template)
            }
        };
        tok.map(|tok| (tok, start, false))
    }

    #[inline(always)]
    pub(super) fn code_token_at(&mut self, newline: bool) -> Read {
        let mut newline = newline;
        loop {
            let start = self.position;
            let byte = *self.source.get(start)?;
            let class = CLASS[usize::from(byte)];
            if class & (SPACE | LINE) != 0 {
                self.position = lexer::spaces_end(self.source, start, &mut newline);
                continue;
            }
            if let Some(tok) = self.read_code(byte, class, start, &mut newline) {
                return Some((tok, start, newline));
            }
        }
    }
}
