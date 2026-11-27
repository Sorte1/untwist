use std::collections::HashSet;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;

const OWN_MARKERS: [&str; 6] = [
    "__filename",
    "import.meta",
    "require.main",
    "module.filename",
    "process.argv",
    "mainModule",
];
const READ_METHODS: [&str; 22] = [
    "readFileSync",
    "readFile",
    "openSync",
    "open",
    "createReadStream",
    "statSync",
    "stat",
    "lstatSync",
    "lstat",
    "watch",
    "watchFile",
    "copyFileSync",
    "copyFile",
    "cpSync",
    "exec",
    "execSync",
    "execFile",
    "execFileSync",
    "spawn",
    "spawnSync",
    "fork",
    "fetch",
];
const OWN_NAME: &str = "__filename";
const REQUIRE: &str = "require";
const PROCESS: &str = "process";
const MODULE: &str = "module";
const ARGV: &str = "argv";
const SLICE: &str = "slice";
const LENGTH: &str = "length";
const DIRNAME: &str = "dirname";
const RUN_IN_THIS_CONTEXT: &str = "runInThisContext";
const READ_FILE_SYNC: &str = "readFileSync";
const SCRIPT_INDEX: u32 = 1;
const FIRST_USER_ARGUMENT: u32 = 2;
const ROUNDS: usize = 8;

fn whole(value: f64) -> Option<u32> {
    let rounded = value as u32;
    (f64::from(rounded) == value).then_some(rounded)
}

fn index_of(expression: &Expression<'_>) -> Option<u32> {
    match expression.without_parentheses() {
        Expression::NumericLiteral(number) => whole(number.value),
        _ => None,
    }
}

fn identifier<'k>(expression: &'k Expression<'_>) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn callee_name<'k>(callee: &'k Expression<'_>) -> Option<&'k str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

fn is_argv(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::StaticMemberExpression(member)
            if member.property.name == ARGV && identifier(&member.object) == Some(PROCESS)
    )
}

fn host_member(object: &Expression<'_>, property: &str) -> bool {
    matches!(
        (identifier(object), property),
        (Some(REQUIRE), "main")
            | (Some(MODULE), "filename")
            | (Some(PROCESS), "mainModule")
            | (Some(PROCESS), ARGV)
    )
}

fn user_arguments(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(callee) = call.callee.without_parentheses() else {
        return false;
    };
    let from = call.arguments.first().and_then(|first| match first {
        Argument::NumericLiteral(number) => whole(number.value),
        _ => None,
    });
    callee.property.name == SLICE
        && is_argv(&callee.object)
        && from.is_some_and(|from| from >= FIRST_USER_ARGUMENT)
}

fn own_text_read<'k, 'a>(argument: &'k Argument<'a>) -> Option<&'k CallExpression<'a>> {
    let Argument::CallExpression(read) = argument else {
        return None;
    };
    (callee_name(&read.callee) == Some(READ_FILE_SYNC)).then_some(&**read)
}

#[derive(Default)]
struct Flow {
    names: HashSet<String>,
    properties: HashSet<String>,
    readers: HashSet<String>,
}

impl Flow {
    fn size(&self) -> usize {
        self.names.len() + self.properties.len() + self.readers.len()
    }

    fn reads(&self, callee: &Expression<'_>) -> bool {
        callee_name(callee)
            .is_some_and(|name| READ_METHODS.contains(&name) || self.readers.contains(name))
    }

    fn carries(&self, expression: &Expression<'_>, params: Option<&HashSet<&str>>) -> bool {
        let mut taint = Taint {
            flow: self,
            params,
            found: false,
        };
        taint.visit_expression(expression);
        taint.found
    }

    fn carried(&self, arguments: &[Argument<'_>], params: Option<&HashSet<&str>>) -> bool {
        arguments.iter().any(|argument| match argument {
            Argument::SpreadElement(spread) => self.carries(&spread.argument, params),
            other => other
                .as_expression()
                .is_some_and(|expression| self.carries(expression, params)),
        })
    }
}

struct Taint<'f, 'p> {
    flow: &'f Flow,
    params: Option<&'p HashSet<&'p str>>,
    found: bool,
}

impl<'a> VisitJs<'a> for Taint<'_, '_> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.found |= match self.params {
            Some(params) => params.contains(name),
            None => name == OWN_NAME || self.flow.names.contains(name),
        };
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found |= self.params.is_none();
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator != UnaryOperator::Typeof {
            walk_js::walk_unary_expression(self, it);
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let property = it.property.name.as_str();
        if self.params.is_none() {
            if is_argv(&it.object) && property == LENGTH {
                return;
            }
            if host_member(&it.object, property) || self.flow.properties.contains(property) {
                self.found = true;
                return;
            }
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.params.is_none() && is_argv(&it.object) {
            if index_of(&it.expression).is_none_or(|index| index == SCRIPT_INDEX) {
                self.found = true;
            }
            return self.visit_expression(&it.expression);
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if callee_name(&it.callee) == Some(DIRNAME) {
            return;
        }
        if user_arguments(it) {
            return walk_js::walk_arguments(self, &it.arguments);
        }
        walk_js::walk_call_expression(self, it);
    }
}

enum Body<'k, 'a> {
    Function(Option<&'k FunctionBody<'a>>),
    Arrow(&'k ArrowFunctionBody<'a>),
}

struct Collect<'f> {
    flow: &'f mut Flow,
}

impl Collect<'_> {
    fn taint_name(&mut self, name: &str, value: &Expression<'_>) {
        if !self.flow.names.contains(name) && self.flow.carries(value, None) {
            self.flow.names.insert(name.to_string());
        }
    }

    fn taint_property(&mut self, name: &str, value: &Expression<'_>) {
        if !self.flow.properties.contains(name) && self.flow.carries(value, None) {
            self.flow.properties.insert(name.to_string());
        }
    }

    fn reader<'a>(&mut self, name: &str, params: &FormalParameters<'a>, body: Body<'_, 'a>) {
        if self.flow.readers.contains(name) {
            return;
        }
        let names: HashSet<&str> = params
            .items
            .iter()
            .filter_map(|param| param.pattern.get_identifier_name())
            .map(|name| name.as_str())
            .collect();
        if names.is_empty() {
            return;
        }
        let mut uses = ParamReads {
            flow: self.flow,
            params: &names,
            found: false,
        };
        match body {
            Body::Function(Some(body)) => uses.visit_function_body(body),
            Body::Function(None) => return,
            Body::Arrow(body) => uses.visit_arrow_function_body(body),
        }
        if uses.found {
            self.flow.readers.insert(name.to_string());
        }
    }

    fn function_value(&mut self, name: &str, value: &Expression<'_>) {
        match value.without_parentheses() {
            Expression::FunctionExpression(function) => {
                self.reader(
                    name,
                    &function.params,
                    Body::Function(function.body.as_deref()),
                );
            }
            Expression::ArrowFunctionExpression(arrow) => {
                self.reader(name, &arrow.params, Body::Arrow(&arrow.body));
            }
            _ => {}
        }
    }
}

impl<'a> VisitJs<'a> for Collect<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (Some(name), Some(init)) = (it.id.get_identifier_name(), &it.init) {
            self.taint_name(name.as_str(), init);
            self.function_value(name.as_str(), init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        match &it.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.taint_name(id.name.as_str(), &it.right);
                self.function_value(id.name.as_str(), &it.right);
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                self.taint_property(member.property.name.as_str(), &it.right);
                self.function_value(member.property.name.as_str(), &it.right);
            }
            _ => {}
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let key = it.key.static_name();
        if let Some(name) = key.as_ref().and_then(|key| key.as_js_str().as_str()) {
            self.taint_property(name, &it.value);
            self.function_value(name, &it.value);
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.reader(
                id.name.as_str(),
                &it.params,
                Body::Function(it.body.as_deref()),
            );
        }
        walk_js::walk_function(self, it, flags);
    }
}

struct ParamReads<'f, 'p> {
    flow: &'f Flow,
    params: &'p HashSet<&'p str>,
    found: bool,
}

impl<'a> VisitJs<'a> for ParamReads<'_, '_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.flow.reads(&it.callee) && self.flow.carried(&it.arguments, Some(self.params)) {
            self.found = true;
        }
        walk_js::walk_call_expression(self, it);
    }
}

struct Find<'f> {
    flow: &'f Flow,
    found: bool,
}

impl<'a> VisitJs<'a> for Find<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let (Some(RUN_IN_THIS_CONTEXT), [only]) =
            (callee_name(&it.callee), it.arguments.as_slice())
        {
            if let Some(read) = own_text_read(only) {
                self.visit_expression(&it.callee);
                self.visit_expression(&read.callee);
                return walk_js::walk_arguments(self, &read.arguments);
            }
        }
        if self.flow.reads(&it.callee) && self.flow.carried(&it.arguments, None) {
            self.found = true;
            return;
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub(super) fn may_read_own_file(program: &Program<'_>) -> bool {
    let text = program.source_text;
    if !OWN_MARKERS.iter().any(|marker| text.contains(marker))
        || !READ_METHODS.iter().any(|method| text.contains(method))
    {
        return false;
    }
    let mut flow = Flow::default();
    for _ in 0..ROUNDS {
        let before = flow.size();
        Collect { flow: &mut flow }.visit_program(program);
        if flow.size() == before {
            break;
        }
    }
    let mut find = Find {
        flow: &flow,
        found: false,
    };
    find.visit_program(program);
    find.found
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_span::SourceType;
    use untwist_core::parse::parse_with;

    use super::may_read_own_file;

    fn reads(text: &str) -> bool {
        let alloc = Allocator::default();
        let kind = match text.starts_with("import ") {
            true => SourceType::mjs(),
            false => SourceType::script(),
        };
        let parsed = parse_with(&alloc, text, kind);
        may_read_own_file(&parsed.program)
    }

    #[test]
    fn a_self_hash_reads_its_own_file() {
        assert!(reads("require('fs').readFileSync(__filename);"));
        assert!(reads("require('fs').readFileSync(process.argv[1]);"));
        assert!(reads(
            "import { readFileSync } from 'fs'; readFileSync(new URL(import.meta.url));"
        ));
        assert!(reads("require('fs').readFileSync(require.main.filename);"));
        assert!(reads("require('fs').statSync(module.filename).size;"));
        assert!(reads(
            "var self = typeof __filename === 'string' ? __filename : null; var p = self; fs.openSync(p, 'r');"
        ));
        assert!(reads(
            "var raw = function (p) { return fs.openSync(p, 'r'); }; var at = __filename; raw(at);"
        ));
        assert!(reads(
            "var o = {}; o.path = __filename; require('fs').readFileSync(o.path);"
        ));
        assert!(reads("fetch(import.meta.url).then(r => r.text());"));
        assert!(reads(
            "require('vm').runInThisContext(String(require('fs').readFileSync(__filename)));"
        ));
    }

    #[test]
    fn naming_its_own_path_without_reading_it_is_not_a_read() {
        assert!(!reads("console.log(__filename); require('fs');"));
        assert!(!reads(
            "require('fs').readFileSync(require('path').dirname(__filename) + '/data.json');"
        ));
        assert!(!reads(
            "require('fs').readFileSync(__dirname + '/data.json');"
        ));
        assert!(!reads("require('fs').readFileSync(process.argv[2]);"));
        assert!(!reads(
            "var [input] = process.argv.slice(2); require('fs').readFileSync(input);"
        ));
        assert!(!reads(
            "if (process.argv.length > 2) require('fs').readFileSync('x');"
        ));
        assert!(!reads(
            "typeof __filename === 'string' && require('fs').readFileSync('a.txt');"
        ));
        assert!(!reads(
            "import { createRequire } from 'node:module'; const require = createRequire(import.meta.url); require('fs').readFileSync('a');"
        ));
    }

    #[test]
    fn running_its_own_text_is_not_a_read() {
        assert!(!reads(
            "require('vm').runInThisContext(require('fs').readFileSync(__filename, 'utf8'));"
        ));
    }
}
