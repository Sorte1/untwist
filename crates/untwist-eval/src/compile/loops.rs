use std::rc::Rc;

use oxc_ast::ast::{
    BindingPattern, DoWhileStatement, Expression, ForStatement, ForStatementInit, ForStatementLeft,
    Statement, VariableDeclarationKind, WhileStatement,
};

use crate::bail::Bail;
use crate::budget::loop_cap;
use crate::compile::assign::{Place, Target};
use crate::compile::declare::{compile_declaration, Declaration};
use crate::compile::expr::compile_expression;
use crate::compile::lexical::{is_lexical, none_proven};
use crate::compile::scope::{LayoutBuilder, Provable, ScopeChain};
use crate::compile::{refuse, Code, Compiler, Lazy, Reach};
use crate::frame::{Frame, FrameLayout, FrameRef};
use crate::interp::Interp;
use crate::names::Name;
use crate::object;
use crate::ops::to_boolean;
use crate::value::{Completion, Eval, Value};

type Label<'a> = Option<&'a str>;
type Labelled<'a> = (Label<'a>, Reach);

enum LoopFlow<'a> {
    Continue,
    Break,
    Complete(Completion<'a>),
}

#[inline(always)]
fn loop_body<'a>(
    interp: &mut Interp<'a>,
    (frame, reach): (&FrameRef<'a>, Reach),
    body: &Lazy<'a>,
) -> Eval<'a, Completion<'a>> {
    interp.check_heap()?;
    body.run(interp, frame, reach)
}

#[cold]
#[inline(never)]
fn labelled<'a>(completion: Completion<'a>, label: Label<'a>) -> LoopFlow<'a> {
    match completion {
        Completion::Continue(Some(found)) if Some(found) == label => LoopFlow::Continue,
        Completion::Break(Some(found)) if Some(found) == label => LoopFlow::Break,
        other => LoopFlow::Complete(other),
    }
}

macro_rules! flow {
    ($completion:expr, $label:expr) => {
        match $completion {
            Completion::Normal | Completion::Continue(None) => {}
            Completion::Break(None) => return Ok(Completion::Normal),
            other => match labelled(other, $label) {
                LoopFlow::Continue => {}
                LoopFlow::Break => return Ok(Completion::Normal),
                LoopFlow::Complete(completion) => return Ok(completion),
            },
        }
    };
}

pub(crate) enum Looped<'a> {
    While(Code<'a>, Lazy<'a>),
    DoWhile(Code<'a>, Lazy<'a>),
    For(ForLoop<'a>),
    Each(ForEach<'a>),
}

pub(crate) fn compile_loop<'a>(
    node: &'a Statement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Looped<'a>> {
    match node {
        Statement::WhileStatement(node) => Some(compile_while(node, scope, cx)),
        Statement::DoWhileStatement(node) => Some(compile_do_while(node, scope, cx)),
        Statement::ForStatement(node) => compile_for(node, scope, cx),
        Statement::ForInStatement(node) => compile_each(
            (&node.left, &node.right, &node.body),
            (false, false),
            scope,
            cx,
        ),
        Statement::ForOfStatement(node) => {
            let each = (&node.left, &node.right, &node.body);
            compile_each(each, (true, node.r#await), scope, cx)
        }
        _ => None,
    }
}

fn compile_while<'a>(
    node: &'a WhileStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Looped<'a> {
    let test = compile_expression(&node.test, scope, cx);
    Looped::While(test, Lazy::new(&node.body, scope.clone()))
}

fn compile_do_while<'a>(
    node: &'a DoWhileStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Looped<'a> {
    let test = compile_expression(&node.test, scope, cx);
    Looped::DoWhile(test, Lazy::new(&node.body, scope.clone()))
}

impl<'a> Looped<'a> {
    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        labelled: Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        match self {
            Looped::While(test, body) => run_while(interp, frame, (test, body), labelled),
            Looped::DoWhile(test, body) => run_do_while(interp, frame, (test, body), labelled),
            Looped::For(parts) => parts.run(interp, frame, labelled),
            Looped::Each(parts) => parts.run(interp, frame, labelled),
        }
    }
}

fn run_while<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    (test, body): (&Code<'a>, &Lazy<'a>),
    (label, reach): Labelled<'a>,
) -> Eval<'a, Completion<'a>> {
    for _ in 0..loop_cap(interp.loop_len_hint) {
        let held = test.run(interp, frame, reach)?;
        if !to_boolean(&held) {
            return Ok(Completion::Normal);
        }
        flow!(loop_body(interp, (frame, reach), body)?, label);
    }
    Err(Bail::LoopCap.into())
}

fn run_do_while<'a>(
    interp: &mut Interp<'a>,
    frame: &FrameRef<'a>,
    (test, body): (&Code<'a>, &Lazy<'a>),
    (label, reach): Labelled<'a>,
) -> Eval<'a, Completion<'a>> {
    for _ in 0..loop_cap(interp.loop_len_hint) {
        flow!(loop_body(interp, (frame, reach), body)?, label);
        let held = test.run(interp, frame, reach)?;
        if !to_boolean(&held) {
            return Ok(Completion::Normal);
        }
    }
    Err(Bail::LoopCap.into())
}

enum ForInit<'a> {
    Absent,
    Declaration(Declaration<'a>),
    Expression(Code<'a>),
}

pub(crate) struct ForLoop<'a> {
    layout: Rc<FrameLayout>,
    init: ForInit<'a>,
    per_iteration: Box<[(u32, Name)]>,
    test: Option<Code<'a>>,
    update: Option<Code<'a>>,
    body: Lazy<'a>,
}

struct Head {
    layout: Rc<FrameLayout>,
    per_iteration: Box<[(u32, Name)]>,
}

fn for_head<'a>(node: &'a ForStatement<'a>, cx: &mut Compiler<'_, 'a>) -> Option<Head> {
    let mut builder = LayoutBuilder::new();
    let mut per_iteration = Vec::new();
    if let Some(ForStatementInit::VariableDeclaration(declaration)) = &node.init {
        if is_lexical(declaration.kind) {
            for declarator in &declaration.declarations {
                let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                    continue;
                };
                let name = cx.names.intern(id.name.as_str());
                let slot = builder.slot_for(name)?;
                if declaration.kind == VariableDeclarationKind::Let {
                    per_iteration.push((slot, name));
                }
            }
        }
    }
    Some(Head {
        layout: builder.finish()?,
        per_iteration: per_iteration.into_boxed_slice(),
    })
}

fn compile_for<'a>(
    node: &'a ForStatement<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Looped<'a>> {
    let head = for_head(node, cx)?;
    let lexical = !head.layout.is_empty();
    let declaring = ScopeChain::new(
        head.layout.clone(),
        none_proven(&head.layout),
        Some(scope.clone()),
    );
    let init = match &node.init {
        None => ForInit::Absent,
        Some(ForStatementInit::VariableDeclaration(declaration)) => {
            ForInit::Declaration(compile_declaration(declaration, &declaring, cx))
        }
        Some(other) => {
            ForInit::Expression(compile_expression(other.as_expression()?, &declaring, cx))
        }
    };
    let inner = match lexical {
        true => declaring.sibling(Provable::All),
        false => declaring,
    };
    Some(Looped::For(ForLoop {
        layout: head.layout,
        init,
        per_iteration: head.per_iteration,
        test: node
            .test
            .as_ref()
            .map(|test| compile_expression(test, &inner, cx)),
        update: node
            .update
            .as_ref()
            .map(|update| compile_expression(update, &inner, cx)),
        body: Lazy::new(&node.body, inner),
    }))
}

impl<'a> ForLoop<'a> {
    fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        labelled: Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        let scope = interp.realm.new_fixed_env(&self.layout, frame);
        match &self.init {
            ForInit::Absent => {}
            ForInit::Declaration(declaration) => declaration.run(interp, &scope, labelled.1)?,
            ForInit::Expression(code) => {
                code.run(interp, &scope, labelled.1)?;
            }
        }
        if self.per_iteration.is_empty() {
            return self.run_shared(interp, &scope, labelled);
        }
        self.run_per_iteration(interp, (&scope, frame), labelled)
    }

    fn run_shared(
        &self,
        interp: &mut Interp<'a>,
        scope: &FrameRef<'a>,
        (label, reach): Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        for _ in 0..loop_cap(interp.loop_len_hint) {
            if let Some(test) = &self.test {
                if !to_boolean(&test.run(interp, scope, reach)?) {
                    return Ok(Completion::Normal);
                }
            }
            flow!(loop_body(interp, (scope, reach), &self.body)?, label);
            if let Some(update) = &self.update {
                update.run(interp, scope, reach)?;
            }
        }
        Err(Bail::LoopCap.into())
    }

    fn run_per_iteration(
        &self,
        interp: &mut Interp<'a>,
        (scope, outer): (&FrameRef<'a>, &FrameRef<'a>),
        (label, reach): Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        let mut iteration = self.next_iteration(interp, scope, outer)?;
        for _ in 0..loop_cap(interp.loop_len_hint) {
            if let Some(test) = &self.test {
                if !to_boolean(&test.run(interp, &iteration, reach)?) {
                    return Ok(Completion::Normal);
                }
            }
            flow!(loop_body(interp, (&iteration, reach), &self.body)?, label);
            iteration = self.next_iteration(interp, &iteration, outer)?;
            if let Some(update) = &self.update {
                update.run(interp, &iteration, reach)?;
            }
        }
        Err(Bail::LoopCap.into())
    }

    fn next_iteration(
        &self,
        interp: &mut Interp<'a>,
        current: &FrameRef<'a>,
        outer: &FrameRef<'a>,
    ) -> Eval<'a, FrameRef<'a>> {
        let next = interp.realm.new_fixed_env(&self.layout, outer);
        for (slot, name) in self.per_iteration.iter() {
            let value = Frame::slot_by_name(current, *name)
                .map_err(Bail::from)?
                .unwrap_or(Value::Undefined);
            Frame::declare_slot(&next, *slot, value);
        }
        interp.realm.charge_wide(&next);
        Ok(next)
    }
}

enum Left<'a> {
    Declared(u32),
    Identifier(Name, &'a str),
    Member(Target<'a>),
    Refuse(&'static &'static str),
}

pub(crate) struct ForEach<'a> {
    of: bool,
    awaits: bool,
    right: Code<'a>,
    left: Left<'a>,
    layout: Rc<FrameLayout>,
    lexical: bool,
    body: Lazy<'a>,
}

const FOR_IN_BINDING: &str = "for-in binding";
const DESTRUCTURING_FOR_IN: &str = "destructuring for-in binding";
const FOR_IN_TARGET: &str = "for-in target";

fn declared_left<'a>(
    left: &'a ForStatementLeft<'a>,
    builder: &mut LayoutBuilder,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Option<Left<'a>>> {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return Some(None);
    };
    let Some(declarator) = declaration.declarations.first() else {
        return Some(Some(Left::Refuse(&FOR_IN_BINDING)));
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return Some(Some(Left::Refuse(&DESTRUCTURING_FOR_IN)));
    };
    let slot = builder.slot_for(cx.names.intern(id.name.as_str()))?;
    Some(Some(Left::Declared(slot)))
}

fn other_left<'a>(
    left: &'a ForStatementLeft<'a>,
    inner: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Left<'a> {
    if let ForStatementLeft::AssignmentTargetIdentifier(id) = left {
        return Left::Identifier(cx.names.intern(id.name.as_str()), id.name.as_str());
    }
    match left.as_member_expression() {
        Some(member) => match Target::member(member, inner, cx) {
            Ok(target) => Left::Member(target),
            Err(why) => Left::Refuse(why),
        },
        None => Left::Refuse(&FOR_IN_TARGET),
    }
}

type EachParts<'a> = (
    &'a ForStatementLeft<'a>,
    &'a Expression<'a>,
    &'a Statement<'a>,
);

fn compile_each<'a>(
    (left, right, body): EachParts<'a>,
    (of, awaits): (bool, bool),
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Option<Looped<'a>> {
    let lexical = matches!(left, ForStatementLeft::VariableDeclaration(declaration)
        if declaration.kind != VariableDeclarationKind::Var);
    let mut builder = LayoutBuilder::new();
    let declared = declared_left(left, &mut builder, cx)?;
    let layout = builder.finish()?;
    let inner = ScopeChain::new(layout.clone(), Provable::All, Some(scope.clone()));
    let left = match declared {
        Some(left) => left,
        None => other_left(left, &inner, cx),
    };
    Some(Looped::Each(ForEach {
        of,
        awaits,
        right: compile_expression(right, scope, cx),
        left,
        layout,
        lexical,
        body: Lazy::new(body, inner),
    }))
}

impl<'a> ForEach<'a> {
    fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        labelled: Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        match self.of {
            true => self.run_of(interp, frame, labelled),
            false => self.run_in(interp, frame, labelled),
        }
    }

    fn scope(
        &self,
        interp: &Interp<'a>,
        frame: &FrameRef<'a>,
        shared: &FrameRef<'a>,
    ) -> FrameRef<'a> {
        match self.lexical {
            true => interp.realm.new_fixed_env(&self.layout, frame),
            false => shared.clone(),
        }
    }

    fn run_in(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        (label, reach): Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        let target = self.right.run(interp, frame, reach)?;
        let keys = interp.enumerable_keys(&target)?;
        let shared = interp.realm.new_fixed_env(&self.layout, frame);
        for key in keys {
            if !interp.still_has_key(&target, &key)? {
                continue;
            }
            let scope = self.scope(interp, frame, &shared);
            self.bind(interp, Value::Str(key), (&scope, reach))?;
            flow!(loop_body(interp, (&scope, reach), &self.body)?, label);
        }
        Ok(Completion::Normal)
    }

    fn run_of(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        (label, reach): Labelled<'a>,
    ) -> Eval<'a, Completion<'a>> {
        if self.awaits {
            return Err(Bail::Unsupported(&"for-await-of statement").into());
        }
        let target = self.right.run(interp, frame, reach)?;
        interp.note_iterator_symbol(&target)?;
        let iterated = interp.iterable_object(&target)?;
        let shared = interp.realm.new_fixed_env(&self.layout, frame);
        let cap = loop_cap(interp.loop_len_hint);
        for index in 0usize.. {
            let Some(item) = object::array_element(&iterated, index) else {
                return Ok(Completion::Normal);
            };
            if index as u64 >= cap {
                return Err(Bail::LoopCap.into());
            }
            let scope = self.scope(interp, frame, &shared);
            self.bind(interp, item, (&scope, reach))?;
            flow!(loop_body(interp, (&scope, reach), &self.body)?, label);
        }
        Err(Bail::LoopCap.into())
    }

    fn bind(
        &self,
        interp: &mut Interp<'a>,
        value: Value<'a>,
        (scope, reach): (&FrameRef<'a>, Reach),
    ) -> Eval<'a, ()> {
        match &self.left {
            Left::Declared(slot) => {
                Frame::declare_slot(scope, *slot, value);
                Ok(())
            }
            Left::Identifier(name, text) => interp.assign_name(scope, *name, text, value),
            Left::Member(target) => match target.place(interp, scope, reach)? {
                Place::Member(object, key) => interp.bind_for_member((object, key), value),
                Place::Binding => Err(refuse::mismatch()),
            },
            Left::Refuse(why) => Err(Bail::Unsupported(why).into()),
        }
    }
}
