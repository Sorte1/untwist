use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::SourceType;
use untwist_core::host_globals::NODE_BUILTIN_MODULES;
use untwist_core::parse::{has_error, parse_with};
use untwist_core::semantic::build_scoping;

use crate::browserify::requests::package_name;
use crate::closed::calls_host_require;
use crate::model::{Bundle, BundleKind, Module, RequireParam};
use crate::path::{dirname_posix, join_posix};
use crate::scan::{is_direct_eval, is_free};

const NODE_SCHEME: &str = "node:";
const LOADER_BUILTINS: [&str; 2] = ["module", "process"];
const PROCESS: &str = "process";
const LOADER_MEMBERS: [&str; 2] = ["mainModule", "getBuiltinModule"];
const REQUIRE: &str = "require";

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

fn is_relative(request: &str) -> bool {
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

fn import_source<'k>(source: &'k Expression<'_>) -> Option<&'k str> {
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
    requests: Vec<String>,
    imports: Vec<String>,
    found: Option<String>,
}

impl<'s> HostUses<'s> {
    pub fn new(scoping: &'s Scoping, host_require: bool, shim: Option<&'s str>) -> Self {
        Self {
            scoping,
            host_require,
            shim,
            requests: Vec::new(),
            imports: Vec::new(),
            found: None,
        }
    }

    pub fn into_parts(self) -> (Vec<String>, Vec<String>, Option<String>) {
        (self.requests, self.imports, self.found)
    }

    fn note(&mut self, what: String) {
        self.found.get_or_insert(what);
    }

    fn is_process(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if id.name == PROCESS && is_free(Some(self.scoping), id))
    }

    fn calls_shim(&self, callee: &Expression<'_>) -> bool {
        matches!(callee.without_parentheses(), Expression::Identifier(id)
            if self.shim == Some(id.name.as_str()) && is_free(Some(self.scoping), id))
    }

    fn calls_host(&self, callee: &Expression<'_>) -> bool {
        self.host_require && (calls_host_require(self.scoping, callee) || self.calls_shim(callee))
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
            false => self.requests.push(request.to_string()),
        }
    }

    fn import(&mut self, it: &ImportExpression<'_>) {
        match import_source(&it.source) {
            Some(request) if leaves_alike(request) => self.requests.push(request.to_string()),
            Some(request) if is_relative(request) => self.imports.push(request.to_string()),
            Some(request) => self.note(format!("import({request:?})")),
            None => self.note("import() of a request that is not one literal".to_string()),
        }
    }
}

impl<'a> VisitJs<'a> for HostUses<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_direct_eval(self.scoping, &it.callee) && payload_imports(it) {
            self.note("import() through eval".to_string());
        }
        if !it.optional && self.calls_host(&it.callee) {
            self.request(it);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.import(it);
        walk_js::walk_import_expression(self, it);
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
    shim: Option<&'n str>,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.found |= matches!(name, REQUIRE | PROCESS | "eval") || self.shim == Some(name);
    }

    fn visit_import_expression(&mut self, _it: &ImportExpression<'a>) {
        self.found = true;
    }
}

fn mentions(program: &Program<'_>, shim: Option<&str>) -> bool {
    let mut mentions = Mentions { shim, found: false };
    mentions.visit_program(program);
    mentions.found
}

pub struct HostUse {
    pub id: String,
    pub what: String,
}

pub fn host_uses(bundle: &mut Bundle<'_>) -> Result<(), HostUse> {
    let shim = shim_name(bundle).map(str::to_string);
    let mut left = Vec::new();
    let mut imports = Vec::new();
    for (id, module) in &bundle.modules {
        let program = module.program();
        if !mentions(program, shim.as_deref()) {
            continue;
        }
        let scoping = build_scoping(program).scoping;
        let host_require = names_host_require(bundle, id, module);
        let mut uses = HostUses::new(&scoping, host_require, shim.as_deref());
        uses.visit_program(program);
        let (requests, relative, found) = uses.into_parts();
        if let Some(what) = found {
            return Err(HostUse {
                id: id.clone(),
                what,
            });
        }
        left.extend(
            requests
                .into_iter()
                .map(|request| (id.clone(), Some(request))),
        );
        imports.extend(relative.into_iter().map(|request| (id.clone(), request)));
    }
    bundle.left_requests.extend(left);
    bundle.left_imports.extend(imports);
    Ok(())
}

fn imports_one_file(bundle: &Bundle<'_>, id: &str, request: &str) -> bool {
    let [input] = bundle.input_names.as_slice() else {
        return false;
    };
    let Some(path) = bundle.path_of(id) else {
        return false;
    };
    let directory = dirname_posix(input);
    let target = join_posix(directory, request);
    let inside = target != ".." && !target.starts_with("../");
    inside && dirname_posix(path.as_str()) == directory && bundle.mirrored_inputs.contains(&target)
}

pub fn unproven_import(bundle: &Bundle<'_>) -> Option<HostUse> {
    bundle
        .left_imports
        .iter()
        .find(|(id, request)| !imports_one_file(bundle, id, request))
        .map(|(id, request)| HostUse {
            id: id.clone(),
            what: format!("import({request:?})"),
        })
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
}
