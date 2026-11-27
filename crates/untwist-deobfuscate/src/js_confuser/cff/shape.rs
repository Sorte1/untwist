use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::SymbolId;

use super::super::names::Names;
use super::super::refs::{inside, Refs};
use super::super::templates::{cff_helper, CffHelper};
use super::value::symbol_of;

pub(super) struct Helpers {
    pub(super) sum: SymbolId,
    pub(super) slice: Option<SymbolId>,
    pub(super) xor: Option<SymbolId>,
    pub(super) sequence: Vec<f64>,
    pub(super) strings: Vec<u16>,
    pub(super) symbols: Vec<SymbolId>,
    pub(super) statements: Vec<Span>,
}

struct Declared<'k, 'a> {
    symbol: SymbolId,
    init: &'k Expression<'a>,
    statement: Span,
}

fn top_vars<'k, 'a>(program: &'k Program<'a>) -> Vec<Declared<'k, 'a>> {
    let mut found = Vec::new();
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                (&declarator.id, &declarator.init)
            {
                if let Some(symbol) = id.symbol_id.get() {
                    found.push(Declared {
                        symbol,
                        init,
                        statement: statement.span(),
                    });
                }
            }
        }
    }
    found
}

fn numbers(init: &Expression<'_>) -> Option<Vec<f64>> {
    let Expression::ArrayExpression(array) = init else {
        return None;
    };
    array
        .elements
        .iter()
        .map(|element| match element.as_expression()? {
            Expression::NumericLiteral(n) => Some(n.value),
            Expression::UnaryExpression(u)
                if u.operator == oxc_syntax::operator::UnaryOperator::UnaryNegation =>
            {
                match &u.argument {
                    Expression::NumericLiteral(n) => Some(-n.value),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

fn outside_symbol(refs: &Refs<'_>, scoping: &Scoping, region: Span) -> Option<SymbolId> {
    let mut outside = refs
        .all
        .iter()
        .filter(|(r, _)| inside(region, r.span))
        .filter_map(|(r, _)| r.symbol)
        .filter(|symbol| !inside(region, scoping.symbol_span(*symbol)));
    let first = outside.next()?;
    outside.all(|other| other == first).then_some(first)
}

pub(super) fn helpers(
    program: &Program<'_>,
    (refs, scoping, names): (&Refs<'_>, &Scoping, &Names),
) -> Option<Helpers> {
    let mut sum = None;
    let mut slice = None;
    let mut xor = None;
    let mut symbols = Vec::new();
    let mut statements = Vec::new();
    for statement in &program.body {
        let Statement::FunctionDeclaration(function) = statement else {
            continue;
        };
        let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) else {
            continue;
        };
        let Some(kind) = cff_helper(program.source_text, statement.span(), names) else {
            continue;
        };
        if !refs.stable(symbol, scoping) {
            return None;
        }
        let read = outside_symbol(refs, scoping, statement.span());
        match kind {
            CffHelper::Sum => sum = Some(symbol),
            CffHelper::Slice => slice = Some((symbol, read?)),
            CffHelper::Xor => xor = Some((symbol, read?)),
            CffHelper::Hash => {}
        }
        symbols.push(symbol);
        statements.push(statement.span());
    }
    let vars = top_vars(program);
    let var_of = |symbol: SymbolId| vars.iter().find(|v| v.symbol == symbol);
    let sequence = match slice {
        Some((_, array)) => {
            let declared = var_of(array)?;
            symbols.push(array);
            statements.push(declared.statement);
            numbers(declared.init)?
        }
        None => Vec::new(),
    };
    let strings = match xor {
        Some((_, text)) => {
            let declared = var_of(text)?;
            symbols.push(text);
            statements.push(declared.statement);
            match declared.init {
                Expression::StringLiteral(literal) => literal.value.encode_utf16().collect(),
                _ => return None,
            }
        }
        None => Vec::new(),
    };
    statements.sort_by_key(|s| s.start);
    statements.dedup();
    Some(Helpers {
        sum: sum?,
        slice: slice.map(|(s, _)| s),
        xor: xor.map(|(s, _)| s),
        sequence,
        strings,
        symbols,
        statements,
    })
}

pub(super) struct Main<'k, 'a> {
    pub(super) symbol: SymbolId,
    pub(super) function: &'k Function<'a>,
    pub(super) states: SymbolId,
    pub(super) scope: SymbolId,
    pub(super) runtime: SymbolId,
    pub(super) arg: Option<SymbolId>,
    pub(super) roots: Vec<String>,
    pub(super) end: f64,
    pub(super) cases: &'k [SwitchCase<'a>],
    pub(super) label: Option<String>,
}

fn param_symbol(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn empty_object(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::ObjectExpression(o) if o.properties.is_empty())
}

pub(super) fn string_key(key: &PropertyKey<'_>, computed: bool) -> Option<String> {
    match key {
        PropertyKey::StringLiteral(lit) => Some(lit.value.as_str()?.to_string()),
        PropertyKey::StaticIdentifier(id) if !computed => Some(id.name.to_string()),
        _ => None,
    }
}

fn root_scopes(init: &Expression<'_>) -> Option<Vec<String>> {
    let Expression::ObjectExpression(object) = init else {
        return None;
    };
    object
        .properties
        .iter()
        .map(|property| match property {
            ObjectPropertyKind::ObjectProperty(p)
                if p.kind == PropertyKind::Init && !p.method && empty_object(&p.value) =>
            {
                string_key(&p.key, p.computed)
            }
            _ => None,
        })
        .collect()
}

fn dispatcher<'k, 'a>(
    function: &'k Function<'a>,
    (scoping, sum, states): (&Scoping, SymbolId, SymbolId),
) -> Option<(f64, &'k [SwitchCase<'a>], Option<String>)> {
    let body = function.body.as_ref()?;
    let [Statement::WhileStatement(looped)] = body.statements.as_slice() else {
        return None;
    };
    let sums = |expr: &Expression<'_>| {
        matches!(expr.without_parentheses(), Expression::CallExpression(call)
            if symbol_of(&call.callee, scoping) == Some(sum)
                && matches!(call.arguments.as_slice(), [arg] if arg.as_expression().and_then(|e| symbol_of(e, scoping)) == Some(states)))
    };
    let Expression::BinaryExpression(test) = &looped.test else {
        return None;
    };
    let end = match &test.right {
        Expression::NumericLiteral(n) => n.value,
        Expression::UnaryExpression(u)
            if u.operator == oxc_syntax::operator::UnaryOperator::UnaryNegation =>
        {
            match &u.argument {
                Expression::NumericLiteral(n) => -n.value,
                _ => return None,
            }
        }
        _ => return None,
    };
    if test.operator != BinaryOperator::StrictInequality || !sums(&test.left) {
        return None;
    }
    let inner = match &looped.body {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only,
            _ => return None,
        },
        other => other,
    };
    let (switch, label) = match inner {
        Statement::SwitchStatement(switch) => (switch, None),
        Statement::LabeledStatement(labeled) => match &labeled.body {
            Statement::SwitchStatement(switch) => (switch, Some(labeled.label.name.to_string())),
            _ => return None,
        },
        _ => return None,
    };
    (sums(&switch.discriminant) && body.directives.is_empty()).then_some((
        end,
        switch.cases.as_slice(),
        label,
    ))
}

pub(super) fn main<'k, 'a>(
    function: &'k Function<'a>,
    scoping: &Scoping,
    sum: SymbolId,
) -> Option<Main<'k, 'a>> {
    if function.r#async || function.generator || function.params.rest.is_some() {
        return None;
    }
    let symbol = function.id.as_ref()?.symbol_id.get()?;
    let params = &function.params.items;
    let (states, scope, runtime) = match params.as_slice() {
        [a, b, c] | [a, b, c, _] => (a, b, c),
        _ => return None,
    };
    let roots = root_scopes(scope.initializer.as_deref()?)?;
    if states.initializer.is_some() || runtime.initializer.is_some() {
        return None;
    }
    let arg = match params.get(3) {
        Some(p) if p.initializer.is_none() => Some(param_symbol(p)?),
        Some(_) => return None,
        None => None,
    };
    let states = param_symbol(states)?;
    let (end, cases, label) = dispatcher(function, (scoping, sum, states))?;
    Some(Main {
        symbol,
        function,
        states,
        scope: param_symbol(scope)?,
        runtime: param_symbol(runtime)?,
        arg,
        roots,
        end,
        cases,
        label,
    })
}
