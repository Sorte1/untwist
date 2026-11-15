use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use untwist_core::host_globals::NODE_BUILTIN_MODULES;
use untwist_core::parse::{has_error, parse_with};
use untwist_core::semantic::build_scoping;

use crate::browserify::requests::package_name;
use crate::closed::calls_host_require;
use crate::host_text::{
    creator_import, function_literal, function_text_refused, global_object, member_name,
    runtime_line, script_text_refused, timer_text_refused, RuntimeLine, CODE_BUILTIN, CONSTRUCTOR,
    EVAL, FUNCTION, MODULE, MODULE_MEMBERS, OWN_FILE_NAMES, PROCESS_MEMBERS, REQUIRE_MEMBERS,
    TIMERS,
};
use crate::model::{Bundle, BundleKind, LeftImport, Module, RequireParam};
use crate::scan::{is_direct_eval, is_free};

const NODE_SCHEME: &str = "node:";
const LOADER_BUILTINS: [&str; 2] = ["module", "process"];
const PROCESS: &str = "process";
const LOADER_MEMBERS: [&str; 2] = ["mainModule", "getBuiltinModule"];
pub const REQUIRE: &str = "require";

pub fn reaches_the_loader(request: &str) -> bool {
    let name = request.strip_prefix(NODE_SCHEME).unwrap_or(request);
    LOADER_BUILTINS.contains(&name)
}

pub fn is_builtin(request: &str) -> bool {
    request.starts_with(NODE_SCHEME) || NODE_BUILTIN_MODULES.contains(&request)
}

pub fn leaves_alike(request: &str) -> bool {
    !reaches_the_loader(request) && (is_builtin(request) || package_name(request).is_some())
}

pub fn is_relative(request: &str) -> bool {
    request.starts_with("./") || request.starts_with("../") || request == "." || request == ".."
}

pub fn literal_request<'k>(argument: &'k Argument<'_>) -> Option<&'k str> {
    match argument {
        Argument::StringLiteral(literal) => literal.value.as_str(),
        Argument::TemplateLiteral(template) => {
            template.single_quasi().and_then(|text| text.as_str())
        }
        _ => None,
    }
}

pub fn import_source<'k>(source: &'k Expression<'_>) -> Option<&'k str> {
    match source.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) => {
            template.single_quasi().and_then(|text| text.as_str())
        }
        _ => None,
    }
}

#[derive(Default)]
struct PayloadImports {
    found: bool,
}

impl<'a> VisitJs<'a> for PayloadImports {
    fn visit_import_expression(&mut self, _it: &ImportExpression<'a>) {
        self.found = true;
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found = true;
    }
}

fn payload_imports(call: &CallExpression<'_>) -> bool {
    let [Argument::StringLiteral(payload)] = call.arguments.as_slice() else {
        return false;
    };
    let Some(text) = payload.value.as_str() else {
        return false;
    };
    if !text.contains("import") {
        return false;
    }
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return false;
    }
    let mut imports = PayloadImports::default();
    imports.visit_program(&parsed.program);
    imports.found
}

pub struct HostUses<'s> {
    scoping: &'s Scoping,
    host_require: bool,
    shim: Option<&'s str>,
    checks_own: bool,
    runtime: RuntimeLine,
    cleared: Vec<Span>,
    requests: Vec<String>,
    imports: Vec<(String, String)>,
    found: Option<String>,
}

impl<'s> HostUses<'s> {
    pub fn new(scoping: &'s Scoping, host_require: bool, shim: Option<&'s str>) -> Self {
        Self {
            scoping,
            host_require,
            shim,
            checks_own: false,
            runtime: RuntimeLine {
                spans: Vec::new(),
                creator_uses: 0,
            },
            cleared: Vec::new(),
            requests: Vec::new(),
            imports: Vec::new(),
            found: None,
        }
    }

    pub fn checking_own(self, checks_own: bool, program: Option<&Program<'_>>) -> Self {
        let shim = self.shim;
        let runtime = match program {
            Some(program) => runtime_line(program, shim),
            None => self.runtime,
        };
        Self {
            checks_own,
            runtime,
            ..self
        }
    }

    pub fn into_parts(self) -> (Vec<String>, Vec<(String, String)>, Option<String>) {
        (self.requests, self.imports, self.found)
    }

    fn note(&mut self, what: String) {
        self.found.get_or_insert(what);
    }

    fn own(&mut self, what: impl FnOnce() -> String) {
        if self.checks_own {
            self.found.get_or_insert_with(what);
        }
    }

    fn clear(&mut self, span: Span) {
        if self.checks_own {
            self.cleared.push(span);
        }
    }

    fn is_cleared(&self, span: Span) -> bool {
        self.cleared.contains(&span)
            || self
                .runtime
                .spans
                .iter()
                .any(|line| line.contains_inclusive(span))
    }

    fn free(&self, expr: &Expression<'_>, name: &str) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if id.name == name && is_free(Some(self.scoping), id))
    }

    fn is_process(&self, expr: &Expression<'_>) -> bool {
        self.free(expr, PROCESS)
    }

    fn calls_shim(&self, callee: &Expression<'_>) -> bool {
        matches!(callee.without_parentheses(), Expression::Identifier(id)
            if self.shim == Some(id.name.as_str()) && is_free(Some(self.scoping), id))
    }

    fn calls_host(&self, callee: &Expression<'_>) -> bool {
        self.host_require && (calls_host_require(self.scoping, callee) || self.calls_shim(callee))
    }

    fn leaves(&mut self, request: &str, what: impl FnOnce() -> String) {
        if self.checks_own && runs_code_texts(request) {
            self.note(format!("{}, {CODE_BUILTIN_TEXTS}", what()));
        }
        self.requests.push(request.to_string());
    }

    fn request(&mut self, call: &CallExpression<'_>) {
        let [argument] = call.arguments.as_slice() else {
            return;
        };
        let Some(request) = literal_request(argument) else {
            return;
        };
        if is_relative(request) {
            return;
        }
        match reaches_the_loader(request) {
            true => self.note(format!("require({request:?})")),
            false => self.leaves(request, || format!("require({request:?})")),
        }
    }

    fn import(&mut self, request: Option<&str>, what: impl Fn(&str) -> String) {
        match request {
            Some(request) if leaves_alike(request) => self.leaves(request, || what(request)),
            Some(request) if is_relative(request) => {
                self.imports.push((request.to_string(), what(request)));
            }
            Some(request) => self.note(what(request)),
            None => self.note("import() of a request that is not one literal".to_string()),
        }
    }

    fn declared_import(&mut self, source: &StringLiteral<'_>) {
        self.import(source.value.as_str(), |request| {
            format!("an import from {request:?}")
        });
    }

    fn runner_call(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) {
        let callee = callee.without_parentheses();
        let Some(refused) = self.runner(callee, arguments) else {
            return;
        };
        self.clear(callee.span());
        if let Some((runner, why)) = refused {
            self.own(|| format!("{runner} with {why}"));
        }
    }

    fn runner(&mut self, callee: &Expression<'_>, arguments: &[Argument<'_>]) -> Option<Refused> {
        match callee {
            Expression::Identifier(id) if !is_free(Some(self.scoping), id) => None,
            Expression::Identifier(id) if id.name == FUNCTION => {
                Some(function_text_refused(arguments).map(|why| (FUNCTION, why)))
            }
            Expression::Identifier(id) if id.name == EVAL => Some(None),
            Expression::Identifier(id) if TIMERS.contains(&id.name.as_str()) => {
                Some(timer_text_refused(arguments.first()).map(|why| ("a string timer", why)))
            }
            Expression::SequenceExpression(sequence) => {
                let last = sequence.expressions.last()?;
                if !self.free(last, EVAL) {
                    return None;
                }
                self.clear(last.without_parentheses().span());
                Some(script_text_refused(arguments.first()).map(|why| ("indirect eval", why)))
            }
            other => self.member_runner(other, arguments),
        }
    }

    fn member_runner(
        &mut self,
        callee: &Expression<'_>,
        arguments: &[Argument<'_>],
    ) -> Option<Refused> {
        let member = callee.as_member_expression()?;
        let name = member_name(member)?;
        let on_global = global_object(member.object()).is_some();
        match name {
            CONSTRUCTOR if function_literal(member.object()) => {
                Some(function_text_refused(arguments).map(|why| ("a function's constructor", why)))
            }
            FUNCTION if on_global => {
                Some(function_text_refused(arguments).map(|why| (FUNCTION, why)))
            }
            EVAL if on_global => {
                Some(script_text_refused(arguments.first()).map(|why| ("indirect eval", why)))
            }
            timer if on_global && TIMERS.contains(&timer) => {
                Some(timer_text_refused(arguments.first()).map(|why| ("a string timer", why)))
            }
            _ => None,
        }
    }

    fn own_member(&mut self, member: &MemberExpression<'_>) {
        if !self.checks_own || self.is_cleared(member.span()) {
            return;
        }
        let object = member.object();
        let name = member_name(member);
        let refused = match name {
            Some(CONSTRUCTOR) if function_literal(object) => Some("a function's constructor"),
            Some(FUNCTION | EVAL) if global_object(object).is_some() => Some("a code runner"),
            _ => None,
        };
        if let Some(what) = refused {
            self.note(what.to_string());
            return;
        }
        let allowed: &[&str] = if self.is_process(object) {
            &PROCESS_MEMBERS
        } else if self.free(object, MODULE) {
            &MODULE_MEMBERS
        } else if self.host_require && self.free(object, REQUIRE) {
            &REQUIRE_MEMBERS
        } else {
            return;
        };
        if name.is_none_or(|name| !allowed.contains(&name)) {
            let holder = match object.without_parentheses() {
                Expression::Identifier(id) => id.name.as_str(),
                _ => "",
            };
            self.note(format!("{holder}.{}", name.unwrap_or("[…]")));
        }
    }
}

type Refused = Option<(&'static str, &'static str)>;

const CODE_BUILTIN_TEXTS: &str = "a built-in that runs code texts";

fn runs_code_texts(request: &str) -> bool {
    request.strip_prefix(NODE_SCHEME).unwrap_or(request) == CODE_BUILTIN
}

impl<'a> VisitJs<'a> for HostUses<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_direct_eval(&it.callee) && payload_imports(it) {
            self.note("import() through eval".to_string());
        }
        if !it.optional && self.calls_host(&it.callee) {
            self.request(it);
        }
        if self.checks_own {
            self.runner_call(&it.callee, &it.arguments);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if self.checks_own {
            self.runner_call(&it.callee, &it.arguments);
        }
        walk_js::walk_new_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.import(import_source(&it.source), |request| {
            format!("import({request:?})")
        });
        walk_js::walk_import_expression(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        let runtime = creator_import(it)
            && it.specifiers.iter().flatten().all(|specifier| {
                let symbol = specifier.local().symbol_id();
                self.scoping.get_resolved_reference_ids(symbol).len() <= self.runtime.creator_uses
            });
        if !runtime {
            self.declared_import(&it.source);
        }
        walk_js::walk_import_declaration(self, it);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.declared_import(&it.source);
        walk_js::walk_export_from_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.declared_import(&it.source);
        walk_js::walk_export_all_declaration(self, it);
    }

    fn visit_import_meta(&mut self, it: &ImportMeta) {
        if !self.is_cleared(it.span) {
            self.own(|| "import.meta".to_string());
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if !self.checks_own || !is_free(Some(self.scoping), it) {
            return;
        }
        let name = it.name.as_str();
        let runner = (name == FUNCTION || name == EVAL) && !self.is_cleared(it.span);
        if runner || OWN_FILE_NAMES.contains(&name) {
            self.note(name.to_string());
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Typeof {
            self.clear(it.argument.without_parentheses().span());
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if it.operator.is_equality() || it.operator == BinaryOperator::Instanceof {
            for side in [&it.left, &it.right] {
                if self.free(side, FUNCTION) {
                    self.clear(side.without_parentheses().span());
                }
            }
        }
        walk_js::walk_binary_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if self.free(it.object(), FUNCTION) {
            self.clear(it.object().without_parentheses().span());
        }
        self.own_member(it);
        walk_js::walk_member_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.is_process(&it.object) && LOADER_MEMBERS.contains(&it.property.name.as_str()) {
            self.note(format!("process.{}", it.property.name));
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_process(&it.object) {
            let named = match it.expression.without_parentheses() {
                Expression::StringLiteral(key) => key.value.as_str(),
                _ => None,
            };
            if named.is_none_or(|name| LOADER_MEMBERS.contains(&name)) {
                self.note("a computed member of process".to_string());
            }
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

fn names_host_require(bundle: &Bundle<'_>, id: &str, module: &Module<'_>) -> bool {
    match bundle.kind {
        BundleKind::Webpack => module.runtime_require != RequireParam::Default,
        BundleKind::Browserify => false,
        BundleKind::Parcel => bundle
            .fresh_requires
            .iter()
            .find(|(fresh, _)| fresh == id)
            .is_none_or(|(_, name)| name != REQUIRE),
        _ => true,
    }
}

fn shim_name<'b>(bundle: &'b Bundle<'_>) -> Option<&'b str> {
    match bundle.kind {
        BundleKind::Esbuild | BundleKind::Rolldown => bundle
            .runtime
            .as_ref()
            .map(|runtime| runtime.require_name.as_str()),
        _ => None,
    }
}

struct Mentions<'n> {
    host_require: bool,
    shim: Option<&'n str>,
    checks_own: bool,
    found: bool,
}

impl Mentions<'_> {
    fn own_member(&self, member: &MemberExpression<'_>) -> bool {
        let object = member.object().without_parentheses();
        let name = member_name(member);
        let allowed: &[&str] = match object {
            Expression::Identifier(id) if id.name == PROCESS => &PROCESS_MEMBERS,
            Expression::Identifier(id) if id.name == MODULE => &MODULE_MEMBERS,
            Expression::Identifier(id) if id.name == REQUIRE => &REQUIRE_MEMBERS,
            _ if global_object(object).is_some() => {
                return name
                    .is_none_or(|name| [FUNCTION, EVAL].contains(&name) || TIMERS.contains(&name));
            }
            _ => return name == Some(CONSTRUCTOR) && function_literal(object),
        };
        name.is_none_or(|name| !allowed.contains(&name))
    }
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        let host = self.host_require && (name == REQUIRE || self.shim == Some(name));
        let own = self.checks_own
            && (name == FUNCTION || OWN_FILE_NAMES.contains(&name) || TIMERS.contains(&name));
        self.found |= host || own || name == EVAL;
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.found |= self.checks_own && self.own_member(it);
        walk_js::walk_member_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.found |=
            names_process(&it.object) && LOADER_MEMBERS.contains(&it.property.name.as_str());
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.found |= names_process(&it.object);
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_import_expression(&mut self, _it: &ImportExpression<'a>) {
        self.found = true;
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found |= self.checks_own;
    }
}

fn names_process(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == PROCESS)
}

fn declares_imports(program: &Program<'_>) -> bool {
    program.body.iter().any(|statement| {
        matches!(
            statement,
            Statement::ImportDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::ExportFromDeclaration(_)
        )
    })
}

fn mentions(
    program: &Program<'_>,
    host_require: bool,
    shim: Option<&str>,
    checks_own: bool,
) -> bool {
    if declares_imports(program) {
        return true;
    }
    let mut mentions = Mentions {
        host_require,
        shim,
        checks_own,
        found: false,
    };
    mentions.visit_program(program);
    mentions.found
}

pub struct HostUse {
    pub id: String,
    pub what: String,
}

pub struct BesideUses {
    pub requests: Vec<String>,
    pub imports: Vec<(String, String)>,
}

pub fn beside_uses<'a>(
    statements: &[&Statement<'a>],
    scoping: &Scoping,
    checks_own: bool,
) -> Result<BesideUses, String> {
    let mut uses = HostUses::new(scoping, false, None).checking_own(checks_own, None);
    for statement in statements {
        uses.visit_statement(statement);
    }
    let (requests, imports, found) = uses.into_parts();
    match found {
        Some(what) => Err(what),
        None => Ok(BesideUses { requests, imports }),
    }
}

type Left = (Vec<(String, Option<String>)>, Vec<LeftImport>);

fn record(left: &mut Left, id: &str, requests: &[String], imports: &[(String, String)]) {
    left.0.extend(
        requests
            .iter()
            .map(|request| (id.to_string(), Some(request.clone()))),
    );
    left.1
        .extend(imports.iter().map(|(request, what)| LeftImport {
            id: id.to_string(),
            request: request.clone(),
            what: what.clone(),
        }));
}

pub fn host_uses(bundle: &mut Bundle<'_>, checks_own: bool) -> Result<(), HostUse> {
    let shim = shim_name(bundle).map(str::to_string);
    let mut left: Left = (Vec::new(), Vec::new());
    for (id, module) in &bundle.modules {
        if bundle.fixed_paths.contains(id) {
            continue;
        }
        let program = module.program();
        let host_require = names_host_require(bundle, id, module);
        if !mentions(program, host_require, shim.as_deref(), checks_own) {
            continue;
        }
        let scoping = build_scoping(program).scoping;
        let mut uses = HostUses::new(&scoping, host_require, shim.as_deref())
            .checking_own(checks_own, Some(program));
        uses.visit_program(program);
        let (requests, relative, found) = uses.into_parts();
        if let Some(what) = found {
            return Err(HostUse {
                id: id.clone(),
                what,
            });
        }
        record(&mut left, id, &requests, &relative);
    }
    if let (Some(entry), Some(beside)) = (bundle.entry_id.as_deref(), bundle.beside_uses()) {
        record(&mut left, entry, &beside.requests, &beside.imports);
    }
    bundle.left_requests.extend(left.0);
    bundle.left_imports.extend(left.1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_built_ins_off_the_loader_and_packages_leave_alike() {
        for request in [
            "fs",
            "node:fs",
            "node:path",
            "foo",
            "@scope/pkg/x.js",
            "core-js/x",
        ] {
            assert!(leaves_alike(request), "{request}");
        }
        for request in [
            "module",
            "node:module",
            "process",
            "node:process",
            "./a",
            "../a",
            "/abs",
            ".",
        ] {
            assert!(!leaves_alike(request), "{request}");
        }
    }

    fn uses(text: &str, host_require: bool) -> (Vec<String>, Option<String>) {
        let alloc = Allocator::default();
        let parsed = parse_with(&alloc, text, SourceType::cjs());
        let scoping = build_scoping(&parsed.program).scoping;
        let mut uses = HostUses::new(&scoping, host_require, None);
        uses.visit_program(&parsed.program);
        let (requests, _, found) = uses.into_parts();
        (requests, found)
    }

    #[test]
    fn imports_leave_only_as_packages_or_built_ins() {
        assert_eq!(uses("import('fs'); import('foo');", false).0, ["fs", "foo"]);
        assert!(uses("import(name);", false).1.is_some());
        assert!(uses("import('module');", false).1.is_some());
        assert!(uses("eval(\"import('./a.js')\");", false).1.is_some());
        assert!(uses("eval(\"Math;\");", false).1.is_none());
    }

    #[test]
    fn host_requests_are_collected_and_loader_routes_refused() {
        assert_eq!(uses("require('foo'); require('./a');", true).0, ["foo"]);
        assert!(uses("require('foo');", false).0.is_empty());
        assert!(uses("require('node:module');", true).1.is_some());
        assert!(uses("process.mainModule;", false).1.is_some());
        assert!(uses("process.getBuiltinModule('fs');", false).1.is_some());
        assert!(uses("process[key];", false).1.is_some());
        assert!(uses("process['env'].X; process.env.Y;", false).1.is_none());
        assert!(uses("var process = {}; process.mainModule;", false)
            .1
            .is_none());
    }

    fn own_found(text: &str) -> Option<String> {
        let alloc = Allocator::default();
        let parsed = parse_with(&alloc, text, SourceType::mjs());
        assert!(!has_error(&parsed.diagnostics), "{text}");
        let scoping = build_scoping(&parsed.program).scoping;
        let mut uses = HostUses::new(&scoping, true, Some("__require"))
            .checking_own(true, Some(&parsed.program));
        uses.visit_program(&parsed.program);
        uses.into_parts().2
    }

    #[test]
    fn static_imports_take_the_route_of_import_calls() {
        let alloc = Allocator::default();
        let text = "import a from 'fs'; import b from 'foo'; export * from './x.js'; export { c } from './y.js';";
        let parsed = parse_with(&alloc, text, SourceType::mjs());
        let scoping = build_scoping(&parsed.program).scoping;
        let mut uses = HostUses::new(&scoping, false, None);
        uses.visit_program(&parsed.program);
        let (requests, relative, found) = uses.into_parts();
        assert_eq!(requests, ["fs", "foo"]);
        assert_eq!(relative.len(), 2);
        assert!(found.is_none());
        for text in [
            "import M from 'module';",
            "import 'node:process';",
            "export * from '/abs.js';",
        ] {
            assert!(own_found(text).is_some(), "{text}");
        }
    }

    #[test]
    fn the_recorded_runtime_line_may_import_create_require() {
        for text in [
            "import { createRequire } from 'node:module'; var __require = createRequire(import.meta.url);",
            "import { createRequire } from 'module'; const require = createRequire(import.meta.url);",
            "import { createRequire } from 'node:module'; var __require = (() => createRequire(import.meta.url))();",
        ] {
            assert_eq!(own_found(text), None, "{text}");
        }
        for text in [
            "import { createRequire } from 'node:module'; const r = createRequire(import.meta.url);",
            "import { createRequire } from 'node:module'; var __require = createRequire(import.meta.url); createRequire('/x');",
            "import M from 'node:module';",
        ] {
            assert!(own_found(text).is_some(), "{text}");
        }
    }

    #[test]
    fn code_runners_pass_only_with_a_constant_text_clear_of_the_host() {
        for text in [
            "Function('return this')();",
            "new Function('a', 'return a')(1);",
            "setTimeout(function () {}, 1);",
            "x instanceof Function; typeof Function; Function.prototype.call;",
            "(0, eval)('1 + 1');",
            "eval('1');",
            "globalThis.setTimeout(f);",
        ] {
            assert_eq!(own_found(text), None, "{text}");
        }
        for text in [
            "Function(\"return import('./1.js')\")();",
            "Function(code)();",
            "(0, eval)(\"import('./1.js')\");",
            "(0, eval)(code);",
            "var e = eval;",
            "var F = Function;",
            "(function () {}).constructor('return process')();",
            "var F = (function () {}).constructor;",
            "[]['filter']['constructor']('return require')();",
            "setTimeout(\"import('./a.js')\");",
            "setTimeout('a' + b);",
            "window.eval(code);",
            "require('vm');",
            "import('node:vm');",
        ] {
            assert!(own_found(text).is_some(), "{text}");
        }
    }

    #[test]
    fn own_file_names_are_read_only_from_the_allow_list() {
        for text in [
            "typeof import.meta;",
            "process.env.NODE_ENV; process.nextTick(f); process.platform;",
            "module.exports = 1; typeof module;",
            "typeof require;",
        ] {
            assert_eq!(own_found(text), None, "{text}");
        }
        for text in [
            "import.meta.url;",
            "import.meta.filename;",
            "import.meta.dirname;",
            "__filename;",
            "__dirname;",
            "require.main;",
            "require.resolve('x');",
            "module.filename;",
            "module.id;",
            "module.parent;",
            "process.argv[1];",
            "process.binding('fs');",
            "process['argv'];",
        ] {
            assert!(own_found(text).is_some(), "{text}");
        }
    }
}
