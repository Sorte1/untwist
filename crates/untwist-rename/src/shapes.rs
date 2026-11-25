use oxc_syntax::symbol::SymbolId;

use crate::usage::Usage;

const SHARED: [&str; 7] = [
    "length",
    "indexOf",
    "lastIndexOf",
    "slice",
    "includes",
    "concat",
    "at",
];

const STRING_ONLY: [&str; 22] = [
    "charAt",
    "charCodeAt",
    "codePointAt",
    "substr",
    "substring",
    "toLowerCase",
    "toUpperCase",
    "trim",
    "trimStart",
    "trimEnd",
    "split",
    "replace",
    "replaceAll",
    "startsWith",
    "endsWith",
    "padStart",
    "padEnd",
    "localeCompare",
    "normalize",
    "repeat",
    "search",
    "matchAll",
];

const ARRAY_ONLY: [&str; 21] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "forEach",
    "map",
    "filter",
    "reduce",
    "reduceRight",
    "some",
    "every",
    "join",
    "sort",
    "reverse",
    "find",
    "findIndex",
    "findLast",
    "flat",
    "flatMap",
    "fill",
];

const EVENT_ONLY: [&str; 12] = [
    "preventDefault",
    "stopPropagation",
    "stopImmediatePropagation",
    "currentTarget",
    "keyCode",
    "clientX",
    "clientY",
    "pageX",
    "pageY",
    "touches",
    "changedTouches",
    "deltaY",
];

const PROMISE: [&str; 3] = ["then", "catch", "finally"];

const POINT: [&str; 2] = ["x", "y"];

const VECTOR: [&str; 4] = ["x", "y", "z", "w"];

const NOT_PLAIN: [&str; 7] = [
    "call",
    "apply",
    "bind",
    "toString",
    "valueOf",
    "constructor",
    "hasOwnProperty",
];

struct Shape {
    word: &'static str,
    needs: &'static [&'static str],
    needs_all: bool,
    allowed: &'static [&'static [&'static str]],
}

const SHAPES: [Shape; 6] = [
    Shape {
        word: "event",
        needs: &EVENT_ONLY,
        needs_all: false,
        allowed: &[],
    },
    Shape {
        word: "promise",
        needs: &["then"],
        needs_all: true,
        allowed: &[&PROMISE],
    },
    Shape {
        word: "vec",
        needs: &["x", "y", "z"],
        needs_all: true,
        allowed: &[&VECTOR],
    },
    Shape {
        word: "point",
        needs: &POINT,
        needs_all: true,
        allowed: &[&POINT],
    },
    Shape {
        word: "str",
        needs: &STRING_ONLY,
        needs_all: false,
        allowed: &[&STRING_ONLY, &SHARED],
    },
    Shape {
        word: "arr",
        needs: &ARRAY_ONLY,
        needs_all: false,
        allowed: &[&ARRAY_ONLY, &SHARED],
    },
];

impl Shape {
    fn fits(&self, props: &[&str]) -> bool {
        let needed = if self.needs_all {
            self.needs.iter().all(|need| props.contains(need))
        } else {
            props.iter().any(|prop| self.needs.contains(prop))
        };
        let allowed = self.allowed.is_empty()
            || props
                .iter()
                .all(|prop| self.allowed.iter().any(|set| set.contains(prop)));
        needed && allowed
    }
}

pub const NUMBER: &str = "num";

const OBJECT: &str = "obj";

const PLAIN_READ: &str = "#read";

fn is_method(prop: &str) -> bool {
    prop != "length"
        && (SHARED.contains(&prop)
            || STRING_ONLY.contains(&prop)
            || ARRAY_ONLY.contains(&prop)
            || PROMISE.contains(&prop))
}

fn is_index(prop: &str) -> bool {
    prop.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_plain(prop: &str) -> bool {
    !NOT_PLAIN.contains(&prop)
        && !SHARED.contains(&prop)
        && !STRING_ONLY.contains(&prop)
        && !ARRAY_ONLY.contains(&prop)
        && !PROMISE.contains(&prop)
}

pub fn is_shape_word(name: &str) -> bool {
    name == NUMBER || name == OBJECT || SHAPES.iter().any(|shape| shape.word == name)
}

pub fn shape_word(usage: &Usage, symbol: SymbolId) -> Option<&'static str> {
    if usage.is_callable(symbol) {
        return None;
    }
    let read = usage.props_read.get(&symbol).map_or(&[][..], Vec::as_slice);
    let mut props: Vec<&str> = read
        .iter()
        .map(String::as_str)
        .filter(|prop| !is_index(prop))
        .map(|prop| {
            let uncalled = is_method(prop) && !usage.calls_method(symbol, prop);
            if uncalled {
                PLAIN_READ
            } else {
                prop
            }
        })
        .collect();
    let indexed = usage.indexed.contains(&symbol) || props.len() < read.len();
    props.sort_unstable();
    props.dedup();
    if props.is_empty() {
        let numeric = !indexed && usage.numeric_only.contains(&symbol);
        return numeric.then_some(NUMBER);
    }
    if usage.is_numeric(symbol) {
        return None;
    }
    if let Some(shape) = SHAPES.iter().find(|shape| shape.fits(&props)) {
        return Some(shape.word);
    }
    if indexed && props == ["length"] {
        return Some("arr");
    }
    props.iter().all(|prop| is_plain(prop)).then_some(OBJECT)
}
