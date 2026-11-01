use crate::bail::{ErrorKind, EvalResult};
use crate::heap::Charge;
use crate::value::{Abrupt, Eval, JsString, ObjKind, ObjRef, PropKey, Value};

pub trait Host<'a> {
    fn spend(&mut self, steps: u64) -> EvalResult<()>;
    fn reserve_live(&mut self, bytes: u64) -> EvalResult<Option<Charge>> {
        let _ = bytes;
        Ok(None)
    }
    fn charges_heap(&self) -> bool {
        false
    }
    fn check_heap(&self) -> EvalResult<()> {
        Ok(())
    }
    fn unit_string(&mut self, unit: u16) -> EvalResult<JsString> {
        Ok(JsString::from_units(vec![unit]))
    }
    fn throw(&self, kind: ErrorKind, message: &str) -> Abrupt<'a>;
    fn note(&mut self, message: &str);
    fn new_object(&mut self, kind: ObjKind<'a>, proto: Option<ObjRef<'a>>) -> Value<'a>;
    fn object_prototype(&self) -> Option<ObjRef<'a>> {
        None
    }
    fn new_array(&mut self, items: Vec<Value<'a>>) -> Value<'a>;
    fn call_value(
        &mut self,
        callee: &Value<'a>,
        this: Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>>;
    fn construct_value(&mut self, callee: &Value<'a>, args: &[Value<'a>]) -> Eval<'a, Value<'a>>;
    fn get_property(&mut self, target: &Value<'a>, key: &PropKey) -> Eval<'a, Value<'a>>;
    fn note_protocol_member(&mut self, target: &Value<'a>, key: &PropKey) -> EvalResult<()>;
    fn function_source(&mut self, target: &Value<'a>) -> Eval<'a, JsString>;
    fn enter_nesting(&mut self) -> EvalResult<()>;
    fn leave_nesting(&mut self);
    fn get_prototype_of(&mut self, value: &Value<'a>) -> Eval<'a, Value<'a>>;
    fn shared_regex(&mut self, pattern: &str, flags: &str) -> Option<regress::Regex> {
        let _ = (pattern, flags);
        None
    }
    fn share_regex(&mut self, pattern: &str, flags: &str, regex: &regress::Regex) {
        let _ = (pattern, flags, regex);
    }
    fn refuse_global_object(&self, target: &Value<'a>) -> EvalResult<()> {
        let _ = target;
        Ok(())
    }
    fn refuse_namespace(&self, target: &Value<'a>) -> EvalResult<()> {
        let _ = target;
        Ok(())
    }
    fn refuse_coercion_call(&mut self, method: &Value<'a>) -> EvalResult<()> {
        let _ = method;
        Ok(())
    }
}
