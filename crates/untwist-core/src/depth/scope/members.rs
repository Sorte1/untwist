use super::{ends_an_operand, is_key_word, Kind, Step, Tracker, AWAIT, YIELD};
use crate::depth::context::Ctx;
use crate::depth::lexer::Tok;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Start,
    Word,
    Computed(usize),
    Value,
    Method,
    Body,
    Decorator(bool),
}

#[derive(Clone, Copy)]
pub(super) struct Members {
    pub(super) depth: usize,
    brackets: usize,
    class: bool,
    key: Key,
    modifiers: u8,
    after_async: bool,
    after_static: bool,
    keyword_name: bool,
}

impl Tracker {
    pub(in crate::depth) fn at_member_name(&self, depth: usize) -> bool {
        self.members.last().is_some_and(|members| {
            members.depth == depth
                && members.brackets == self.brackets.len()
                && members.key == Key::Word
        })
    }

    pub(in crate::depth) fn after_decorator(&self, depth: usize) -> bool {
        self.members.last().is_some_and(|members| {
            members.class
                && members.depth == depth
                && members.brackets == self.brackets.len()
                && members.key == Key::Decorator(false)
        })
    }

    pub(in crate::depth) fn at_member_start(
        &self,
        depth: usize,
        newline: bool,
        modifier: bool,
    ) -> bool {
        let Some(members) = self.members.last() else {
            return false;
        };
        if !members.class || members.depth != depth || members.brackets != self.brackets.len() {
            return false;
        }
        match members.key {
            Key::Start => true,
            Key::Word => newline || modifier,
            _ => false,
        }
    }

    pub(super) fn open_members(&mut self, step: &Step, top: Ctx, parent: Option<Ctx>) {
        if step.tok != Tok::BraceL || step.after <= step.before {
            return;
        }
        let class_body = top == Ctx::BStat
            && step.prev != Tok::Arrow
            && self.classes.last() == Some(&(step.after - 1));
        let object = top == Ctx::BExpr && parent != Some(Ctx::JOTag);
        if object || class_body {
            self.members.push(Members {
                depth: step.after,
                brackets: self.brackets.len(),
                class: class_body,
                key: Key::Start,
                modifiers: 0,
                after_async: false,
                after_static: false,
                keyword_name: false,
            });
        }
    }

    pub(super) fn member_brace(&mut self, step: &Step) {
        let Some(members) = self.members.last().copied() else {
            return;
        };
        if step.tok != Tok::BraceL || members.depth + 1 != step.after {
            return;
        }
        if members.class && members.key == Key::Word && members.after_static {
            self.push(AWAIT, step.after, Kind::Body);
        }
        if let Some(last) = self.members.last_mut() {
            let body = step.prev == Tok::ParenR || matches!(members.key, Key::Word | Key::Method);
            last.key = if body { Key::Body } else { Key::Value };
        }
    }

    pub(super) fn keyword_key(&self, step: &Step) -> bool {
        matches!(step.tok, Tok::KwFunction | Tok::KwClass)
            && self.members.last().is_some_and(|members| {
                members.depth == step.before
                    && members.brackets == self.brackets.len()
                    && matches!(members.key, Key::Start | Key::Word)
            })
    }

    pub(super) fn member_step(&mut self, step: &Step, word: &[u8], keyword_key: bool) -> bool {
        let Some(mut members) = self.members.last().copied() else {
            return false;
        };
        let Key::Decorator(operand) = members.key else {
            self.member_rest(members, step, word, keyword_key);
            return false;
        };
        if decorator_step(&mut members, step, operand, self.brackets.len()) {
            self.store_members(members);
            return false;
        }
        members.reset();
        self.member_rest(members, step, word, keyword_key);
        true
    }

    fn member_rest(&mut self, mut members: Members, step: &Step, word: &[u8], keyword_key: bool) {
        let brackets = self.brackets.len();
        if keyword_key {
            self.member_word(&mut members, step, word);
            members.keyword_name = step.after > step.before;
            self.store_members(members);
            return;
        }
        if members.depth != step.after {
            let params = members.depth + 1 + usize::from(members.keyword_name);
            if step.tok == Tok::ParenL && params == step.after {
                self.method_params(&mut members, step.after);
                self.store_members(members);
            }
            return;
        }
        if let Key::Computed(level) = members.key {
            if step.tok == Tok::BracketR && level + 1 == brackets {
                members.key = Key::Word;
                members.after_async = false;
                members.after_static = false;
                self.store_members(members);
            }
            return;
        }
        if brackets != members.brackets {
            return;
        }
        self.member_token(&mut members, step, word);
        self.store_members(members);
    }

    fn member_token(&mut self, members: &mut Members, step: &Step, word: &[u8]) {
        match (step.tok, members.key) {
            (Tok::BracketL, Key::Start | Key::Word) => {
                if members.key == Key::Word && members.after_async && !step.newline {
                    members.modifiers |= AWAIT;
                }
                members.key = Key::Computed(members.brackets);
            }
            (Tok::Comma, _) if !members.class => members.reset(),
            (Tok::Semi, _) if members.class => {
                self.end_expressions(step.after, &[Kind::Arrow, Kind::Field]);
                members.reset();
            }
            (Tok::BraceR, Key::Body | Key::Method) if members.class => members.reset(),
            (Tok::Star, Key::Start | Key::Word) => members.star(step.newline),
            (Tok::At, _) if members.class => self.member_decorator(members, step),
            (Tok::Eq, Key::Word) if members.class => {
                self.push(0, step.after, Kind::Field);
                members.key = Key::Value;
            }
            _ if is_key_word(step.tok) => self.member_word(members, step, word),
            _ => members.key = Key::Value,
        }
    }

    fn member_decorator(&mut self, members: &mut Members, step: &Step) {
        let ended = matches!(members.key, Key::Value | Key::Word);
        let operand = members.key == Key::Word || ends_an_operand(step.prev);
        if ended && step.newline && operand {
            self.end_expressions(step.after, &[Kind::Field]);
            members.reset();
        }
        members.key = if members.key == Key::Start {
            Key::Decorator(true)
        } else {
            Key::Value
        };
    }

    fn member_word(&mut self, members: &mut Members, step: &Step, word: &[u8]) {
        let new_member = members.class
            && step.newline
            && members.key == Key::Value
            && ends_an_operand(step.prev)
            && !matches!(step.tok, Tok::KwIn | Tok::KwInstanceof);
        if new_member {
            self.end_expressions(step.after, &[Kind::Field]);
            members.reset();
        }
        if !matches!(members.key, Key::Start | Key::Word) {
            members.key = Key::Value;
            return;
        }
        if members.key == Key::Word && members.after_async && !step.newline {
            members.modifiers |= AWAIT;
        }
        members.key = Key::Word;
        members.keyword_name = false;
        members.after_async = step.tok == Tok::NameAsync && word == b"async";
        members.after_static = step.tok == Tok::NameStatic && word == b"static";
    }

    fn method_params(&mut self, members: &mut Members, depth: usize) {
        match members.key {
            Key::Word => {}
            Key::Computed(_) => return,
            _ => {
                members.key = Key::Value;
                return;
            }
        }
        self.push(members.modifiers, depth, Kind::Params);
        members.key = Key::Method;
    }

    fn store_members(&mut self, members: Members) {
        if let Some(last) = self.members.last_mut() {
            *last = members;
        }
    }
}

impl Members {
    fn reset(&mut self) {
        self.key = Key::Start;
        self.keyword_name = false;
        self.modifiers = 0;
        self.after_async = false;
        self.after_static = false;
    }

    fn star(&mut self, newline: bool) {
        if self.key == Key::Word && self.after_async && !newline {
            self.modifiers |= AWAIT;
        }
        self.modifiers |= YIELD;
        self.key = Key::Start;
        self.after_async = false;
    }
}

fn decorator_step(members: &mut Members, step: &Step, operand: bool, brackets: usize) -> bool {
    let depth = members.depth;
    if step.before == depth && step.after == depth && brackets > members.brackets {
        if step.tok == Tok::BracketR && brackets == members.brackets + 1 {
            members.key = Key::Decorator(false);
        }
        return true;
    }
    if step.before > depth {
        if step.after == depth {
            members.key = Key::Decorator(false);
        }
        return step.after >= depth;
    }
    if step.before < depth || step.after < depth {
        return false;
    }
    if step.after > depth {
        return true;
    }
    match step.tok {
        Tok::Dot | Tok::QuestionDot => members.key = Key::Decorator(true),
        Tok::KwNew | Tok::BracketL if operand => {}
        tok if operand && is_key_word(tok) => members.key = Key::Decorator(false),
        _ => return false,
    }
    true
}
