use crate::bail::{Bail, ErrorKind};
use crate::host::Host;
use crate::interp::Interp;
use crate::object::{self, lookup_property};
use crate::ops::to_property_key;
use crate::value::{Abrupt, Eval, FnCode, JsString, ObjKind, PropKey, Value};

const NON_PRIMITIVE_KEY: &str =
    "a non-primitive property key in an operator, whose conversion order the evaluator does not model";

const GENERATOR_PROTOTYPE: &str =
    "`instanceof` with a generator function, whose own prototype the evaluator does not model";

fn bound_target<'a>(constructor: &Value<'a>) -> Option<Value<'a>> {
    match &constructor.as_object()?.borrow().kind {
        ObjKind::Bound(bound) => Some(bound.target.clone()),
        _ => None,
    }
}

fn is_generator(constructor: &Value<'_>) -> bool {
    constructor
        .as_object()
        .is_some_and(|obj| match &obj.borrow().kind {
            ObjKind::Function(closure) => {
                matches!(closure.code, FnCode::Function(function) if function.generator)
            }
            _ => false,
        })
}

impl<'a> Interp<'a> {
    #[inline]
    pub fn operator_key(&mut self, value: &Value<'a>) -> Eval<'a, PropKey> {
        if let Value::Number(n) = value {
            return Ok(crate::ops::number_key(*n));
        }
        if value.as_object().is_some() && object::decoding() {
            return Err(Bail::Unsupported(&NON_PRIMITIVE_KEY).into());
        }
        to_property_key(value, self)
    }

    #[inline]
    pub(crate) fn element(
        &mut self,
        target: &Value<'a>,
        held: &Value<'a>,
    ) -> Eval<'a, Option<Value<'a>>> {
        let Value::Number(n) = held else {
            return Ok(None);
        };
        let index = *n as u32;
        if f64::from(index) != *n {
            return Ok(None);
        }
        match target {
            Value::Object(obj) => {
                let found = match &obj.borrow().kind {
                    ObjKind::Array(items) => items
                        .get(index as usize)
                        .filter(|value| !matches!(value, Value::Undefined))
                        .cloned(),
                    _ => None,
                };
                if found.is_some() {
                    self.budget.spend(1)?;
                }
                Ok(found)
            }
            Value::Str(text) => match text.code_unit_at(index as usize) {
                Some(unit) => {
                    self.budget.spend(1)?;
                    Ok(Some(Value::Str(self.realm.unit_string(unit)?)))
                }
                None => Ok(None),
            },
            _ => Ok(None),
        }
    }

    pub fn has_property(&mut self, key: &PropKey, target: &Value<'a>) -> Eval<'a, bool> {
        crate::state::note_key_observation(target);
        let Some(obj) = target.as_object() else {
            return Err(self.throw(
                ErrorKind::TypeError,
                "`in` requires an object on the right-hand side",
            ));
        };
        let mut own = lookup_property(obj, key)?;
        if own.is_none() && object::is_prototype_key(key) {
            own = self.read_after_waking(obj, key)?;
        }
        object::refuse_undefined_dense_index(obj, key, own.as_ref())?;
        let inherited = self.realm.intrinsic_member(target, key)?;
        if own.is_none() && object::decoding() && object::is_arguments(obj) {
            crate::interp::refuse_arguments_miss(key, inherited.as_ref())?;
        }
        if own.is_none() && inherited.is_none() {
            self.realm.refuse_global_object(obj)?;
            crate::unmodelled::refuse_unmodelled_miss(target, key)?;
        }
        Ok(own.is_some() || inherited.is_some())
    }

    pub fn is_instance_of(&mut self, value: &Value<'a>, constructor: &Value<'a>) -> Eval<'a, bool> {
        if !constructor.is_callable() {
            return Err(self.instanceof_error("right-hand side of `instanceof` is not callable"));
        }
        if object::decoding() {
            let marker = PropKey::Name(JsString::from_str("@@hasInstance"));
            self.get_property(constructor, &marker)?;
        }
        if let Some(target) = bound_target(constructor) {
            return self.is_instance_of(value, &target);
        }
        let key = self.prototype_key.clone();
        let prototype = self.get_property(constructor, &key)?;
        if prototype.as_object().is_none()
            && value.as_object().is_some()
            && is_generator(constructor)
        {
            return Err(Bail::Unsupported(&GENERATOR_PROTOTYPE).into());
        }
        self.shares_prototype(&prototype, value)
    }

    #[cold]
    #[inline(never)]
    fn instanceof_error(&self, message: &str) -> Abrupt<'a> {
        self.throw(ErrorKind::TypeError, message)
    }

    fn shares_prototype(&mut self, prototype: &Value<'a>, value: &Value<'a>) -> Eval<'a, bool> {
        let Some(start) = value.as_object() else {
            return Ok(false);
        };
        let Some(target) = prototype.as_object() else {
            return Err(
                self.instanceof_error("function has a non-object prototype in an instanceof check")
            );
        };
        let mut link = self.realm.prototype_of(start)?;
        for _ in 0..object::PROTO_CHAIN_CAP {
            let Some(current) = link else {
                return Ok(false);
            };
            if std::rc::Rc::ptr_eq(&current, target) {
                return Ok(true);
            }
            link = self.realm.prototype_of(&current)?;
        }
        Err(Bail::ChainTooDeep(&"prototype").into())
    }
}
