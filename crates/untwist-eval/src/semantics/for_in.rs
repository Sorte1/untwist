use crate::bail::Bail;
use crate::builtins::args::write_property;
use crate::host::Host;
use crate::interp::Interp;
use crate::object;
use crate::ops::string_key;
use crate::semantics::enumerate::note_enumeration_intrinsics;
use crate::value::{Eval, JsString, ObjKind, PropKey, Value};

impl<'a> Interp<'a> {
    pub(crate) fn enumerable_keys(&mut self, target: &Value<'a>) -> Eval<'a, Vec<JsString>> {
        if object::decoding() {
            note_enumeration_intrinsics(target);
        }
        if let Some(obj) = target.as_object() {
            self.realm.refuse_global_object(obj)?;
        }
        if self.inherits_keys(target)? {
            return Err(Bail::Unsupported(&"for-in over an object that inherits keys").into());
        }
        Ok(crate::builtins::object::own_keys(self, target)?)
    }

    #[inline(never)]
    fn inherits_keys(&self, target: &Value<'a>) -> Eval<'a, bool> {
        let start = match target {
            Value::Object(obj) => obj.borrow().proto.clone(),
            Value::Str(_) => self.realm.string_prototype(),
            Value::Number(_) => self.realm.number_prototype(),
            Value::Bool(_) => self.realm.object_prototype(),
            Value::Undefined | Value::Null => return Ok(false),
        };
        let decoding = object::decoding();
        if !decoding && self.realm.supplied_prototypes_hold_enumerable_keys(target) {
            return Ok(true);
        }
        let Some(start) = start else {
            return Ok(false);
        };
        let found = object::walk_prototype_chain(start, |link| {
            if self.realm.is_intrinsic_prototype(link) {
                if decoding {
                    return Some(false);
                }
                return object::has_enumerable_own_key(link).then_some(true);
            }
            object::has_own_keys(link).then_some(true)
        })?;
        Ok(found == Some(true))
    }

    pub(crate) fn still_has_key(&mut self, target: &Value<'a>, key: &JsString) -> Eval<'a, bool> {
        let Some(obj) = target.as_object() else {
            return Ok(true);
        };
        let key = string_key(key.clone());
        if object::own_property(obj, &key).is_some() {
            return Ok(true);
        }
        self.has_property(&key, target)
    }

    pub(crate) fn note_iterator_symbol(&mut self, target: &Value<'a>) -> Eval<'a, ()> {
        if !object::decoding() || target.as_object().is_none() {
            return Ok(());
        }
        let key = PropKey::Name(JsString::from_str("@@iterator"));
        self.get_property(target, &key)?;
        Ok(())
    }

    pub(crate) fn iterable_object(
        &mut self,
        target: &Value<'a>,
    ) -> Eval<'a, crate::value::ObjRef<'a>> {
        let Some(obj) = target.as_object() else {
            return Err(Bail::Unsupported(&"for-of over a non-iterable value").into());
        };
        let is_dense =
            matches!(obj.borrow().kind, ObjKind::Array(_)) || obj.borrow().kind.is_arguments();
        if !is_dense {
            return Err(Bail::Unsupported(&"for-of over a non-array iterable").into());
        }
        Ok(obj.clone())
    }

    pub(crate) fn bind_for_member(
        &mut self,
        (object, key): (Value<'a>, PropKey),
        value: Value<'a>,
    ) -> Eval<'a, ()> {
        let Some(obj) = object.as_object() else {
            return Err(Bail::Unsupported(&"for-in target").into());
        };
        if let Some(name) = self.realm.hostile_name(obj) {
            return Err(Bail::hostile_global(name).into());
        }
        crate::object::refuse_proto_write(&key)?;
        match write_property(self, obj, key, value) {
            Ok(()) => Ok(()),
            Err(bail) => self.refused_write(bail),
        }
    }
}
