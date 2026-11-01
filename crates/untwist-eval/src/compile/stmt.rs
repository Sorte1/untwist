use std::rc::Rc;

use oxc_ast::ast::{IfStatement, LabeledStatement, Statement};
use oxc_span::GetSpan;

use crate::bail::Bail;
use crate::compile::block::{compile_block, compile_switch, compile_try, BlockBody};
use crate::compile::declare::compile_declaration;
use crate::compile::expr::compile_expression;
use crate::compile::loops::{compile_loop, Looped};
use crate::compile::scope::ScopeChain;
use crate::compile::Placeholder;
use crate::compile::{guard, refuse, Compiled, Compiler, Guard, Lazy, Leveled, Reach};
use crate::frame::FrameRef;
use crate::interp::Interp;
use crate::ops::to_boolean;
use crate::semantics::statements::{statement_label, ANNEX_B_FUNCTION};
use crate::value::{Abrupt, Completion, Eval, Value};

pub(crate) type RawStmt<'a> = Box<dyn Compiled<'a> + 'a>;

impl<'a> Placeholder for RawStmt<'a> {
    fn placeholder() -> Self {
        Box::new(
            |_: &mut Interp<'a>, _: &FrameRef<'a>, _: Reach| -> Eval<'a, Completion<'a>> {
                Ok(Completion::Normal)
            },
        )
    }
}

pub(crate) type Stmt<'a> = Guard<RawStmt<'a>>;

pub(crate) fn statement<'a>(
    body: impl Fn(&mut Interp<'a>, &FrameRef<'a>, Reach) -> Eval<'a, Completion<'a>> + 'a,
) -> Stmt<'a> {
    guard(Box::new(Leveled(body)))
}

pub(crate) fn lazies<'a>(
    statements: &'a [Statement<'a>],
    scope: &Rc<ScopeChain>,
) -> Box<[Lazy<'a>]> {
    statements
        .iter()
        .map(|node| Lazy::new(node, scope.clone()))
        .collect()
}

pub(crate) fn run_lazies<'a>(
    interp: &mut Interp<'a>,
    (frame, reach): (&FrameRef<'a>, Reach),
    statements: &[Lazy<'a>],
) -> Eval<'a, Completion<'a>> {
    for statement in statements {
        let completion = statement.run(interp, frame, reach)?;
        if !matches!(completion, Completion::Normal) {
            return Ok(completion);
        }
    }
    Ok(Completion::Normal)
}

pub(crate) fn compile_statement<'a>(
    node: &'a Statement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Stmt<'a> {
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_compiled_statement(node.span());
    cx.deeper(|cx| compile_simple(node, scope, cx))
}

fn compile_simple<'a>(
    node: &'a Statement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Stmt<'a> {
    match node {
        Statement::ExpressionStatement(node) => {
            let code = compile_expression(&node.expression, scope, cx);
            statement(move |interp, frame, reach| {
                code.run(interp, frame, reach)?;
                Ok(Completion::Normal)
            })
        }
        Statement::EmptyStatement(_)
        | Statement::FunctionDeclaration(_)
        | Statement::DebuggerStatement(_) => statement(|_, _, _| Ok(Completion::Normal)),
        Statement::ReturnStatement(node) => match &node.argument {
            Some(argument) => {
                let code = compile_expression(argument, scope, cx);
                statement(move |interp, frame, reach| {
                    Ok(Completion::Return(code.run(interp, frame, reach)?))
                })
            }
            None => statement(|_, _, _| Ok(Completion::Return(Value::Undefined))),
        },
        Statement::VariableDeclaration(node) => {
            let declaration = compile_declaration(node, scope, cx);
            statement(move |interp, frame, reach| {
                declaration.run(interp, frame, reach)?;
                Ok(Completion::Normal)
            })
        }
        Statement::BlockStatement(node) => {
            match compile_block(&node.body, scope, cx).map(BlockBody::into_plain) {
                Some(Ok(body)) => {
                    statement(move |interp, frame, reach| run_lazies(interp, (frame, reach), &body))
                }
                Some(Err(body)) => {
                    statement(move |interp, frame, reach| body.run(interp, frame, reach))
                }
                None => refuse::statement(),
            }
        }
        _ => compile_control(node, scope, cx),
    }
}

fn compile_control<'a>(
    node: &'a Statement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Stmt<'a> {
    match node {
        Statement::IfStatement(node) => compile_if(node, scope, cx),
        Statement::LabeledStatement(node) => compile_labeled(node, scope, cx),
        Statement::TryStatement(node) => match compile_try(node, scope, cx) {
            Some(parts) => statement(move |interp, frame, reach| parts.run(interp, frame, reach)),
            None => refuse::statement(),
        },
        Statement::SwitchStatement(node) => match compile_switch(node, scope, cx) {
            Some(parts) => statement(move |interp, frame, reach| parts.run(interp, frame, reach)),
            None => refuse::statement(),
        },
        Statement::ThrowStatement(node) => {
            let code = compile_expression(&node.argument, scope, cx);
            statement(move |interp, frame, reach| {
                Err(Abrupt::throw(code.run(interp, frame, reach)?))
            })
        }
        Statement::BreakStatement(node) => {
            let label = statement_label(&node.label);
            statement(move |_, _, _| Ok(Completion::Break(label)))
        }
        Statement::ContinueStatement(node) => {
            let label = statement_label(&node.label);
            statement(move |_, _, _| Ok(Completion::Continue(label)))
        }
        other => match compile_loop(other, scope, cx) {
            Some(looped) => {
                statement(move |interp, frame, reach| looped.run(interp, frame, (None, reach)))
            }
            None => refuse::statement(),
        },
    }
}

fn compile_if<'a>(
    node: &'a IfStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Stmt<'a> {
    let nested_function = [Some(&node.consequent), node.alternate.as_ref()]
        .into_iter()
        .flatten()
        .any(|arm| matches!(arm, Statement::FunctionDeclaration(_)));
    let test = compile_expression(&node.test, scope, cx);
    let span = node.consequent.span();
    let consequent = Lazy::new(&node.consequent, scope.clone());
    let alternate = node
        .alternate
        .as_ref()
        .map(|alternate| Lazy::new(alternate, scope.clone()));
    statement(move |interp, frame, reach| {
        if nested_function {
            return Err(Bail::Unsupported(&ANNEX_B_FUNCTION).into());
        }
        let held = test.run(interp, frame, reach)?;
        if to_boolean(&held) {
            let entered = crate::state::enter_gate(span);
            let result = consequent.run(interp, frame, reach);
            crate::state::leave_gate(entered, matches!(result, Ok(Completion::Normal)));
            return result;
        }
        match &alternate {
            Some(alternate) => alternate.run(interp, frame, reach),
            None => Ok(Completion::Normal),
        }
    })
}

enum LabeledBody<'a> {
    Loop(Looped<'a>),
    Other(Lazy<'a>),
}

fn compile_labeled<'a>(
    node: &'a LabeledStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Stmt<'a> {
    let nested_function = matches!(node.body, Statement::FunctionDeclaration(_));
    let label = Some(node.label.name.as_str());
    let body = match compile_loop(&node.body, scope, cx) {
        Some(looped) => LabeledBody::Loop(looped),
        None => LabeledBody::Other(Lazy::new(&node.body, scope.clone())),
    };
    statement(move |interp, frame, reach| {
        if nested_function {
            return Err(Bail::Unsupported(&ANNEX_B_FUNCTION).into());
        }
        let completion = match &body {
            LabeledBody::Loop(looped) => looped.run(interp, frame, (label, reach))?,
            LabeledBody::Other(other) => other.run(interp, frame, reach)?,
        };
        match completion {
            Completion::Break(Some(found)) if Some(found) == label => Ok(Completion::Normal),
            other => Ok(other),
        }
    })
}
