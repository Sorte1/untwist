use crate::object::{self, Consulted};
use crate::value::{IntrinsicProto, ObjKind, Value};

pub(crate) fn note_enumeration_intrinsics(target: &Value<'_>) {
    let mut tags = vec![IntrinsicProto::Object];
    if target.is_callable() {
        tags.push(IntrinsicProto::Function);
    }
    match target {
        Value::Str(_) => tags.push(IntrinsicProto::String),
        Value::Number(_) => tags.push(IntrinsicProto::Number),
        Value::Object(obj) => match obj.borrow().kind {
            ObjKind::Array(_) | ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => {
                tags.push(IntrinsicProto::Array)
            }
            ObjKind::RegExp(_) => tags.push(IntrinsicProto::RegExp),
            _ => {}
        },
        _ => {}
    }
    object::note_intrinsics(&tags, Consulted::Any);
}
