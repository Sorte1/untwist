use std::rc::Rc;

use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, Expression, MemberExpression, SimpleAssignmentTarget,
    UpdateExpression,
};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UpdateOperator};

use crate::compile::depth::Reach;
use crate::compile::expr::{binding, compile_expression, node, refused, Binding, Key};
use crate::compile::scope::{finish_store, unlaid, Near, ScopeChain};
use crate::compile::{refuse, Code, Compiler};
use crate::frame::{Frame, FrameRef, Stored};
use crate::host::Host;
use crate::interp::Interp;
use crate::ops::{binary_fast, to_boolean, to_number_value};
use crate::semantics::assign::binary_of;
use crate::value::{Eval, PropKey, Value};

pub(super) const PRIVATE_FIELD: &str = "private field access";
const DESTRUCTURING_TARGET: &str = "destructuring assignment target";
const UPDATE_TARGET: &str = "update target";

pub(super) enum Target<'a> {
    Binding(Binding<'a>),
    Member(Code<'a>, Key<'a>),
}

type Refusal = &'static &'static str;

pub(super) enum Place<'a> {
    Binding,
    Member(Value<'a>, PropKey),
}

impl<'a> Target<'a> {
    pub(super) fn member(
        member: &'a MemberExpression<'a>,
        scope: &Rc<ScopeChain>,
        cx: &mut Compiler<'_, 'a>,
    ) -> Result<Self, Refusal> {
        match member {
            MemberExpression::PrivateFieldExpression(_) => Err(&PRIVATE_FIELD),
            _ => Ok(Target::Member(
                compile_expression(member.object(), scope, cx),
                Key::of(member, scope, cx),
            )),
        }
    }

    fn assignment(
        target: &'a AssignmentTarget<'a>,
        scope: &Rc<ScopeChain>,
        cx: &mut Compiler<'_, 'a>,
    ) -> Result<Self, Refusal> {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
            return Ok(Target::Binding(binding(id, scope, cx)));
        }
        match target.as_member_expression() {
            Some(member) => Target::member(member, scope, cx),
            None => Err(&DESTRUCTURING_TARGET),
        }
    }

    fn simple(
        target: &'a SimpleAssignmentTarget<'a>,
        scope: &Rc<ScopeChain>,
        cx: &mut Compiler<'_, 'a>,
    ) -> Result<Self, Refusal> {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = target {
            return Ok(Target::Binding(binding(id, scope, cx)));
        }
        match target.as_member_expression() {
            Some(member) => Target::member(member, scope, cx),
            None => Err(&UPDATE_TARGET),
        }
    }

    pub(super) fn place(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, Place<'a>> {
        match self {
            Target::Binding(_) => Ok(Place::Binding),
            Target::Member(object, key) => {
                let target = object.run(interp, frame, reach)?;
                let key = key.eval(interp, frame, reach)?;
                Ok(Place::Member(target, key))
            }
        }
    }

    fn read(
        &self,
        place: &Place<'a>,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
    ) -> Eval<'a, Value<'a>> {
        match (self, place) {
            (Target::Binding(binding), _) => binding.resolved.read(interp, frame, binding.text),
            (_, Place::Member(target, key)) => interp.get_property(target, key),
            (_, Place::Binding) => Err(refuse::mismatch()),
        }
    }

    fn write(
        &self,
        place: Place<'a>,
        value: Value<'a>,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
    ) -> Eval<'a, ()> {
        match (self, place) {
            (Target::Binding(binding), _) => {
                crate::state::note_capture(&value);
                let named = (binding.name, binding.text);
                binding.resolved.write(interp, frame, named, value)
            }
            (_, Place::Member(target, key)) => interp.write_member(target, key, value),
            (_, Place::Binding) => Err(refuse::mismatch()),
        }
    }
}

fn logical_of(operator: AssignmentOperator) -> Option<LogicalOperator> {
    match operator {
        AssignmentOperator::LogicalAnd => Some(LogicalOperator::And),
        AssignmentOperator::LogicalOr => Some(LogicalOperator::Or),
        AssignmentOperator::LogicalNullish => Some(LogicalOperator::Coalesce),
        _ => None,
    }
}

pub(super) fn compile_assignment<'a>(
    assignment: &'a AssignmentExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let target = match Target::assignment(&assignment.left, scope, cx) {
        Ok(target) => target,
        Err(why) => return refused(why),
    };
    let right = compile_expression(&assignment.right, scope, cx);
    match logical_of(assignment.operator) {
        Some(logical) => logical_assignment(target, logical, right),
        None => arithmetic_assignment(
            target,
            binary_of(assignment.operator),
            right,
            assignment.span,
        ),
    }
}

fn arithmetic_assignment<'a>(
    target: Target<'a>,
    operator: Option<BinaryOperator>,
    right: Code<'a>,
    span: oxc_span::Span,
) -> Code<'a> {
    match target {
        Target::Binding(binding) => binding_assignment(binding, operator, right, span),
        Target::Member(object, key) => member_assignment((object, key), operator, right, span),
    }
}

fn member_assignment<'a>(
    (object, key): (Code<'a>, Key<'a>),
    operator: Option<BinaryOperator>,
    right: Code<'a>,
    span: oxc_span::Span,
) -> Code<'a> {
    node(move |interp, frame, reach| {
        let target = object.run(interp, frame, reach)?;
        let key = key.eval(interp, frame, reach)?;
        let value = match operator {
            None => right.run(interp, frame, reach)?,
            Some(operator) => {
                let current = interp.get_property(&target, &key)?;
                let right = right.run(interp, frame, reach)?;
                binary_fast(operator, &current, &right, interp)?
            }
        };
        interp.write_member(target, key, value.clone())?;
        crate::state::note_rebind(span, &value);
        Ok(value)
    })
}

fn binding_assignment<'a>(
    binding: Binding<'a>,
    operator: Option<BinaryOperator>,
    right: Code<'a>,
    span: oxc_span::Span,
) -> Code<'a> {
    let near = binding.resolved.near();
    node(move |interp, frame, reach| {
        let value = match operator {
            None => right.run(interp, frame, reach)?,
            Some(operator) => {
                let current = binding.resolved.read(interp, frame, binding.text)?;
                let right = right.run(interp, frame, reach)?;
                binary_fast(operator, &current, &right, interp)?
            }
        };
        let written = value.clone();
        crate::state::note_capture(&written);
        let stored = match near {
            Near::Local(slot) => frame.borrow_mut().body.store_slot(slot, written),
            Near::Parent(slot) => Frame::store_in_parent(frame, slot, written),
            Near::Far => {
                let named = (binding.name, binding.text);
                binding.resolved.write(interp, frame, named, written)?;
                Some(Stored::Done)
            }
        };
        finish_store(interp, stored.ok_or_else(unlaid)?)?;
        crate::state::note_rebind(span, &value);
        Ok(value)
    })
}

fn logical_assignment<'a>(
    target: Target<'a>,
    logical: LogicalOperator,
    right: Code<'a>,
) -> Code<'a> {
    node(move |interp, frame, reach| {
        let place = target.place(interp, frame, reach)?;
        let current = target.read(&place, interp, frame)?;
        let take = match logical {
            LogicalOperator::And => to_boolean(&current),
            LogicalOperator::Or => !to_boolean(&current),
            LogicalOperator::Coalesce => current.is_nullish(),
        };
        if !take {
            return Ok(current);
        }
        let value = right.run(interp, frame, reach)?;
        target.write(place, value.clone(), interp, frame)?;
        drop(current);
        Ok(value)
    })
}

pub(super) fn compile_update<'a>(
    update: &'a UpdateExpression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let target = match Target::simple(&update.argument, scope, cx) {
        Ok(target) => target,
        Err(why) => return refused(why),
    };
    let (increment, prefix) = (update.operator == UpdateOperator::Increment, update.prefix);
    if let Target::Binding(binding) = target {
        return binding_update(binding, (increment, prefix));
    }
    node(move |interp, frame, reach| {
        let place = target.place(interp, frame, reach)?;
        let current = target.read(&place, interp, frame)?;
        let old = match &current {
            Value::Number(n) => *n,
            _ => to_number_value(&current, interp)?,
        };
        let new = if increment { old + 1.0 } else { old - 1.0 };
        target.write(place, Value::Number(new), interp, frame)?;
        Ok(Value::Number(if prefix { new } else { old }))
    })
}

fn binding_update<'a>(binding: Binding<'a>, (increment, prefix): (bool, bool)) -> Code<'a> {
    let near = binding.resolved.near();
    node(move |interp, frame, _| {
        let current = match near {
            Near::Local(slot) => frame.borrow().body.read_slot(slot).ok_or_else(unlaid)?,
            Near::Parent(slot) => Frame::parent_slot(frame, slot).ok_or_else(unlaid)?,
            Near::Far => binding.resolved.read(interp, frame, binding.text)?,
        };
        let old = match &current {
            Value::Number(n) => *n,
            _ => to_number_value(&current, interp)?,
        };
        let new = if increment { old + 1.0 } else { old - 1.0 };
        let written = Value::Number(new);
        crate::state::note_capture(&written);
        let stored = match near {
            Near::Local(slot) => frame.borrow_mut().body.store_slot(slot, written),
            Near::Parent(slot) => Frame::store_in_parent(frame, slot, written),
            Near::Far => {
                let named = (binding.name, binding.text);
                binding.resolved.write(interp, frame, named, written)?;
                Some(Stored::Done)
            }
        };
        finish_store(interp, stored.ok_or_else(unlaid)?)?;
        Ok(Value::Number(if prefix { new } else { old }))
    })
}

pub(super) fn compile_delete<'a>(
    argument: &'a Expression<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Code<'a> {
    let Some(member) = argument.as_member_expression() else {
        return node(|interp, _, _| interp.delete_binding());
    };
    let target = match Target::member(member, scope, cx) {
        Ok(target) => target,
        Err(why) => return refused(why),
    };
    node(
        move |interp, frame, reach| match target.place(interp, frame, reach)? {
            Place::Member(object, key) => interp.delete_member(object, key),
            Place::Binding => Err(refuse::mismatch()),
        },
    )
}
