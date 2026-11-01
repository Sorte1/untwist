use std::rc::Rc;

use super::{Realm, ARRAY_PROTO, FUNCTION_PROTO, OBJECT_PROTO, REGEXP_PROTO};
use crate::bail::{Bail, EvalResult};
use crate::value::{ObjKind, ObjRef, Value};

const IMPLICIT_PROTOTYPE: &str = "a prototype the evaluator leaves implicit";

impl<'a> Realm<'a> {
    pub fn prototype_of(&self, obj: &ObjRef<'a>) -> EvalResult<Option<ObjRef<'a>>> {
        if self.is_linked_prototype(obj) {
            crate::object::note_prototype_links();
        }
        if let Some(proto) = obj.borrow().proto.clone() {
            return Ok(Some(proto));
        }
        if Rc::ptr_eq(obj, &self.protos[OBJECT_PROTO]) {
            return Ok(None);
        }
        match self.implicit_prototype(obj) {
            Some(proto) => Ok(Some(proto)),
            None => Err(Bail::Unsupported(&IMPLICIT_PROTOTYPE)),
        }
    }

    fn is_linked_prototype(&self, obj: &ObjRef<'a>) -> bool {
        !Rc::ptr_eq(obj, &self.protos[OBJECT_PROTO]) && self.is_intrinsic_prototype(obj)
    }

    fn implicit_prototype(&self, obj: &ObjRef<'a>) -> Option<ObjRef<'a>> {
        let proto = match &obj.borrow().kind {
            ObjKind::Function(closure) if !closure.is_generator_or_async() => {
                &self.protos[FUNCTION_PROTO]
            }
            ObjKind::Native(..) => &self.protos[FUNCTION_PROTO],
            ObjKind::Bound(bound) if self.is_plain_function(&bound.target) => {
                &self.protos[FUNCTION_PROTO]
            }
            ObjKind::Array(_) => &self.protos[ARRAY_PROTO],
            ObjKind::RegExp(_) => &self.protos[REGEXP_PROTO],
            ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => &self.protos[OBJECT_PROTO],
            ObjKind::Plain(_) if self.is_intrinsic_prototype(obj) => &self.protos[OBJECT_PROTO],
            _ => return None,
        };
        Some(proto.clone())
    }

    fn is_plain_function(&self, target: &Value<'a>) -> bool {
        let Some(obj) = target.as_object() else {
            return false;
        };
        let borrowed = obj.borrow();
        let proto_is_function = borrowed
            .proto
            .as_ref()
            .is_none_or(|proto| Rc::ptr_eq(proto, &self.protos[FUNCTION_PROTO]));
        let plain = match &borrowed.kind {
            ObjKind::Function(closure) => !closure.is_generator_or_async(),
            ObjKind::Native(..) => true,
            _ => false,
        };
        plain && proto_is_function
    }
}
