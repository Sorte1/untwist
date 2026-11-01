use oxc_ast::ast::*;
use rustc_hash::FxHashMap;

use crate::amd::factory::{callback, pair_up, target_of, Callback, Dependency, Target, EXPORTS};
use crate::matcher::argument_expression;

pub struct Define<'k, 'a> {
    pub id: Option<String>,
    pub callback: Callback<'k, 'a>,
    pub pairs: Vec<Dependency>,
}

fn define_call<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k CallExpression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.without_parentheses() else {
        return None;
    };
    (!call.optional && callee.name == "define").then_some(call)
}

pub fn starts_with_define(top: &[Statement<'_>]) -> bool {
    top.iter()
        .find(|statement| !matches!(statement, Statement::EmptyStatement(_)))
        .is_some_and(|statement| define_call(statement).is_some())
}

fn text(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(literal)
            if !literal.lone_surrogates && !literal.value.is_empty() =>
        {
            Some(literal.value.as_str().to_string())
        }
        _ => None,
    }
}

fn dependency_names(expr: &Expression<'_>) -> Option<Vec<String>> {
    let Expression::ArrayExpression(array) = expr else {
        return None;
    };
    array
        .elements
        .iter()
        .map(|element| {
            let name = text(element.as_expression()?)?;
            match name.as_str() {
                "exports" => Some(EXPORTS.to_string()),
                "module" | "require" => None,
                _ if name.contains('!') => None,
                _ => Some(name),
            }
        })
        .collect()
}

fn parse<'k, 'a>(call: &'k CallExpression<'a>) -> Option<Define<'k, 'a>> {
    let args: Vec<&Expression<'a>> = call
        .arguments
        .iter()
        .map(argument_expression)
        .collect::<Option<_>>()?;
    let (id, dependencies, factory) = match args.as_slice() {
        [factory] => (None, None, *factory),
        [first, factory] => match first.without_parentheses() {
            Expression::ArrayExpression(_) => (None, Some(dependency_names(first)?), *factory),
            _ => (Some(text(first)?), None, *factory),
        },
        [id, dependencies, factory] => (
            Some(text(id)?),
            Some(dependency_names(dependencies)?),
            *factory,
        ),
        _ => return None,
    };
    let callback = callback(factory)?;
    let names = callback.names()?;
    let dependencies = dependencies.unwrap_or_default();
    if names.len() > dependencies.len() {
        return None;
    }
    Some(Define {
        id,
        callback,
        pairs: pair_up(&names, dependencies),
    })
}

pub fn defines<'k, 'a>(top: &'k [Statement<'a>]) -> Option<Vec<Define<'k, 'a>>> {
    let mut found = Vec::new();
    for statement in top {
        if matches!(statement, Statement::EmptyStatement(_)) {
            continue;
        }
        found.push(parse(define_call(statement)?)?);
    }
    (!found.is_empty()).then_some(found)
}

pub fn identified<'k, 'a>(
    mut defines: Vec<Define<'k, 'a>>,
) -> Option<Vec<(String, Define<'k, 'a>)>> {
    let anonymous = defines.iter().filter(|define| define.id.is_none()).count();
    if anonymous > 1 || (anonymous == 1 && defines.len() > 1) {
        return None;
    }
    let mut seen: FxHashMap<String, ()> = FxHashMap::default();
    let mut named = Vec::with_capacity(defines.len());
    for mut define in defines.drain(..) {
        let id = define.id.take().unwrap_or_else(|| "main".to_string());
        if seen.insert(id.clone(), ()).is_some() {
            return None;
        }
        named.push((id, define));
    }
    Some(named)
}

pub fn acyclic_and_resolved(modules: &[(String, Define<'_, '_>)]) -> bool {
    let index: FxHashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(position, (id, _))| (id.as_str(), position))
        .collect();
    let has = |candidate: &str| index.contains_key(candidate);
    let mut waiting = vec![0usize; modules.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); modules.len()];
    for (position, (id, define)) in modules.iter().enumerate() {
        for (_, dependency) in &define.pairs {
            if dependency.is_empty() || dependency == EXPORTS {
                continue;
            }
            match target_of(has, id, dependency) {
                Target::Module(target) => {
                    waiting[position] += 1;
                    dependents[index[target.as_str()]].push(position);
                }
                Target::External => {}
                Target::Missing => return false,
            }
        }
    }
    let mut ready: Vec<usize> = (0..modules.len()).filter(|&at| waiting[at] == 0).collect();
    let mut done = 0;
    while let Some(next) = ready.pop() {
        done += 1;
        for &dependent in &dependents[next] {
            waiting[dependent] -= 1;
            if waiting[dependent] == 0 {
                ready.push(dependent);
            }
        }
    }
    done == modules.len()
}
