use super::context::Head;
use super::keywords;
use super::levels::Level;
use super::lexer::{
    self, Tok, BEFORE_EXPR, CLASS, ID_START, LINE, LINE_TOKEN, RUN, SPACE, SPECIAL,
};
use super::typescript::maybe_typescript;
use super::words::{awaits_an_operand, may_end_a_statement};
use super::{
    mode_of, DepthRefusal, Mode, Scanner, Watch, PENDING_ARROW, PENDING_BODY, PENDING_OPERAND,
    PENDING_SEMI, PENDING_SPECULATIVE,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Other,
    Space,
    Blank,
    Word,
    Digit,
    Quote,
    ParenL,
    ParenR,
    BracketL,
    BracketR,
    BraceL,
    BraceR,
    Comma,
    Semi,
    Dot,
    Punct,
    Minus,
}

static ACTION: [Action; 256] = build_action();

const fn build_action() -> [Action; 256] {
    let mut table = [Action::Other; 256];
    let mut index = 0;
    while index < 256 {
        let class = CLASS[index];
        table[index] = if index == b' ' as usize {
            Action::Space
        } else if class & (SPACE | LINE) != 0 {
            Action::Blank
        } else if class & ID_START != 0 {
            Action::Word
        } else {
            punctuation_action(index as u8)
        };
        index += 1;
    }
    table
}

const fn punctuation_action(byte: u8) -> Action {
    match byte {
        b'0'..=b'9' => Action::Digit,
        b'"' | b'\'' => Action::Quote,
        b'(' => Action::ParenL,
        b')' => Action::ParenR,
        b'[' => Action::BracketL,
        b']' => Action::BracketR,
        b'{' => Action::BraceL,
        b'}' => Action::BraceR,
        b',' => Action::Comma,
        b';' => Action::Semi,
        b'.' => Action::Dot,
        b'-' => Action::Minus,
        b'=' | b'+' | b'*' | b'&' | b'|' | b'!' | b'%' | b'^' | b'?' | b':' | b'>' | b'~'
        | b'@' => Action::Punct,
        _ => Action::Other,
    }
}

#[derive(Clone, Copy)]
struct Hot {
    position: usize,
    prev: Tok,
    pending: u8,
    expr: bool,
    newline: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Refused(DepthRefusal),
    Leave,
    End,
}

impl From<DepthRefusal> for Stop {
    fn from(refusal: DepthRefusal) -> Stop {
        Stop::Refused(refusal)
    }
}

type Step = Result<(), Stop>;

impl Scanner<'_> {
    #[inline(always)]
    fn hot(&self) -> Hot {
        Hot {
            position: self.position,
            prev: self.prev,
            pending: self.pending,
            expr: self.contexts.expr_allowed,
            newline: false,
        }
    }

    #[inline(always)]
    fn store(&mut self, hot: &Hot) {
        self.position = hot.position;
        self.prev = hot.prev;
        self.pending = hot.pending;
        self.contexts.expr_allowed = hot.expr;
    }

    #[inline(always)]
    fn reload(&self, hot: &mut Hot) {
        let newline = hot.newline;
        *hot = self.hot();
        hot.newline = newline;
    }

    #[inline(always)]
    fn flow(&self) -> Step {
        if self.mode == Mode::Code {
            Ok(())
        } else {
            Err(Stop::Leave)
        }
    }

    pub(super) fn code_loop(&mut self) -> Result<bool, DepthRefusal> {
        let mut hot = self.hot();
        let source = self.source;
        let stop = loop {
            if let Err(stop) = self.step(source, &mut hot) {
                break stop;
            }
        };
        self.store(&hot);
        match stop {
            Stop::Refused(refusal) => Err(refusal),
            Stop::Leave => Ok(false),
            Stop::End => Ok(true),
        }
    }

    #[inline(always)]
    fn step(&mut self, source: &[u8], hot: &mut Hot) -> Step {
        let start = hot.position;
        let Some(&byte) = source.get(start) else {
            return Err(Stop::End);
        };
        match ACTION[usize::from(byte)] {
            Action::Space => {
                hot.position = start + 1;
                return Ok(());
            }
            Action::Blank => {
                hot.position = lexer::spaces_end(source, start, &mut hot.newline);
                return Ok(());
            }
            Action::Word => self.word_hot(source, hot, start)?,
            Action::Digit => {
                hot.position = lexer::number_end(source, start);
                self.literal(hot, Tok::Num, start)?;
            }
            Action::Quote => {
                hot.position = lexer::string_end(source, start + 1, byte);
                self.literal(hot, Tok::Str, start)?;
            }
            Action::Minus if hot.newline || hot.prev == Tok::Start => {
                self.other(hot, byte, start)?;
            }
            Action::Punct | Action::Minus => self.punct_read(source, hot, start)?,
            Action::Dot => self.dot_hot(source, hot, start)?,
            Action::Other => self.other(hot, byte, start)?,
            Action::ParenL => self.structural(hot, Tok::ParenL, start, Self::paren_hot)?,
            Action::ParenR => self.structural(hot, Tok::ParenR, start, Self::close_context_hot)?,
            Action::BracketL => {
                self.structural(hot, Tok::BracketL, start, Self::bracket_open_hot)?;
            }
            Action::BracketR => {
                self.structural(hot, Tok::BracketR, start, Self::bracket_close_hot)?;
            }
            Action::BraceL => self.structural(hot, Tok::BraceL, start, Self::brace_hot)?,
            Action::BraceR => self.structural(hot, Tok::BraceR, start, Self::close_context_hot)?,
            Action::Comma => self.structural(hot, Tok::Comma, start, Self::comma_hot)?,
            Action::Semi => self.structural(hot, Tok::Semi, start, Self::semi_hot)?,
        }
        if source.get(hot.position) == Some(&b' ') {
            hot.position += 1;
        }
        Ok(())
    }

    #[inline(always)]
    fn word_hot(&mut self, source: &[u8], hot: &mut Hot, start: usize) -> Step {
        let (end, rare) = lexer::word_span(source, start + 1);
        hot.position = end;
        if matches!(hot.prev, Tok::Dot | Tok::QuestionDot) {
            return self.literal(hot, Tok::Name, start);
        }
        let tok = if rare {
            self.escaped_word(source.get(start..end).unwrap_or_default())
        } else {
            keywords::classify(source, start, end)
        };
        match tok {
            Tok::Name if !hot.expr && maybe_typescript(source, (start, end), rare) => {
                self.slow(hot, Tok::Name, start)
            }
            Tok::Name => self.literal(hot, Tok::Name, start),
            tok @ (Tok::KwFunction | Tok::KwClass) => self.function_hot(hot, tok, start),
            tok => self.punct_hot(hot, tok, start),
        }
    }

    #[inline(always)]
    fn punct_read(&mut self, source: &[u8], hot: &mut Hot, start: usize) -> Step {
        let (tok, length) = lexer::punctuator(source, start);
        hot.position = start + length;
        self.punct_hot(hot, tok, start)
    }

    #[inline(always)]
    fn dot_hot(&mut self, source: &[u8], hot: &mut Hot, start: usize) -> Step {
        if matches!(source.get(start + 1), Some(b'.' | b'0'..=b'9')) {
            return self.other(hot, b'.', start);
        }
        hot.position = start + 1;
        self.punct_hot(hot, Tok::Dot, start)
    }

    #[inline(always)]
    fn literal(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Step {
        if !self.settle(hot, tok, start)? {
            return self.slow(hot, tok, start);
        }
        hot.expr = false;
        hot.prev = tok;
        hot.newline = false;
        Ok(())
    }

    #[inline(always)]
    fn punct_hot(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Step {
        let flags = tok.flags();
        if flags & SPECIAL != 0 {
            return self.special_hot(hot, tok, start);
        }
        if !self.settle(hot, tok, start)? {
            return self.slow(hot, tok, start);
        }
        if flags & RUN != 0 {
            self.frames.count_run()?;
        }
        if flags & LINE_TOKEN != 0 {
            self.weigh()?;
        }
        hot.expr = flags & BEFORE_EXPR != 0;
        hot.prev = tok;
        hot.newline = false;
        Ok(())
    }

    #[inline(always)]
    fn special_hot(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Step {
        if matches!(
            tok,
            Tok::KwCase | Tok::KwDefault | Tok::NameAsync | Tok::NameLet | Tok::NameAwait
        ) {
            return self.contextual_hot(hot, tok, start);
        }
        if !matches!(tok, Tok::Colon | Tok::Question | Tok::Arrow)
            || !self.settle(hot, tok, start)?
        {
            return self.slow(hot, tok, start);
        }
        self.frames.count_run()?;
        match tok {
            Tok::Question => self.levels.push(&mut self.frames, Level::Question),
            Tok::Colon => {
                self.contexts.conditional_colon = self.frames.top.questions > 0;
                self.levels.close_to_question(&mut self.frames)?;
            }
            _ => hot.pending |= PENDING_ARROW,
        }
        self.weigh()?;
        hot.expr = true;
        hot.prev = tok;
        hot.newline = false;
        if tok != Tok::Colon {
            return Ok(());
        }
        self.contexts.colon();
        self.mode = mode_of(self.contexts.top());
        self.flow()
    }

    #[inline(never)]
    fn contextual_hot(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Step {
        if !self.settle(hot, tok, start)? {
            return self.slow(hot, tok, start);
        }
        let flags = tok.flags();
        if flags & RUN != 0 {
            self.frames.count_run()?;
        }
        if flags & LINE_TOKEN != 0 {
            self.weigh()?;
        }
        self.contexts.expr_allowed = hot.expr;
        self.contexts.update(tok, hot.prev, hot.newline)?;
        hot.expr = self.contexts.expr_allowed;
        if tok == Tok::NameAwait {
            if !awaits_an_operand(hot.prev) {
                self.restart_precise();
                return Err(Stop::End);
            }
            hot.pending |= PENDING_OPERAND;
        }
        hot.prev = tok;
        hot.newline = false;
        Ok(())
    }

    #[inline(always)]
    fn function_hot(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Step {
        if !self.settle(hot, tok, start)? {
            return self.slow(hot, tok, start);
        }
        self.frames.count_run()?;
        self.weigh()?;
        self.contexts
            .open_function(hot.prev, hot.newline, tok == Tok::KwClass)?;
        if tok == Tok::KwClass {
            self.note_class_head();
        }
        hot.expr = false;
        hot.prev = tok;
        hot.newline = false;
        Ok(())
    }

    #[inline(always)]
    fn settle(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Result<bool, DepthRefusal> {
        if may_end_a_statement(hot.prev, hot.newline) {
            self.end_statement_before(hot.pending, tok, start, hot.position);
        }
        if hot.pending == 0 {
            return Ok(true);
        }
        if hot.pending != PENDING_SEMI || tok == Tok::KwElse {
            if hot.pending & !(PENDING_SPECULATIVE | PENDING_BODY | PENDING_ARROW) == 0 {
                return self.settle_pending(hot, tok, start);
            }
            return Ok(false);
        }
        hot.pending = 0;
        self.frames.reset_run();
        if self.levels.len() > self.frames.top.level_base as usize {
            self.levels.close_all(&mut self.frames)?;
        }
        self.weigh()?;
        Ok(true)
    }

    #[inline(never)]
    fn settle_pending(
        &mut self,
        hot: &mut Hot,
        tok: Tok,
        start: usize,
    ) -> Result<bool, DepthRefusal> {
        let word = self.source.get(start..hot.position).unwrap_or_default();
        if hot.pending & PENDING_SPECULATIVE != 0
            && !self.speculative.resolve(&mut self.frames, tok, word)?
        {
            hot.pending &= !PENDING_SPECULATIVE;
        }
        if hot.pending & (PENDING_BODY | PENDING_ARROW) != 0 {
            self.prev = hot.prev;
            self.pending = hot.pending;
            self.resolve_body(tok)?;
            hot.pending = self.pending;
        }
        Ok(true)
    }

    #[inline(always)]
    fn slow(&mut self, hot: &mut Hot, tok: Tok, start: usize) -> Step {
        self.store(hot);
        self.slow_token(tok, start, hot.newline)?;
        self.reload(hot);
        hot.newline = false;
        self.flow()
    }

    #[inline(never)]
    fn slow_token(&mut self, tok: Tok, start: usize, newline: bool) -> Result<(), DepthRefusal> {
        self.token(tok, start, newline)
    }

    #[inline(always)]
    fn other(&mut self, hot: &mut Hot, byte: u8, start: usize) -> Step {
        self.store(hot);
        let mut newline = hot.newline;
        let advanced = self.generic(byte, start, &mut newline)?;
        self.reload(hot);
        hot.newline = newline && !advanced;
        self.flow()
    }

    #[inline(always)]
    fn structural(
        &mut self,
        hot: &mut Hot,
        tok: Tok,
        start: usize,
        apply: fn(&mut Self, &mut Hot) -> Step,
    ) -> Step {
        hot.position = start + 1;
        if !self.settle(hot, tok, start)? {
            return self.slow(hot, tok, start);
        }
        let result = apply(self, hot);
        hot.prev = tok;
        hot.newline = false;
        result
    }

    #[inline(always)]
    fn bracket_open_hot(&mut self, hot: &mut Hot) -> Step {
        if self.index_signature(hot.prev, hot.position, hot.newline)? {
            self.restart_precise();
            return Err(Stop::End);
        }
        hot.expr = true;
        let head = self.head_after(hot.prev, Tok::BracketL);
        Ok(self.open(head)?)
    }

    #[inline(always)]
    fn head_after(&self, prev: Tok, tok: Tok) -> Head {
        if prev.declares() {
            self.declaration_head(prev, tok)
        } else {
            Head::None
        }
    }

    #[cold]
    #[inline(never)]
    fn declaration_head(&self, prev: Tok, tok: Tok) -> Head {
        if self.binds_pattern(prev, tok) {
            Head::Pattern
        } else {
            Head::None
        }
    }

    #[inline(always)]
    fn semi_hot(&mut self, hot: &mut Hot) -> Step {
        hot.expr = true;
        hot.pending |= PENDING_SEMI;
        Ok(())
    }

    #[inline(always)]
    fn bracket_close_hot(&mut self, hot: &mut Hot) -> Step {
        hot.expr = false;
        Ok(self.close_hot(hot)?)
    }

    #[inline(always)]
    fn open_hot(&mut self) -> Result<(), DepthRefusal> {
        self.open(Head::None)
    }

    #[inline(always)]
    fn paren_hot(&mut self, hot: &mut Hot) -> Step {
        let candidate = hot.expr || (hot.prev == Tok::NameAsync && !hot.newline);
        self.open_hot()?;
        if candidate {
            self.speculative.candidate(self.frames.top_index());
            hot.pending |= PENDING_SPECULATIVE;
        }
        self.contexts.open_paren(hot.prev)?;
        hot.expr = true;
        Ok(())
    }

    #[inline(always)]
    fn brace_hot(&mut self, hot: &mut Hot) -> Step {
        let head = self.head_after(hot.prev, Tok::BraceL);
        self.open(head)?;
        self.weigh()?;
        self.contexts.expr_allowed = hot.expr;
        self.contexts.open_brace(hot.prev, hot.newline)?;
        hot.expr = true;
        Ok(())
    }

    #[inline(always)]
    fn comma_hot(&mut self, hot: &mut Hot) -> Step {
        hot.expr = true;
        self.frames.reset_run();
        self.levels.close_conditional_and_arrow(&mut self.frames)?;
        Ok(self.weigh()?)
    }

    #[inline(always)]
    fn close_hot(&mut self, hot: &mut Hot) -> Result<(), DepthRefusal> {
        let Some(frame) = self.frames.close() else {
            return Ok(());
        };
        self.speculative.closed(frame.speculative);
        self.levels
            .close_frame(&self.frames, frame.level_base as usize)?;
        match frame.head {
            Head::None => {}
            Head::If => self.pend_body_hot(hot, Level::IfBody),
            Head::Other => self.pend_body_hot(hot, Level::Body),
            Head::Pattern => hot.pending |= self.watch_for(Watch::Ended),
        }
        Ok(())
    }

    #[inline(always)]
    fn pend_body_hot(&mut self, hot: &mut Hot, level: Level) {
        hot.pending |= PENDING_BODY;
        self.body = level;
    }

    #[inline(always)]
    fn close_context_hot(&mut self, hot: &mut Hot) -> Step {
        self.close_hot(hot)?;
        self.weigh()?;
        hot.expr = self.contexts.close();
        let mode = mode_of(self.contexts.top());
        if mode == Mode::Code {
            return Ok(());
        }
        self.mode = mode;
        Err(Stop::Leave)
    }

    pub(super) fn fast(&mut self, tok: Tok, newline: bool) -> Result<(), DepthRefusal> {
        match tok {
            Tok::ParenL => self.paren_left(newline),
            Tok::ParenR | Tok::BraceR => self.close_with_context(),
            Tok::BracketL => {
                if self.index_signature(self.prev, self.position, newline)? {
                    self.restart_precise();
                }
                self.contexts.expr_allowed = true;
                self.open(self.pattern_head(Tok::BracketL))
            }
            Tok::BracketR => {
                self.contexts.expr_allowed = false;
                self.close()
            }
            Tok::Comma => self.comma(),
            Tok::Semi => {
                self.contexts.expr_allowed = true;
                self.pending |= PENDING_SEMI;
                Ok(())
            }
            Tok::Dot => {
                self.contexts.expr_allowed = false;
                self.frames.count_run()
            }
            _ => self.plain(tok, newline),
        }
    }

    #[inline(never)]
    fn generic(
        &mut self,
        byte: u8,
        start: usize,
        newline: &mut bool,
    ) -> Result<bool, DepthRefusal> {
        let class = CLASS[usize::from(byte)];
        let Some(tok) = self.read_code(byte, class, start, newline) else {
            return Ok(false);
        };
        self.token(tok, start, *newline)?;
        Ok(true)
    }
}
