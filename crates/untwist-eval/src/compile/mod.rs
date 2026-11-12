mod assign;
mod block;
mod call;
mod declare;
mod defaults;
pub(crate) mod depth;
pub(crate) mod eligible;
pub(crate) mod entry;
mod expr;
pub(crate) mod konst;
mod lexical;
mod literal;
mod loops;
pub(crate) mod program;
mod refuse;
pub(crate) mod scope;
mod stmt;

use std::cell::OnceCell;
use std::rc::Rc;

use oxc_ast::ast::{Expression, Statement};
use rustc_hash::FxHashMap;

use crate::bail::{Bail, EvalResult};
use crate::budget::CALL_STACK_GROWTH;
use crate::frame::{FrameBody, FrameLayout, FrameRef};
use crate::interp::Interp;
use crate::names::Names;
use crate::stack;
use crate::value::{Closure, Completion, Eval, FnCode, Value};

pub(crate) use call::CompiledFn;
pub(crate) use depth::Reach;
use scope::ScopeChain;
pub(crate) use stmt::Stmt;

pub(crate) trait Placeholder {
    fn placeholder() -> Self;
}

pub(crate) struct Guard<T: Placeholder>(T);

impl<T: Placeholder> std::ops::Deref for Guard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Placeholder> Drop for Guard<T> {
    fn drop(&mut self) {
        let inner = std::mem::replace(&mut self.0, T::placeholder());
        stack::maybe_grow(move || drop(inner));
    }
}

pub(crate) trait Evaluate<'a> {
    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Value<'a>>;
}

type RawCode<'a> = Box<dyn Evaluate<'a> + 'a>;

struct Plain<F>(F);

impl<'a, F> Evaluate<'a> for Plain<F>
where
    F: Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Value<'a>>,
{
    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Value<'a>> {
        (self.0)(interp, frame, reach)
    }
}

pub(crate) struct Leveled<F>(pub(crate) F);

impl<'a, F> Evaluate<'a> for Leveled<F>
where
    F: Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Value<'a>>,
{
    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Value<'a>> {
        if reach.checks() && !stack::has_room() {
            return depth::grown_eval(self, interp, frame, reach);
        }
        depth::level(interp, reach, |interp| {
            interp.budget.spend(1)?;
            (self.0)(interp, frame, reach)
        })
    }
}

impl<'a, F> Compiled<'a> for Leveled<F>
where
    F: Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Completion<'a>>,
{
    fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        if reach.checks() && !stack::has_room() {
            return depth::grown_run(self, interp, frame, reach);
        }
        depth::level(interp, reach, |interp| {
            interp.budget.spend(1)?;
            (self.0)(interp, frame, reach)
        })
    }
}

impl Placeholder for RawCode<'_> {
    fn placeholder() -> Self {
        Box::new(Plain(|_: &mut Interp<'_>, _: &FrameRef<'_>, _: Reach| {
            Ok(Value::Undefined)
        }))
    }
}

pub(crate) struct Code<'a>(Guard<RawCode<'a>>);

pub(crate) fn leveled<'a>(
    body: impl Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Value<'a>> + 'a,
) -> Code<'a> {
    Code(Guard(Box::new(Leveled(body))))
}

impl<'a> Code<'a> {
    #[inline(always)]
    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Value<'a>> {
        self.0.eval(interp, frame, reach)
    }
}

pub(crate) fn guard<T: Placeholder>(inner: T) -> Guard<T> {
    Guard(inner)
}

pub(crate) trait Compiled<'a> {
    fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>>;
}

impl<'a, F> Compiled<'a> for F
where
    F: Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Completion<'a>>,
{
    fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        self(interp, frame, reach)
    }
}

#[inline(always)]
pub(crate) fn checked<'a, T>(
    interp: &mut Interp<'a>,
    body: impl FnOnce(&mut Interp<'a>) -> Eval<'a, T>,
) -> Eval<'a, T> {
    interp.budget.enter_nesting()?;
    let result = if stack::has_room() {
        body(interp)
    } else {
        stack::grow(CALL_STACK_GROWTH, || body(interp))
    };
    interp.budget.leave_nesting();
    result
}

pub(crate) struct Compiler<'c, 'a> {
    pub(crate) names: &'c mut Names<'a>,
    pub(crate) functions: &'c mut FnTable<'a>,
    pub(crate) unfoldable: Option<oxc_span::Span>,
    depth: u32,
    deepest: u32,
}

impl Compiler<'_, '_> {
    pub(crate) fn deeper<T>(&mut self, inner: impl FnOnce(&mut Self) -> T) -> T {
        self.depth = self.depth.saturating_add(1);
        self.reach(0);
        let compiled = inner(self);
        self.depth = self.depth.saturating_sub(1);
        compiled
    }

    pub(crate) fn reach(&mut self, below: u32) {
        self.deepest = self.deepest.max(self.depth.saturating_add(below));
    }

    fn levels(&self) -> u32 {
        depth::unit_levels(self.deepest)
    }
}

struct Unit<C> {
    code: C,
    levels: u32,
}

pub(crate) struct Lazy<'a> {
    node: &'a Statement<'a>,
    scope: Rc<ScopeChain>,
    code: OnceCell<Unit<Stmt<'a>>>,
}

impl<'a> Lazy<'a> {
    pub(crate) fn new(node: &'a Statement<'a>, scope: Rc<ScopeChain>) -> Self {
        Self {
            node,
            scope,
            code: OnceCell::new(),
        }
    }

    pub(crate) fn scope(&self) -> &Rc<ScopeChain> {
        &self.scope
    }

    #[inline]
    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        match self.code.get() {
            Some(unit) => unit.run(interp, frame, reach),
            None => self.first_run(interp, frame, reach),
        }
    }

    #[inline(never)]
    fn first_run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        let compiled = stack::maybe_grow(|| {
            #[cfg(feature = "test-hooks")]
            let _compiling = crate::test_hooks::Compiling::enter();
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_compiled_unit(oxc_span::GetSpan::span(self.node));
            let mut cx = interp.compiler();
            let code = stmt::compile_statement(self.node, &self.scope, &mut cx);
            Unit {
                code,
                levels: cx.levels(),
            }
        });
        self.code.get_or_init(|| compiled).run(interp, frame, reach)
    }
}

impl<C> Unit<C> {
    #[inline(always)]
    fn reach(&self, interp: &Interp<'_>, given: Reach) -> Reach {
        let reach = depth::unit_reach(interp, given, self.levels);
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_unit_run(reach.checks());
        reach
    }
}

impl<'a> Unit<Stmt<'a>> {
    #[inline(always)]
    fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        given: Reach,
    ) -> Eval<'a, Completion<'a>> {
        let reach = self.reach(interp, given);
        self.code.run(interp, frame, reach)
    }
}

impl<'a> Unit<Code<'a>> {
    #[inline(always)]
    fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        given: Reach,
    ) -> Eval<'a, Value<'a>> {
        let reach = self.reach(interp, given);
        self.code.run(interp, frame, reach)
    }
}

pub(crate) struct LazyExpr<'a> {
    node: &'a Expression<'a>,
    scope: Rc<ScopeChain>,
    code: OnceCell<Unit<Code<'a>>>,
}

impl<'a> LazyExpr<'a> {
    pub(crate) fn new(node: &'a Expression<'a>, scope: Rc<ScopeChain>) -> Self {
        Self {
            node,
            scope,
            code: OnceCell::new(),
        }
    }

    pub(crate) fn eval(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Value<'a>> {
        if let Some(unit) = self.code.get() {
            return unit.eval(interp, frame, reach);
        }
        #[cfg(feature = "test-hooks")]
        let compiling = crate::test_hooks::Compiling::enter();
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_compiled_unit(oxc_span::GetSpan::span(self.node));
        let mut cx = interp.compiler();
        let code = expr::compile_expression(self.node, &self.scope, &mut cx);
        let compiled = Unit {
            code,
            levels: cx.levels(),
        };
        #[cfg(feature = "test-hooks")]
        drop(compiling);
        self.code
            .get_or_init(|| compiled)
            .eval(interp, frame, reach)
    }
}

struct FnEntry<'a> {
    chain: Option<Rc<ScopeChain>>,
    compiled: Option<Option<Rc<CompiledFn<'a>>>>,
}

#[derive(Default)]
pub(crate) struct FnTable<'a> {
    entries: FxHashMap<(usize, usize), FnEntry<'a>>,
}

const ROOT_KEY: usize = 0;

const UNLAID_FUNCTION: &str = "a function the compiler could not lay out";

fn code_address(code: FnCode<'_>) -> usize {
    match code {
        FnCode::Function(function) => std::ptr::from_ref(function) as usize,
        FnCode::Arrow(arrow) => std::ptr::from_ref(arrow) as usize,
    }
}

fn layout_key(layout: &Rc<FrameLayout>) -> usize {
    Rc::as_ptr(layout) as usize
}

fn env_key(env: &FrameRef<'_>) -> Option<usize> {
    let frame = env.borrow();
    if frame.parent.is_none() {
        return Some(ROOT_KEY);
    }
    match &frame.body {
        FrameBody::Fixed { layout, .. } => Some(layout_key(layout)),
        FrameBody::Dynamic(_) => None,
    }
}

impl<'a> FnTable<'a> {
    pub(crate) fn register(&mut self, code: FnCode<'a>, chain: &Rc<ScopeChain>) {
        if chain.is_root() {
            return;
        }
        let key = (code_address(code), layout_key(&chain.layout));
        self.entries.entry(key).or_insert_with(|| FnEntry {
            chain: Some(chain.clone()),
            compiled: None,
        });
    }
}

impl<'a> Interp<'a> {
    pub(crate) fn compiler(&mut self) -> Compiler<'_, 'a> {
        Compiler {
            names: &mut self.names,
            functions: &mut self.functions,
            unfoldable: None,
            depth: 0,
            deepest: 0,
        }
    }

    #[inline]
    pub(crate) fn compiled(&mut self, closure: &Closure<'a>) -> EvalResult<Rc<CompiledFn<'a>>> {
        self.compiled_entry(closure)
            .ok_or(Bail::Unsupported(&UNLAID_FUNCTION))
    }

    fn compiled_entry(&mut self, closure: &Closure<'a>) -> Option<Rc<CompiledFn<'a>>> {
        #[cfg(feature = "test-hooks")]
        let _compiling = crate::test_hooks::Compiling::enter();
        let key = (code_address(closure.code), env_key(&closure.env)?);
        let chain = match self.functions.entries.get(&key) {
            Some(FnEntry {
                compiled: Some(done),
                ..
            }) => return done.clone(),
            Some(entry) => entry.chain.clone(),
            None if key.1 == ROOT_KEY => None,
            None => return None,
        };
        let compiled = self.compile_function(closure, chain.as_ref()).map(Rc::new);
        if compiled.is_none() && key.1 == ROOT_KEY {
            return None;
        }
        let entry = self.functions.entries.entry(key).or_insert(FnEntry {
            chain,
            compiled: None,
        });
        entry.compiled = Some(compiled.clone());
        compiled
    }
}
