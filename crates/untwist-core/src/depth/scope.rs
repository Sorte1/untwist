mod members;

use super::context::Ctx;
use super::lexer::Tok;
use super::{DepthRefusal, Goal, Scanner};
use members::Members;

pub(super) const AWAIT: u8 = 1;
pub(super) const YIELD: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Function,
    Body,
    ArrowBody,
    Params,
    Arrow,
    Field,
    Unary,
}

#[derive(Clone, Copy)]
struct Scope {
    flags: u8,
    depth: usize,
    brackets: usize,
    questions: u32,
    kind: Kind,
    operand: bool,
}

pub(super) struct Tracker {
    base: u8,
    scopes: Vec<Scope>,
    members: Vec<Members>,
    brackets: Vec<usize>,
    classes: Vec<usize>,
    parens: Vec<bool>,
    closed_async_paren: bool,
    pending_body: Option<u8>,
    pending_arrow: Option<u8>,
    prev: Tok,
    before_prev: Tok,
    newline_before_prev: bool,
    declarations: Vec<(usize, usize)>,
    binding_next: bool,
    started: Option<Tok>,
    after_binding: bool,
    postfix: bool,
    dos: Vec<(usize, usize)>,
    do_while_head: bool,
    do_while_paren: Option<usize>,
    pub(super) after_do_while: bool,
    pub(super) delegate: bool,
    pub(super) argument: Option<(u8, bool)>,
    pub(super) last_span: (usize, usize),
    pub(super) word_reading: Option<(bool, bool)>,
}

pub(super) struct Step {
    pub(super) tok: Tok,
    pub(super) expr_before: bool,
    pub(super) after_declarer: bool,
    pub(super) head: bool,
    pub(super) unary: bool,
    pub(super) prev: Tok,
    pub(super) newline: bool,
    pub(super) before: usize,
    pub(super) after: usize,
    pub(super) span: (usize, usize),
}

impl Tracker {
    pub(super) fn new(goal: Goal) -> Tracker {
        Tracker {
            base: if goal == Goal::Module { AWAIT } else { 0 },
            scopes: Vec::new(),
            members: Vec::new(),
            brackets: Vec::new(),
            classes: Vec::new(),
            parens: Vec::new(),
            closed_async_paren: false,
            pending_body: None,
            pending_arrow: None,
            prev: Tok::Start,
            before_prev: Tok::Start,
            newline_before_prev: false,
            declarations: Vec::new(),
            binding_next: false,
            started: None,
            after_binding: false,
            postfix: false,
            dos: Vec::new(),
            do_while_head: false,
            do_while_paren: None,
            after_do_while: false,
            delegate: false,
            argument: None,
            last_span: (0, 0),
            word_reading: None,
        }
    }

    fn declarations(&mut self, step: &Step, declarer: Option<Tok>, after_declarer: bool) -> bool {
        let follows = std::mem::take(&mut self.binding_next) || after_declarer;
        let binding = follows && step.tok.is_name();
        let level = (step.after, self.brackets.len());
        let started = self.started.take();
        while self
            .declarations
            .last()
            .is_some_and(|&(depth, brackets)| (depth, brackets) > level)
        {
            self.declarations.pop();
        }
        if let Some(declarer) = declarer {
            self.declarations.push(level);
            self.started = (declarer != Tok::KwVar && declarer != Tok::KwConst).then_some(declarer);
            return false;
        }
        let after_binding = std::mem::replace(&mut self.after_binding, binding);
        let declarator_ends = after_binding
            && step.newline
            && !matches!(
                step.tok,
                Tok::Eq | Tok::Comma | Tok::Semi | Tok::KwIn | Tok::NameOf
            );
        let unopened = started.is_some_and(|declarer| !opens_a_binding(declarer, step));
        if unopened || declarator_ends {
            self.declarations.pop();
            return false;
        }
        if self.declarations.last() != Some(&level) {
            return binding;
        }
        let breaks = step.newline
            && !after_declarer
            && ends_an_operand(step.prev)
            && starts_an_operand(step, self.postfix);
        if step.tok == Tok::Semi || breaks {
            self.declarations.pop();
        } else if step.tok == Tok::Comma {
            self.binding_next = true;
        }
        binding && !breaks
    }

    pub(super) fn flags(&self) -> u8 {
        self.scopes.last().map_or(self.base, |scope| scope.flags)
    }

    pub(super) fn at_top_level(&self) -> bool {
        self.scopes.is_empty()
    }

    fn push(&mut self, flags: u8, depth: usize, kind: Kind) {
        self.scopes.push(Scope {
            flags,
            depth,
            brackets: self.brackets.len(),
            questions: 0,
            kind,
            operand: true,
        });
    }

    fn close_to(&mut self, depth: usize) {
        let mut arrow_closed = false;
        while let Some(scope) = self.scopes.last().copied() {
            if scope.depth <= depth {
                break;
            }
            self.scopes.pop();
            if scope.kind == Kind::Params {
                self.pending_body = Some(scope.flags);
            }
            arrow_closed = scope.kind == Kind::ArrowBody && scope.depth == depth + 1;
        }
        if arrow_closed {
            self.end_complete_arrows(depth);
        }
        while self
            .members
            .last()
            .is_some_and(|members| members.depth > depth)
        {
            self.members.pop();
        }
        while self.brackets.last().is_some_and(|&level| level > depth) {
            self.brackets.pop();
        }
        while self.classes.last().is_some_and(|&level| level > depth) {
            self.classes.pop();
        }
    }

    fn at_level(&self, scope: &Scope, depth: usize) -> bool {
        scope.depth == depth && scope.brackets == self.brackets.len()
    }

    fn end_expressions(&mut self, depth: usize, kinds: &[Kind]) {
        while let Some(scope) = self.scopes.last().copied() {
            if !kinds.contains(&scope.kind) || !self.at_level(&scope, depth) {
                break;
            }
            self.scopes.pop();
        }
    }

    fn end_complete_arrows(&mut self, depth: usize) {
        let brackets = self.brackets.len();
        while self.scopes.last().is_some_and(|scope| {
            scope.kind == Kind::Arrow
                && scope.depth == depth
                && scope.brackets == brackets
                && scope.questions == 0
        }) {
            self.scopes.pop();
        }
    }

    fn close_brackets(&mut self, depth: usize) {
        let brackets = self.brackets.len();
        while self
            .scopes
            .last()
            .is_some_and(|scope| scope.depth == depth && scope.brackets > brackets)
        {
            self.scopes.pop();
        }
    }

    fn conditional(&mut self, tok: Tok, depth: usize) {
        let brackets = self.brackets.len();
        while let Some(scope) = self.scopes.last_mut() {
            if scope.kind != Kind::Arrow || scope.depth != depth || scope.brackets != brackets {
                return;
            }
            if tok == Tok::Question {
                scope.questions += 1;
                return;
            }
            if scope.questions > 0 {
                scope.questions -= 1;
                return;
            }
            self.scopes.pop();
        }
    }

    fn statement_break(&mut self, step: &Step) {
        let awaiting = step.prev == Tok::NameAwait && self.flags() & AWAIT != 0;
        let yield_ended = step.prev == Tok::Semi && self.prev == Tok::NameYield;
        let broken = step.newline
            && !awaiting
            && ends_an_operand(step.prev)
            && starts_an_operand(step, self.postfix);
        if yield_ended || broken {
            self.end_expressions(step.before, &[Kind::Arrow, Kind::Unary]);
        }
    }

    fn record(&mut self, step: &Step) {
        self.before_prev = self.prev;
        self.prev = step.tok;
        self.newline_before_prev = step.newline;
        self.postfix = step.tok == Tok::IncDec && !step.expr_before && !step.newline;
        self.last_span = step.span;
    }

    pub(super) fn after_for(&self) -> bool {
        self.before_prev == Tok::KwFor
    }

    pub(super) fn postfix(&self) -> bool {
        self.postfix
    }

    fn unary_step(&mut self, step: &Step, word: &[u8]) {
        let brackets = self.brackets.len();
        let Some(scope) = self.scopes.last_mut() else {
            return;
        };
        if scope.kind != Kind::Unary || scope.depth != step.before || scope.brackets != brackets {
            return;
        }
        if !scope.operand {
            scope.operand = !is_prefix(step.tok, word);
            return;
        }
        let member = matches!(step.prev, Tok::Dot | Tok::QuestionDot);
        let continues = matches!(
            step.tok,
            Tok::Dot
                | Tok::QuestionDot
                | Tok::BracketL
                | Tok::BracketR
                | Tok::ParenL
                | Tok::BackQuote
        ) || (step.tok == Tok::IncDec && !step.newline);
        if !member && !continues {
            self.scopes.pop();
        }
    }

    fn push_argument(&mut self, step: &Step, word: &[u8]) {
        let Some((flag, unary)) = self.argument.take() else {
            return;
        };
        let flags = self.flags() | flag;
        let kind = if unary { Kind::Unary } else { Kind::Arrow };
        self.push(flags, step.before, kind);
        if let Some(scope) = self.scopes.last_mut() {
            scope.operand = !is_prefix(step.tok, word);
        }
    }

    fn do_while(&mut self, step: &Step) {
        self.after_do_while = false;
        let level = (step.after, self.brackets.len());
        while self.dos.last().is_some_and(|&open| open > level) {
            self.dos.pop();
        }
        let head = std::mem::take(&mut self.do_while_head);
        match step.tok {
            Tok::KwDo => self.dos.push(level),
            Tok::KwWhile if self.dos.last() == Some(&level) => {
                self.dos.pop();
                self.do_while_head = true;
            }
            Tok::ParenL if head => self.do_while_paren = Some(step.after),
            Tok::ParenR if self.do_while_paren == Some(step.before) => {
                self.do_while_paren = None;
                self.after_do_while = true;
            }
            _ => {}
        }
    }

    fn arrow_flags(&self, prev: Tok, closed_async_paren: bool) -> u8 {
        let async_head = (prev == Tok::ParenR && closed_async_paren)
            || (prev.is_name() && self.before_prev == Tok::NameAsync && !self.newline_before_prev);
        if async_head {
            AWAIT
        } else {
            0
        }
    }

    fn open_body(&mut self, step: &Step) {
        if let Some(flags) = self.pending_body.take() {
            if step.tok == Tok::BraceL {
                self.push(flags, step.after, Kind::Body);
            }
        }
        if let Some(flags) = self.pending_arrow.take() {
            if step.tok == Tok::BraceL {
                self.push(flags, step.after, Kind::ArrowBody);
            } else {
                self.push(flags, step.before, Kind::Arrow);
            }
        }
    }

    fn structure(&mut self, step: &Step, closed_async_paren: bool, ctx: Ctx, keyword_key: bool) {
        match step.tok {
            Tok::KwFunction | Tok::KwClass if keyword_key => {}
            Tok::BracketL => self.brackets.push(step.after),
            Tok::BracketR if self.brackets.last() == Some(&step.after) => {
                self.brackets.pop();
                self.close_brackets(step.after);
            }
            Tok::ParenL => self
                .parens
                .push(step.prev == Tok::NameAsync && !step.newline),
            Tok::ParenR => self.closed_async_paren = self.parens.pop().unwrap_or(false),
            Tok::KwFunction if ctx.is_function() => {
                let flags = if step.prev == Tok::NameAsync && !step.newline {
                    AWAIT
                } else {
                    0
                };
                self.push(flags, step.after, Kind::Function);
            }
            Tok::KwClass if ctx.is_function() => {
                self.push(self.flags(), step.after, Kind::Function);
                self.classes.push(step.after);
            }
            Tok::Star if step.prev == Tok::KwFunction => self.mark_generator(),
            Tok::Arrow => {
                self.pending_arrow = Some(self.arrow_flags(step.prev, closed_async_paren))
            }
            Tok::Comma | Tok::Semi => self.end_expressions(step.after, &[Kind::Arrow]),
            Tok::Question | Tok::Colon => self.conditional(step.tok, step.after),
            _ => {}
        }
    }

    fn mark_generator(&mut self) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.kind == Kind::Function {
                scope.flags |= YIELD;
            }
        }
    }
}

fn opens_a_binding(declarer: Tok, step: &Step) -> bool {
    if declarer == Tok::NameUsing {
        return using_binds(step.tok, step.newline, step.head);
    }
    if declarer == Tok::BracketL {
        return step.tok == Tok::BracketL;
    }
    step.tok.is_name() || matches!(step.tok, Tok::BracketL | Tok::BraceL)
}

pub(super) fn using_binds(tok: Tok, newline: bool, head: bool) -> bool {
    !newline && ((tok.is_name() && !(tok == Tok::NameOf && head)) || tok == Tok::BraceL)
}

fn is_prefix(tok: Tok, word: &[u8]) -> bool {
    match tok {
        Tok::Op => matches!(word, b"!" | b"~" | b"+" | b"-"),
        Tok::KwTypeof | Tok::KwVoid | Tok::KwDelete | Tok::KwNew | Tok::NameAwait => true,
        Tok::IncDec => true,
        _ => false,
    }
}

fn is_key_word(tok: Tok) -> bool {
    tok.is_name()
        || (Tok::KwBreak as u8..=Tok::KwDelete as u8).contains(&(tok as u8))
        || matches!(tok, Tok::Str | Tok::Num | Tok::PrivateName)
}

fn ends_an_operand(tok: Tok) -> bool {
    tok.is_name()
        || matches!(
            tok,
            Tok::Num
                | Tok::Str
                | Tok::Regex
                | Tok::BackQuote
                | Tok::ParenR
                | Tok::BracketR
                | Tok::BraceR
                | Tok::KwThis
                | Tok::KwNull
                | Tok::KwTrue
                | Tok::KwFalse
                | Tok::IncDec
                | Tok::JsxTagEnd
        )
}

fn starts_an_operand(step: &Step, postfix: bool) -> bool {
    let tok = step.tok;
    let closed = step.prev == Tok::JsxTagEnd || (step.prev == Tok::IncDec && postfix);
    if closed && matches!(tok, Tok::BracketL | Tok::ParenL | Tok::BackQuote) {
        return true;
    }
    (is_key_word(tok)
        && !matches!(tok, Tok::KwIn | Tok::KwInstanceof)
        && !(tok == Tok::NameOf && step.head))
        || matches!(tok, Tok::BraceL | Tok::IncDec)
        || step.unary
}

pub(super) fn operand_word(tok: Tok) -> bool {
    is_key_word(tok) && !matches!(tok, Tok::KwIn | Tok::KwInstanceof | Tok::PrivateName)
}

impl Scanner<'_> {
    pub(super) fn precise_token(
        &mut self,
        tok: Tok,
        start: usize,
        newline: bool,
    ) -> Result<(), DepthRefusal> {
        let tok = self.clause_name(tok);
        self.settle_operator(tok, start, newline);
        let prev = self.prev;
        let head = self.contexts.top() == Ctx::PStat;
        let using = prev == Tok::NameUsing && tok.is_name() && using_binds(tok, newline, head);
        let after_declarer = self.binds(prev) || using;
        let expr_before = self.contexts.expr_allowed;
        let before = self.contexts.len();
        self.token(tok, start, newline)?;
        if tok == Tok::NameLet && self.tracker.after_do_while {
            self.contexts.let_declares = true;
        }
        let source = self.source;
        let word = source.get(start..self.position).unwrap_or_default();
        let step = Step {
            tok,
            expr_before,
            after_declarer,
            head,
            unary: tok == Tok::Op && matches!(word, b"!" | b"~"),
            prev,
            newline,
            before,
            after: self.contexts.len(),
            span: (start, self.position),
        };
        self.track(&step, word);
        Ok(())
    }

    fn track(&mut self, step: &Step, word: &[u8]) {
        let closed_async_paren = std::mem::take(&mut self.tracker.closed_async_paren);
        self.tracker.statement_break(step);
        self.tracker.open_body(step);
        self.tracker.close_to(step.after);
        self.tracker.unary_step(step, word);
        self.tracker.push_argument(step, word);
        let keyword_key = self.tracker.keyword_key(step);
        self.tracker.member_brace(step);
        if self.tracker.member_step(step, word, keyword_key) && step.tok != Tok::At {
            self.contexts.clear_decorator(step.before);
        }
        let top = self.contexts.top();
        self.tracker
            .structure(step, closed_async_paren, top, keyword_key);
        let parent = self.contexts.below_top();
        self.tracker.open_members(step, top, parent);
        let declares = matches!(step.tok, Tok::KwVar | Tok::KwConst | Tok::NameUsing)
            || (step.tok == Tok::NameLet && self.contexts.let_declares);
        let declarer = declares.then_some(
            if step.tok == Tok::NameLet && self.contexts.let_bracket_only {
                Tok::BracketL
            } else {
                step.tok
            },
        );
        let binding = self
            .tracker
            .declarations(step, declarer, step.after_declarer);
        if binding {
            self.prev = Tok::NameBinding;
        }
        self.tracker.do_while(step);
        let opens_an_assignment = self.opens_an_assignment(step);
        self.tracker.delegate =
            step.tok == Tok::Star && step.prev == Tok::Eq && self.tracker.prev == Tok::NameYield;
        self.tracker.record(step);
        if !binding && matches!(step.tok, Tok::NameAwait | Tok::NameYield) {
            self.operator_reading(step, opens_an_assignment);
        } else {
            self.tracker.word_reading = None;
        }
    }
}
