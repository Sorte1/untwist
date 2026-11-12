use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};

use crate::bail::{Bail, ErrorKind};
use crate::builtins::args::write_property;
use crate::compile::scope::CONSTANT_ASSIGNMENT;
use crate::env::{declare, outermost, Assigned};
use crate::frame::FrameRef;
use crate::host::Host;
use crate::interp::Interp;
use crate::names::Name;
use crate::object;
use crate::value::{Eval, PropKey, Value};

impl<'a> Interp<'a> {
    #[inline(always)]
    pub(crate) fn assign_name(
        &mut self,
        env: &FrameRef<'a>,
        name: Name,
        text: &str,
        value: Value<'a>,
    ) -> Eval<'a, ()> {
        if self.has_root_constants() && self.writes_root_constant(env, name)? {
            return Err(self.throw(ErrorKind::TypeError, CONSTANT_ASSIGNMENT));
        }
        match crate::env::assign(env, name, value.clone())? {
            Assigned::Written => return Ok(()),
            Assigned::Immutable if self.strict => {
                return Err(self.throw(ErrorKind::TypeError, CONSTANT_ASSIGNMENT))
            }
            Assigned::Immutable => return Ok(()),
            Assigned::Constant => return Err(self.throw(ErrorKind::TypeError, CONSTANT_ASSIGNMENT)),
            Assigned::Unbound if self.strict => {
                return Err(self.throw(ErrorKind::ReferenceError, &format!("{text} is not defined")))
            }
            Assigned::Unbound => {}
        }
        declare(&outermost(env), name, value);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn write_member(
        &mut self,
        object: Value<'a>,
        key: PropKey,
        value: Value<'a>,
    ) -> Eval<'a, ()> {
        let Some(obj) = object.as_object() else {
            return self.primitive_write(&object);
        };
        if let Some(bail) = self.realm.refused_write_target(obj, &key) {
            return Err(bail.into());
        }
        match write_property(self, obj, key, value) {
            Ok(()) => Ok(()),
            Err(bail) => self.refused_write(bail),
        }
    }

    #[cold]
    #[inline(never)]
    fn primitive_write(&self, object: &Value<'a>) -> Eval<'a, ()> {
        let nullish = matches!(object, Value::Undefined | Value::Null);
        if !nullish && !self.strict {
            return Ok(());
        }
        Err(crate::unmodelled::throw_or_refuse(
            self,
            ErrorKind::TypeError,
            "cannot set a property on a primitive",
            &"a property write on a primitive, which sloppy code ignores",
        ))
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn refused_write(&self, bail: Bail) -> Eval<'a, ()> {
        if bail != Bail::Refused || object::decoding() {
            return Err(bail.into());
        }
        if !self.strict {
            return Ok(());
        }
        Err(self.throw(ErrorKind::TypeError, "a property's attributes refuse it"))
    }
}

pub(crate) fn binary_of(op: AssignmentOperator) -> Option<BinaryOperator> {
    match op {
        AssignmentOperator::Assign => None,
        AssignmentOperator::Addition => Some(BinaryOperator::Addition),
        AssignmentOperator::Subtraction => Some(BinaryOperator::Subtraction),
        AssignmentOperator::Multiplication => Some(BinaryOperator::Multiplication),
        AssignmentOperator::Division => Some(BinaryOperator::Division),
        AssignmentOperator::Remainder => Some(BinaryOperator::Remainder),
        AssignmentOperator::Exponential => Some(BinaryOperator::Exponential),
        AssignmentOperator::BitwiseAnd => Some(BinaryOperator::BitwiseAnd),
        AssignmentOperator::BitwiseOR => Some(BinaryOperator::BitwiseOR),
        AssignmentOperator::BitwiseXOR => Some(BinaryOperator::BitwiseXOR),
        AssignmentOperator::ShiftLeft => Some(BinaryOperator::ShiftLeft),
        AssignmentOperator::ShiftRight => Some(BinaryOperator::ShiftRight),
        AssignmentOperator::ShiftRightZeroFill => Some(BinaryOperator::ShiftRightZeroFill),
        AssignmentOperator::LogicalAnd
        | AssignmentOperator::LogicalOr
        | AssignmentOperator::LogicalNullish => None,
    }
}
