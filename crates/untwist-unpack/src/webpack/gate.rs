use oxc_ast::ast::*;
use rustc_hash::FxHashSet;

use untwist_core::ast_util::prop::identifier_name;

use crate::matcher::{
    argument_expression, call_parts, const_member, expression_statement, param_names,
};
use crate::model::{BundleKind, Runtime};
use crate::path::PosixPath;
use crate::runtime::runtime_path;
use crate::webpack::container::module_factories;
use crate::webpack::runtime_module::{member_names, needed, COMPUTED};
use crate::UnpackOptions;

const BIND: &str = "bind";
const RUNTIME_NAME: &str = "webpack";
const RUNTIME_FILE: &str = "webpack.js";
const HOST_OWN: [&str; 6] = ["resolve", "cache", "main", "extensions", "length", "name"];

pub struct Prelude<'r, 'a> {
    pub runtime: Option<&'r Runtime<'a>>,
    pub mentioned: FxHashSet<String>,
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || !c.is_ascii()
}

fn mentions_member(text: &str, name: &str) -> bool {
    if text.contains("\\u") {
        return true;
    }
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| is_name_char(c) || c == '.') {
            return false;
        }
        let after = text[at + name.len()..].trim_start();
        matches!(after.chars().next(), Some('.' | '[' | '/' | '?'))
    })
}

fn marks(statement: &Statement<'_>, require: &str, exports: &str) -> bool {
    let Some((callee, arguments)) = expression_statement(statement).and_then(call_parts) else {
        return false;
    };
    let target = arguments
        .first()
        .and_then(argument_expression)
        .and_then(identifier_name);
    let object = const_member(callee, "r").and_then(identifier_name);
    arguments.len() == 1 && object == Some(require) && target == Some(exports)
}

fn claims_runtime(id: &str) -> bool {
    let named = id.ends_with(RUNTIME_NAME) || id.ends_with(RUNTIME_FILE);
    named
        && PosixPath::from_module_id(id, false)
            .is_some_and(|path| path.as_str() == runtime_path(BundleKind::Webpack))
}

pub fn routable(
    source: &str,
    container: &Expression<'_>,
    prelude: &Prelude<'_, '_>,
    options: UnpackOptions,
) -> bool {
    if options.conservative {
        return true;
    }
    let runtime = prelude.runtime;
    let routes = |name: &str| runtime.and_then(|runtime| needed(runtime, name)).is_some();
    let unroutable: Vec<&str> = runtime
        .iter()
        .flat_map(|runtime| &runtime.helpers)
        .map(|helper| helper.name.as_str())
        .chain(prelude.mentioned.iter().map(String::as_str))
        .filter(|name| !routes(name))
        .collect();
    let factories = module_factories(container);
    factories.into_iter().all(|(id, factory)| {
        if claims_runtime(&id) {
            return false;
        }
        let params = param_names(factory.params);
        let Some(require) = params.get(2) else {
            return true;
        };
        let exports = params.get(1).copied().unwrap_or_default();
        let harmony = factory
            .body
            .statements
            .iter()
            .any(|s| marks(s, require, exports));
        if !harmony && runtime.is_some() && unroutable.is_empty() {
            return true;
        }
        let span = factory.body.span;
        let text = source
            .get(span.start as usize..span.end as usize)
            .unwrap_or(source);
        if !mentions_member(text, require) {
            return true;
        }
        let defined_elsewhere = |name: &str| {
            !harmony
                && runtime.is_some()
                && name != COMPUTED
                && !HOST_OWN.contains(&name)
                && !unroutable.contains(&name)
        };
        member_names(require, factory.body)
            .iter()
            .filter(|name| name.as_str() != BIND)
            .all(|name| routes(name) || defined_elsewhere(name))
    })
}
