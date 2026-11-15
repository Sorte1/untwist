use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::FxHashSet;
use untwist_core::parse::{has_error, parse_with};

use crate::esbuild::layout::require_creators;
use crate::rolldown::shape::require_shim;

pub const FUNCTION: &str = "Function";
pub const EVAL: &str = "eval";
pub const CONSTRUCTOR: &str = "constructor";
pub const TIMERS: [&str; 2] = ["setTimeout", "setInterval"];
pub const GLOBAL_OBJECTS: [&str; 4] = ["globalThis", "window", "self", "global"];
pub const OWN_FILE_NAMES: [&str; 2] = ["__filename", "__dirname"];
pub const MODULE: &str = "module";
pub const MODULE_MEMBERS: [&str; 3] = ["exports", "hot", "loaded"];
pub const REQUIRE_MEMBERS: [&str; 1] = ["apply"];
pub const CODE_BUILTIN: &str = "vm";
pub const PROCESS_MEMBERS: [&str; 40] = [
    "env",
    "nextTick",
    "platform",
    "version",
    "versions",
    "browser",
    "cwd",
    "emitWarning",
    "exit",
    "exitCode",
    "on",
    "once",
    "off",
    "emit",
    "addListener",
    "removeListener",
    "removeAllListeners",
    "listeners",
    "listenerCount",
    "prependListener",
    "stdout",
    "stderr",
    "stdin",
    "hrtime",
    "title",
    "pid",
    "ppid",
    "arch",
    "memoryUsage",
    "cpuUsage",
    "uptime",
    "release",
    "umask",
    "features",
    "config",
    "kill",
    "noDeprecation",
    "throwDeprecation",
    "traceDeprecation",
    "domain",
];
const TEXT_NAMES: [&str; 12] = [
    "require",
    "module",
    "exports",
    "process",
    "__filename",
    "__dirname",
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "globalThis",
    "global",
];
const RUNTIME_DECLARED: [&str; 2] = ["require", "__require"];
const NODE_MODULE: [&str; 2] = ["module", "node:module"];
const CREATE_REQUIRE: &str = "createRequire";

#[derive(Default)]
struct TextUses {
    found: bool,
}

impl<'a> VisitJs<'a> for TextUses {
    fn visit_import_expression(&mut self, _it: &ImportExpression<'a>) {
        self.found = true;
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= TEXT_NAMES.contains(&it.name.as_str());
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.found |= it.property.name == CONSTRUCTOR;
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.found |= !matches!(it.expression.without_parentheses(),
            Expression::StringLiteral(key) if key.value != CONSTRUCTOR);
        walk_js::walk_computed_member_expression(self, it);
    }
}

fn reaches_host(text: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return true;
    }
    let mut uses = TextUses::default();
    uses.visit_program(&parsed.program);
    uses.found
}

fn constant_text<'k>(argument: &'k Argument<'_>) -> Option<&'k str> {
    match argument {
        Argument::StringLiteral(literal) => literal.value.as_str(),
        Argument::TemplateLiteral(template) => {
            template.single_quasi().and_then(|text| text.as_str())
        }
        _ => None,
    }
}

pub fn function_text_refused(arguments: &[Argument<'_>]) -> Option<&'static str> {
    let texts: Option<Vec<&str>> = arguments.iter().map(constant_text).collect();
    let Some(texts) = texts else {
        return Some("a code text that is not constant");
    };
    let (body, params) = texts.split_last()?;
    let source = format!(
        "(function anonymous({}\n) {{\n{body}\n}})",
        params.join(",")
    );
    reaches_host(&source).then_some("a code text that reaches the host")
}

pub fn script_text_refused(argument: Option<&Argument<'_>>) -> Option<&'static str> {
    let argument = argument?;
    match constant_text(argument) {
        Some(text) => reaches_host(text).then_some("a code text that reaches the host"),
        None => Some("a code text that is not constant"),
    }
}

pub fn timer_text_refused(argument: Option<&Argument<'_>>) -> Option<&'static str> {
    match argument? {
        Argument::StringLiteral(_) => script_text_refused(argument),
        Argument::TemplateLiteral(template) if template.expressions.is_empty() => {
            script_text_refused(argument)
        }
        Argument::TemplateLiteral(_) | Argument::BinaryExpression(_) => {
            Some("a code text that is not constant")
        }
        _ => None,
    }
}

fn names_a_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::ObjectExpression(_)
    )
}

pub fn function_literal(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::ClassExpression(_) => true,
        Expression::StaticMemberExpression(member) => names_a_literal(&member.object),
        Expression::ComputedMemberExpression(member) => names_a_literal(&member.object),
        Expression::CallExpression(call) => {
            let prototype_of = matches!(call.callee.without_parentheses(),
                Expression::StaticMemberExpression(member)
                    if member.property.name == "getPrototypeOf");
            prototype_of
                && call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_some_and(function_literal)
        }
        _ => false,
    }
}

pub fn global_object<'k>(expr: &'k Expression<'_>) -> Option<&'k str> {
    match expr.without_parentheses() {
        Expression::Identifier(id) if GLOBAL_OBJECTS.contains(&id.name.as_str()) => {
            Some(id.name.as_str())
        }
        _ => None,
    }
}

pub fn member_name<'k>(member: &'k MemberExpression<'_>) -> Option<&'k str> {
    match member {
        MemberExpression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(member) => {
            match member.expression.without_parentheses() {
                Expression::StringLiteral(key) => key.value.as_str(),
                _ => None,
            }
        }
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

#[derive(Default)]
struct Creators<'n> {
    names: Vec<&'n str>,
    count: usize,
}

impl<'a> VisitJs<'a> for Creators<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.count += usize::from(self.names.contains(&it.name.as_str()));
    }
}

pub struct RuntimeLine {
    pub spans: Vec<Span>,
    pub creator_uses: usize,
}

pub fn runtime_line(program: &Program<'_>, shim: Option<&str>) -> RuntimeLine {
    let creators: FxHashSet<&str> = require_creators(&program.body);
    let mut spans = Vec::new();
    let mut counted = Creators {
        names: creators.iter().copied().collect(),
        count: 0,
    };
    if creators.is_empty() {
        return RuntimeLine {
            spans,
            creator_uses: 0,
        };
    }
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            let named = matches!(&declarator.id,
                BindingPattern::BindingIdentifier(id)
                    if RUNTIME_DECLARED.contains(&id.name.as_str()) || shim == Some(id.name.as_str()));
            let Some(init) = declarator.init.as_ref().filter(|_| named) else {
                continue;
            };
            if require_shim(&creators, init) {
                spans.push(init.span());
                counted.visit_expression(init);
            }
        }
    }
    RuntimeLine {
        spans,
        creator_uses: counted.count,
    }
}

pub fn creator_import(import: &ImportDeclaration<'_>) -> bool {
    let from_module = import
        .source
        .value
        .as_str()
        .is_some_and(|source| NODE_MODULE.contains(&source));
    let only_creators = import.specifiers.iter().flatten().all(|specifier| {
        matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(named)
            if named.imported.name() == CREATE_REQUIRE)
    });
    from_module && only_creators && import.import_kind.is_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments_of(text: &str, check: fn(&[Argument<'_>]) -> Option<&'static str>) -> bool {
        let alloc = Allocator::default();
        let parsed = parse_with(&alloc, text, SourceType::script());
        let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.first() else {
            panic!("an expression statement");
        };
        let Expression::CallExpression(call) = &statement.expression else {
            panic!("a call");
        };
        check(&call.arguments).is_some()
    }

    #[test]
    fn a_constant_text_free_of_host_names_passes() {
        assert!(!arguments_of(
            "Function('return this');",
            function_text_refused
        ));
        assert!(!arguments_of(
            "Function('a', 'b', 'return a + b');",
            function_text_refused
        ));
        assert!(!arguments_of("Function();", function_text_refused));
    }

    #[test]
    fn a_text_that_reaches_the_host_or_is_not_constant_refuses() {
        for text in [
            "Function(\"return import('./1.js')\");",
            "Function('return process.mainModule');",
            "Function('return require');",
            "Function('return import.meta');",
            "Function('return (function(){}).constructor(\"x\")');",
            "Function('return this[k]');",
            "Function(text);",
            "Function('a', body);",
            "Function('return (');",
        ] {
            assert!(arguments_of(text, function_text_refused), "{text}");
        }
    }

    #[test]
    fn timers_refuse_only_text_that_reaches_the_host() {
        let first = |arguments: &[Argument<'_>]| timer_text_refused(arguments.first());
        assert!(!arguments_of("setTimeout(f, 1);", first));
        assert!(!arguments_of("setTimeout('1');", first));
        assert!(arguments_of("setTimeout(\"import('./a.js')\");", first));
        assert!(arguments_of("setTimeout('x' + y);", first));
    }
}
