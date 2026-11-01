use crate::bail::Bail;
use crate::interp::Interp;
use crate::object::{self, is_length_key, is_registered_intrinsic};
use crate::value::{Eval, JsString, ObjKind, ObjRef, PropKey, Value};

const ENGINE_KEPT: &str = "deleting a key the engine keeps";

impl<'a> Interp<'a> {
    pub(crate) fn delete_binding(&mut self) -> Eval<'a, Value<'a>> {
        Err(Bail::Unsupported(&"delete of a binding").into())
    }

    pub(crate) fn delete_member(&mut self, target: Value<'a>, key: PropKey) -> Eval<'a, Value<'a>> {
        let Some(obj) = target.as_object() else {
            crate::unmodelled::refuse_while_decoding(&"a delete on a primitive")?;
            if let Value::Str(text) = &target {
                if object::string_property(text, &key).is_some() {
                    return Err(Bail::Unsupported(&ENGINE_KEPT).into());
                }
            }
            return Ok(Value::Bool(true));
        };
        if let Some(name) = self.realm.hostile_name(obj) {
            return Err(Bail::hostile_global(name).into());
        }
        if object::decoding() && object::is_dense_index(obj, &key) {
            return Err(Bail::Unsupported(&"a delete that leaves an array hole").into());
        }
        if is_engine_kept_key(obj, &key) {
            return Err(Bail::Unsupported(&ENGINE_KEPT).into());
        }
        if object::own_attributes(obj, &key).is_some_and(|attrs| !attrs.configurable()) {
            return self
                .refused_write(Bail::Refused)
                .map(|()| Value::Bool(false));
        }
        crate::state::note_delete(obj, &key);
        obj.borrow_mut().props.shift_remove(&key);
        clear_deleted_array_slot(obj, &key);
        Ok(Value::Bool(true))
    }
}

fn clear_deleted_array_slot(obj: &ObjRef<'_>, key: &PropKey) {
    let PropKey::Index(index) = key else {
        return;
    };
    let mut borrowed = obj.borrow_mut();
    match &mut borrowed.kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => {
            if let Some(slot) = items.get_mut(*index as usize) {
                *slot = Value::Undefined;
            }
        }
        ObjKind::MappedArguments(mapped) => {
            mapped.unmap(*index as usize);
            if let Some(slot) = mapped.items.get_mut(*index as usize) {
                *slot = Value::Undefined;
            }
        }
        _ => {}
    }
}

fn is_engine_kept_key(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    let decoding = object::decoding();
    let kind_kept = match &obj.borrow().kind {
        ObjKind::Array(_) => decoding && is_length_key(key),
        ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => {
            is_length_key(key) || *key == PropKey::Name(JsString::from_str("callee"))
        }
        ObjKind::Function(_) | ObjKind::Native(_, _) | ObjKind::Bound(_) => {
            decoding && *key == PropKey::Name(JsString::from_str("prototype"))
        }
        ObjKind::RegExp(_) => *key == PropKey::Name(JsString::from_str("lastIndex")),
        _ => false,
    };
    kind_kept || (is_registered_intrinsic(obj) && (decoding || is_length_key(key)))
}
