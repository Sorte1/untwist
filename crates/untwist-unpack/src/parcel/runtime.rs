use crate::canon::{expression_shape, function_shape, statement_shape_renamed};
use oxc_ast::ast::*;

const KNOWN: [&str; 9] = [
    include_str!("runtimes/dev.txt"),
    include_str!("runtimes/dev-unminified.txt"),
    include_str!("runtimes/dev-default.txt"),
    include_str!("runtimes/min.txt"),
    include_str!("runtimes/min-prepared.txt"),
    include_str!("runtimes/min-unminified.txt"),
    include_str!("runtimes/min-default.txt"),
    include_str!("runtimes/dev-unsafe.txt"),
    include_str!("runtimes/min-unsafe.txt"),
];

const KEY: &str = "%KEY%";
const KNOWN_DEFINITIONS: [&str; 5] = [
    include_str!("runtimes/hoisted.txt"),
    include_str!("runtimes/hoisted-default.txt"),
    include_str!("runtimes/hoisted-min.txt"),
    include_str!("runtimes/hoisted-min-prepared.txt"),
    include_str!("runtimes/hoisted-min-default.txt"),
];

const LEGACY: [&str; 6] = [
    include_str!("runtimes/legacy-dev.txt"),
    include_str!("runtimes/legacy-dev-default.txt"),
    include_str!("runtimes/legacy-dev-unsafe.txt"),
    include_str!("runtimes/legacy-min.txt"),
    include_str!("runtimes/legacy-min-default.txt"),
    include_str!("runtimes/legacy-min-unsafe.txt"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    Current,
    Legacy,
}

pub fn proven(function: &Function<'_>, source_text: &str) -> Option<Generation> {
    let text = function_shape(function, source_text)?;
    if KNOWN.contains(&text.as_str()) {
        return Some(Generation::Current);
    }
    LEGACY
        .contains(&text.as_str())
        .then_some(Generation::Legacy)
}

pub const STORES: [&str; 2] = ["$parcel$modules", "$parcel$inits"];
const REQUIRE: &str = "parcelRequire";
const GLOBAL: &str = "$parcel$global";
const ERROR: &str = "Error";

fn shape(
    statement: &Statement<'_>,
    head: Option<&Expression<'_>>,
    source_text: &str,
    renames: &[(&str, &str)],
) -> (String, Vec<String>) {
    match head {
        Some(head) => expression_shape(head, source_text, renames),
        None => statement_shape_renamed(statement, source_text, renames),
    }
}

pub fn defined(
    statement: &Statement<'_>,
    head: Option<&Expression<'_>>,
    [require, object]: [&str; 2],
    key: &str,
    source_text: &str,
) -> Option<[String; 2]> {
    if require == object || require == ERROR || object == ERROR {
        return None;
    }
    let (_, free) = shape(statement, head, source_text, &[]);
    let others: Vec<&str> = free
        .iter()
        .map(String::as_str)
        .filter(|name| ![require, object, ERROR].contains(name))
        .collect();
    let [first, second] = others.as_slice() else {
        return None;
    };
    let found = [(*first, *second), (*second, *first)]
        .into_iter()
        .find_map(|(modules, inits)| {
            let renames = [
                (require, REQUIRE),
                (object, GLOBAL),
                (modules, STORES[0]),
                (inits, STORES[1]),
            ];
            let (text, _) = shape(statement, head, source_text, &renames);
            KNOWN_DEFINITIONS
                .iter()
                .any(|known| known.replace(KEY, key) == text)
                .then(|| [modules.to_string(), inits.to_string()])
        });
    found
}
