use super::lexer::Tok;
use super::scope::{operand_word, Step, AWAIT, YIELD};
use super::{Goal, Scanner, PENDING_ARROW, PENDING_BODY};

fn is_assignment(text: &[u8]) -> bool {
    text.ends_with(b"=") && !matches!(text, b"==" | b"===" | b"!=" | b"!==" | b"<=" | b">=")
}

pub(super) fn awaits_an_operand(prev: Tok) -> bool {
    !matches!(
        prev,
        Tok::KwNew | Tok::KwExtends | Tok::At | Tok::KwClass | Tok::KwFunction
    )
}

fn binds_an_operand(prev: Tok) -> bool {
    !matches!(
        prev,
        Tok::Slash
            | Tok::Star
            | Tok::Logical
            | Tok::KwNew
            | Tok::KwExtends
            | Tok::KwTypeof
            | Tok::KwVoid
            | Tok::KwDelete
            | Tok::KwInstanceof
            | Tok::At
            | Tok::NameAwait
            | Tok::Dot
            | Tok::QuestionDot
    )
}

#[inline(always)]
pub(super) fn may_end_a_statement(prev: Tok, newline: bool) -> bool {
    prev == Tok::BraceR || (newline && ends_an_expression(prev))
}

fn ends_an_expression(prev: Tok) -> bool {
    matches!(
        prev,
        Tok::Name
            | Tok::Num
            | Tok::Str
            | Tok::Regex
            | Tok::BackQuote
            | Tok::ParenR
            | Tok::BracketR
            | Tok::KwThis
            | Tok::KwNull
            | Tok::KwTrue
            | Tok::KwFalse
    )
}

fn starts_a_statement(tok: Tok, word: &[u8]) -> bool {
    match tok {
        Tok::Name => {
            !word.contains(&b'\\')
                && !matches!(word, b"as" | b"satisfies" | b"implements" | b"from")
        }
        Tok::NameAsync
        | Tok::NameYield
        | Tok::NameAwait
        | Tok::NameLet
        | Tok::NameStatic
        | Tok::NameUsing
        | Tok::Num
        | Tok::Str => true,
        Tok::KwCatch
        | Tok::KwElse
        | Tok::KwFinally
        | Tok::KwWhile
        | Tok::KwExtends
        | Tok::KwIn
        | Tok::KwInstanceof => false,
        _ => (Tok::KwBreak as u8..=Tok::KwDelete as u8).contains(&(tok as u8)),
    }
}

pub(super) fn reading_is_doubtful(tok: Tok, first: u8) -> bool {
    match tok {
        Tok::Slash | Tok::Regex | Tok::JsxTagStart | Tok::BraceL | Tok::IncDec => true,
        Tok::KwFunction | Tok::KwClass | Tok::NameOf | Tok::NameAsync | Tok::NameLet => true,
        Tok::At => true,
        Tok::Op => matches!(first, b'/' | b'<'),
        _ => false,
    }
}

impl Scanner<'_> {
    #[inline(never)]
    pub(super) fn starts_a_new_statement(&self, pending: u8, tok: Tok, word: &[u8]) -> bool {
        pending & (PENDING_BODY | PENDING_ARROW) == 0
            && !self.contexts.decorated_at(self.contexts.len())
            && starts_a_statement(tok, word)
    }

    pub(super) fn opens_an_assignment(&self, step: &Step) -> bool {
        if step.tok != Tok::NameYield {
            return awaits_an_operand(step.prev);
        }
        let (from, to) = self.tracker.last_span;
        let text = self.source.get(from..to).unwrap_or_default();
        match step.prev {
            Tok::Op => is_assignment(text),
            Tok::KwIn => step.head,
            Tok::IncDec => self.tracker.postfix(),
            Tok::Star => self.tracker.delegate,
            prev => binds_an_operand(prev),
        }
    }

    pub(super) fn operator_reading(&mut self, step: &Step, position: bool) {
        let flags = self.tracker.flags();
        let flag = if step.tok == Tok::NameAwait {
            if self.goal == Goal::Module && self.tracker.at_top_level() {
                self.divergent = true;
            }
            AWAIT
        } else {
            YIELD
        };
        let reading = flags & flag != 0 && position;
        self.tracker.word_reading = Some((reading, position));
        self.contexts.expr_allowed = reading != self.contexts.flip;
    }

    pub(super) fn settle_operator(&mut self, tok: Tok, start: usize, newline: bool) {
        let prev = self.prev;
        if !matches!(prev, Tok::NameAwait | Tok::NameYield) {
            return;
        }
        let Some((reading, position)) = self.tracker.word_reading.take() else {
            return;
        };
        let flag = if prev == Tok::NameAwait { AWAIT } else { YIELD };
        let scoped = self.tracker.flags() & flag != 0;
        let word = !newline && operand_word(tok) && !(prev == Tok::NameAwait && tok == Tok::NameOf);
        let operator = if word {
            position
        } else {
            if reading_is_doubtful(tok, self.source.get(start).copied().unwrap_or(0)) {
                self.contexts.doubted = true;
            }
            reading != self.contexts.flip
        };
        if word && operator && !scoped {
            self.tracker.argument = Some((flag, prev == Tok::NameAwait));
        }
        let heads_a_loop = scoped || self.tracker.after_for();
        self.settle_prev(prev, operator, newline, tok == Tok::ParenL && heads_a_loop);
    }

    fn settle_prev(&mut self, prev: Tok, operator: bool, newline: bool, awaited_paren: bool) {
        self.prev = match (operator, prev) {
            (false, _) => Tok::Name,
            (true, Tok::NameYield) if newline => Tok::Semi,
            (true, Tok::NameYield) => Tok::Eq,
            (true, _) => Tok::Op,
        };
        self.contexts.expr_allowed = operator;
        if prev == Tok::NameAwait && awaited_paren {
            self.contexts.expr_allowed = false;
            if operator || self.tracker.after_for() {
                self.prev = Tok::NameAwait;
            }
        }
    }
}
