use std::rc::Rc;

use oxc_ast::ast::{BindingPattern, CatchClause, Function, Statement, SwitchStatement};

use crate::bail::Bail;
use crate::compile::expr::compile_expression;
use crate::compile::lexical::{lexical_layout, pending, progressive, scan};
use crate::compile::scope::{LayoutBuilder, ScopeChain};
use crate::compile::stmt::{lazies, run_lazies};
use crate::compile::{Code, Compiler, Lazy, LazyExpr, Reach};
use crate::env::hoist_function_declarations;
use crate::frame::{Frame, FrameLayout, FrameRef, SlotKind};
use crate::interp::Interp;
use crate::ops::strict_equals;
use crate::semantics::statements::{block_declares_bindings, declares_function, ANNEX_B_FUNCTION};
use crate::value::{Abrupt, Completion, Eval, FnCode, Value};

const DESTRUCTURING_CATCH: &str = "destructuring catch parameter";

struct BlockFrame<'a> {
    layout: Rc<FrameLayout>,
    functions: Box<[(u32, &'a Function<'a>)]>,
    annex_b: bool,
}

pub(crate) struct BlockBody<'a> {
    frame: Option<BlockFrame<'a>>,
    statements: Box<[Lazy<'a>]>,
}

pub(crate) fn compile_block<'a>(
    statements: &'a [Statement<'a>],
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<BlockBody<'a>> {
    if !block_declares_bindings(statements) {
        return Some(BlockBody {
            frame: None,
            statements: lazies(statements, scope),
        });
    }
    let mut builder = LayoutBuilder::new();
    let mut functions = Vec::new();
    for (text, function) in hoist_function_declarations(statements) {
        functions.push((builder.slot_for(cx.names.intern(text))?, function));
    }
    let declared = scan(cx.names, &mut builder, statements)?;
    let layout = builder.finish()?;
    let head = ScopeChain::new(
        layout.clone(),
        pending(&layout, &declared)?,
        Some(scope.clone()),
    );
    for (_, function) in &functions {
        cx.functions.register(FnCode::Function(function), &head);
    }
    Some(BlockBody {
        frame: Some(BlockFrame {
            layout,
            functions: functions.into_boxed_slice(),
            annex_b: declares_function(statements),
        }),
        statements: progressive(statements, &head, &declared),
    })
}

impl<'a> BlockBody<'a> {
    pub(crate) fn into_plain(self) -> Result<Box<[Lazy<'a>]>, BlockBody<'a>> {
        match self.frame {
            None => Ok(self.statements),
            Some(_) => Err(self),
        }
    }

    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        let Some(block) = &self.frame else {
            return run_lazies(interp, (frame, reach), &self.statements);
        };
        if block.annex_b && !interp.strict {
            return Err(Bail::Unsupported(&ANNEX_B_FUNCTION).into());
        }
        let inner = interp.realm.new_fixed_env(&block.layout, frame);
        for (slot, function) in block.functions.iter() {
            let closure = interp.make_closure(FnCode::Function(function), &inner);
            Frame::declare_slot(&inner, *slot, closure);
        }
        run_lazies(interp, (&inner, reach), &self.statements)
    }
}

enum CatchParam {
    Absent,
    Slot(u32),
    Destructuring,
}

struct Handler<'a> {
    layout: Rc<FrameLayout>,
    param: CatchParam,
    nested_function: bool,
    body: Box<[Lazy<'a>]>,
}

pub(crate) struct Try<'a> {
    block: BlockBody<'a>,
    handler: Option<Handler<'a>>,
    finalizer: Option<BlockBody<'a>>,
}

fn compile_handler<'a>(
    handler: &'a CatchClause<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Handler<'a>> {
    let mut slots = Vec::new();
    let param = match handler.param.as_ref().map(|param| &param.pattern) {
        None => CatchParam::Absent,
        Some(BindingPattern::BindingIdentifier(id)) => {
            slots.push((cx.names.intern(id.name.as_str()), SlotKind::Mutable));
            CatchParam::Slot(0)
        }
        Some(_) => CatchParam::Destructuring,
    };
    let statements = &handler.body.body;
    let (layout, declared) = lexical_layout(cx.names, slots, statements)?;
    let head = ScopeChain::new(
        layout.clone(),
        pending(&layout, &declared)?,
        Some(scope.clone()),
    );
    Some(Handler {
        layout,
        param,
        nested_function: declares_function(statements),
        body: progressive(statements, &head, &declared),
    })
}

pub(crate) fn compile_try<'a>(
    node: &'a oxc_ast::ast::TryStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Try<'a>> {
    let handler = match &node.handler {
        Some(handler) => Some(compile_handler(handler, scope, cx)?),
        None => None,
    };
    let finalizer = match &node.finalizer {
        Some(finalizer) => Some(compile_block(&finalizer.body, scope, cx)?),
        None => None,
    };
    Some(Try {
        block: compile_block(&node.block.body, scope, cx)?,
        handler,
        finalizer,
    })
}

impl<'a> Try<'a> {
    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        let attempted = self.block.run(interp, frame, reach);
        let after_catch = match attempted {
            Err(abrupt) => match abrupt.into_thrown() {
                Ok(thrown) => self.run_handler(interp, (frame, reach), thrown),
                Err(bail) => Err(bail),
            },
            other => other,
        };
        let Some(finalizer) = &self.finalizer else {
            return after_catch;
        };
        if matches!(&after_catch, Err(abrupt) if abrupt.is_bail()) {
            return after_catch;
        }
        let finally_completion = finalizer.run(interp, frame, reach)?;
        if !matches!(finally_completion, Completion::Normal) {
            return Ok(finally_completion);
        }
        after_catch
    }

    fn run_handler(
        &self,
        interp: &mut Interp<'a>,
        (frame, reach): (&FrameRef<'a>, Reach),
        thrown: Value<'a>,
    ) -> Eval<'a, Completion<'a>> {
        let Some(handler) = &self.handler else {
            return Err(Abrupt::throw(thrown));
        };
        if handler.nested_function {
            return Err(Bail::Unsupported(&ANNEX_B_FUNCTION).into());
        }
        let scope = interp.realm.new_fixed_env(&handler.layout, frame);
        match handler.param {
            CatchParam::Absent => {}
            CatchParam::Slot(slot) => Frame::declare_slot(&scope, slot, thrown),
            CatchParam::Destructuring => return Err(Bail::Unsupported(&DESTRUCTURING_CATCH).into()),
        }
        run_lazies(interp, (&scope, reach), &handler.body)
    }
}

struct Case<'a> {
    test: Option<LazyExpr<'a>>,
    body: Box<[Lazy<'a>]>,
}

pub(crate) struct Switch<'a> {
    nested_function: bool,
    discriminant: Code<'a>,
    layout: Rc<FrameLayout>,
    cases: Box<[Case<'a>]>,
    default: Option<usize>,
}

pub(crate) fn compile_switch<'a>(
    node: &'a SwitchStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Switch<'a>> {
    let mut builder = LayoutBuilder::new();
    let mut declared = Vec::new();
    for case in &node.cases {
        declared.extend(scan(cx.names, &mut builder, &case.consequent)?);
    }
    let layout = builder.finish()?;
    let inner = ScopeChain::new(
        layout.clone(),
        pending(&layout, &declared)?,
        Some(scope.clone()),
    );
    let cases = node
        .cases
        .iter()
        .map(|case| Case {
            test: case
                .test
                .as_ref()
                .map(|test| LazyExpr::new(test, inner.clone())),
            body: lazies(&case.consequent, &inner),
        })
        .collect();
    Some(Switch {
        nested_function: node
            .cases
            .iter()
            .any(|case| declares_function(&case.consequent)),
        discriminant: compile_expression(&node.discriminant, scope, cx),
        layout,
        cases,
        default: node.cases.iter().position(|case| case.test.is_none()),
    })
}

impl<'a> Switch<'a> {
    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Completion<'a>> {
        if self.nested_function {
            return Err(Bail::Unsupported(&ANNEX_B_FUNCTION).into());
        }
        let discriminant = self.discriminant.run(interp, frame, reach)?;
        let scope = interp.realm.new_fixed_env(&self.layout, frame);
        let mut start = None;
        for (index, case) in self.cases.iter().enumerate() {
            let Some(test) = &case.test else {
                continue;
            };
            if strict_equals(&discriminant, &test.eval(interp, &scope, reach)?) {
                start = Some(index);
                break;
            }
        }
        let Some(start) = start.or(self.default) else {
            return Ok(Completion::Normal);
        };
        for case in self.cases.iter().skip(start) {
            match run_lazies(interp, (&scope, reach), &case.body)? {
                Completion::Normal => {}
                Completion::Break(None) => return Ok(Completion::Normal),
                other => return Ok(other),
            }
        }
        Ok(Completion::Normal)
    }
}
