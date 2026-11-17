use oxc_ast::ast::*;

use super::reach::{key_of, member_of, Key};
use super::Scan;

const RUNNER_MODULES: [&str; 5] = ["vm", "module", "worker_threads", "child_process", "repl"];

const NODE_SCHEME: &str = "node:";

const TEXT_MODULE_SCHEMES: [&str; 3] = ["data:", "blob:", "javascript:"];

pub(super) const LOADERS: [&str; 3] = ["require", "getBuiltinModule", "createRequire"];

const REQUIRE: &str = "require";

const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];

fn is_text_module(specifier: &str) -> bool {
    let lower = specifier.trim_start().to_ascii_lowercase();
    TEXT_MODULE_SCHEMES
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

fn is_runner_module(specifier: &str) -> bool {
    let name = specifier.strip_prefix(NODE_SCHEME).unwrap_or(specifier);
    RUNNER_MODULES.contains(&name)
}

pub(super) fn is_safe_specifier(specifier: &str) -> bool {
    !is_text_module(specifier) && !is_runner_module(specifier)
}

pub(super) fn names_a_safe_module(source: Option<&Expression<'_>>) -> bool {
    source.is_some_and(|source| match key_of(source) {
        Key::Name(specifier) => is_safe_specifier(specifier),
        Key::Other | Key::Unknown(_) => false,
    })
}

fn is_require(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::Identifier(id) if id.name.as_str() == REQUIRE
    )
}

impl Scan<'_> {
    fn member_name(&self, key: Key<'_>) -> Option<String> {
        match key {
            Key::Name(name) => Some(name.to_string()),
            Key::Other => None,
            Key::Unknown(expression) => self.known_key(expression),
        }
    }

    pub(super) fn names_a_loader(&self, key: Key<'_>) -> bool {
        self.member_name(key)
            .is_some_and(|name| LOADERS.contains(&name.as_str()))
    }

    pub(super) fn calls_require(&self, callee: &Expression<'_>) -> bool {
        is_require(callee)
            || member_of(callee)
                .is_some_and(|(_, key)| self.member_name(key).as_deref() == Some(REQUIRE))
    }

    pub(super) fn reads_off_require(&self, object: &Expression<'_>, key: Key<'_>) -> bool {
        is_require(object)
            && self
                .member_name(key)
                .is_some_and(|name| !FORWARDERS.contains(&name.as_str()))
    }

    pub(super) fn is_require_name(&self, expression: &Expression<'_>) -> bool {
        is_require(expression)
    }
}
