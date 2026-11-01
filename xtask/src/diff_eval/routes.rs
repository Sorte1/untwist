use untwist_eval::builtins::shape::NativeShape;

use super::pool::Rng;

const ROUTES: usize = 6;

pub fn callee(entry: &NativeShape) -> String {
    match (entry.owner, entry.name) {
        ("global", name) => name.to_string(),
        (owner, "") => owner.to_string(),
        (owner, name) => format!("{owner}.{name}"),
    }
}

fn argument_list(count: usize) -> String {
    (0..count)
        .map(|index| format!("a{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn function_call(rng: &mut Rng, entry: &NativeShape, count: usize) -> String {
    let list = argument_list(count);
    let target = callee(entry);
    match rng.below(4) {
        0 => format!("{target}.apply(undefined, [{list}])"),
        1 => format!("{target}.bind(undefined)({list})"),
        2 => format!("[{list}].map({target})"),
        _ => {
            let construct = entry.owner == "RegExp" && rng.chance(50);
            let new = if construct { "new " } else { "" };
            format!("{new}{target}({list})")
        }
    }
}

pub fn method_call(rng: &mut Rng, entry: &NativeShape, count: usize, natural: bool) -> String {
    let list = argument_list(count);
    let separator = if list.is_empty() { "" } else { ", " };
    let target = callee(entry);
    match rng.below(ROUTES) {
        0 if natural => format!("r.{}({list})", entry.name),
        1 => format!("{target}.apply(r, [{list}])"),
        2 => format!("{target}.bind(r)({list})"),
        3 => format!("Reflect.apply({target}, r, [{list}])"),
        4 => format!("[{list}].map({target}, r)"),
        _ => format!("{target}.call(r{separator}{list})"),
    }
}

pub const GLOBAL_NAMES: &[&str] = &[
    "Array",
    "Symbol",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "URIError",
    "EvalError",
    "Map",
    "Set",
    "Promise",
    "Reflect",
    "Proxy",
    "WeakMap",
    "Date",
    "Object",
    "String",
    "Number",
    "Boolean",
    "RegExp",
    "Function",
    "Math",
    "JSON",
    "globalThis",
    "window",
    "self",
    "global",
    "frames",
    "escape",
    "unescape",
    "atob",
    "btoa",
    "parseInt",
    "parseFloat",
    "isNaN",
    "decodeURIComponent",
    "undefined",
    "NaN",
    "Infinity",
    "someUndeclared",
    "document",
    "navigator",
    "process",
    "require",
];

pub fn typeof_text(rng: &mut Rng) -> (String, String) {
    let name = GLOBAL_NAMES[rng.below(GLOBAL_NAMES.len())];
    let decode = format!("(function () {{ return [typeof {name}]; }})()");
    (format!("typeof {name}"), decode)
}
