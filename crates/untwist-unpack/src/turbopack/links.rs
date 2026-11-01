use oxc_ast::ast::*;
use oxc_syntax::operator::AssignmentOperator;
use rustc_hash::FxHashSet;

use super::layout::{self, Id};
use crate::matcher::expression_statement;
use crate::path::normalize_posix;

pub const NAMESPACE: &str = "turbopack:server";

#[derive(Clone, Debug, Default)]
pub struct TurboView {
    pub ids: FxHashSet<Id>,
}

pub struct Loader {
    pub runtime: String,
    pub name: String,
    pub entry: Id,
}

pub struct Group {
    pub chunks: Vec<usize>,
    pub loader: usize,
    pub runtime: usize,
    pub view: TurboView,
}

fn named(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

fn string_argument(call: &CallExpression<'_>) -> Option<String> {
    match call.arguments.as_slice() {
        [Argument::StringLiteral(text)] if !text.lone_surrogates => Some(text.value.to_string()),
        _ => None,
    }
}

fn runtime_spec(expr: &Expression<'_>) -> Option<(String, String)> {
    let Expression::CallExpression(outer) = expr else {
        return None;
    };
    let Expression::CallExpression(inner) = &outer.callee else {
        return None;
    };
    if outer.optional || inner.optional || !named(&inner.callee, "require") {
        return None;
    }
    let spec = string_argument(inner)?;
    let relative = spec.starts_with("./") || spec.starts_with("../");
    relative.then_some((spec, string_argument(outer)?))
}

fn method_call<'k, 'a>(
    statement: &'k Statement<'a>,
    object: &str,
) -> Option<(&'k str, &'k CallExpression<'a>)> {
    let Expression::CallExpression(call) = expression_statement(statement)? else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    let fits = !call.optional && !member.optional && named(&member.object, object);
    fits.then(|| (member.property.name.as_str(), &**call))
}

pub fn loader(program: &Program<'_>) -> Option<Loader> {
    let (first, rest) = program.body.split_first()?;
    let (last, middle) = rest.split_last()?;
    let Statement::VariableDeclaration(declaration) = first else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
        return None;
    };
    let (runtime, name) = runtime_spec(declarator.init.as_ref()?)?;
    let object = binding.name.as_str();
    let chunks = middle.iter().all(|statement| {
        method_call(statement, object)
            .is_some_and(|(method, call)| method == "c" && string_argument(call).is_some())
    });
    let (method, call) = method_call(last, object)?;
    let entry = match call.arguments.as_slice() {
        [argument] => Id::of(argument.as_expression()?)?,
        _ => return None,
    };
    let fits = program.directives.is_empty()
        && declaration.kind == VariableDeclarationKind::Var
        && chunks
        && method == "m";
    fits.then_some(Loader {
        runtime,
        name,
        entry,
    })
}

fn object_arrow(expr: &Expression<'_>) -> bool {
    let Expression::ArrowFunctionExpression(arrow) = expr else {
        return false;
    };
    matches!(
        arrow
            .body
            .as_expression()
            .map(Expression::without_parentheses),
        Some(Expression::ObjectExpression(_))
    )
}

pub fn is_runtime(program: &Program<'_>) -> bool {
    let Some(Expression::AssignmentExpression(assign)) =
        program.body.last().and_then(expression_statement)
    else {
        return false;
    };
    let AssignmentTarget::StaticMemberExpression(target) = &assign.left else {
        return false;
    };
    assign.operator == AssignmentOperator::Assign
        && target.property.name == "exports"
        && named(&target.object, "module")
        && object_arrow(&assign.right)
}

pub fn runtime_path(loader: &Loader) -> Option<String> {
    let normalized = normalize_posix(&loader.runtime);
    (!normalized.starts_with("node_modules/")).then_some(normalized)
}

pub fn group(programs: &[Program<'_>]) -> Option<Group> {
    if programs.len() < 3 {
        return None;
    }
    let mut chunks = Vec::new();
    let mut loaders = Vec::new();
    let mut runtimes = Vec::new();
    let mut ids = FxHashSet::default();
    let mut keys = FxHashSet::default();
    for (input, program) in programs.iter().enumerate() {
        if let Some(server) = layout::server(program) {
            for def in &server.defs {
                if !keys.insert(def.id.key.clone()) {
                    return None;
                }
                ids.insert(def.id.clone());
            }
            chunks.push(input);
        } else if loader(program).is_some() {
            loaders.push(input);
        } else if is_runtime(program) {
            runtimes.push(input);
        } else if layout::probe_top(&program.body) {
            return None;
        }
    }
    let ([loader_at], [runtime]) = (loaders.as_slice(), runtimes.as_slice()) else {
        return None;
    };
    let entry = loader(&programs[*loader_at])?.entry;
    let fits = !chunks.is_empty() && ids.contains(&entry) && !keys.contains(super::ENTRY_ID);
    fits.then(|| Group {
        chunks,
        loader: *loader_at,
        runtime: *runtime,
        view: TurboView { ids },
    })
}
