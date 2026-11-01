use std::sync::OnceLock;

use crate::builtins::shape::SHAPES;
use crate::builtins::Realm;
use crate::object::root_name;
use crate::value::{IntrinsicProto, NativeId, ObjKind, ObjRef};

static NATIVE_LABELS: OnceLock<Vec<Option<String>>> = OnceLock::new();

pub fn shape_label(owner: &str, name: &str) -> String {
    match (owner, name) {
        ("global", name) => name.to_string(),
        (owner, "") => owner.to_string(),
        (owner, name) => format!("{owner}.{name}"),
    }
}

fn build_native_labels() -> Vec<Option<String>> {
    let (realm, _globals) = Realm::with_globals();
    let table = realm.natives();
    (0..table.len())
        .map(|index| {
            let (_, shape) = table.get(NativeId(index as u16))?;
            SHAPES
                .iter()
                .find(|entry| std::ptr::eq(&entry.shape, shape))
                .filter(|entry| entry.owner != "hostile")
                .map(|entry| shape_label(entry.owner, entry.name))
        })
        .collect()
}

pub fn native_label(id: NativeId) -> Option<&'static str> {
    NATIVE_LABELS
        .get_or_init(build_native_labels)
        .get(id.0 as usize)?
        .as_deref()
}

pub fn intrinsic_label(obj: &ObjRef<'_>) -> Option<String> {
    let tag = obj.borrow().trace_tag();
    if let Some(proto) = IntrinsicProto::from_bit(tag) {
        return Some(format!("{}.prototype", proto_owner(proto)));
    }
    root_name(obj).map(str::to_string)
}

fn proto_owner(proto: IntrinsicProto) -> &'static str {
    match proto {
        IntrinsicProto::Object => "Object",
        IntrinsicProto::Array => "Array",
        IntrinsicProto::Function => "Function",
        IntrinsicProto::String => "String",
        IntrinsicProto::Number => "Number",
        IntrinsicProto::RegExp => "RegExp",
    }
}

pub fn natural_prototype(obj: &ObjRef<'_>) -> Option<String> {
    let borrowed = obj.borrow();
    let owner = match &borrowed.kind {
        ObjKind::Plain(_) | ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => "Object",
        ObjKind::Array(_) => "Array",
        ObjKind::Function(closure) => closure.class_tag(),
        ObjKind::Bound(bound) => {
            let target = bound.target.as_object()?.clone();
            drop(borrowed);
            return natural_prototype(&target);
        }
        ObjKind::Native(_, _) => "Function",
        ObjKind::Error(kind) => kind.name(),
        ObjKind::RegExp(_) => "RegExp",
    };
    Some(format!("{owner}.prototype"))
}
