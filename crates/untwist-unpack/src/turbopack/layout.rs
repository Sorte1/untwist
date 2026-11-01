use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::{number_key, prop_name};

use crate::frame::Frame;
use crate::matcher::expression_statement;

const HOLDER: &str = "TURBOPACK";
const MAX_SAFE_ID: f64 = 9_007_199_254_740_991.0;
const OTHER_CHUNKS: &str = "otherChunks";
const RUNTIME_MODULE_IDS: &str = "runtimeModuleIds";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    pub key: String,
    pub number: bool,
}

impl Id {
    pub fn of(expr: &Expression<'_>) -> Option<Self> {
        match expr {
            Expression::NumericLiteral(literal) => {
                let value = literal.value;
                let whole = value.fract() == 0.0 && (0.0..=MAX_SAFE_ID).contains(&value);
                whole.then(|| Self {
                    key: number_key(value),
                    number: true,
                })
            }
            Expression::StringLiteral(literal) if !literal.lone_surrogates => Some(Self {
                key: literal.value.to_string(),
                number: false,
            }),
            _ => None,
        }
    }
}

pub struct Def<'k, 'a> {
    pub id: Id,
    pub params: &'k FormalParameters<'a>,
    pub body: &'k FunctionBody<'a>,
}

pub struct Layout<'k, 'a> {
    pub defs: Vec<Def<'k, 'a>>,
    pub entries: Vec<Id>,
    pub strict: bool,
}

fn holder_member(expr: &Expression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
        return false;
    };
    !member.optional
        && member.property.name == HOLDER
        && matches!(&member.object, Expression::Identifier(id) if id.name == "globalThis")
}

fn holder_target(target: &AssignmentTarget<'_>) -> bool {
    let AssignmentTarget::StaticMemberExpression(member) = target else {
        return false;
    };
    !member.optional
        && member.property.name == HOLDER
        && matches!(&member.object, Expression::Identifier(id) if id.name == "globalThis")
}

fn empty_array(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::ArrayExpression(array) if array.elements.is_empty())
}

fn fresh_holder(expr: &Expression<'_>, operator: AssignmentOperator) -> bool {
    let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
        return false;
    };
    assign.operator == operator && holder_target(&assign.left) && empty_array(&assign.right)
}

fn holder(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::LogicalExpression(logical) => {
            logical.operator == LogicalOperator::Or
                && holder_member(&logical.left)
                && fresh_holder(&logical.right, AssignmentOperator::Assign)
        }
        other => fresh_holder(other, AssignmentOperator::LogicalOr),
    }
}

fn pushed<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k ArrayExpression<'a>> {
    let Expression::CallExpression(call) = expression_statement(statement)? else {
        return None;
    };
    let Expression::StaticMemberExpression(push) = &call.callee else {
        return None;
    };
    if call.optional || push.optional || push.property.name != "push" || !holder(&push.object) {
        return None;
    }
    match call.arguments.as_slice() {
        [Argument::ArrayExpression(array)] => Some(array),
        _ => None,
    }
}

pub fn probe_top(top: &[Statement<'_>]) -> bool {
    top.iter()
        .find(|statement| !matches!(statement, Statement::EmptyStatement(_)))
        .is_some_and(|statement| pushed(statement).is_some())
}

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    probe_top(frame.top) || server_array(frame.top).is_some()
}

fn exports_array<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k ArrayExpression<'a>> {
    let Expression::AssignmentExpression(assign) = expression_statement(statement)? else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(target) = &assign.left else {
        return None;
    };
    let fits = assign.operator == AssignmentOperator::Assign
        && !target.optional
        && target.property.name == "exports"
        && matches!(&target.object, Expression::Identifier(id) if id.name == "module");
    match &assign.right {
        Expression::ArrayExpression(array) if fits => Some(array),
        _ => None,
    }
}

fn strict_arrow<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k ArrayExpression<'a>> {
    let Expression::CallExpression(call) = expression_statement(statement)? else {
        return None;
    };
    let Expression::ArrowFunctionExpression(arrow) = call.callee.without_parentheses() else {
        return None;
    };
    let plain = !call.optional
        && call.arguments.is_empty()
        && !arrow.r#async
        && arrow.params.items.is_empty()
        && arrow.params.rest.is_none();
    let body = arrow.body.as_function_body()?;
    let strict = matches!(body.directives.as_slice(), [only] if only.directive == "use strict");
    let [inner] = body.statements.as_slice() else {
        return None;
    };
    (plain && strict).then(|| exports_array(inner)).flatten()
}

fn server_array<'k, 'a>(top: &'k [Statement<'a>]) -> Option<(&'k ArrayExpression<'a>, bool)> {
    let mut statements = top
        .iter()
        .filter(|statement| !matches!(statement, Statement::EmptyStatement(_)));
    let (Some(only), None) = (statements.next(), statements.next()) else {
        return None;
    };
    exports_array(only)
        .map(|array| (array, false))
        .or_else(|| strict_arrow(only).map(|array| (array, true)))
}

pub fn server<'k, 'a>(source: &'k Program<'a>) -> Option<Layout<'k, 'a>> {
    if !source.directives.is_empty() {
        return None;
    }
    let (array, strict) = server_array(&source.body)?;
    let mut defs = Vec::new();
    modules(&elements(array)?, &mut defs)?;
    let entries = Vec::new();
    (!defs.is_empty() && defined_once(&defs, &entries)).then_some(Layout {
        defs,
        entries,
        strict,
    })
}

fn typeof_document(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::Typeof
            && matches!(&unary.argument, Expression::Identifier(id) if id.name == "document"))
}

fn object_text(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::StringLiteral(text) if text.value == "object")
}

fn current_script(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::StaticMemberExpression(member)
        if !member.optional
            && member.property.name == "currentScript"
            && matches!(&member.object, Expression::Identifier(id) if id.name == "document"))
}

fn void_zero(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier(id) => id.name == "undefined",
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void
                && matches!(&unary.argument, Expression::NumericLiteral(zero) if zero.value == 0.0)
        }
        _ => false,
    }
}

fn script(expr: &Expression<'_>) -> bool {
    let Expression::ConditionalExpression(conditional) = expr else {
        return matches!(expr, Expression::StringLiteral(_));
    };
    let Expression::BinaryExpression(test) = &conditional.test else {
        return false;
    };
    let compared = (object_text(&test.left) && typeof_document(&test.right))
        || (typeof_document(&test.left) && object_text(&test.right));
    matches!(
        test.operator,
        BinaryOperator::Equality | BinaryOperator::StrictEquality
    ) && compared
        && current_script(&conditional.consequent)
        && void_zero(&conditional.alternate)
}

fn elements<'k, 'a>(array: &'k ArrayExpression<'a>) -> Option<Vec<&'k Expression<'a>>> {
    array
        .elements
        .iter()
        .map(|element| element.as_expression())
        .collect()
}

fn factory<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k FormalParameters<'a>, &'k FunctionBody<'a>)> {
    match expr {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                return None;
            };
            Some((&arrow.params, body))
        }
        Expression::FunctionExpression(function)
            if function.id.is_none() && !function.r#async && !function.generator =>
        {
            Some((&function.params, function.body.as_deref()?))
        }
        _ => None,
    }
}

fn modules<'k, 'a>(items: &[&'k Expression<'a>], defs: &mut Vec<Def<'k, 'a>>) -> Option<()> {
    let mut pending: Option<Id> = None;
    for item in items {
        if let Some(id) = Id::of(item) {
            if pending.replace(id).is_some() {
                return None;
            }
            continue;
        }
        let (params, body) = factory(item)?;
        defs.push(Def {
            id: pending.take()?,
            params,
            body,
        });
    }
    pending.is_none().then_some(())
}

fn plain_array<'k, 'a>(
    property: &'k ObjectPropertyKind<'a>,
) -> Option<(String, Vec<&'k Expression<'a>>)> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    if property.computed
        || property.method
        || property.shorthand
        || property.kind != PropertyKind::Init
    {
        return None;
    }
    let Expression::ArrayExpression(array) = &property.value else {
        return None;
    };
    Some((prop_name(&property.key)?.into_owned(), elements(array)?))
}

fn registration(expr: &Expression<'_>) -> Option<Vec<Id>> {
    let Expression::ObjectExpression(object) = expr else {
        return None;
    };
    let [first, second] = object.properties.as_slice() else {
        return None;
    };
    let (mut others, mut runtime) = (None, None);
    for (name, items) in [plain_array(first)?, plain_array(second)?] {
        let slot = match name.as_str() {
            OTHER_CHUNKS => &mut others,
            RUNTIME_MODULE_IDS => &mut runtime,
            _ => return None,
        };
        if slot.replace(items).is_some() {
            return None;
        }
    }
    if !others?.is_empty() {
        return None;
    }
    runtime?.into_iter().map(Id::of).collect()
}

fn defined_once(defs: &[Def<'_, '_>], entries: &[Id]) -> bool {
    let mut keys = FxHashSet::default();
    let unique = defs.iter().all(|def| keys.insert(def.id.key.as_str()));
    let ids: FxHashSet<&Id> = defs.iter().map(|def| &def.id).collect();
    unique && entries.iter().all(|id| ids.contains(id))
}

pub fn layout<'k, 'a>(source: &'k Program<'a>) -> Option<Layout<'k, 'a>> {
    if !source.directives.is_empty() {
        return None;
    }
    let mut defs = Vec::new();
    let mut entries: Option<Vec<Id>> = None;
    for statement in &source.body {
        if matches!(statement, Statement::EmptyStatement(_)) {
            continue;
        }
        let items = elements(pushed(statement)?)?;
        let (first, rest) = items.split_first()?;
        if !script(first) {
            return None;
        }
        match rest {
            [only] => {
                if entries.replace(registration(only)?).is_some() {
                    return None;
                }
            }
            [_, _, ..] => modules(rest, &mut defs)?,
            [] => return None,
        }
    }
    let entries = entries.unwrap_or_default();
    (!defs.is_empty() && defined_once(&defs, &entries)).then_some(Layout {
        defs,
        entries,
        strict: false,
    })
}
