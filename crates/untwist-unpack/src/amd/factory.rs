use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::Ident;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use rustc_hash::FxHashSet;
use untwist_core::parse::parse_program;
use untwist_core::pass::{Note, NoteKind};

use crate::closed::{Allowed, Check};
use crate::esbuild::build::has_use_strict;
use crate::matcher::{anonymous_factory, Factory};
use crate::model::{module_from_body_with, Bodies, Bundle, BundleKind};
use crate::path::{dirname_posix, join_posix};
use crate::runtime::{js_string, synthetic_statements};
use crate::scan::{Frame, Policy, Scan, Step};
use crate::specifier::ModulePaths;

pub const EXPORTS: &str = "\0exports";
pub const MODULE: &str = "\0module";
pub const REQUIRE: &str = "\0require";
const GUARD: &str = "__amd_exports";
const HOST_NAMES: [&str; 5] = ["require", "module", "exports", "__dirname", "__filename"];
const SPECIALS: [(&str, &str); 3] = [
    ("require", REQUIRE),
    ("module", MODULE),
    ("exports", EXPORTS),
];

pub type Dependency = (String, String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returns {
    Guarded,
    Assigned,
    Dropped,
}

#[derive(Clone, Copy)]
pub struct Callback<'k, 'a> {
    pub expr: &'k Expression<'a>,
    pub params: &'k FormalParameters<'a>,
    pub body: &'k FunctionBody<'a>,
}

pub fn callback<'k, 'a>(expr: &'k Expression<'a>) -> Option<Callback<'k, 'a>> {
    let expr = expr.without_parentheses();
    let asynchronous = match expr {
        Expression::FunctionExpression(function) => function.r#async,
        Expression::ArrowFunctionExpression(arrow) => arrow.r#async,
        _ => return None,
    };
    if asynchronous {
        return None;
    }
    let Factory { params, body } = anonymous_factory(expr)?;
    Some(Callback { expr, params, body })
}

impl<'k, 'a> Callback<'k, 'a> {
    pub fn scope(&self) -> Option<ScopeId> {
        match self.expr {
            Expression::FunctionExpression(function) => function.scope_id.get(),
            Expression::ArrowFunctionExpression(arrow) => arrow.scope_id.get(),
            _ => None,
        }
    }

    pub fn names(&self) -> Option<Vec<&'k str>> {
        if self.params.rest.is_some() {
            return None;
        }
        self.params
            .items
            .iter()
            .map(|item| match (&item.pattern, &item.initializer) {
                (BindingPattern::BindingIdentifier(id), None) => Some(id.name.as_str()),
                _ => None,
            })
            .collect()
    }
}

pub fn pair_up(names: &[&str], dependencies: Vec<String>) -> Vec<Dependency> {
    let count = names.len().max(dependencies.len());
    let mut dependencies = dependencies.into_iter();
    (0..count)
        .map(|index| {
            let name = names.get(index).copied().unwrap_or_default();
            (name.to_string(), dependencies.next().unwrap_or_default())
        })
        .collect()
}

struct Sound<'s, 'k, 'a> {
    check: Check<'s, 'k, 'a>,
    returns: usize,
    loose: bool,
}

impl<'a> Policy<'a> for Sound<'_, '_, 'a> {
    fn done(&self) -> bool {
        self.check.done()
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        self.check.reference(frame, it);
    }

    fn call(&mut self, frame: Frame, it: &CallExpression<'a>) -> Step {
        self.check.call(frame, it)
    }

    fn returns(&mut self, frame: Frame) {
        if frame.functions + frame.arrows == 0 {
            self.returns += 1;
        }
    }

    fn this(&mut self, frame: Frame) {
        self.loose |= frame.functions == 0;
    }

    fn with(&mut self) {
        self.check.with();
    }
}

fn closed_and_shaped(scoping: &Scoping, callback: &Callback<'_, '_>) -> bool {
    let helpers = FxHashSet::default();
    let allowed = Allowed {
        mapped_params: callback.params.items.len(),
        helpers: &helpers,
    };
    let factory = Factory {
        params: callback.params,
        body: callback.body,
    };
    let mut sound = Sound {
        check: Check::factory(scoping, factory, &allowed),
        returns: 0,
        loose: false,
    };
    let mut scan = Scan::new(&mut sound);
    scan.visit_formal_parameters(callback.params);
    scan.visit_function_body(callback.body);
    let ends_in_return = matches!(
        callback.body.statements.last(),
        Some(Statement::ReturnStatement(_))
    );
    let shaped = !sound.loose && (sound.returns == 0 || (sound.returns == 1 && ends_in_return));
    shaped && sound.check.into_captured().is_none()
}

fn named_only_as_mapped(
    scoping: &Scoping,
    callback: &Callback<'_, '_>,
    pairs: &[Dependency],
) -> bool {
    let Some(scope) = callback.scope() else {
        return false;
    };
    SPECIALS.iter().all(|&(name, special)| {
        scoping.get_binding(scope, Ident::from(name)).is_none()
            || pairs
                .iter()
                .any(|(param, dependency)| param == name && dependency == special)
    })
}

fn reads_host(scoping: &Scoping, callback: &Callback<'_, '_>) -> bool {
    let Some(scope) = callback.scope() else {
        return true;
    };
    let free = scoping.root_unresolved_references();
    HOST_NAMES.iter().any(|name| {
        free.get(*name).is_some_and(|references| {
            references.iter().any(|&reference| {
                scoping
                    .scope_ancestors(scoping.get_reference(reference).scope_id())
                    .any(|ancestor| ancestor == scope)
            })
        })
    })
}

struct Redeclared<'n> {
    names: &'n [&'n str],
    found: bool,
}

impl<'a> VisitJs<'a> for Redeclared<'_> {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        self.found |= it.is_declaration()
            && it
                .id
                .as_ref()
                .is_some_and(|id| self.names.contains(&id.name.as_str()));
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}
}

fn redeclares_a_parameter(callback: &Callback<'_, '_>) -> bool {
    let Some(names) = callback.names() else {
        return true;
    };
    let mut redeclared = Redeclared {
        names: &names,
        found: false,
    };
    redeclared.visit_function_body(callback.body);
    redeclared.found
}

pub fn sound(
    scoping: &Scoping,
    callback: &Callback<'_, '_>,
    pairs: &[Dependency],
    host_free: bool,
) -> bool {
    closed_and_shaped(scoping, callback)
        && named_only_as_mapped(scoping, callback, pairs)
        && !redeclares_a_parameter(callback)
        && !(host_free && reads_host(scoping, callback))
}

pub fn guard_free(source: &Program<'_>) -> bool {
    !source.source_text.contains(GUARD)
}

fn never_undefined(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NewExpression(_)
    )
}

fn assignment<'a>(alloc: &'a Allocator, value: Option<Expression<'a>>) -> Option<Statement<'a>> {
    let mut statement =
        synthetic_statements(alloc, "module.exports = void 0;".to_string())?.pop()?;
    if let (Some(value), Statement::ExpressionStatement(statement)) = (value, &mut statement) {
        if let Expression::AssignmentExpression(assign) = &mut statement.expression {
            assign.right = value;
        }
    }
    Some(statement)
}

fn guarded<'a>(alloc: &'a Allocator, value: Expression<'a>) -> Option<Vec<Statement<'a>>> {
    let text =
        format!("const {GUARD} = void 0; if ({GUARD} !== undefined) module.exports = {GUARD};");
    let mut statements: Vec<Statement<'a>> =
        synthetic_statements(alloc, text)?.into_iter().collect();
    let Some(Statement::VariableDeclaration(declaration)) = statements.first_mut() else {
        return None;
    };
    declaration.declarations.first_mut()?.init = Some(value);
    Some(statements)
}

fn finish_returns<'a>(alloc: &'a Allocator, program: &mut Program<'a>, returns: Returns) -> bool {
    let argument = if matches!(program.body.last(), Some(Statement::ReturnStatement(_))) {
        let Some(Statement::ReturnStatement(ret)) = program.body.pop() else {
            return false;
        };
        ret.unbox().argument
    } else {
        None
    };
    let appended: Option<Vec<Statement<'a>>> = match (returns, argument) {
        (Returns::Dropped, None) | (Returns::Guarded, None) => Some(Vec::new()),
        (Returns::Dropped, Some(value)) => {
            let builder = AstBuilder::new(alloc);
            Some(vec![Statement::new_expression_statement(
                value.span(),
                value,
                &builder,
            )])
        }
        (Returns::Assigned, value) => assignment(alloc, value).map(|statement| vec![statement]),
        (Returns::Guarded, Some(value)) if never_undefined(&value) => {
            assignment(alloc, Some(value)).map(|statement| vec![statement])
        }
        (Returns::Guarded, Some(value)) => guarded(alloc, value),
    };
    let Some(appended) = appended else {
        return false;
    };
    program.body.extend(appended);
    true
}

fn use_strict<'a>(alloc: &'a Allocator) -> Option<Directive<'a>> {
    let parsed = parse_program(alloc, alloc.alloc_str("\"use strict\";"));
    parsed.program.directives.into_iter().next()
}

pub fn module_program<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    callback: &Callback<'_, 'a>,
    returns: Returns,
    bodies: &mut Bodies<'a>,
) -> Option<Program<'a>> {
    let mut program = module_from_body_with(alloc, source, callback.body, bodies);
    program.directives = callback.body.directives.clone_in(alloc);
    if has_use_strict(&source.directives) && !has_use_strict(&program.directives) {
        program.directives.push(use_strict(alloc)?);
    }
    if matches!(bodies, Bodies::Planned(_)) || finish_returns(alloc, &mut program, returns) {
        return Some(program);
    }
    None
}

pub enum Target {
    Module(String),
    External,
    Missing,
}

pub fn is_relative(dependency: &str) -> bool {
    dependency.starts_with("./") || dependency.starts_with("../")
}

pub fn target_of(has: impl Fn(&str) -> bool, from: &str, dependency: &str) -> Target {
    if is_relative(dependency) {
        let joined = join_posix(dirname_posix(from), dependency);
        return if has(&joined) {
            Target::Module(joined)
        } else {
            Target::Missing
        };
    }
    if has(dependency) {
        Target::Module(dependency.to_string())
    } else {
        Target::External
    }
}

fn alias(param: &str, value: &str) -> String {
    if param.is_empty() || param == value {
        String::new()
    } else {
        format!("var {param} = {value};\n")
    }
}

fn prelude_line(pair: &Dependency, specifier: impl Fn(&str) -> String) -> String {
    let (param, dependency) = (pair.0.as_str(), pair.1.as_str());
    match dependency {
        "" if param.is_empty() => String::new(),
        "" => format!("var {param};\n"),
        EXPORTS => alias(param, "exports"),
        MODULE => alias(param, "module"),
        REQUIRE => alias(param, "require"),
        _ if param.is_empty() => format!("require({});\n", js_string(&specifier(dependency))),
        _ => format!(
            "var {param} = require({});\n",
            js_string(&specifier(dependency))
        ),
    }
}

fn prelude_of(bundle: &Bundle<'_>, paths: &ModulePaths, id: &str) -> String {
    let Some(module) = bundle.modules.get(id) else {
        return String::new();
    };
    let lookup = bundle.kind == BundleKind::Amd;
    let specifier = |dependency: &str| {
        let target = if lookup {
            target_of(
                |candidate| bundle.modules.contains_key(candidate),
                id,
                dependency,
            )
        } else {
            Target::External
        };
        match target {
            Target::Module(target) => paths.get(&target).map_or_else(
                || dependency.to_string(),
                |path| paths.specifier(module.path.as_str(), path),
            ),
            Target::External | Target::Missing => dependency.to_string(),
        }
    };
    module
        .dependencies
        .iter()
        .map(|pair| prelude_line(pair, specifier))
        .collect()
}

pub fn prepend_dependencies<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    let paths = ModulePaths::of(bundle);
    let preludes: Vec<(String, String)> = bundle
        .modules
        .keys()
        .map(|id| (id.clone(), prelude_of(bundle, &paths, id)))
        .collect();
    let mut notes = Vec::new();
    for (id, text) in preludes {
        if text.is_empty() {
            continue;
        }
        let Some(statements) = synthetic_statements(alloc, text) else {
            notes.push(Note {
                pass: "unpack",
                kind: NoteKind::Refusal,
                message: super::notes::prelude_unbuilt(&id),
                span: None,
            });
            continue;
        };
        let Some(module) = bundle.modules.get_mut(&id) else {
            continue;
        };
        let body = &mut module.program_mut().body;
        let rest: Vec<Statement<'a>> = body.drain(..).collect();
        body.extend(statements);
        body.extend(rest);
    }
    notes
}
