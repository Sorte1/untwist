use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::SourceType;
use untwist_core::parse::{has_error, parse_with};

use crate::host_request::{import_source, is_relative, literal_request, HostUse, REQUIRE};
use crate::model::Bundle;
use crate::path::{dirname_posix, join_posix};

const URL_MARKS: [char; 4] = ['%', '?', '#', '\\'];

fn imports_one_file(bundle: &Bundle<'_>, id: &str, request: &str) -> bool {
    let [input] = bundle.input_names.as_slice() else {
        return false;
    };
    let Some(path) = bundle.path_of(id) else {
        return false;
    };
    if request.contains(URL_MARKS) {
        return false;
    }
    let directory = dirname_posix(input);
    let target = join_posix(directory, request);
    let inside = target != ".." && !target.starts_with("../");
    inside
        && dirname_posix(path.as_str()) == directory
        && bundle.mirrored_inputs.contains(&target)
        && target != *input
}

pub fn unproven_import(bundle: &Bundle<'_>, proves: bool, imported_back: bool) -> Option<HostUse> {
    bundle
        .left_imports
        .iter()
        .find(|left| !proves || imported_back || !imports_one_file(bundle, &left.id, &left.request))
        .map(|left| HostUse {
            id: left.id.clone(),
            what: left.what.clone(),
        })
}

#[derive(Default)]
struct Relative {
    requests: Vec<String>,
}

impl Relative {
    fn push(&mut self, request: Option<&str>) {
        if let Some(request) = request.filter(|request| is_relative(request)) {
            self.requests.push(request.to_string());
        }
    }
}

impl<'a> VisitJs<'a> for Relative {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.push(it.source.value.as_str());
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.push(it.source.value.as_str());
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.push(it.source.value.as_str());
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.push(import_source(&it.source));
        walk_js::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let named = matches!(it.callee.without_parentheses(),
            Expression::Identifier(id) if id.name == REQUIRE);
        if let (true, [argument]) = (named, it.arguments.as_slice()) {
            self.push(literal_request(argument));
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn relative_requests(text: &str) -> Option<Vec<String>> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::mjs());
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let mut relative = Relative::default();
    relative.visit_program(&parsed.program);
    Some(relative.requests)
}

pub fn imported_back(bundle: &Bundle<'_>, inputs: &[(&str, &str)]) -> bool {
    inputs.iter().any(|(name, text)| {
        if bundle.input_names.iter().any(|input| input == name) {
            return false;
        }
        let directory = dirname_posix(name);
        relative_requests(text).is_none_or(|requests| {
            requests.iter().any(|request| {
                let target = join_posix(directory, request);
                bundle.input_names.contains(&target)
            })
        })
    })
}
