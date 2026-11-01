use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::Budget;
use crate::fold::FOLD_STEP_BUDGET;
use crate::host::Host;
use crate::value::{error_value, Abrupt, Eval, JsString, ObjKind, ObjRef, PropKey, Value};

pub(crate) struct BudgetHost {
    budget: Budget,
}

impl Default for BudgetHost {
    fn default() -> Self {
        Self {
            budget: Budget::new(FOLD_STEP_BUDGET),
        }
    }
}

impl Host<'static> for BudgetHost {
    fn spend(&mut self, steps: u64) -> EvalResult<()> {
        self.budget.spend(steps)
    }

    fn throw(&self, kind: ErrorKind, message: &str) -> Abrupt<'static> {
        error_abrupt(kind, message)
    }

    fn note(&mut self, _message: &str) {}

    fn new_object(
        &mut self,
        kind: ObjKind<'static>,
        proto: Option<ObjRef<'static>>,
    ) -> Value<'static> {
        Value::object(kind, proto)
    }

    fn new_array(&mut self, items: Vec<Value<'static>>) -> Value<'static> {
        Value::array(items, None)
    }

    fn call_value(
        &mut self,
        _callee: &Value<'static>,
        _this: Value<'static>,
        _args: &[Value<'static>],
    ) -> Eval<'static, Value<'static>> {
        Err(Bail::Unsupported(&"call during constant folding").into())
    }

    fn construct_value(
        &mut self,
        _callee: &Value<'static>,
        _args: &[Value<'static>],
    ) -> Eval<'static, Value<'static>> {
        Err(Bail::Unsupported(&"new during constant folding").into())
    }

    fn get_property(
        &mut self,
        _target: &Value<'static>,
        _key: &PropKey,
    ) -> Eval<'static, Value<'static>> {
        Err(Bail::Unsupported(&"property read during constant folding").into())
    }

    fn note_protocol_member(&mut self, _target: &Value<'static>, _key: &PropKey) -> EvalResult<()> {
        Ok(())
    }

    fn function_source(&mut self, _target: &Value<'static>) -> Eval<'static, JsString> {
        Err(Bail::Unsupported(&"toString during constant folding").into())
    }

    fn enter_nesting(&mut self) -> EvalResult<()> {
        self.budget.enter_nesting()
    }

    fn leave_nesting(&mut self) {
        self.budget.leave_nesting();
    }

    fn get_prototype_of(&mut self, _value: &Value<'static>) -> Eval<'static, Value<'static>> {
        Err(Bail::Unsupported(&"getPrototypeOf during constant folding").into())
    }
}

fn error_abrupt(kind: ErrorKind, message: &str) -> Abrupt<'static> {
    Abrupt::throw(error_value(kind, message))
}
