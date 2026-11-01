use super::escape;
use super::lexer::{self, Tok, CLASS, HIGH, ID_START, LINE, SPACE};
use super::{Ctx, DepthRefusal, Goal, Scanner, Watch, PENDING_WATCH};

const MODIFIERS: &[&[u8]] = &[
    b"static",
    b"accessor",
    b"declare",
    b"readonly",
    b"public",
    b"private",
    b"protected",
    b"override",
    b"abstract",
    b"async",
    b"in",
    b"out",
    b"const",
    b"export",
    b"default",
];

fn is_member_prefix(word: &[u8]) -> bool {
    matches!(word, b"get" | b"set") || MODIFIERS.contains(&word)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Peeked {
    Word(usize, usize),
    Punct(u8, usize),
    Ellipsis(usize),
    Unknown,
}

#[inline(always)]
fn starts_a_member(prev: Tok) -> bool {
    prev.is_name()
        || matches!(
            prev,
            Tok::BraceL
                | Tok::BraceR
                | Tok::Semi
                | Tok::Star
                | Tok::Str
                | Tok::Num
                | Tok::PrivateName
                | Tok::BracketR
        )
}

#[inline(always)]
pub(super) fn maybe_typescript(source: &[u8], (start, end): (usize, usize), escaped: bool) -> bool {
    escaped
        || (matches!(end - start, 2 | 9 | 10)
            && matches!(
                source.get(start..end),
                Some(b"as" | b"satisfies" | b"implements")
            ))
}

struct Word {
    bytes: [u8; 12],
    length: usize,
}

impl Word {
    fn decode(raw: &[u8]) -> Word {
        let mut bytes = [0u8; 12];
        let length = if raw.contains(&b'\\') {
            escape::unescaped(raw, &mut bytes).unwrap_or(0)
        } else if raw.len() > bytes.len() {
            0
        } else {
            bytes[..raw.len()].copy_from_slice(raw);
            raw.len()
        };
        Word { bytes, length }
    }

    fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.length).unwrap_or_default()
    }
}

fn html_comment(rest: &[u8], script: bool, line_start: bool) -> bool {
    (rest.starts_with(b"<!--") && (script || line_start))
        || (rest.starts_with(b"-->") && script && line_start)
}

fn skip_trivia(source: &[u8], mut position: usize, script: bool) -> Option<(usize, bool)> {
    let mut newline = false;
    loop {
        let byte = *source.get(position)?;
        let class = CLASS[usize::from(byte)];
        if class & (SPACE | LINE) != 0 {
            newline |= class & LINE != 0;
            position += 1;
            continue;
        }
        if class & HIGH != 0 {
            let (code, width) = lexer::decode(source, position);
            if !lexer::is_unicode_space(code) && !lexer::is_unicode_line(code) {
                return Some((position, newline));
            }
            newline |= lexer::is_unicode_line(code);
            position += width;
            continue;
        }
        let rest = &source[position..];
        if rest.starts_with(b"//") || html_comment(rest, script, newline) {
            position = lexer::line_end(source, position + 2);
        } else if rest.starts_with(b"/*") {
            let (end, crossed) = lexer::block_comment_end(source, position + 2);
            newline |= crossed;
            position = end;
        } else {
            return Some((position, newline));
        }
    }
}

fn word_end(source: &[u8], start: usize, byte: u8) -> usize {
    let from = match byte {
        b'\\' => start,
        _ if byte >= 0x80 => start + lexer::decode(source, start).1,
        _ => start + 1,
    };
    lexer::word_span(source, from).0.max(start + 1)
}

fn peek(source: &[u8], position: usize, script: bool) -> Peeked {
    let Some((start, _)) = skip_trivia(source, position, script) else {
        return Peeked::Unknown;
    };
    let byte = source[start];
    if CLASS[usize::from(byte)] & ID_START != 0 || byte == b'\\' || byte >= 0x80 {
        return Peeked::Word(start, word_end(source, start, byte));
    }
    if source.get(start..start + 3) == Some(b"...") {
        return Peeked::Ellipsis(start + 3);
    }
    Peeked::Punct(byte, start + 1)
}

impl Scanner<'_> {
    fn word_at(&self, (from, to): (usize, usize)) -> Word {
        Word::decode(self.source.get(from..to).unwrap_or_default())
    }

    fn refuse_typescript(&self) -> Result<(), DepthRefusal> {
        if self.contexts.flip {
            return Ok(());
        }
        Err(DepthRefusal::Typescript)
    }

    fn refuses_typescript(&self) -> bool {
        self.refuse_typescript().is_err()
    }

    fn defer_typescript(&mut self) -> bool {
        let Err(refusal) = self.refuse_typescript() else {
            return false;
        };
        self.refusal = Some(refusal);
        self.position = self.source.len();
        true
    }

    pub(super) fn typescript_word(
        &mut self,
        start: usize,
        newline: bool,
    ) -> Result<(), DepthRefusal> {
        if !self.refuses_typescript() {
            return Ok(());
        }
        let decoded = self.word_at((start, self.position));
        let word = decoded.as_bytes();
        if word == b"implements" && self.contexts.in_class_head() && self.prev != Tok::KwClass {
            return self.refuse_typescript();
        }
        if newline || !matches!(word, b"as" | b"satisfies") || self.names_without_operator() {
            return Ok(());
        }
        if self.member_name_position() {
            return Ok(());
        }
        if self.prev.is_name() && self.member_or_parameter() {
            return Ok(());
        }
        self.refuse_typescript()
    }

    fn names_without_operator(&self) -> bool {
        let clause = self.pending & PENDING_WATCH != 0
            && matches!(
                self.watch,
                Watch::Module
                    | Watch::From
                    | Watch::ImportName
                    | Watch::Label
                    | Watch::Alias
                    | Watch::Closed
            );
        clause
            || matches!(
                self.prev,
                Tok::Dot
                    | Tok::QuestionDot
                    | Tok::KwFunction
                    | Tok::KwClass
                    | Tok::NameUsing
                    | Tok::KwBreak
                    | Tok::KwContinue
            )
            || self.binds(self.prev)
            || (self.prev == Tok::NameLet && self.contexts.top() == Ctx::PStat)
    }

    fn member_or_parameter(&mut self) -> bool {
        if !self.precise {
            self.restart_precise();
            return true;
        }
        if matches!(self.prev, Tok::NameAwait | Tok::NameYield) {
            return false;
        }
        let decoded = self.word_at(self.tracker.last_span);
        let prev = decoded.as_bytes();
        if prev == b"async" && self.arrow_follows() {
            return true;
        }
        is_member_prefix(prev) && self.tracker.at_member_name(self.contexts.len())
    }

    fn member_name_position(&mut self) -> bool {
        let depth = self.contexts.len();
        if !self.contexts.in_class_body() {
            return false;
        }
        if self.precise {
            return self.tracker.after_decorator(depth)
                || self.tracker.at_member_start(depth, false, false);
        }
        if self.prev.is_name() && !self.contexts.decorated_at(depth) {
            return false;
        }
        self.restart_precise();
        true
    }

    fn arrow_follows(&self) -> bool {
        let script = self.goal == Goal::Script;
        skip_trivia(self.source, self.position, script).is_some_and(|(at, newline)| {
            !newline && self.source.get(at..at + 2) == Some(b"=>".as_slice())
        })
    }

    #[cold]
    #[inline(never)]
    pub(super) fn note_class_head(&mut self) {
        let depth = self.contexts.len();
        while self
            .class_heads
            .last()
            .is_some_and(|&(open, _)| open >= depth)
        {
            self.class_heads.pop();
        }
        self.class_heads.push((depth, self.frames.open_count()));
    }

    fn at_class_head_level(&mut self) -> bool {
        let depth = self.contexts.len();
        while self
            .class_heads
            .last()
            .is_some_and(|&(open, _)| open > depth)
        {
            self.class_heads.pop();
        }
        self.class_heads.last() == Some(&(depth, self.frames.open_count()))
    }

    pub(super) fn angle_in_class_head(&mut self) -> bool {
        self.refuses_typescript()
            && self.contexts.in_class_head()
            && self.at_class_head_level()
            && self.defer_typescript()
    }

    pub(super) fn import_equals(&mut self) {
        self.defer_typescript();
    }

    pub(super) fn default_interface(&mut self, start: usize, newline: bool) {
        if !newline && self.word_at((start, self.position)).as_bytes() == b"interface" {
            self.defer_typescript();
        }
    }

    #[inline(always)]
    pub(super) fn index_signature(
        &self,
        prev: Tok,
        position: usize,
        newline: bool,
    ) -> Result<bool, DepthRefusal> {
        if !starts_a_member(prev) || !self.contexts.in_class_body() {
            return Ok(false);
        }
        self.index_signature_follows(position, newline)
    }

    #[cold]
    #[inline(never)]
    fn index_signature_follows(
        &self,
        position: usize,
        newline: bool,
    ) -> Result<bool, DepthRefusal> {
        let script = self.goal == Goal::Script;
        if !self.refuses_typescript()
            || !self.at_member_level()
            || !index_pattern(self.source, position, script)
        {
            return Ok(false);
        }
        if !self.precise {
            return Ok(true);
        }
        let decoded = self.word_at(self.tracker.last_span);
        let modifier = MODIFIERS.contains(&decoded.as_bytes());
        if self
            .tracker
            .at_member_start(self.contexts.len(), newline, modifier)
        {
            self.refuse_typescript()?;
        }
        Ok(false)
    }

    fn at_member_level(&self) -> bool {
        let depth = self.contexts.len();
        let frames = self.frames.open_count();
        self.class_heads
            .iter()
            .rev()
            .take_while(|&&(open, _)| open + 1 >= depth)
            .any(|&(open, at)| open + 1 == depth && at + 1 == frames)
    }
}

fn index_pattern(source: &[u8], position: usize, script: bool) -> bool {
    match peek(source, position, script) {
        Peeked::Ellipsis(_) => true,
        Peeked::Word(start, end) => match peek(source, end, script) {
            Peeked::Punct(b':' | b',', _) => true,
            Peeked::Punct(b'?', after) => {
                matches!(
                    peek(source, after, script),
                    Peeked::Punct(b':' | b',' | b']', _)
                )
            }
            Peeked::Word(..) => MODIFIERS.contains(&&source[start..end]),
            _ => false,
        },
        _ => false,
    }
}
