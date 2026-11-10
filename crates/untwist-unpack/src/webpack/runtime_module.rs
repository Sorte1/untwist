use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::model::{Bundle, BundleKind, Module, Runtime};
use crate::runtime::{js_string, module_of, runtime_path, synthetic_statements};
use crate::specifier::ModulePaths;
use crate::webpack::shadow;
use oxc_syntax::scope::ScopeFlags;

pub const PATHS: &str = "__webpack_module_paths__";
pub const COMPUTED: &str = "[]";

#[derive(Default)]
pub struct Uses {
    pub names: FxHashSet<String>,
    pub written: Vec<(String, Option<u32>)>,
    pub computed: bool,
    pub bare: bool,
}

struct Members<'n> {
    object: &'n str,
    uses: Uses,
}

fn declared_head<'k, 'a>(left: &'k ForStatementLeft<'a>) -> Option<&'k VariableDeclaration<'a>> {
    match left {
        ForStatementLeft::VariableDeclaration(declaration) => Some(declaration),
        _ => None,
    }
}

impl Members<'_> {
    fn shadowed_by(&self, statements: &[Statement<'_>]) -> bool {
        shadow::lexical_name(statements, self.object)
            || shadow::hoisted_name(statements, false, self.object)
    }

    fn is_object(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Identifier(id) if id.name == self.object)
    }

    fn own_member<'k>(&self, expr: &'k Expression<'_>) -> Option<&'k str> {
        match expr {
            Expression::StaticMemberExpression(member) if self.is_object(&member.object) => {
                Some(member.property.name.as_str())
            }
            _ => None,
        }
    }

    fn write(&mut self, name: &str, definition: Option<u32>) {
        self.uses.names.insert(name.to_string());
        self.uses.written.push((name.to_string(), definition));
    }

    fn ignores_arguments(&self, call: &CallExpression<'_>) -> bool {
        let Some(callee) = crate::matcher::const_member(&call.callee, "call") else {
            return false;
        };
        let Expression::FunctionExpression(function) = callee.without_parentheses() else {
            return false;
        };
        let silent = function.params.items.is_empty() && function.params.rest.is_none();
        silent
            && function
                .body
                .as_ref()
                .is_some_and(|body| !own_arguments(body))
    }

    fn binds_object(&self, call: &CallExpression<'_>) -> bool {
        let Some(bound) = crate::matcher::const_member(&call.callee, "bind") else {
            return false;
        };
        let target = self.is_object(bound) || self.own_member(bound).is_some();
        target
            && matches!(call.arguments.first(), Some(Argument::Identifier(id)) if id.name == self.object)
    }
}

impl<'a> VisitJs<'a> for Members<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let body = it
            .body
            .as_ref()
            .is_some_and(|body| self.shadowed_by(&body.statements));
        let named = it.id.as_ref().is_some_and(|id| id.name == self.object);
        if !body && !named && !shadow::params_bind_name(&it.params, self.object) {
            walk_js::walk_function(self, it, flags);
        }
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let body = match &it.body {
            ArrowFunctionBody::FunctionBody(body) => self.shadowed_by(&body.statements),
            _ => false,
        };
        if !body && !shadow::params_bind_name(&it.params, self.object) {
            walk_js::walk_arrow_function_expression(self, it);
        }
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        if !shadow::lexical_name(&it.body, self.object) {
            walk_js::walk_block_statement(self, it);
        }
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        let head = match &it.init {
            Some(ForStatementInit::VariableDeclaration(declaration)) => Some(&**declaration),
            _ => None,
        };
        if !shadow::for_head_lexical_name(head, self.object) {
            walk_js::walk_for_statement(self, it);
        }
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        if !shadow::for_head_lexical_name(declared_head(&it.left), self.object) {
            walk_js::walk_for_in_statement(self, it);
        }
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if !shadow::for_head_lexical_name(declared_head(&it.left), self.object) {
            walk_js::walk_for_of_statement(self, it);
        }
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        let shadowed = it
            .cases
            .iter()
            .any(|case| shadow::lexical_name(&case.consequent, self.object));
        if shadowed {
            self.visit_expression(&it.discriminant);
        } else {
            walk_js::walk_switch_statement(self, it);
        }
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        let bound = it
            .param
            .as_ref()
            .is_some_and(|param| shadow::binds_name(&param.pattern, self.object));
        if !bound {
            walk_js::walk_catch_clause(self, it);
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.is_object(&it.object) {
            self.uses.names.insert(it.property.name.to_string());
            return;
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_object(&it.object) {
            self.uses.names.insert(COMPUTED.to_string());
            self.uses.computed = true;
            self.visit_expression(&it.expression);
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.uses.bare |= it.name == self.object;
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.is_object(&it.callee) {
            for argument in &it.arguments {
                self.visit_argument(argument);
            }
            return;
        }
        if self.ignores_arguments(it) {
            self.visit_expression(&it.callee);
            for argument in &it.arguments {
                if !matches!(argument, Argument::Identifier(id) if id.name == self.object) {
                    self.visit_argument(argument);
                }
            }
            return;
        }
        if self.binds_object(it) {
            self.visit_expression(&it.callee);
            for argument in it.arguments.iter().skip(1) {
                self.visit_argument(argument);
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        match it.operator {
            UnaryOperator::Typeof if self.is_object(&it.argument) => {}
            UnaryOperator::Delete => match self.own_member(&it.argument) {
                Some(name) => self.write(name, None),
                None => walk_js::walk_unary_expression(self, it),
            },
            _ => walk_js::walk_unary_expression(self, it),
        }
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        match &it.argument {
            SimpleAssignmentTarget::StaticMemberExpression(member)
                if self.is_object(&member.object) =>
            {
                self.write(member.property.name.as_str(), None);
            }
            _ => walk_js::walk_update_expression(self, it),
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        match &it.left {
            AssignmentTarget::StaticMemberExpression(member) if self.is_object(&member.object) => {
                let plain = it.operator == AssignmentOperator::Assign;
                self.write(
                    member.property.name.as_str(),
                    plain.then_some(it.span.start),
                );
                self.visit_expression(&it.right);
            }
            _ => walk_js::walk_assignment_expression(self, it),
        }
    }

    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        match it {
            AssignmentTarget::StaticMemberExpression(member) if self.is_object(&member.object) => {
                self.write(member.property.name.as_str(), None);
            }
            _ => walk_js::walk_assignment_target(self, it),
        }
    }
}

#[derive(Default)]
struct OwnArguments {
    depth: u32,
    found: bool,
}

impl<'a> VisitJs<'a> for OwnArguments {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= self.depth == 0 && it.name == "arguments";
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }
}

fn own_arguments(body: &FunctionBody<'_>) -> bool {
    let mut scan = OwnArguments::default();
    scan.visit_function_body(body);
    scan.found
}

pub fn uses_in<'k, 'a: 'k>(
    object: &str,
    statements: impl IntoIterator<Item = &'k Statement<'a>>,
) -> Uses {
    let mut members = Members {
        object,
        uses: Uses::default(),
    };
    for statement in statements {
        members.visit_statement(statement);
    }
    members.uses
}

pub fn member_names(object: &str, body: &FunctionBody<'_>) -> FxHashSet<String> {
    member_names_in(object, &body.statements)
}

pub fn member_names_in<'k, 'a: 'k>(
    object: &str,
    statements: impl IntoIterator<Item = &'k Statement<'a>>,
) -> FxHashSet<String> {
    uses_in(object, statements).names
}

pub struct Needs {
    index: FxHashMap<String, usize>,
    members: Vec<Vec<usize>>,
    routes: Vec<bool>,
}

impl Needs {
    pub fn of(runtime: &Runtime<'_>) -> Self {
        let mut index: FxHashMap<String, usize> = FxHashMap::default();
        for (at, helper) in runtime.helpers.iter().enumerate() {
            index.insert(helper.name.clone(), at);
        }
        let mut routes: Vec<bool> = runtime
            .helpers
            .iter()
            .map(|helper| helper.movable)
            .collect();
        let mut members = Vec::with_capacity(runtime.helpers.len());
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); runtime.helpers.len()];
        for (at, helper) in runtime.helpers.iter().enumerate() {
            let mut wanted = Vec::new();
            for name in uses_in(&runtime.require_name, [&helper.statement]).names {
                match index.get(&name) {
                    Some(&target) => {
                        wanted.push(target);
                        dependents[target].push(at);
                    }
                    None => routes[at] = false,
                }
            }
            members.push(wanted);
        }
        let mut failing: Vec<usize> = (0..routes.len()).filter(|&at| !routes[at]).collect();
        while let Some(at) = failing.pop() {
            for &dependent in &dependents[at] {
                if routes[dependent] {
                    routes[dependent] = false;
                    failing.push(dependent);
                }
            }
        }
        Self {
            index,
            members,
            routes,
        }
    }

    pub fn routes(&self, name: &str) -> bool {
        self.index.get(name).is_some_and(|&at| self.routes[at])
    }

    pub fn closure<'n>(&self, roots: impl IntoIterator<Item = &'n str>) -> Option<Vec<usize>> {
        let mut wanted = Vec::new();
        for root in roots {
            let &at = self.index.get(root)?;
            wanted.push(at);
        }
        let mut chosen = vec![false; self.routes.len()];
        let mut indices = Vec::new();
        while let Some(at) = wanted.pop() {
            if !self.routes[at] {
                return None;
            }
            if !std::mem::replace(&mut chosen[at], true) {
                indices.push(at);
                wanted.extend(&self.members[at]);
            }
        }
        indices.sort_unstable();
        Some(indices)
    }
}

pub fn needed(runtime: &Runtime<'_>, root: &str) -> Option<Vec<usize>> {
    Needs::of(runtime).closure([root])
}

fn paths_object(bundle: &Bundle<'_>) -> String {
    let from = runtime_path(BundleKind::Webpack);
    let paths = ModulePaths::of(bundle);
    let entries: Vec<String> = bundle
        .modules
        .iter()
        .map(|(id, module)| {
            let path = paths.specifier(&from, module.path.as_str());
            format!("{}: {}", js_string(id), js_string(&path))
        })
        .collect();
    format!("{{ {} }}", entries.join(", "))
}

pub fn runtime_module<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    indices: &[usize],
) -> Option<Module<'a>> {
    let source_text = bundle.modules.values().next()?.program().source_text;
    let head = format!(
        "var {PATHS} = {};\nvar {name} = function (id) {{ return require({PATHS}[id]); }};",
        paths_object(bundle),
        name = bundle.runtime.as_ref()?.require_name,
    );
    let runtime = bundle.runtime.take()?;
    let mut statements = synthetic_statements(alloc, head)?;
    let mut wanted = indices.iter().copied().peekable();
    for (index, helper) in runtime.helpers.into_iter().enumerate() {
        if wanted.next_if_eq(&index).is_some() {
            statements.push(helper.statement);
        }
    }
    let tail = format!("module.exports = {};", runtime.require_name);
    statements.extend(synthetic_statements(alloc, tail)?);
    module_of(alloc, source_text, BundleKind::Webpack, statements)
}

pub fn prepend<'a>(program: &mut Program<'a>, first: AVec<'a, Statement<'a>>) {
    let rest = std::mem::replace(&mut program.body, first);
    program.body.extend(rest);
}
