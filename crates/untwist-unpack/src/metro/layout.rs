use oxc_ast::ast::*;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::number_key;

use crate::matcher::{argument_expression, param_names, Factory};
use crate::model::BundleKind;
use crate::runtime::runtime_id;

pub const FACTORY_PARAMS: usize = 7;
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

pub struct Def<'k, 'a> {
    pub id: String,
    pub numeric: bool,
    pub function: &'k Function<'a>,
    pub deps: Option<&'k ArrayExpression<'a>>,
}

impl<'k, 'a> Def<'k, 'a> {
    pub fn factory(&self) -> Option<Factory<'k, 'a>> {
        Some(Factory {
            params: &self.function.params,
            body: self.function.body.as_deref()?,
            arrow: false,
        })
    }

    pub fn params(&self) -> Vec<&'a str> {
        param_names(&self.function.params)
    }
}

pub struct Layout<'k, 'a> {
    pub define: &'k str,
    pub prelude: usize,
    pub defs: Vec<Def<'k, 'a>>,
    pub runs: Vec<(String, bool)>,
    pub numeric: bool,
}

fn called<'k, 'a>(statement: &'k Statement<'a>) -> Option<(&'k str, &'k CallExpression<'a>)> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::CallExpression(call) = &statement.expression else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    (!call.optional).then_some((callee.name.as_str(), call))
}

fn define_prefix<'k>(statement: &'k Statement<'_>) -> Option<&'k str> {
    called(statement)?.0.strip_suffix("__d")
}

pub fn module_id(expression: &Expression<'_>) -> Option<String> {
    match expression {
        Expression::NumericLiteral(literal)
            if literal.value.fract() == 0.0
                && literal.value >= 0.0
                && literal.value <= MAX_SAFE_INTEGER =>
        {
            Some(number_key(literal.value))
        }
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        _ => None,
    }
}

pub fn typed_id(expression: &Expression<'_>) -> Option<(String, bool)> {
    let numeric = matches!(expression, Expression::NumericLiteral(_));
    module_id(expression).map(|id| (id, numeric))
}

fn argument<'k, 'a>(call: &'k CallExpression<'a>, index: usize) -> Option<&'k Expression<'a>> {
    call.arguments
        .get(index)
        .and_then(argument_expression)
        .map(Expression::without_parentheses)
}

fn is_inert(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
            element
                .as_expression()
                .is_some_and(|element| is_inert(element.without_parentheses()))
        }),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            !property.computed
                && !property.method
                && property.kind == PropertyKind::Init
                && is_inert(property.value.without_parentheses())
        }),
        _ => false,
    }
}

fn factory_function<'k, 'a>(expression: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    let Expression::FunctionExpression(function) = expression else {
        return None;
    };
    if function.r#async || function.generator || function.params.rest.is_some() {
        return None;
    }
    let names = param_names(&function.params);
    if names.len() != FACTORY_PARAMS || function.params.items.len() != FACTORY_PARAMS {
        return None;
    }
    let distinct: FxHashSet<&str> = names.iter().copied().collect();
    if distinct.len() != FACTORY_PARAMS
        || function
            .params
            .items
            .iter()
            .any(|p| p.initializer.is_some())
    {
        return None;
    }
    function.body.as_ref()?;
    Some(function)
}

fn dependency_list<'k, 'a>(expression: &'k Expression<'a>) -> Option<&'k ArrayExpression<'a>> {
    let Expression::ArrayExpression(array) = expression else {
        return None;
    };
    array
        .elements
        .iter()
        .all(|element| element.as_expression().and_then(module_id).is_some())
        .then_some(array)
}

fn definition<'k, 'a>(call: &'k CallExpression<'a>) -> Option<Def<'k, 'a>> {
    if call.arguments.len() < 2 || call.arguments.iter().any(|a| a.is_spread()) {
        return None;
    }
    let function = factory_function(argument(call, 0)?)?;
    let (id, numeric) = typed_id(argument(call, 1)?)?;
    let deps = match argument(call, 2) {
        Some(expression) => Some(dependency_list(expression)?),
        None => None,
    };
    let extras_inert = (3..call.arguments.len()).all(|at| argument(call, at).is_some_and(is_inert));
    extras_inert.then_some(Def {
        id,
        numeric,
        function,
        deps,
    })
}

fn run_target(statement: &Statement<'_>, prefix: &str) -> Option<(String, bool)> {
    let (name, call) = called(statement)?;
    let run_prefix = name.strip_suffix("__r")?;
    if (run_prefix != prefix && !run_prefix.is_empty()) || call.arguments.len() != 1 {
        return None;
    }
    typed_id(argument(call, 0)?)
}

fn chosen_prefix<'k>(top: &'k [Statement<'_>]) -> Option<&'k str> {
    let mut counts: FxHashMap<&str, (usize, usize)> = FxHashMap::default();
    for (order, statement) in top.iter().enumerate() {
        let Some(prefix) = define_prefix(statement) else {
            continue;
        };
        let valid = called(statement)
            .and_then(|(_, call)| definition(call))
            .is_some();
        counts.entry(prefix).or_insert((order, 0)).1 += usize::from(valid);
    }
    counts
        .into_iter()
        .filter(|(_, (_, count))| *count > 0)
        .max_by_key(|(_, (first, count))| (*count, std::cmp::Reverse(*first)))
        .map(|(prefix, _)| prefix)
}

fn keyable(id: &str) -> bool {
    id != "__proto__" && id != super::ENTRY_ID
}

fn deps_typed(deps: Option<&ArrayExpression<'_>>, numeric: bool) -> bool {
    deps.is_none_or(|deps| {
        deps.elements.iter().all(|element| {
            element
                .as_expression()
                .and_then(typed_id)
                .is_some_and(|(id, kind)| kind == numeric && keyable(&id))
        })
    })
}

pub fn probe(top: &[Statement<'_>]) -> bool {
    top.iter().any(|statement| {
        called(statement).is_some_and(|(name, call)| {
            name.ends_with("__d")
                && argument(call, 0).is_some_and(|f| matches!(f, Expression::FunctionExpression(_)))
                && argument(call, 1).and_then(module_id).is_some()
        })
    })
}

pub fn layout<'k, 'a>(top: &'k [Statement<'a>]) -> Option<Layout<'k, 'a>> {
    let prefix = chosen_prefix(top)?;
    let prelude = top
        .iter()
        .position(|statement| define_prefix(statement) == Some(prefix))?;
    let define = called(&top[prelude])?.0;
    let mut defs = Vec::new();
    let mut runs = Vec::new();
    for statement in top.iter().skip(prelude) {
        if matches!(statement, Statement::EmptyStatement(_)) {
            continue;
        }
        if define_prefix(statement) == Some(prefix) {
            if !runs.is_empty() {
                return None;
            }
            defs.push(definition(called(statement)?.1)?);
            continue;
        }
        runs.push(run_target(statement, prefix)?);
    }
    let numeric = defs.first()?.numeric;
    let ids: FxHashSet<&str> = defs.iter().map(|def| def.id.as_str()).collect();
    let runtime = runtime_id(BundleKind::Metro);
    let closed = ids.len() == defs.len()
        && !ids.contains(runtime.as_str())
        && runs.iter().all(|(run, _)| ids.contains(run.as_str()));
    let typed = defs
        .iter()
        .all(|def| def.numeric == numeric && keyable(&def.id))
        && runs.iter().all(|(_, run)| *run == numeric)
        && defs.iter().all(|def| deps_typed(def.deps, numeric));
    (closed && typed).then_some(Layout {
        define,
        prelude,
        defs,
        runs,
        numeric,
    })
}
