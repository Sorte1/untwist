use crate::bail::Bail;
use crate::interp::Interp;
use crate::value::{Closure, Eval, ObjKind, Value};

impl<'a> Interp<'a> {
    pub(crate) fn read_this(&self) -> Eval<'a, Value<'a>> {
        let boxed = matches!(self.this_value, Value::Object(_));
        if !self.strict && !boxed {
            return Err(
                Bail::Unsupported(&"sloppy-mode this bound to a primitive or nothing").into(),
            );
        }
        Ok(self.this_value.clone())
    }

    pub(crate) fn bind_lexical_this(&mut self, closure: &mut Value<'a>) {
        let this = self.this_value.clone();
        let Some(obj) = closure.as_object() else {
            return;
        };
        let mut borrowed = obj.borrow_mut();
        let ObjKind::Function(existing) = &borrowed.kind else {
            return;
        };
        let rebound = std::rc::Rc::new(Closure {
            code: existing.code,
            env: existing.env.clone(),
            lexical_this: Some(Box::new(this)),
            strict: existing.strict,
            plain: existing.plain,
        });
        borrowed.kind = ObjKind::Function(rebound);
    }
}
