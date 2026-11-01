use crate::value::{ObjKind, Value};

pub type Kinds = u16;

pub const MISSING: Kinds = 1 << 0;
pub const UNDEFINED: Kinds = 1 << 1;
pub const NULL: Kinds = 1 << 2;
pub const BOOL: Kinds = 1 << 3;
pub const NUMBER: Kinds = 1 << 4;
pub const STRING: Kinds = 1 << 5;
pub const ARRAY: Kinds = 1 << 6;
pub const ARGUMENTS: Kinds = 1 << 7;
pub const PLAIN: Kinds = 1 << 8;
pub const CALLABLE: Kinds = 1 << 9;
pub const REGEXP: Kinds = 1 << 10;
pub const ERROR: Kinds = 1 << 11;

pub const NONE: Kinds = 0;
pub const ANY: Kinds = (1 << 12) - 1;
pub const ABSENT: Kinds = MISSING | UNDEFINED;
pub const OBJECT: Kinds = ARRAY | ARGUMENTS | PLAIN | CALLABLE | REGEXP | ERROR;

#[derive(Debug, PartialEq, Eq)]
pub struct Shape {
    pub receiver: Kinds,
    pub args: &'static [Kinds],
    pub rest: Kinds,
    pub coerces_in_spec_order: bool,
}

impl Shape {
    pub const fn new(receiver: Kinds, args: &'static [Kinds], rest: Kinds) -> Self {
        Self {
            receiver,
            args,
            rest,
            coerces_in_spec_order: false,
        }
    }

    const fn in_spec_order(self) -> Self {
        Self {
            coerces_in_spec_order: true,
            ..self
        }
    }

    pub fn admits(&self, this: &Value<'_>, args: &[Value<'_>]) -> bool {
        if self.receiver & kind_of(this) == 0 {
            return false;
        }
        for (index, accepted) in self.args.iter().enumerate() {
            let kind = args.get(index).map_or(MISSING, kind_of);
            if accepted & kind == 0 {
                return false;
            }
        }
        if self.rest == ANY {
            return true;
        }
        args.iter()
            .skip(self.args.len())
            .all(|value| self.rest & kind_of(value) != 0)
    }
}

pub const REFUSED: Shape = Shape::new(NONE, &[], NONE);

pub fn kind_of(value: &Value<'_>) -> Kinds {
    match value {
        Value::Undefined => UNDEFINED,
        Value::Null => NULL,
        Value::Bool(_) => BOOL,
        Value::Number(_) => NUMBER,
        Value::Str(_) => STRING,
        Value::Object(obj) => match obj.borrow().kind {
            ObjKind::Array(_) => ARRAY,
            ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => ARGUMENTS,
            ObjKind::Plain(_) => PLAIN,
            ObjKind::Function(_) | ObjKind::Bound(_) | ObjKind::Native(_, _) => CALLABLE,
            ObjKind::RegExp(_) => REGEXP,
            ObjKind::Error(_) => ERROR,
        },
    }
}

pub struct NativeShape {
    pub owner: &'static str,
    pub name: &'static str,
    pub shape: Shape,
}

const fn entry(owner: &'static str, name: &'static str, shape: Shape) -> NativeShape {
    NativeShape { owner, name, shape }
}

const ANY1: &[Kinds] = &[ANY];
const ANY2: &[Kinds] = &[ANY, ANY];
const PRESENT1: &[Kinds] = &[ANY & !MISSING];
const NOT_REGEXP: Kinds = ANY & !REGEXP;
const NOT_CALLABLE: Kinds = ANY & !CALLABLE;
const IGNORED_SPACE: Kinds = MISSING | UNDEFINED | NULL | BOOL | PLAIN | ARRAY | REGEXP;
const ASSIGNABLE: Kinds = ARRAY | PLAIN | REGEXP;
const PROTOTYPE: Kinds = PLAIN;

const fn any_args(args: &'static [Kinds]) -> Shape {
    Shape::new(ANY, args, ANY)
}

const fn on(receiver: Kinds, args: &'static [Kinds]) -> Shape {
    Shape::new(receiver, args, ANY)
}

pub static SHAPES: &[NativeShape] = &[
    entry("Array.prototype", "concat", on(ARRAY, &[])),
    entry("Array.prototype", "filter", on(ARRAY, ANY2)),
    entry("Array.prototype", "forEach", on(ARRAY, ANY2)),
    entry("Array.prototype", "includes", on(ARRAY, ANY2)),
    entry("Array.prototype", "indexOf", on(ARRAY, ANY2)),
    entry("Array.prototype", "join", on(ARRAY, ANY1)),
    entry("Array.prototype", "map", on(ARRAY, ANY2)),
    entry("Array.prototype", "pop", on(ARRAY, &[])),
    entry("Array.prototype", "push", on(ARRAY, &[])),
    entry("Array.prototype", "reduce", on(ARRAY, ANY2)),
    entry("Array.prototype", "reverse", on(ARRAY, &[])),
    entry("Array.prototype", "shift", on(ARRAY, &[])),
    entry("Array.prototype", "slice", on(ARRAY | ARGUMENTS, ANY2)),
    entry("Array.prototype", "splice", on(ARRAY, ANY2)),
    entry("Array.prototype", "toString", on(ARRAY, &[])),
    entry("Array.prototype", "unshift", on(ARRAY, &[])),
    entry("Function.prototype", "apply", on(CALLABLE, ANY2)),
    entry("Function.prototype", "bind", on(CALLABLE, ANY1)),
    entry("Function.prototype", "call", on(CALLABLE, ANY1)),
    entry("Function.prototype", "toString", on(ANY, &[])),
    entry("JSON", "parse", any_args(&[ANY, NOT_CALLABLE])),
    entry(
        "JSON",
        "stringify",
        any_args(&[ANY, NOT_CALLABLE & !ARRAY, IGNORED_SPACE]),
    ),
    entry("Math", "abs", any_args(ANY1)),
    entry("Math", "ceil", any_args(ANY1)),
    entry("Math", "floor", any_args(ANY1)),
    entry("Math", "imul", any_args(ANY2)),
    entry("Math", "log", any_args(ANY1)),
    entry("Math", "max", any_args(&[])),
    entry("Math", "min", any_args(&[])),
    entry("Math", "pow", any_args(ANY2)),
    entry("Math", "random", REFUSED),
    entry("Math", "round", any_args(ANY1)),
    entry("Math", "sign", any_args(ANY1)),
    entry("Math", "sqrt", any_args(ANY1)),
    entry("Math", "trunc", any_args(ANY1)),
    entry("Number", "", any_args(ANY1)),
    entry("Number", "isFinite", any_args(ANY1)),
    entry("Number", "isInteger", any_args(ANY1)),
    entry("Number", "isNaN", any_args(ANY1)),
    entry("Number", "parseFloat", any_args(ANY1)),
    entry("Number", "parseInt", any_args(ANY2)),
    entry("Number.prototype", "toFixed", on(NUMBER, ANY1)),
    entry("Number.prototype", "toString", on(NUMBER, ANY1)),
    entry("Number.prototype", "valueOf", on(NUMBER, &[])),
    entry("Object", "", any_args(&[ABSENT | NULL | OBJECT])),
    entry("Object", "assign", any_args(&[ASSIGNABLE])),
    entry("Object", "create", any_args(&[PROTOTYPE, ABSENT])),
    entry(
        "Object",
        "defineProperty",
        any_args(&[ASSIGNABLE, ANY, PLAIN]),
    ),
    entry("Object", "entries", any_args(ANY1)),
    entry("Object", "freeze", any_args(ANY1)),
    entry("Object", "getPrototypeOf", any_args(ANY1)),
    entry("Object", "isFrozen", any_args(ANY1)),
    entry("Object", "keys", any_args(ANY1)),
    entry("Object", "values", any_args(ANY1)),
    entry("Object.prototype", "hasOwnProperty", any_args(ANY1)),
    entry("Object.prototype", "toString", any_args(&[])),
    entry("Object.prototype", "valueOf", on(OBJECT, &[])),
    entry("RegExp", "", any_args(&[NOT_REGEXP, ANY])),
    entry("RegExp.prototype", "exec", on(REGEXP, ANY1)),
    entry("RegExp.prototype", "test", on(REGEXP, ANY1)),
    entry("RegExp.prototype", "toString", on(REGEXP, &[])),
    entry("String", "", any_args(ANY1)),
    entry("String", "fromCharCode", any_args(&[])),
    entry("String.prototype", "charAt", any_args(ANY1)),
    entry("String.prototype", "charCodeAt", any_args(ANY1)),
    entry("String.prototype", "codePointAt", any_args(ANY1)),
    entry("String.prototype", "concat", any_args(&[])),
    entry("String.prototype", "endsWith", any_args(&[NOT_REGEXP, ANY])),
    entry("String.prototype", "includes", any_args(&[NOT_REGEXP, ANY])),
    entry("String.prototype", "indexOf", any_args(ANY2)),
    entry("String.prototype", "lastIndexOf", any_args(ANY2)),
    entry("String.prototype", "match", any_args(ANY1)),
    entry("String.prototype", "padEnd", any_args(ANY2)),
    entry("String.prototype", "padStart", any_args(ANY2)),
    entry("String.prototype", "repeat", any_args(ANY1)),
    entry("String.prototype", "replace", any_args(ANY2)),
    entry("String.prototype", "replaceAll", any_args(ANY2)),
    entry("String.prototype", "search", any_args(ANY1)),
    entry("String.prototype", "slice", any_args(ANY2)),
    entry("String.prototype", "split", any_args(ANY2).in_spec_order()),
    entry(
        "String.prototype",
        "startsWith",
        any_args(&[NOT_REGEXP, ANY]),
    ),
    entry("String.prototype", "substr", any_args(ANY2)),
    entry("String.prototype", "substring", any_args(ANY2)),
    entry("String.prototype", "toLowerCase", any_args(&[])),
    entry("String.prototype", "toString", on(STRING, &[])),
    entry("String.prototype", "toUpperCase", any_args(&[])),
    entry("String.prototype", "trim", any_args(&[])),
    entry("String.prototype", "valueOf", on(STRING, &[])),
    entry("global", "Boolean", any_args(ANY1)),
    entry("global", "atob", any_args(PRESENT1)),
    entry("global", "btoa", any_args(PRESENT1)),
    entry("global", "decodeURI", any_args(ANY1)),
    entry("global", "decodeURIComponent", any_args(ANY1)),
    entry("global", "encodeURI", any_args(ANY1)),
    entry("global", "encodeURIComponent", any_args(ANY1)),
    entry("global", "escape", any_args(ANY1)),
    entry("global", "isFinite", any_args(ANY1)),
    entry("global", "isNaN", any_args(ANY1)),
    entry("global", "parseFloat", any_args(ANY1)),
    entry("global", "parseInt", any_args(ANY2)),
    entry("global", "unescape", any_args(ANY1)),
    entry("hostile", "", REFUSED),
];

pub fn lookup(owner: &str, name: &str) -> Option<&'static Shape> {
    let found = SHAPES
        .binary_search_by(|probe| (probe.owner, probe.name).cmp(&(owner, name)))
        .ok()?;
    SHAPES.get(found).map(|entry| &entry.shape)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Realm;

    #[test]
    fn the_table_is_sorted_and_names_every_registered_native_once() {
        assert!(SHAPES
            .windows(2)
            .all(|pair| (pair[0].owner, pair[0].name) < (pair[1].owner, pair[1].name)));
        let (realm, _globals) = Realm::with_globals();
        let natives = realm.natives();
        assert_eq!(natives.unlisted(), 0);
        let registered: std::collections::BTreeSet<usize> = (0..natives.len())
            .filter_map(|index| natives.get(crate::value::NativeId(index as u16)))
            .map(|(_, shape)| std::ptr::from_ref(shape) as usize)
            .collect();
        let listed: std::collections::BTreeSet<usize> = SHAPES
            .iter()
            .map(|entry| std::ptr::from_ref(&entry.shape) as usize)
            .collect();
        assert_eq!(
            natives.len(),
            registered.len(),
            "a native is registered twice"
        );
        assert_eq!(registered, listed);
    }
}
