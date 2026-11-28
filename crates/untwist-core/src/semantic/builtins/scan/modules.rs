use oxc_ast::ast::*;

use super::reach::{key_of, member_of, Key};
use super::Scan;

const RUNNER_MODULES: [&str; 5] = ["vm", "module", "worker_threads", "child_process", "repl"];

const PROCESS_MODULE: &str = "process";

const NODE_SCHEME: &str = "node";

const EXTERNAL_SCHEMES: [&str; 3] = ["file", "http", "https"];

const DEFAULT_EXPORT: &str = "default";

pub(super) const LOADERS: [&str; 3] = ["require", "getBuiltinModule", "createRequire"];

const REQUIRE: &str = "require";

const BUILTIN_LOADER: &str = "getBuiltinModule";

const FORWARDERS: [&str; 3] = ["call", "apply", "bind"];

fn url_whitespace(character: char) -> bool {
    character <= ' '
}

fn normalised(specifier: &str) -> String {
    specifier
        .trim_matches(url_whitespace)
        .chars()
        .filter(|character| !matches!(character, '\t' | '\n' | '\r'))
        .collect::<String>()
        .to_ascii_lowercase()
}

fn url_scheme(specifier: &str) -> Option<&str> {
    let (scheme, _) = specifier.split_once(':')?;
    let mut characters = scheme.chars();
    let first = characters.next()?;
    let valid = first.is_ascii_alphabetic()
        && characters
            .all(|character| character.is_ascii_alphanumeric() || "+-.".contains(character));
    valid.then_some(scheme)
}

fn module_name(specifier: &str) -> &str {
    let name = specifier
        .strip_prefix(NODE_SCHEME)
        .and_then(|rest| rest.strip_prefix(':'))
        .unwrap_or(specifier);
    let name = name.split(['?', '#']).next().unwrap_or(name);
    name.trim_end_matches('/')
}

fn runs_code(specifier: &str) -> bool {
    let known_scheme = url_scheme(specifier)
        .is_none_or(|scheme| scheme == NODE_SCHEME || EXTERNAL_SCHEMES.contains(&scheme));
    !known_scheme || RUNNER_MODULES.contains(&module_name(specifier))
}

fn is_process(specifier: &str) -> bool {
    module_name(&normalised(specifier)) == PROCESS_MODULE
}

pub(super) fn is_safe_specifier(specifier: &str) -> bool {
    !runs_code(&normalised(specifier))
}

pub(super) fn names_a_safe_module(source: Option<&Expression<'_>>) -> bool {
    source.is_some_and(|source| match key_of(source) {
        Key::Name(specifier) => is_safe_specifier(specifier) && !is_process(specifier),
        Key::Other | Key::Unknown(_) => false,
    })
}

fn names_process(source: Option<&Expression<'_>>) -> bool {
    source.is_some_and(
        |source| matches!(key_of(source), Key::Name(specifier) if is_process(specifier)),
    )
}

pub(super) fn source_is_process(source: &StringLiteral<'_>) -> bool {
    source.value.as_str().is_none_or(is_process)
}

fn export_name<'n>(name: &'n ModuleExportName<'_>) -> Option<&'n str> {
    match name {
        ModuleExportName::IdentifierName(id) => Some(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => Some(id.name.as_str()),
        ModuleExportName::StringLiteral(literal) => literal.value.as_str(),
    }
}

fn is_require(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::Identifier(id) if id.name.as_str() == REQUIRE
    )
}

impl Scan<'_> {
    pub(super) fn judge_module_name(&mut self, name: &ModuleExportName<'_>, process: bool) {
        let reaches = export_name(name).is_none_or(|name| {
            self.pattern_name_reaches(name) || (process && name == DEFAULT_EXPORT)
        });
        if reaches {
            self.not_allowed();
        }
    }

    pub(super) fn requires_process(&self, call: &CallExpression<'_>) -> bool {
        self.calls_require(&call.callee)
            && names_process(call.arguments.first().and_then(Argument::as_expression))
    }

    pub(super) fn reads_host_name(&self, key: Key<'_>) -> bool {
        self.member_name(key).is_some_and(|name| {
            self.read_allowed(super::allow::Root::Host(PROCESS_MODULE), Key::Name(&name))
        })
    }

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

    pub(super) fn is_builtin_loader(&self, expression: &Expression<'_>) -> bool {
        let Some((object, key)) = member_of(expression) else {
            return false;
        };
        let named = match key {
            Key::Name(name) => name == BUILTIN_LOADER,
            Key::Other => false,
            Key::Unknown(_) => self.member_name(key).as_deref() == Some(BUILTIN_LOADER),
        };
        named
            && matches!(
                self.allow_root(object),
                Some(super::allow::Root::Host(PROCESS_MODULE))
            )
    }

    pub(super) fn loads_a_safe_builtin(&self, call: &CallExpression<'_>) -> bool {
        !call.optional
            && call.arguments.len() == 1
            && self.is_builtin_loader(&call.callee)
            && names_a_safe_module(call.arguments.first().and_then(Argument::as_expression))
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
