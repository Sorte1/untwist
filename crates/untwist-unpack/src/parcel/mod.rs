mod build;
mod layout;
mod link;
pub mod links;
pub mod notes;
mod runtime;
mod verify;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use rustc_hash::FxHashMap;
use untwist_core::pass::Note;

use crate::esbuild::mentions::{closure, mentioned};
use crate::frame::Frame;
use crate::links::Links;
use crate::matcher::unreassigned_var;
use crate::model::{Bodies, Bundle};

pub use layout::{HOISTED_PARAMS, REGISTRY_PARAMS};

pub const ENTRY_ID: &str = "index";
pub const GLOBAL_PARAM: &str = "__globalThis";
pub const REGISTRY_NAMES: [&str; REGISTRY_PARAMS] = ["require", "module", "exports", GLOBAL_PARAM];
pub const HOISTED_NAMES: [&str; HOISTED_PARAMS] = ["module", "exports"];

const INHERITED: [&str; 12] = [
    "constructor",
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

const BUILTINS: [&str; 57] = [
    "assert",
    "assert/strict",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "dns/promises",
    "domain",
    "events",
    "fs",
    "fs/promises",
    "http",
    "http2",
    "https",
    "inspector",
    "inspector/promises",
    "module",
    "net",
    "os",
    "path",
    "path/posix",
    "path/win32",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "readline/promises",
    "repl",
    "stream",
    "stream/consumers",
    "stream/promises",
    "stream/web",
    "string_decoder",
    "sys",
    "timers",
    "timers/promises",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "util/types",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
    "test",
    "sqlite",
    "sea",
];

const NODE_ONLY: [&str; 3] = ["test", "sqlite", "sea"];

pub fn is_builtin(name: &str) -> bool {
    match name.strip_prefix("node:") {
        Some(rest) => !rest.is_empty() && BUILTINS.contains(&rest),
        None => BUILTINS.contains(&name) && !NODE_ONLY.contains(&name),
    }
}

pub fn bare(spec: &str) -> bool {
    !spec.is_empty() && !spec.starts_with('.') && !spec.starts_with('/')
}

pub fn single_string<'k>(call: &'k CallExpression<'_>) -> Option<&'k str> {
    match call.arguments.as_slice() {
        [Argument::StringLiteral(text)] if !text.lone_surrogates => Some(text.value.as_str()),
        _ => None,
    }
}

pub fn resolve<'d>(deps: &'d [(String, String)], spec: &'d str) -> &'d str {
    deps.binary_search_by(|(key, _)| key.as_str().cmp(spec))
        .map_or(spec, |at| deps[at].1.as_str())
}

fn helper_init(init: &Expression<'_>) -> bool {
    match init {
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
        Expression::Identifier(id) => id.name == "globalThis",
        _ => false,
    }
}

fn helper_name<'k>(statement: &'k Statement<'_>, early: bool) -> Option<&'k str> {
    match statement {
        Statement::FunctionDeclaration(function) => function.id.as_ref().map(|id| id.name.as_str()),
        Statement::VariableDeclaration(declaration) if early => {
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let fits = unreassigned_var(declaration.kind)
                && declarator.init.as_ref().is_some_and(helper_init);
            fits.then_some(id.name.as_str())
        }
        _ => None,
    }
}

pub(crate) fn candidates(
    top: &[Statement<'_>],
    layout: &layout::Hoisted<'_, '_>,
) -> FxHashMap<String, usize> {
    let mut names = FxHashMap::default();
    for (index, statement) in top.iter().enumerate() {
        if layout.prelude.contains(&index) || !layout.defs_at(index).is_empty() {
            continue;
        }
        if let Some(name) = helper_name(statement, index < layout.first) {
            names.insert(name.to_string(), index);
        }
    }
    names
}

pub(crate) fn moved_helpers<'k, 'a: 'k>(
    top: &[Statement<'a>],
    candidates: &FxHashMap<String, usize>,
    statements: impl IntoIterator<Item = &'k Statement<'a>>,
) -> Vec<usize> {
    closure(top, candidates, mentioned(candidates, statements))
}

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    layout::probe(frame)
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
    links: &Links,
) -> Option<Bundle<'a>> {
    let view = links.parcel.as_ref();
    let layout = layout::layout(frame, source)?;
    if layout.is_chunk() && !view.is_some_and(|view| view.chunk) {
        return None;
    }
    if !matches!(bodies, Bodies::Taken(_)) && !verify::verify(source, &layout, view) {
        return None;
    }
    let mut bundle = build::bundle(alloc, source, &layout, bodies, view)?;
    bundle.link = view.map(|view| view.namespace.clone());
    Some(bundle)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    link::link(alloc, bundle);
    Vec::new()
}
