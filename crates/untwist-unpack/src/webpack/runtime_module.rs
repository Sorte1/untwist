use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use rustc_hash::FxHashSet;

use crate::model::{Bundle, BundleKind, Module, Runtime};
use crate::runtime::{js_string, module_of, runtime_path, synthetic_statements};
use crate::specifier::ModulePaths;
use crate::webpack::shadow;
use oxc_syntax::scope::ScopeFlags;

pub const PATHS: &str = "__webpack_module_paths__";
pub const COMPUTED: &str = "[]";

struct Members<'n> {
    object: &'n str,
    names: FxHashSet<String>,
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
        if matches!(&it.object, Expression::Identifier(id) if id.name == self.object) {
            self.names.insert(it.property.name.to_string());
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if matches!(&it.object, Expression::Identifier(id) if id.name == self.object) {
            self.names.insert(COMPUTED.to_string());
        }
        walk_js::walk_computed_member_expression(self, it);
    }
}

pub fn member_names(object: &str, body: &FunctionBody<'_>) -> FxHashSet<String> {
    member_names_in(object, &body.statements)
}

pub fn member_names_in<'k, 'a: 'k>(
    object: &str,
    statements: impl IntoIterator<Item = &'k Statement<'a>>,
) -> FxHashSet<String> {
    let mut members = Members {
        object,
        names: FxHashSet::default(),
    };
    for statement in statements {
        members.visit_statement(statement);
    }
    members.names
}

fn members(runtime: &Runtime<'_>, statement: &Statement<'_>) -> FxHashSet<String> {
    let mut members = Members {
        object: &runtime.require_name,
        names: FxHashSet::default(),
    };
    members.visit_statement(statement);
    members.names
}

pub fn needed(runtime: &Runtime<'_>, root: &str) -> Option<Vec<usize>> {
    let mut wanted = vec![root.to_string()];
    let mut chosen: FxHashSet<usize> = FxHashSet::default();
    while let Some(name) = wanted.pop() {
        let index = runtime
            .helpers
            .iter()
            .rposition(|helper| helper.name == name)?;
        let helper = &runtime.helpers[index];
        if !helper.movable {
            return None;
        }
        if chosen.insert(index) {
            wanted.extend(members(runtime, &helper.statement));
        }
    }
    let mut indices: Vec<usize> = chosen.into_iter().collect();
    indices.sort_unstable();
    Some(indices)
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
    for (index, helper) in runtime.helpers.into_iter().enumerate() {
        if indices.contains(&index) {
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
