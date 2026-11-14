use oxc_ast::ast::*;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::prop::prop_name;

use super::stem::meaningful;
use crate::matcher::{argument_expression, call_parts, const_member};

const MODULE: &str = "module";
const EXPORTS: &str = "exports";
const REQUIRE: &str = "require";
const DEFAULT: &str = "default";
const INLINE: &str = " ";
const CROWDED: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Definition,
    Other,
}

pub struct Candidate {
    pub name: String,
    pub default: bool,
    pub imported: usize,
    defined: bool,
    position: usize,
}

impl Candidate {
    fn principal(&self) -> bool {
        self.defined
            && self.name.starts_with(|ch: char| ch.is_ascii_uppercase())
            && self.name.chars().any(|ch| ch.is_ascii_lowercase())
    }
}

fn definition(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::ObjectExpression(_)
    )
}

fn variable_items(declaration: &VariableDeclaration<'_>) -> Vec<(String, Shape)> {
    declaration
        .declarations
        .iter()
        .filter_map(|declarator| {
            let id = declarator.id.get_binding_identifier()?;
            let shape = match declarator.init.as_ref().is_some_and(definition) {
                true => Shape::Definition,
                false => Shape::Other,
            };
            Some((id.name.to_string(), shape))
        })
        .collect()
}

fn named(id: Option<&BindingIdentifier<'_>>) -> Vec<(String, Shape)> {
    id.map(|id| (id.name.to_string(), Shape::Definition))
        .into_iter()
        .collect()
}

fn declaration_items(declaration: &Declaration<'_>) -> Vec<(String, Shape)> {
    match declaration {
        Declaration::VariableDeclaration(declaration) => variable_items(declaration),
        Declaration::FunctionDeclaration(function) => named(function.id.as_ref()),
        Declaration::ClassDeclaration(class) => named(class.id.as_ref()),
        _ => Vec::new(),
    }
}

fn statement_items(statement: &Statement<'_>) -> Vec<(String, Shape)> {
    match statement {
        Statement::VariableDeclaration(declaration) => variable_items(declaration),
        Statement::FunctionDeclaration(function) => named(function.id.as_ref()),
        Statement::ClassDeclaration(class) => named(class.id.as_ref()),
        Statement::ExportDeclaration(export) => declaration_items(&export.declaration),
        _ => Vec::new(),
    }
}

fn declared(program: &Program<'_>) -> FxHashMap<String, (Shape, usize)> {
    let mut found = FxHashMap::default();
    for (at, statement) in program.body.iter().enumerate() {
        for (name, shape) in statement_items(statement) {
            found.entry(name).or_insert((shape, at));
        }
    }
    found
}

fn esm_exports(program: &Program<'_>) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    let local = specifier.local.name().to_string();
                    found.push((specifier.exported.name().to_string(), local));
                }
            }
            Statement::ExportDeclaration(export) => {
                for (name, _) in declaration_items(&export.declaration) {
                    found.push((name.clone(), name));
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let local = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        function.id.as_ref().map(|id| id.name.to_string())
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        class.id.as_ref().map(|id| id.name.to_string())
                    }
                    ExportDefaultDeclarationKind::Identifier(id) => Some(id.name.to_string()),
                    _ => None,
                };
                if let Some(local) = local {
                    found.push((DEFAULT.to_string(), local));
                }
            }
            _ => {}
        }
    }
    found
}

fn is_identifier(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn exports_object(expr: &Expression<'_>) -> bool {
    is_identifier(expr, EXPORTS)
        || const_member(expr, EXPORTS).is_some_and(|object| is_identifier(object, MODULE))
}

fn local_of(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.to_string()),
        Expression::FunctionExpression(function) => {
            function.id.as_ref().map(|id| id.name.to_string())
        }
        Expression::ClassExpression(class) => class.id.as_ref().map(|id| id.name.to_string()),
        _ => None,
    }
}

fn getter_local(expr: &Expression<'_>) -> Option<String> {
    let body = match expr.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => body,
            other => return local_of(other.as_expression()?),
        },
        Expression::FunctionExpression(function) => function.body.as_ref()?,
        _ => return None,
    };
    match body.statements.as_slice() {
        [Statement::ExpressionStatement(statement)] => local_of(&statement.expression),
        [Statement::ReturnStatement(statement)] => local_of(statement.argument.as_ref()?),
        _ => None,
    }
}

fn harmony_exports(expr: &Expression<'_>, found: &mut Vec<(String, String)>) {
    let Some((callee, arguments)) = call_parts(expr) else {
        return;
    };
    let helper = const_member(callee, "d").is_some_and(|object| is_identifier(object, REQUIRE));
    if !helper
        || !arguments
            .first()
            .and_then(argument_expression)
            .is_some_and(exports_object)
    {
        return;
    }
    match (
        arguments.get(1).and_then(argument_expression),
        arguments.get(2),
    ) {
        (Some(Expression::ObjectExpression(object)), None) => {
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    continue;
                };
                let (Some(key), Some(local)) =
                    (prop_name(&property.key), getter_local(&property.value))
                else {
                    continue;
                };
                found.push((key.to_string(), local));
            }
        }
        (Some(Expression::StringLiteral(key)), Some(getter)) => {
            let local = argument_expression(getter).and_then(getter_local);
            if let (Some(key), Some(local)) = (key.value.as_str(), local) {
                found.push((key.to_string(), local));
            }
        }
        _ => {}
    }
}

fn commonjs_exports(program: &Program<'_>) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for statement in &program.body {
        let Statement::ExpressionStatement(statement) = statement else {
            continue;
        };
        let expr = statement.expression.without_parentheses();
        harmony_exports(expr, &mut found);
        let Expression::AssignmentExpression(assignment) = expr else {
            continue;
        };
        let Some(target) = assignment.left.as_simple_assignment_target() else {
            continue;
        };
        let Some(member) = target.as_member_expression() else {
            continue;
        };
        let Some(key) = member.static_property_name().and_then(|key| key.as_str()) else {
            continue;
        };
        let object = member.object();
        if is_identifier(object, MODULE) && key == EXPORTS {
            if let Some(local) = local_of(&assignment.right) {
                found.push((DEFAULT.to_string(), local));
            }
        } else if exports_object(object) {
            match local_of(&assignment.right) {
                Some(local) => found.push((key.to_string(), local)),
                None if definition(&assignment.right) => {
                    found.push((key.to_string(), format!("{INLINE}{key}")))
                }
                None => {}
            }
        }
    }
    found
}

pub fn candidates(
    program: &Program<'_>,
    esm: bool,
    imported: impl Fn(&str) -> usize,
) -> Vec<Candidate> {
    let declared = declared(program);
    let exports = match esm {
        true => esm_exports(program),
        false => commonjs_exports(program),
    };
    let mut found: Vec<Candidate> = Vec::new();
    for (at, (exported, local)) in exports.into_iter().enumerate() {
        let (local, (shape, position)) = match local.strip_prefix(INLINE) {
            Some(key) => (key.to_string(), (Shape::Definition, at)),
            None => match declared.get(&local) {
                Some(&found) => (local, found),
                None => continue,
            },
        };
        if !meaningful(&local) {
            continue;
        }
        let default = exported == DEFAULT;
        match found.iter_mut().find(|known| known.name == local) {
            Some(known) => {
                known.default |= default;
                known.imported += imported(&exported);
            }
            None => found.push(Candidate {
                name: local,
                default,
                imported: imported(&exported),
                defined: shape == Shape::Definition,
                position,
            }),
        }
    }
    found
}

pub fn default_export(found: &[Candidate]) -> Option<&str> {
    found
        .iter()
        .find(|candidate| candidate.default && candidate.defined)
        .map(|candidate| candidate.name.as_str())
}

pub fn main_export(found: &[Candidate]) -> Option<&str> {
    if let Some(default) = default_export(found) {
        return Some(default);
    }
    let principal: Vec<&Candidate> = found
        .iter()
        .filter(|candidate| candidate.principal())
        .collect();
    let pool: Vec<&Candidate> = match principal.is_empty() {
        true => found.iter().filter(|candidate| candidate.defined).collect(),
        false => principal,
    };
    let most = pool
        .iter()
        .filter(|candidate| candidate.imported > 0)
        .max_by(|a, b| {
            a.imported
                .cmp(&b.imported)
                .then(b.position.cmp(&a.position))
        });
    let total: usize = pool.iter().map(|candidate| candidate.imported).sum();
    let crowded = pool.len() > CROWDED;
    match (most, pool.as_slice()) {
        (Some(most), _) if !crowded || most.imported * 2 > total => Some(&most.name),
        (Some(_), _) => None,
        (None, [only]) => Some(&only.name),
        _ => None,
    }
}
