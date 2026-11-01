use super::lexer::Tok;
use super::DepthRefusal;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Ctx {
    BStat,
    BExpr,
    BTmpl,
    PStat,
    PExpr,
    QTmpl,
    FStat,
    FExpr,
    FExprGen,
    FGen,
    JOTag,
    JCTag,
    JExpr,
}

impl Ctx {
    pub(super) fn is_expr(self) -> bool {
        matches!(
            self,
            Ctx::BExpr | Ctx::PExpr | Ctx::QTmpl | Ctx::FExpr | Ctx::FExprGen | Ctx::JExpr
        )
    }

    pub(super) fn is_function(self) -> bool {
        matches!(self, Ctx::FStat | Ctx::FExpr | Ctx::FExprGen | Ctx::FGen)
    }

    fn is_generator(self) -> bool {
        matches!(self, Ctx::FExprGen | Ctx::FGen)
    }
}

const GENERATOR: u8 = 1;
const CLASS: u8 = 2;
const HERITAGE_BLOCK: u8 = 4;
const CLASS_HEAD: u8 = 8;

pub(super) struct Contexts {
    stack: Vec<Ctx>,
    functions: Vec<u8>,
    cap: usize,
    pub(super) expr_allowed: bool,
    pub(super) conditional_colon: bool,
    async_prev: Tok,
    async_newline: bool,
    await_after_for: bool,
    pub(super) let_declares: bool,
    pub(super) let_body: bool,
    pub(super) let_bracket_only: bool,
    cases: Vec<usize>,
    case_colon: bool,
    decorator: Option<Decorator>,
    pub(super) flip: bool,
    pub(super) doubted: bool,
}

impl Contexts {
    #[cold]
    #[inline(never)]
    pub(super) fn doubt(&mut self, reading: bool) -> bool {
        self.doubted = true;
        reading != self.flip
    }

    pub(super) fn new(cap: usize) -> Contexts {
        let mut stack = Vec::with_capacity(cap.min(16));
        stack.push(Ctx::BStat);
        Contexts {
            stack,
            functions: Vec::new(),
            cap,
            expr_allowed: true,
            conditional_colon: false,
            async_prev: Tok::Start,
            async_newline: false,
            await_after_for: false,
            let_declares: false,
            let_body: false,
            let_bracket_only: false,
            cases: Vec::new(),
            case_colon: false,
            decorator: None,
            flip: false,
            doubted: false,
        }
    }

    #[inline(always)]
    pub(super) fn top(&self) -> Ctx {
        self.stack.last().copied().unwrap_or(Ctx::BStat)
    }

    #[inline(always)]
    fn push(&mut self, ctx: Ctx) -> Result<(), DepthRefusal> {
        if self.stack.len() >= self.cap {
            return Err(DepthRefusal::Nesting);
        }
        if ctx.is_function() {
            self.functions
                .push(u8::from(ctx.is_generator()) * GENERATOR);
        }
        self.stack.push(ctx);
        Ok(())
    }

    #[inline(always)]
    fn pop(&mut self) -> Ctx {
        if self.stack.len() <= 1 {
            return Ctx::BStat;
        }
        let ctx = self.stack.pop().unwrap_or(Ctx::BStat);
        if ctx.is_function() {
            self.functions.pop();
        }
        ctx
    }

    fn in_generator(&self) -> bool {
        self.functions
            .last()
            .is_some_and(|flags| flags & GENERATOR != 0)
    }

    pub(super) fn len(&self) -> usize {
        self.stack.len()
    }

    pub(super) fn below_top(&self) -> Option<Ctx> {
        self.stack
            .len()
            .checked_sub(2)
            .and_then(|index| self.stack.get(index).copied())
    }

    pub(super) fn in_class_head(&self) -> bool {
        self.top().is_function()
            && self
                .functions
                .last()
                .is_some_and(|flags| flags & CLASS_HEAD != 0)
    }

    #[inline(always)]
    pub(super) fn in_class_body(&self) -> bool {
        self.top() == Ctx::BStat
            && self.below_top().is_some_and(Ctx::is_function)
            && self
                .functions
                .last()
                .is_some_and(|flags| flags & CLASS != 0)
    }

    pub(super) fn update(
        &mut self,
        tok: Tok,
        prev: Tok,
        newline: bool,
    ) -> Result<(), DepthRefusal> {
        self.expr_allowed = match tok {
            Tok::ParenR | Tok::BraceR => self.close(),
            Tok::BraceL => self.open_brace(prev, newline)?,
            Tok::DollarBraceL => self.push(Ctx::BTmpl).map(|()| true)?,
            Tok::ParenL => self.open_paren(prev).map(|()| true)?,
            Tok::IncDec => self.expr_allowed || newline,
            Tok::KwFunction => self.open_function(prev, newline, false)?,
            Tok::KwClass => self.open_function(prev, newline, true)?,
            Tok::NameAsync => self.async_head(prev, newline),
            Tok::NameAwait => {
                self.await_after_for = prev == Tok::KwFor;
                false
            }
            Tok::KwCase | Tok::KwDefault => {
                if prev != Tok::KwExport {
                    self.open_case();
                }
                true
            }
            Tok::NameLet => {
                self.read_let(prev, newline);
                false
            }
            Tok::Colon => self.colon(),
            Tok::BackQuote => self.back_quote()?,
            Tok::Star => self.star(prev),
            Tok::NameOf => self.of_is_a_keyword(prev, newline),
            Tok::NameYield => self.in_generator(),
            Tok::JsxTagStart => self.tag_start()?,
            Tok::At => self.decorate(prev, newline),
            Tok::JsxTagEnd => self.tag_end(prev),
            Tok::Slash if prev == Tok::JsxTagStart => self.closing_tag()?,
            other => other.before_expr(),
        };
        Ok(())
    }

    #[inline(always)]
    pub(super) fn close(&mut self) -> bool {
        if self.stack.len() == 1 {
            return true;
        }
        let mut out = self.pop();
        if out == Ctx::BStat && self.top().is_function() && !self.heritage_block_closed() {
            out = self.pop();
        }
        !out.is_expr()
    }

    fn heritage_block_closed(&mut self) -> bool {
        let Some(flags) = self.functions.last_mut() else {
            return false;
        };
        let open = *flags & HERITAGE_BLOCK != 0;
        *flags &= !HERITAGE_BLOCK;
        open
    }

    #[inline(always)]
    pub(super) fn open_brace(&mut self, prev: Tok, newline: bool) -> Result<bool, DepthRefusal> {
        self.close_class_head(prev);
        let ctx = match self.top() {
            Ctx::JOTag => Ctx::BExpr,
            Ctx::JExpr => Ctx::BTmpl,
            Ctx::FExpr | Ctx::FStat if self.expr_allowed => self.function_brace(prev),
            _ if self.brace_is_block(prev, newline) => Ctx::BStat,
            _ => Ctx::BExpr,
        };
        self.push(ctx)?;
        Ok(true)
    }

    #[inline(always)]
    fn brace_is_block(&mut self, prev: Tok, newline: bool) -> bool {
        let parent = self.top();
        if matches!(parent, Ctx::FExpr | Ctx::FStat) {
            return true;
        }
        match prev {
            Tok::KwElse | Tok::Start | Tok::ParenR | Tok::Arrow | Tok::BraceR | Tok::KwDo => true,
            Tok::Semi => parent != Ctx::PStat,
            Tok::BraceL => parent == Ctx::BStat,
            Tok::KwVar | Tok::KwConst => false,
            Tok::Colon | Tok::KwReturn => self.brace_after(prev, newline, parent),
            _ if prev.is_name() => self.brace_after(prev, newline, parent),
            _ => !self.expr_allowed,
        }
    }

    #[inline(never)]
    fn function_brace(&mut self, prev: Tok) -> Ctx {
        let Some(flags) = self.functions.last_mut() else {
            return Ctx::BStat;
        };
        if *flags & CLASS == 0 {
            return Ctx::BStat;
        }
        if prev == Tok::Arrow {
            *flags |= HERITAGE_BLOCK;
            return Ctx::BStat;
        }
        Ctx::BExpr
    }

    #[inline(never)]
    fn brace_after(&mut self, prev: Tok, newline: bool, parent: Ctx) -> bool {
        match prev {
            Tok::Colon if matches!(parent, Ctx::BStat | Ctx::BExpr) => {
                !parent.is_expr() && !self.conditional_colon
            }
            Tok::Colon => !self.expr_allowed,
            Tok::KwReturn => newline,
            Tok::NameOf if newline && self.expr_allowed && parent == Ctx::PStat => {
                !self.doubt(true)
            }
            _ if self.expr_allowed => newline,
            Tok::NameLet => !self.let_binds() && newline && parent == Ctx::BStat,
            Tok::NameStatic if self.in_class_body() => true,
            _ => newline && parent == Ctx::BStat,
        }
    }

    #[inline(always)]
    pub(super) fn open_paren(&mut self, prev: Tok) -> Result<(), DepthRefusal> {
        if prev == Tok::KwClass && self.top().is_function() {
            self.unmark_class();
        }
        let statement = matches!(prev, Tok::KwIf | Tok::KwFor | Tok::KwWith | Tok::KwWhile)
            || (prev == Tok::NameAwait && self.awaited_head());
        self.push(if statement { Ctx::PStat } else { Ctx::PExpr })?;
        self.expr_allowed = true;
        Ok(())
    }

    fn open_case(&mut self) {
        let depth = self.stack.len();
        while self.cases.last().is_some_and(|&open| open >= depth) {
            self.cases.pop();
        }
        if self.cases.len() < self.cap {
            self.cases.push(depth);
        }
    }

    #[inline(never)]
    fn close_case(&mut self) -> bool {
        let depth = self.stack.len();
        while self.cases.last().is_some_and(|&open| open > depth) {
            self.cases.pop();
        }
        if self.cases.last() == Some(&depth) {
            self.cases.pop();
            return true;
        }
        false
    }

    fn starts_a_statement_list_item(&self, prev: Tok, newline: bool) -> bool {
        if prev.declares() && (prev != Tok::NameLet || self.let_binds()) {
            return false;
        }
        let top = self.top();
        match prev {
            Tok::Start | Tok::Semi | Tok::BraceR | Tok::KwExport => top == Ctx::BStat,
            Tok::BraceL => top == Ctx::BStat,
            Tok::Colon => top == Ctx::BStat && !self.conditional_colon && self.case_colon,
            Tok::KwReturn => newline && top == Ctx::BStat,
            _ => newline && !self.expr_allowed && top == Ctx::BStat,
        }
    }

    pub(super) fn let_binds(&self) -> bool {
        self.let_declares && !self.let_bracket_only
    }

    fn read_let(&mut self, prev: Tok, newline: bool) {
        let label = prev == Tok::Colon
            && self.top() == Ctx::BStat
            && !self.conditional_colon
            && !self.case_colon;
        let single = std::mem::take(&mut self.let_body) || label;
        self.let_bracket_only = single;
        self.let_declares = single || self.starts_a_statement_list_item(prev, newline);
    }

    fn of_is_a_keyword(&self, prev: Tok, newline: bool) -> bool {
        !self.expr_allowed && (!newline || self.top() == Ctx::PStat) && !self.declares_a_name(prev)
    }

    fn declares_a_name(&self, prev: Tok) -> bool {
        match prev {
            Tok::KwVar | Tok::KwConst => true,
            Tok::NameLet => self.let_binds() || self.top() == Ctx::PStat,
            _ => false,
        }
    }

    fn async_head(&mut self, prev: Tok, newline: bool) -> bool {
        self.async_prev = if prev == Tok::NameOf && self.expr_allowed {
            Tok::Op
        } else {
            prev
        };
        self.async_newline = newline;
        false
    }

    #[cold]
    #[inline(never)]
    fn awaited_head(&mut self) -> bool {
        std::mem::take(&mut self.await_after_for)
    }

    fn decorate(&mut self, prev: Tok, newline: bool) -> bool {
        let depth = self.stack.len();
        let fresh = self.decorator.is_none_or(|open| open.depth != depth);
        if fresh || !ends_a_decorator(prev) {
            self.decorator = Some(Decorator {
                prev,
                newline,
                expr_allowed: self.expr_allowed,
                depth,
            });
        }
        true
    }

    pub(super) fn decorated_at(&self, depth: usize) -> bool {
        self.decorator.is_some_and(|open| open.depth == depth)
    }

    pub(super) fn clear_decorator(&mut self, depth: usize) {
        if self.decorated_at(depth) {
            self.decorator = None;
        }
    }

    #[cold]
    #[inline(never)]
    fn decorated(&mut self, prev: Tok, class: bool) -> Option<bool> {
        let open = self.decorator?;
        if !class || open.depth != self.stack.len() {
            return None;
        }
        self.decorator = None;
        if !ends_a_decorator(prev) {
            return None;
        }
        let expr_allowed = std::mem::replace(&mut self.expr_allowed, open.expr_allowed);
        let expression = self.function_expression(open.prev, open.newline);
        self.expr_allowed = expr_allowed;
        Some(expression)
    }

    #[cold]
    #[inline(never)]
    fn names_a_member(&self, prev: Tok) -> bool {
        let prev = match self.decorator {
            Some(open) if open.depth == self.stack.len() && ends_a_decorator(prev) => open.prev,
            _ => prev,
        };
        let modifier = prev.is_name()
            && !matches!(
                prev,
                Tok::NameAwait | Tok::NameYield | Tok::NameOf | Tok::NameAsync
            );
        modifier
            || matches!(
                prev,
                Tok::BraceL
                    | Tok::Semi
                    | Tok::BraceR
                    | Tok::BracketR
                    | Tok::ParenR
                    | Tok::Str
                    | Tok::Num
                    | Tok::PrivateName
                    | Tok::BackQuote
            )
    }

    pub(super) fn open_function(
        &mut self,
        prev: Tok,
        newline: bool,
        class: bool,
    ) -> Result<bool, DepthRefusal> {
        if self.in_class_body() && self.names_a_member(prev) {
            return Ok(false);
        }
        let decorated = if self.decorator.is_some() {
            self.decorated(prev, class)
        } else {
            None
        };
        let expression = if let Some(expression) = decorated {
            expression
        } else if prev == Tok::NameAsync && !newline {
            self.function_expression(self.async_prev, self.async_newline)
        } else {
            self.function_expression(prev, newline)
        };
        self.push(if expression { Ctx::FExpr } else { Ctx::FStat })?;
        if class {
            self.mark_class();
        }
        Ok(false)
    }

    #[inline(always)]
    fn function_expression(&self, prev: Tok, newline: bool) -> bool {
        let top = self.top();
        let operator = prev.before_expr() || (prev == Tok::NameOf && self.expr_allowed);
        operator
            && !matches!(prev, Tok::KwElse | Tok::KwDefault)
            && !(prev == Tok::Semi && top != Ctx::PStat)
            && !(prev == Tok::KwReturn && newline)
            && !(matches!(prev, Tok::Colon | Tok::BraceL)
                && top == Ctx::BStat
                && self.statement_colon(prev))
    }

    fn statement_colon(&self, prev: Tok) -> bool {
        prev == Tok::BraceL || !self.conditional_colon
    }

    #[cold]
    fn mark_class(&mut self) {
        if let Some(flags) = self.functions.last_mut() {
            *flags |= CLASS | CLASS_HEAD;
        }
    }

    #[cold]
    #[inline(never)]
    fn unmark_class(&mut self) {
        if let Some(flags) = self.functions.last_mut() {
            *flags &= !(CLASS | CLASS_HEAD);
        }
    }

    fn close_class_head(&mut self, prev: Tok) {
        if prev == Tok::Arrow || !self.top().is_function() {
            return;
        }
        if let Some(flags) = self.functions.last_mut() {
            *flags &= !CLASS_HEAD;
        }
    }

    pub(super) fn colon(&mut self) -> bool {
        if self.conditional_colon {
            return true;
        }
        self.case_colon = !self.cases.is_empty() && self.close_case();
        if self.top().is_function() {
            self.pop();
        }
        true
    }

    fn back_quote(&mut self) -> Result<bool, DepthRefusal> {
        if self.top() == Ctx::QTmpl {
            self.pop();
        } else {
            self.push(Ctx::QTmpl)?;
        }
        Ok(false)
    }

    pub(super) fn opens_a_template(&self) -> bool {
        self.top() != Ctx::QTmpl
    }

    fn star(&mut self, prev: Tok) -> bool {
        if prev == Tok::KwFunction {
            if let Some(top) = self.stack.last_mut() {
                *top = if *top == Ctx::FExpr {
                    Ctx::FExprGen
                } else {
                    Ctx::FGen
                };
            }
            if let Some(flags) = self.functions.last_mut() {
                *flags |= GENERATOR;
            }
        }
        true
    }

    fn tag_start(&mut self) -> Result<bool, DepthRefusal> {
        self.push(Ctx::JExpr)?;
        self.push(Ctx::JOTag)?;
        Ok(false)
    }

    pub(super) fn tag_end_closes_an_element(&self, prev: Tok) -> bool {
        let top = self.top();
        (top == Ctx::JOTag && prev == Tok::Slash) || top == Ctx::JCTag
    }

    fn tag_end(&mut self, prev: Tok) -> bool {
        let out = self.pop();
        if (out == Ctx::JOTag && prev == Tok::Slash) || out == Ctx::JCTag {
            self.pop();
            self.top() == Ctx::JExpr
        } else {
            true
        }
    }

    fn closing_tag(&mut self) -> Result<bool, DepthRefusal> {
        self.pop();
        self.pop();
        self.push(Ctx::JCTag)?;
        Ok(false)
    }
}

fn ends_a_decorator(prev: Tok) -> bool {
    prev.is_name()
        || matches!(
            prev,
            Tok::ParenR
                | Tok::BracketR
                | Tok::PrivateName
                | Tok::BackQuote
                | Tok::KwExport
                | Tok::KwDefault
        )
}

#[derive(Clone, Copy)]
struct Decorator {
    prev: Tok,
    newline: bool,
    expr_allowed: bool,
    depth: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Head {
    None,
    If,
    Other,
    Pattern,
}

#[derive(Clone, Copy)]
pub(super) struct Frame {
    run_start: u64,
    pub(super) level_base: u32,
    pub(super) questions: u32,
    pub(super) head: Head,
    pub(super) speculative: bool,
}

impl Frame {
    #[inline(always)]
    pub(super) fn fresh(level_base: usize, head: Head) -> Frame {
        Frame {
            run_start: 0,
            level_base: u32::try_from(level_base).unwrap_or(u32::MAX),
            questions: 0,
            head,
            speculative: false,
        }
    }
}

pub(super) struct Frames {
    parents: Vec<Frame>,
    pub(super) top: Frame,
    cap: usize,
    units: u64,
    limit: u64,
    pub(super) max_units: u64,
}

impl Frames {
    pub(super) fn new(limit: u32) -> Frames {
        let cap = usize::try_from(limit).unwrap_or(usize::MAX);
        Frames {
            parents: Vec::with_capacity(cap.min(16)),
            top: Frame::fresh(0, Head::None),
            cap,
            units: 0,
            limit: u64::from(limit),
            max_units: 0,
        }
    }

    #[inline(always)]
    pub(super) fn open_count(&self) -> u64 {
        self.parents.len() as u64
    }

    #[inline(always)]
    pub(super) fn top_index(&self) -> usize {
        self.parents.len()
    }

    pub(super) fn get(&mut self, index: usize) -> Option<&mut Frame> {
        if index == self.parents.len() {
            Some(&mut self.top)
        } else {
            self.parents.get_mut(index)
        }
    }

    #[inline(always)]
    fn record(&mut self) -> Result<(), DepthRefusal> {
        if self.units > self.max_units {
            self.max_units = self.units;
            if self.units > self.limit {
                return Err(DepthRefusal::Nesting);
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub(super) fn count_run(&mut self) -> Result<(), DepthRefusal> {
        self.units += 1;
        self.record()
    }

    #[inline(always)]
    pub(super) fn reset_run(&mut self) {
        self.units = self.top.run_start;
    }

    #[inline(always)]
    pub(super) fn open(&mut self, mut frame: Frame) -> Result<(), DepthRefusal> {
        self.units += 2;
        if self.units > self.limit || self.parents.len() >= self.cap {
            self.max_units = self.max_units.max(self.units);
            return Err(DepthRefusal::Nesting);
        }
        if self.parents.len() == self.parents.capacity() {
            self.grow();
        }
        frame.run_start = self.units;
        self.parents.push(std::mem::replace(&mut self.top, frame));
        self.record()
    }

    #[cold]
    fn grow(&mut self) {
        let room = self.cap - self.parents.len();
        self.parents
            .reserve_exact(self.parents.len().max(16).min(room));
    }

    #[inline(always)]
    pub(super) fn close(&mut self) -> Option<Frame> {
        let parent = self.parents.pop()?;
        let frame = std::mem::replace(&mut self.top, parent);
        self.units = frame.run_start.saturating_sub(1);
        Some(frame)
    }
}
