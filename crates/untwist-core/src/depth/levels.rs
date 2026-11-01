use super::context::Frames;
use super::lexer::{is_plain_oxc_identifier, Tok};
use super::DepthRefusal;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Level {
    Question,
    Arrow,
    Body,
    IfBody,
}

pub(super) struct Levels {
    stack: Vec<Level>,
    pub(super) weight: u64,
    weight_max: u64,
}

impl Levels {
    pub(super) fn new(weight_max: u64) -> Levels {
        Levels {
            stack: Vec::new(),
            weight: 0,
            weight_max,
        }
    }

    #[inline]
    pub(super) fn len(&self) -> usize {
        self.stack.len()
    }

    #[inline]
    pub(super) fn depth(&self, frames: &Frames) -> u64 {
        frames.open_count() + self.stack.len() as u64
    }

    #[inline]
    pub(super) fn add(&mut self, depth: u64) -> Result<(), DepthRefusal> {
        self.weight = self.weight.saturating_add(depth).saturating_add(1);
        if self.weight > self.weight_max {
            return Err(DepthRefusal::PrintSize);
        }
        Ok(())
    }

    pub(super) fn push(&mut self, frames: &mut Frames, level: Level) {
        if level == Level::Question {
            frames.top.questions = frames.top.questions.saturating_add(1);
        }
        self.stack.push(level);
    }

    fn pop(&mut self, frames: &mut Frames) -> Result<Option<Level>, DepthRefusal> {
        let Some(level) = self.stack.pop() else {
            return Ok(None);
        };
        if level == Level::Question {
            frames.top.questions = frames.top.questions.saturating_sub(1);
        }
        self.add(self.depth(frames))?;
        Ok(Some(level))
    }

    fn base(frames: &Frames) -> usize {
        frames.top.level_base as usize
    }

    pub(super) fn close_all(&mut self, frames: &mut Frames) -> Result<(), DepthRefusal> {
        let base = Self::base(frames);
        while self.stack.len() > base {
            self.pop(frames)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub(super) fn close_conditional_and_arrow(
        &mut self,
        frames: &mut Frames,
    ) -> Result<(), DepthRefusal> {
        let base = Self::base(frames);
        if self.stack.len() <= base {
            return Ok(());
        }
        while self.stack.len() > base
            && matches!(self.stack.last(), Some(Level::Question | Level::Arrow))
        {
            self.pop(frames)?;
        }
        Ok(())
    }

    pub(super) fn close_to_question(&mut self, frames: &mut Frames) -> Result<(), DepthRefusal> {
        if frames.top.questions == 0 {
            return Ok(());
        }
        let base = Self::base(frames);
        while self.stack.len() > base {
            if self.pop(frames)? == Some(Level::Question) {
                break;
            }
        }
        Ok(())
    }

    pub(super) fn close_if_body(&mut self, frames: &mut Frames) -> Result<(), DepthRefusal> {
        let base = Self::base(frames);
        if self.stack.len() > base && self.stack.last() == Some(&Level::IfBody) {
            self.pop(frames)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub(super) fn close_frame(&mut self, frames: &Frames, base: usize) -> Result<(), DepthRefusal> {
        while self.stack.len() > base {
            self.stack.pop();
            self.add(self.depth(frames))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    None,
    Open(usize),
    AfterSpread(usize),
    AfterBinding(usize),
}

pub(super) struct Speculative {
    pending: Pending,
    pub(super) open: u32,
    pub(super) max: u32,
    pub(super) pairs: u64,
    parens_limit: u32,
    pairs_limit: u64,
}

impl Speculative {
    pub(super) fn new(parens_limit: u32, pairs_limit: u64) -> Speculative {
        Speculative {
            pending: Pending::None,
            open: 0,
            max: 0,
            pairs: 0,
            parens_limit,
            pairs_limit,
        }
    }

    pub(super) fn candidate(&mut self, frame: usize) {
        self.pending = Pending::Open(frame);
    }

    pub(super) fn resolve(
        &mut self,
        frames: &mut Frames,
        tok: Tok,
        word: &[u8],
    ) -> Result<bool, DepthRefusal> {
        let (frame, verdict) = match self.pending {
            Pending::None => return Ok(false),
            Pending::Open(frame) => (frame, after_open(tok)),
            Pending::AfterSpread(frame) => (frame, after_spread(tok, word)),
            Pending::AfterBinding(frame) => (frame, after_binding(tok)),
        };
        self.pending = Pending::None;
        match verdict {
            Verdict::Maybe => self.flag(frames, frame).map(|()| false),
            Verdict::Spread => {
                self.pending = Pending::AfterSpread(frame);
                Ok(true)
            }
            Verdict::Binding => {
                self.pending = Pending::AfterBinding(frame);
                Ok(true)
            }
            Verdict::No => Ok(false),
        }
    }

    fn flag(&mut self, frames: &mut Frames, index: usize) -> Result<(), DepthRefusal> {
        let Some(frame) = frames.get(index) else {
            return Ok(());
        };
        frame.speculative = true;
        self.pairs = self.pairs.saturating_add(u64::from(self.open));
        self.open = self.open.saturating_add(1);
        self.max = self.max.max(self.open);
        if self.open > self.parens_limit || self.pairs > self.pairs_limit {
            return Err(DepthRefusal::SpeculativeParens);
        }
        Ok(())
    }

    pub(super) fn closed(&mut self, speculative: bool) {
        if speculative {
            self.open = self.open.saturating_sub(1);
        }
    }
}

enum Verdict {
    Maybe,
    Spread,
    Binding,
    No,
}

fn after_open(tok: Tok) -> Verdict {
    match tok {
        Tok::BracketL | Tok::BraceL => Verdict::Maybe,
        Tok::Ellipsis => Verdict::Spread,
        Tok::KwThis => Verdict::Binding,
        _ if tok.is_name() && tok != Tok::NameEnum => Verdict::Binding,
        _ => Verdict::No,
    }
}

fn after_spread(tok: Tok, word: &[u8]) -> Verdict {
    match tok {
        Tok::Name if is_plain_oxc_identifier(word) => Verdict::No,
        _ if tok.is_literal() => Verdict::No,
        _ => Verdict::Maybe,
    }
}

fn after_binding(tok: Tok) -> Verdict {
    match tok {
        Tok::Comma | Tok::Eq | Tok::ParenR => Verdict::Maybe,
        _ => Verdict::No,
    }
}
