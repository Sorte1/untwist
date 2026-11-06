use oxc_ast::ast::{RegExpLiteral, TemplateElement};

use crate::bail::Bail;
use crate::budget::{props_bytes, ARRAY_HEADER_BYTES, OBJECT_BYTES};
use crate::builtins::args::{charge_for_length, reserve_text, slot_bytes};
use crate::interp::Interp;
use crate::ops::to_js_string;
use crate::value::{Eval, JsObject, JsString, ObjKind, ObjRef, PropMap, Value};

pub(crate) fn template_chunk<'a>(quasi: &TemplateElement<'_>) -> Eval<'a, JsString> {
    let Some(cooked) = &quasi.value.cooked else {
        return Err(Bail::Unsupported(&"template with an invalid escape").into());
    };
    Ok(crate::value::js_string_from_js_str(*cooked))
}

impl<'a> Interp<'a> {
    pub(crate) fn append_template_text(
        &mut self,
        out: &JsString,
        piece: &JsString,
    ) -> Eval<'a, JsString> {
        charge_for_length(self, out.len() + piece.len())?;
        let charge = reserve_text(self, out.len() + piece.len())?;
        Ok(out.concat_charged(piece, charge)?)
    }

    pub(crate) fn append_template_value(
        &mut self,
        out: &JsString,
        value: &Value<'a>,
    ) -> Eval<'a, JsString> {
        let rendered = to_js_string(value, self)?;
        self.append_template_text(out, &rendered)
    }

    pub(crate) fn array_hole(&mut self) -> Eval<'a, Value<'a>> {
        crate::unmodelled::refuse_while_decoding(&crate::object::HOLE)?;
        Ok(Value::Undefined)
    }

    pub(crate) fn finish_array_literal(&mut self, items: Vec<Value<'a>>) -> Eval<'a, Value<'a>> {
        let extra = ARRAY_HEADER_BYTES + slot_bytes(items.capacity());
        let charge = self.reserve(extra)?;
        let array = self.realm.new_array(items, self.realm.array_prototype());
        if let (Some(obj), Some(charge)) = (array.as_object(), charge) {
            if let Some(held) = obj.borrow().charge.as_ref() {
                held.absorb(charge);
            }
        }
        Ok(array)
    }

    pub(crate) fn finish_object_literal(&mut self, props: PropMap<'a>) -> Value<'a> {
        let bytes = OBJECT_BYTES + props_bytes(0, props.capacity());
        let obj: ObjRef<'a> = std::rc::Rc::new(std::cell::RefCell::new(JsObject {
            kind: ObjKind::Plain(0),
            props,
            proto: self.realm.object_prototype(),
            charge: self.realm.heap_charge(bytes).map(Box::new),
        }));
        self.realm.track(&obj);
        Value::Object(obj)
    }
}

impl<'a> Interp<'a> {
    pub(crate) fn eval_regexp_literal(
        &mut self,
        node: &'a RegExpLiteral<'a>,
    ) -> Eval<'a, Value<'a>> {
        let Some(raw) = node.raw else {
            return Err(Bail::Unsupported(&"regex literal without raw text").into());
        };
        let Some((pattern, flags)) = split_regex_literal(raw.as_str()) else {
            return Err(Bail::Unsupported(&"regex literal shape").into());
        };
        let proto = self.realm.regexp_prototype();
        crate::builtins::regexp::compile_regexp(
            self,
            &JsString::from_str(pattern),
            &JsString::from_str(flags),
            proto,
        )
    }
}

fn split_regex_literal(raw: &str) -> Option<(&str, &str)> {
    let body = raw.strip_prefix('/')?;
    let end = body.rfind('/')?;
    Some((&body[..end], &body[end + 1..]))
}
