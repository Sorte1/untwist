mod context;
mod escape;
mod hot;
mod jsx;
mod keywords;
mod levels;
mod lexer;
mod limits;
mod read;
mod scope;
mod typescript;
#[cfg(target_arch = "wasm32")]
mod wasm;
mod words;

use context::{Contexts, Ctx, Frame, Frames, Head};
use levels::{Level, Levels, Speculative};
use lexer::{Tok, BEFORE_EXPR, LINE_TOKEN, RUN, SPECIAL};
use scope::Tracker;

#[cfg(not(target_arch = "wasm32"))]
use limits::noted;
pub use limits::{
    limits_for, C_P1, LIFT_DEPTH_MAX, UNIT_STACK_BYTES, WASM_NESTING, WASM_SHADOW_STACK_BYTES,
    WASM_V8_ROOM,
};
#[cfg(target_arch = "wasm32")]
use wasm::noted;
#[cfg(target_arch = "wasm32")]
pub use wasm::{capped_above_the_input, set_wasm_input_nesting, wasm_budget_refusals};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Limits {
    pub nesting: u32,
    pub speculative_parens: u32,
    pub speculative_pairs: u64,
    pub print_bytes: u64,
}

pub const UNLIMITED: Limits = Limits {
    nesting: u32::MAX,
    speculative_parens: u32::MAX,
    speculative_pairs: u64::MAX,
    print_bytes: u64::MAX,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DepthScan {
    pub nesting: u32,
    pub speculative_parens: u32,
    pub speculative_pairs: u64,
    pub print_weight: u64,
    pub regexes: u32,
    pub templates: u32,
    pub jsx_elements: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthRefusal {
    Nesting,
    SpeculativeParens,
    PrintSize,
    Typescript,
}

pub const PRINT_GROWTH: u64 = 4;
pub const PRINT_LENGTH_FACTOR: u64 = 3;

pub fn print_estimate(scan: &DepthScan, source_len: usize) -> u64 {
    PRINT_GROWTH
        .saturating_mul(scan.print_weight)
        .saturating_add(printed_length(source_len))
}

fn printed_length(source_len: usize) -> u64 {
    u64::try_from(source_len)
        .unwrap_or(u64::MAX)
        .saturating_mul(PRINT_LENGTH_FACTOR)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Goal {
    Module,
    Script,
}

pub fn scan(source: &str, limits: &Limits) -> Result<DepthScan, DepthRefusal> {
    let (module, divergent) = scan_goal(source, limits, Goal::Module)?;
    if !divergent {
        return Ok(module);
    }
    let (script, _) = scan_goal(source, limits, Goal::Script)?;
    Ok(module.widest(&script))
}

pub fn scan_as(source: &str, limits: &Limits, goal: Goal) -> Result<DepthScan, DepthRefusal> {
    scan_goal(source, limits, goal).map(|(result, _)| result)
}

struct Pass {
    result: DepthScan,
    divergent: bool,
    doubted: bool,
}

fn scan_goal(source: &str, limits: &Limits, goal: Goal) -> Result<(DepthScan, bool), DepthRefusal> {
    noted(scan_flips(source, limits, goal), limits)
}

fn scan_flips(
    source: &str,
    limits: &Limits,
    goal: Goal,
) -> Result<(DepthScan, bool), DepthRefusal> {
    let first = scan_pass(source, limits, goal, false)?;
    if !first.doubted {
        return Ok((first.result, first.divergent));
    }
    let flipped = scan_pass(source, limits, goal, true)?;
    let divergent = first.divergent || flipped.divergent;
    Ok((first.result.bounded_by(&flipped.result), divergent))
}

fn scan_pass(source: &str, limits: &Limits, goal: Goal, flip: bool) -> Result<Pass, DepthRefusal> {
    let mut scanner = Scanner::new(source.as_bytes(), limits, goal, flip)?;
    scanner.contexts.flip = flip;
    let outcome = scanner.run();
    if scanner.restart {
        let mut precise = Scanner::new(source.as_bytes(), limits, goal, true)?;
        precise.contexts.flip = flip;
        precise.run()?;
        return Ok(precise.pass());
    }
    outcome?;
    Ok(scanner.pass())
}

impl DepthScan {
    fn bounded_by(&self, other: &DepthScan) -> DepthScan {
        DepthScan {
            nesting: self.nesting.max(other.nesting),
            speculative_parens: self.speculative_parens.max(other.speculative_parens),
            speculative_pairs: self.speculative_pairs.max(other.speculative_pairs),
            print_weight: self.print_weight.max(other.print_weight),
            ..*self
        }
    }

    fn widest(&self, other: &DepthScan) -> DepthScan {
        DepthScan {
            nesting: self.nesting.max(other.nesting),
            speculative_parens: self.speculative_parens.max(other.speculative_parens),
            speculative_pairs: self.speculative_pairs.max(other.speculative_pairs),
            print_weight: self.print_weight.max(other.print_weight),
            regexes: self.regexes.max(other.regexes),
            templates: self.templates.max(other.templates),
            jsx_elements: self.jsx_elements.max(other.jsx_elements),
        }
    }
}

const PENDING_SEMI: u8 = 1;
const PENDING_BODY: u8 = 2;
const PENDING_ARROW: u8 = 4;
const PENDING_HEAD: u8 = 8;
const PENDING_TAG: u8 = 16;
const PENDING_SPECULATIVE: u8 = 32;
const PENDING_OPERAND: u8 = 64;
const PENDING_WATCH: u8 = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Watch {
    Label,
    Ended,
    Module,
    From,
    Specifier,
    Braced,
    Using,
    Default,
    ImportName,
    Alias,
    Closed,
    Named,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Code,
    Template,
    Children,
    Tag,
}

fn mode_of(ctx: Ctx) -> Mode {
    match ctx {
        Ctx::QTmpl => Mode::Template,
        Ctx::JExpr => Mode::Children,
        Ctx::JOTag | Ctx::JCTag => Mode::Tag,
        _ => Mode::Code,
    }
}

struct Scanner<'s> {
    source: &'s [u8],
    position: usize,
    contexts: Contexts,
    frames: Frames,
    levels: Levels,
    speculative: Speculative,
    prev: Tok,
    pending: u8,
    body: Level,
    head: Head,
    else_closes: bool,
    watch: Watch,
    mode: Mode,
    goal: Goal,
    divergent: bool,
    precise: bool,
    restart: bool,
    refusal: Option<DepthRefusal>,
    class_heads: Vec<(usize, u64)>,
    tracker: Tracker,
    regexes: u32,
    templates: u32,
    jsx_elements: u32,
}

impl<'s> Scanner<'s> {
    fn new(
        source: &'s [u8],
        limits: &Limits,
        goal: Goal,
        precise: bool,
    ) -> Result<Scanner<'s>, DepthRefusal> {
        let Some(room) = limits.print_bytes.checked_sub(printed_length(source.len())) else {
            return Err(DepthRefusal::PrintSize);
        };
        let context_cap = usize::try_from(limits.nesting)
            .unwrap_or(usize::MAX)
            .saturating_mul(2)
            .saturating_add(16);
        Ok(Scanner {
            source,
            position: 0,
            contexts: Contexts::new(context_cap),
            frames: Frames::new(limits.nesting),
            levels: Levels::new(room / PRINT_GROWTH),
            speculative: Speculative::new(limits.speculative_parens, limits.speculative_pairs),
            prev: Tok::Start,
            pending: 0,
            body: Level::Body,
            head: Head::None,
            else_closes: false,
            watch: Watch::Ended,
            mode: Mode::Code,
            goal,
            divergent: false,
            precise,
            restart: false,
            refusal: None,
            class_heads: Vec::new(),
            tracker: Tracker::new(goal),
            regexes: 0,
            templates: 0,
            jsx_elements: 0,
        })
    }

    fn pass(&self) -> Pass {
        Pass {
            result: self.finish(),
            divergent: self.divergent,
            doubted: self.contexts.doubted,
        }
    }

    fn finish(&self) -> DepthScan {
        DepthScan {
            nesting: u32::try_from(self.frames.max_units).unwrap_or(u32::MAX),
            speculative_parens: self.speculative.max,
            speculative_pairs: self.speculative.pairs,
            print_weight: self.levels.weight,
            regexes: self.regexes,
            templates: self.templates,
            jsx_elements: self.jsx_elements,
        }
    }

    fn run(&mut self) -> Result<(), DepthRefusal> {
        loop {
            if self.mode == Mode::Code && !self.precise {
                if self.code_loop()? {
                    break;
                }
                continue;
            }
            let Some((tok, start, crossed)) = self.mode_token() else {
                break;
            };
            if self.restart {
                break;
            }
            if self.precise {
                self.precise_token(tok, start, crossed)?;
            } else {
                self.token(tok, start, crossed)?;
            }
        }
        if let Some(refusal) = self.refusal {
            return Err(refusal);
        }
        if self.restart {
            return Ok(());
        }
        if self.pending & PENDING_SEMI != 0 {
            self.resolve_semi(Tok::Start)?;
        }
        Ok(())
    }

    #[inline(never)]
    fn token(&mut self, tok: Tok, start: usize, newline: bool) -> Result<(), DepthRefusal> {
        let tok = self.clause_name(tok);
        if tok == Tok::Name && !self.contexts.expr_allowed {
            self.typescript_word(start, newline)?;
        }
        if self.pending != 0 && self.resolve(tok, start, newline)? {
            self.contexts.update(tok, self.prev, newline)?;
            self.mode = mode_of(self.contexts.top());
        } else {
            self.fast(tok, newline)?;
        }
        self.prev = tok;
        Ok(())
    }

    #[inline(always)]
    fn plain(&mut self, tok: Tok, newline: bool) -> Result<(), DepthRefusal> {
        let flags = tok.flags();
        if flags & SPECIAL != 0 {
            return self.special(tok, newline);
        }
        if flags & RUN != 0 {
            self.frames.count_run()?;
        }
        if flags & LINE_TOKEN != 0 {
            self.weigh()?;
        }
        self.contexts.expr_allowed = flags & BEFORE_EXPR != 0;
        Ok(())
    }

    #[inline(always)]
    fn weigh(&mut self) -> Result<(), DepthRefusal> {
        self.levels.add(self.levels.depth(&self.frames))
    }

    #[inline(always)]
    fn comma(&mut self) -> Result<(), DepthRefusal> {
        self.contexts.expr_allowed = true;
        self.frames.reset_run();
        self.levels.close_conditional_and_arrow(&mut self.frames)?;
        self.weigh()
    }

    #[inline(always)]
    fn paren_left(&mut self, newline: bool) -> Result<(), DepthRefusal> {
        self.open_paren(newline)?;
        self.contexts.open_paren(self.prev)
    }

    #[inline(always)]
    fn close_with_context(&mut self) -> Result<(), DepthRefusal> {
        self.close()?;
        self.weigh()?;
        self.contexts.expr_allowed = self.contexts.close();
        self.mode = mode_of(self.contexts.top());
        Ok(())
    }

    fn resolve(&mut self, tok: Tok, start: usize, newline: bool) -> Result<bool, DepthRefusal> {
        if self.pending & PENDING_WATCH != 0 {
            self.resolve_watch(tok, start, newline);
        }
        if self.pending & PENDING_OPERAND != 0 {
            self.pending &= !PENDING_OPERAND;
            self.check_operand(tok, start, newline);
        }
        if self.pending & PENDING_SEMI != 0 {
            self.resolve_semi(tok)?;
        }
        let forced = self.pending & PENDING_TAG != 0 && self.resolve_tag(tok)?;
        if self.pending & PENDING_SPECULATIVE != 0 {
            let word = &self.source[start..self.position];
            if !self.speculative.resolve(&mut self.frames, tok, word)? {
                self.pending &= !PENDING_SPECULATIVE;
            }
        }
        if self.pending & (PENDING_BODY | PENDING_ARROW) != 0 {
            self.resolve_body(tok)?;
        }
        if self.pending & PENDING_HEAD != 0 && !matches!(tok, Tok::ParenL | Tok::NameAwait) {
            self.pending &= !PENDING_HEAD;
        }
        Ok(forced)
    }

    fn resolve_semi(&mut self, tok: Tok) -> Result<(), DepthRefusal> {
        self.pending &= !PENDING_SEMI;
        if tok == Tok::KwElse {
            self.levels.close_conditional_and_arrow(&mut self.frames)?;
        } else {
            self.frames.reset_run();
            self.levels.close_all(&mut self.frames)?;
        }
        self.levels.add(self.levels.depth(&self.frames))
    }

    fn resolve_tag(&mut self, tok: Tok) -> Result<bool, DepthRefusal> {
        self.pending &= !PENDING_TAG;
        if tok == Tok::Slash {
            return Ok(true);
        }
        self.jsx_elements = self.jsx_elements.saturating_add(1);
        self.open(Head::None)?;
        Ok(false)
    }

    fn resolve_body(&mut self, tok: Tok) -> Result<(), DepthRefusal> {
        if self.pending & PENDING_BODY != 0 {
            self.pending &= !PENDING_BODY;
            self.contexts.let_body = tok == Tok::NameLet;
            if tok == Tok::KwIf && self.prev == Tok::KwElse {
                if self.else_closes {
                    self.levels.close_if_body(&mut self.frames)?;
                }
            } else if tok != Tok::BraceL {
                self.levels.push(&mut self.frames, self.body);
            }
        }
        if self.pending & PENDING_ARROW != 0 {
            self.pending &= !PENDING_ARROW;
            if tok != Tok::BraceL {
                self.levels.push(&mut self.frames, Level::Arrow);
            }
        }
        Ok(())
    }

    #[inline(never)]
    fn special(&mut self, tok: Tok, newline: bool) -> Result<(), DepthRefusal> {
        if matches!(tok, Tok::Star | Tok::IncDec) && self.prev != Tok::KwFunction {
            self.frames.count_run()?;
            let expr = self.contexts.expr_allowed || newline;
            self.contexts.expr_allowed = tok == Tok::Star || expr;
            self.mode = mode_of(self.contexts.top());
            return Ok(());
        }
        if matches!(tok, Tok::KwIf | Tok::KwFor | Tok::KwWhile | Tok::KwWith) {
            let head = if tok == Tok::KwIf {
                Head::If
            } else {
                Head::Other
            };
            self.head_keyword(head)?;
            self.weigh()?;
            self.contexts.expr_allowed = false;
            self.mode = mode_of(self.contexts.top());
            return Ok(());
        }
        if tok == Tok::Colon {
            self.contexts.conditional_colon = self.frames.top.questions > 0;
        }
        self.structure(tok, newline)?;
        self.levels_after(tok)?;
        if tok.flags() & LINE_TOKEN != 0 && tok != Tok::Semi {
            self.levels.add(self.levels.depth(&self.frames))?;
        }
        if tok == Tok::BackQuote && self.contexts.opens_a_template() {
            self.templates = self.templates.saturating_add(1);
        }
        self.contexts.update(tok, self.prev, newline)?;
        if tok == Tok::KwClass {
            self.note_class_head();
        }
        self.mode = mode_of(self.contexts.top());
        if matches!(tok, Tok::NameAwait | Tok::NameYield) {
            if !self.precise && !words::awaits_an_operand(self.prev) {
                self.restart_precise();
            }
            self.pending |= PENDING_OPERAND;
        }
        Ok(())
    }

    fn check_operand(&mut self, tok: Tok, start: usize, newline: bool) {
        if self.precise {
            return;
        }
        let prev = self.prev;
        if !newline && scope::operand_word(tok) && !(prev == Tok::NameAwait && tok == Tok::NameOf) {
            self.prev = if prev == Tok::NameYield {
                Tok::Eq
            } else {
                Tok::Op
            };
            self.contexts.expr_allowed = true;
            return;
        }
        if reading_depends_on_the_operator(prev, tok, self.source[start]) {
            self.restart_precise();
        }
    }

    fn structure(&mut self, tok: Tok, newline: bool) -> Result<(), DepthRefusal> {
        match tok {
            Tok::ParenL => self.open_paren(newline),
            Tok::BracketL | Tok::BraceL => self.open(self.pattern_head(tok)),
            Tok::DollarBraceL => self.open(Head::None),
            Tok::ParenR | Tok::BracketR | Tok::BraceR => self.close(),
            Tok::Comma => {
                self.frames.reset_run();
                self.levels.close_conditional_and_arrow(&mut self.frames)
            }
            Tok::Semi => {
                self.pending |= PENDING_SEMI;
                Ok(())
            }
            Tok::JsxTagStart => {
                self.pending |= PENDING_TAG;
                Ok(())
            }
            Tok::JsxTagEnd if self.contexts.tag_end_closes_an_element(self.prev) => self.close(),
            Tok::KwBreak | Tok::KwContinue => self.start_watch(Watch::Label),
            Tok::KwDebugger => self.start_watch(Watch::Ended),
            Tok::KwImport | Tok::KwExport => self.start_watch(Watch::Module),
            Tok::NameUsing => {
                self.pending |= self.watch_for(Watch::Using);
                Ok(())
            }
            Tok::KwIf => self.head_keyword(Head::If),
            Tok::KwFor | Tok::KwWhile | Tok::KwWith => self.head_keyword(Head::Other),
            _ if tok.flags() & RUN != 0 => self.frames.count_run(),
            _ => Ok(()),
        }
    }

    fn head_keyword(&mut self, head: Head) -> Result<(), DepthRefusal> {
        self.pending |= PENDING_HEAD;
        self.head = head;
        self.frames.count_run()
    }

    #[inline(always)]
    fn open_paren(&mut self, newline: bool) -> Result<(), DepthRefusal> {
        let head = if self.pending & PENDING_HEAD != 0 {
            self.pending &= !PENDING_HEAD;
            self.head
        } else {
            Head::None
        };
        let candidate = self.contexts.expr_allowed
            || (self.prev == Tok::NameAsync && !newline)
            || (self.prev == Tok::NameBinding && newline);
        self.open(head)?;
        if candidate {
            self.speculative.candidate(self.frames.top_index());
            self.pending |= PENDING_SPECULATIVE;
        }
        Ok(())
    }

    #[inline(always)]
    fn open(&mut self, head: Head) -> Result<(), DepthRefusal> {
        self.frames.open(Frame::fresh(self.levels.len(), head))
    }

    #[inline(always)]
    fn close(&mut self) -> Result<(), DepthRefusal> {
        let Some(frame) = self.frames.close() else {
            return Ok(());
        };
        self.speculative.closed(frame.speculative);
        self.levels
            .close_frame(&self.frames, frame.level_base as usize)?;
        match frame.head {
            Head::None => {}
            Head::If => self.pend_body(Level::IfBody),
            Head::Other => self.pend_body(Level::Body),
            Head::Pattern => self.pending |= self.watch_for(Watch::Ended),
        }
        Ok(())
    }

    pub(super) fn pattern_head(&self, tok: Tok) -> Head {
        let braced = self.pending & PENDING_WATCH != 0 && self.watch == Watch::Braced;
        if braced || self.binds_pattern(self.prev, tok) {
            Head::Pattern
        } else {
            Head::None
        }
    }

    fn watch_for(&mut self, watch: Watch) -> u8 {
        self.watch = watch;
        PENDING_WATCH
    }

    fn start_watch(&mut self, watch: Watch) -> Result<(), DepthRefusal> {
        self.pending |= self.watch_for(watch);
        self.frames.count_run()
    }

    fn resolve_watch(&mut self, tok: Tok, start: usize, newline: bool) {
        let state = self.watch;
        let watch = match state {
            Watch::Closed | Watch::Named => Watch::Module,
            other => other,
        };
        match self.next_watch(watch, tok, start, newline) {
            Some(Watch::Module) if tok == Tok::BraceR => self.watch = Watch::Closed,
            Some(Watch::Module) if state == Watch::Closed && tok.is_name() => {
                self.watch = Watch::Named
            }
            Some(watch) => self.watch = watch,
            None => self.pending &= !PENDING_WATCH,
        }
    }

    fn next_watch(&mut self, watch: Watch, tok: Tok, start: usize, newline: bool) -> Option<Watch> {
        let word = self.source.get(start..self.position).unwrap_or_default();
        match (watch, tok) {
            (Watch::Label, _) if tok.is_name() && !newline => Some(Watch::Ended),
            (Watch::Module | Watch::From, Tok::Str)
                if watch == Watch::From || self.prev == Tok::KwImport =>
            {
                Some(Watch::Specifier)
            }
            (Watch::Specifier, Tok::KwWith) => Some(Watch::Braced),
            (Watch::Specifier, Tok::Name) if word == b"assert" && !newline => Some(Watch::Braced),
            (Watch::Braced, Tok::BraceL) => Some(Watch::Braced),
            (Watch::Using, _) => self.using_watch(tok, newline),
            (Watch::Module | Watch::From | Watch::ImportName, Tok::Name) if word == b"from" => {
                Some(Watch::From)
            }
            (Watch::Module | Watch::From | Watch::ImportName, Tok::Name)
                if word == b"as" && self.prev != Tok::KwImport =>
            {
                Some(Watch::Alias)
            }
            (Watch::Alias, _) if continues_a_clause(tok) => Some(Watch::Module),
            (Watch::Module, _) if tok.is_name() && self.prev == Tok::KwImport => {
                Some(Watch::ImportName)
            }
            (Watch::ImportName, Tok::Eq) => {
                self.import_equals();
                None
            }
            (Watch::Module, Tok::KwDefault) if self.prev == Tok::KwExport => Some(Watch::Default),
            (Watch::Default, _) => {
                self.default_interface(start, newline);
                None
            }
            (Watch::Module | Watch::From | Watch::ImportName, _) if continues_a_clause(tok) => {
                Some(Watch::Module)
            }
            _ => None,
        }
    }

    fn clause_name(&self, tok: Tok) -> Tok {
        if self.pending & PENDING_WATCH == 0
            || !(Tok::KwBreak as u8..=Tok::KwDelete as u8).contains(&(tok as u8))
        {
            return tok;
        }
        let specifier = match self.watch {
            Watch::Alias => true,
            Watch::Module | Watch::From | Watch::ImportName | Watch::Closed | Watch::Named => {
                matches!(self.prev, Tok::BraceL | Tok::Comma)
            }
            _ => false,
        };
        if specifier {
            Tok::Name
        } else {
            tok
        }
    }

    fn using_watch(&mut self, tok: Tok, newline: bool) -> Option<Watch> {
        let head = self.contexts.top() == Ctx::PStat;
        if !scope::using_binds(tok, newline, head) {
            return None;
        }
        if !self.precise {
            self.restart_precise();
        }
        (tok == Tok::BraceL).then_some(Watch::Braced)
    }

    pub(super) fn statement_ended(&self) -> bool {
        self.pending & PENDING_WATCH != 0
            && matches!(self.watch, Watch::Label | Watch::Ended | Watch::Specifier)
    }

    fn restart_precise(&mut self) {
        self.restart = true;
        self.position = self.source.len();
    }

    fn pend_body(&mut self, level: Level) {
        self.pending |= PENDING_BODY;
        self.body = level;
    }

    fn levels_after(&mut self, tok: Tok) -> Result<(), DepthRefusal> {
        match tok {
            Tok::Question => self.levels.push(&mut self.frames, Level::Question),
            Tok::Colon => self.levels.close_to_question(&mut self.frames)?,
            Tok::Arrow => self.pending |= PENDING_ARROW,
            Tok::KwDo => self.pend_body(Level::Body),
            Tok::KwElse => {
                self.pend_body(Level::Body);
                self.else_closes = self.prev != Tok::BraceR;
            }
            _ => {}
        }
        Ok(())
    }
}

fn continues_a_clause(tok: Tok) -> bool {
    tok.is_name()
        || matches!(
            tok,
            Tok::KwDefault | Tok::Star | Tok::BraceL | Tok::BraceR | Tok::Comma | Tok::Str
        )
}

fn reading_depends_on_the_operator(prev: Tok, tok: Tok, first: u8) -> bool {
    match tok {
        Tok::Slash | Tok::Regex | Tok::JsxTagStart | Tok::BraceL => true,
        Tok::KwFunction | Tok::KwClass | Tok::IncDec | Tok::NameOf | Tok::NameAsync => true,
        Tok::NameLet | Tok::At => true,
        Tok::ParenL => prev == Tok::NameYield,
        Tok::Op => matches!(first, b'/' | b'<'),
        _ => false,
    }
}
