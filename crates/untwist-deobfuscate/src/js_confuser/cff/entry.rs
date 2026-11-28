use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::super::refs::Refs;
use super::super::resolution::Spans;
use super::build::{Context, Plan, Syms, Wrap};
use super::shape::{main, string_key, Helpers, Main};
use super::structure::structure;
use super::value::symbol_of;
use super::walk::Walker;

struct WrapperShape {
    span: Span,
    rest: String,
    rest_symbol: SymbolId,
    vector: Vec<f64>,
    fresh: String,
}

struct Scan<'s> {
    pub(super) scoping: &'s Scoping,
    main: SymbolId,
    scope: SymbolId,
    runtime: SymbolId,
    pub(super) helpers: &'s Helpers,
    found: Vec<WrapperShape>,
}

fn entry_vector(array: &Expression<'_>, helpers: &Helpers, scoping: &Scoping) -> Option<Vec<f64>> {
    let Expression::ArrayExpression(array) = array.without_parentheses() else {
        return None;
    };
    let mut out = Vec::new();
    for element in &array.elements {
        match element {
            ArrayExpressionElement::SpreadElement(spread) => {
                let Expression::CallExpression(call) = &spread.argument else {
                    return None;
                };
                if helpers.slice.is_none() || symbol_of(&call.callee, scoping) != helpers.slice {
                    return None;
                }
                let bounds: Vec<f64> = call
                    .arguments
                    .iter()
                    .map(|a| match a.as_expression()? {
                        Expression::NumericLiteral(n) => Some(n.value),
                        _ => None,
                    })
                    .collect::<Option<_>>()?;
                let [start, end] = bounds.as_slice() else {
                    return None;
                };
                let whole = |v: f64| v.fract() == 0.0 && v >= 0.0;
                if !whole(*start) || !whole(*end) || start > end {
                    return None;
                }
                out.extend_from_slice(helpers.sequence.get(*start as usize..*end as usize)?);
            }
            other => {
                let value = match other.as_expression()? {
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
                out.push(value);
            }
        }
    }
    Some(out)
}

fn fresh_scope(object: &Expression<'_>, scope: SymbolId, scoping: &Scoping) -> Option<String> {
    let Expression::ObjectExpression(object) = object else {
        return None;
    };
    let mut fresh = None;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(p) = property else {
            return None;
        };
        if p.kind != PropertyKind::Init || p.method {
            return None;
        }
        let key = string_key(&p.key, p.computed)?;
        match &p.value {
            Expression::ObjectExpression(o) if o.properties.is_empty() => {
                if fresh.replace(key).is_some() {
                    return None;
                }
            }
            value => {
                let passed = match value.without_parentheses() {
                    Expression::StaticMemberExpression(m) => {
                        symbol_of(&m.object, scoping) == Some(scope)
                            && m.property.name.as_str() == key
                    }
                    Expression::ComputedMemberExpression(m) => {
                        symbol_of(&m.object, scoping) == Some(scope)
                            && matches!(&m.expression, Expression::StringLiteral(s) if s.value.as_str() == Some(key.as_str()))
                    }
                    _ => false,
                };
                if !passed {
                    return None;
                }
            }
        }
    }
    fresh
}

impl Scan<'_> {
    fn wrapper(&self, function: &Function<'_>) -> Option<WrapperShape> {
        let rest = function.params.rest.as_ref()?;
        let BindingPattern::BindingIdentifier(rest_id) = &rest.rest.argument else {
            return None;
        };
        if !function.params.items.is_empty()
            || function.r#async
            || function.generator
            || function.id.is_some()
        {
            return None;
        }
        let body = function.body.as_ref()?;
        let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
            return None;
        };
        let Some(Expression::CallExpression(call)) = &ret.argument else {
            return None;
        };
        if symbol_of(&call.callee, self.scoping) != Some(self.main) || !body.directives.is_empty() {
            return None;
        }
        let rest_symbol = rest_id.symbol_id.get()?;
        let [Argument::ArrayExpression(_), object, runtime, passed] = call.arguments.as_slice()
        else {
            return None;
        };
        let runtime_ok = runtime
            .as_expression()
            .and_then(|e| symbol_of(e, self.scoping))
            == Some(self.runtime);
        let passed_ok = passed
            .as_expression()
            .and_then(|e| symbol_of(e, self.scoping))
            == Some(rest_symbol);
        if !runtime_ok || !passed_ok {
            return None;
        }
        let array = call.arguments[0].as_expression()?;
        Some(WrapperShape {
            span: function.span,
            rest: rest_id.name.to_string(),
            rest_symbol,
            vector: entry_vector(array, self.helpers, self.scoping)?,
            fresh: fresh_scope(object.as_expression()?, self.scope, self.scoping)?,
        })
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if !it.is_declaration() {
            if let Some(found) = self.wrapper(it) {
                self.found.push(found);
                return;
            }
        }
        walk_js::walk_function(self, it, flags);
    }
}

struct Entry {
    pub(super) spans: Vec<Span>,
    vector: Vec<f64>,
    context: Context,
    extra: Vec<SymbolId>,
}

fn call_of<'k, 'a>(
    expr: &'k Expression<'a>,
    main: SymbolId,
    scoping: &Scoping,
) -> Option<&'k Expression<'a>> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    if symbol_of(&call.callee, scoping) != Some(main) || call.optional {
        return None;
    }
    match call.arguments.as_slice() {
        [Argument::ArrayExpression(_)] => call.arguments[0].as_expression(),
        _ => None,
    }
}

fn bare_var<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(SymbolId, Option<&'k Expression<'a>>)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    (declaration.kind == VariableDeclarationKind::Var)
        .then_some((id.symbol_id.get()?, declarator.init.as_ref()))
}

fn function_entry(
    rest: &[Statement<'_>],
    main: SymbolId,
    helpers: &Helpers,
    scoping: &Scoping,
) -> Option<Entry> {
    let [flag_statement, result_statement, Statement::IfStatement(check)] = rest else {
        return None;
    };
    let (flag, None) = bare_var(flag_statement)? else {
        return None;
    };
    let (result, Some(init)) = bare_var(result_statement)? else {
        return None;
    };
    let array = call_of(init, main, scoping)?;
    let returns = match &check.consequent {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
            _ => None,
        },
        Statement::ReturnStatement(ret) => ret.argument.as_ref(),
        _ => None,
    };
    let shaped = check.alternate.is_none()
        && symbol_of(&check.test, scoping) == Some(flag)
        && returns.and_then(|r| symbol_of(r, scoping)) == Some(result);
    shaped.then(|| Entry {
        spans: rest.iter().map(GetSpan::span).collect(),
        vector: entry_vector(array, helpers, scoping).unwrap_or_default(),
        context: Context::Function(flag),
        extra: vec![flag, result],
    })
}

fn program_entry(
    rest: &[Statement<'_>],
    main: SymbolId,
    helpers: &Helpers,
    scoping: &Scoping,
) -> Option<Entry> {
    let [Statement::ExpressionStatement(statement)] = rest else {
        return None;
    };
    let array = call_of(&statement.expression, main, scoping)?;
    Some(Entry {
        spans: vec![statement.span],
        vector: entry_vector(array, helpers, scoping)?,
        context: Context::Program,
        extra: Vec::new(),
    })
}

pub(super) struct Finder<'s> {
    pub(super) scoping: &'s Scoping,
    pub(super) spans: &'s Spans,
    pub(super) refs: &'s Refs<'s>,
    pub(super) helpers: &'s Helpers,
    pub(super) plans: Vec<Plan>,
    pub(super) mains: Vec<Span>,
    pub(super) notes: Vec<&'static str>,
    pub(super) extras: Vec<SymbolId>,
}

impl Finder<'_> {
    fn consider(&mut self, statements: &[Statement<'_>], body: Span, in_function: bool) {
        for (index, statement) in statements.iter().enumerate() {
            let Statement::FunctionDeclaration(function) = statement else {
                continue;
            };
            let Some(found) = main(function, self.scoping, self.helpers.sum) else {
                continue;
            };
            self.mains.push(statement.span());
            let rest = &statements[index + 1..];
            let entry = match in_function {
                true if index == 0 => {
                    function_entry(rest, found.symbol, self.helpers, self.scoping)
                }
                true => None,
                false => program_entry(rest, found.symbol, self.helpers, self.scoping),
            };
            let Some(entry) = entry.filter(|e| !e.vector.is_empty()) else {
                self.notes
                    .push("its entry is not the js-confuser call form");
                continue;
            };
            match self.plan(&found, statement.span(), body, entry) {
                Ok(plan) => self.plans.push(plan),
                Err(reason) => self.notes.push(reason),
            }
        }
    }

    fn plan(
        &mut self,
        found: &Main<'_, '_>,
        statement: Span,
        body: Span,
        entry: Entry,
    ) -> Result<Plan, &'static str> {
        let mut scan = Scan {
            scoping: self.scoping,
            main: found.symbol,
            scope: found.scope,
            runtime: found.runtime,
            helpers: self.helpers,
            found: Vec::new(),
        };
        if let Some(body) = &found.function.body {
            scan.visit_function_body(body);
        }
        let calls = self.refs.of(found.symbol);
        if calls.len() != scan.found.len() + 1 || !self.refs.stable(found.symbol, self.scoping) {
            return Err(
                "its main function is used other than through its entry and function wrappers",
            );
        }
        let mut walker = Walker::new(found, (self.scoping, self.spans), self.helpers.sum);
        let graph = walker
            .graph(entry.vector)
            .ok_or("a state of its machine could not be stepped")?;
        let out =
            structure(&graph).ok_or("its recovered blocks do not form nested if statements")?;
        let mut wrappers = FxHashMap::default();
        for shape in scan.found {
            let graph = walker
                .graph(shape.vector)
                .ok_or("a state of a flattened function could not be stepped")?;
            let wrapped = structure(&graph).ok_or("a flattened function's blocks do not nest")?;
            self.extras.push(shape.rest_symbol);
            wrappers.insert(
                shape.span,
                Wrap {
                    rest: shape.rest,
                    rest_symbol: shape.rest_symbol,
                    fresh: shape.fresh,
                    out: wrapped,
                },
            );
        }
        self.extras.extend(entry.extra.iter().copied());
        Ok(Plan {
            body,
            main_statement: statement,
            entry: entry.spans,
            context: entry.context,
            roots: found.roots.clone(),
            out,
            wrappers,
            vectors: walker.vectors,
            syms: Syms {
                main: found.symbol,
                states: found.states,
                scope: found.scope,
                runtime: found.runtime,
                arg: found.arg,
                sum: self.helpers.sum,
                slice: self.helpers.slice,
                xor: self.helpers.xor,
                entry: entry.extra,
            },
        })
    }
}

impl<'a> VisitJs<'a> for Finder<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.consider(&it.body, it.span, false);
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.consider(&it.statements, it.span, true);
        walk_js::walk_function_body(self, it);
    }
}
