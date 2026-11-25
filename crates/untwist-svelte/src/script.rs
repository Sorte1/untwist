use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BindingPattern, CallExpression, Expression,
    ExpressionStatement, FormalParameters, Function, IdentifierReference, Program, Statement,
    StaticMemberExpression, VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
};
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::runtime::{symbol_of, Rt, Runtime};
use crate::text::{Piece, Site};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Signal,
    Getter,
    Dep(Span),
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub key: String,
    pub flags: u32,
    pub fallback: Option<Span>,
}

#[derive(Default)]
pub struct Facts {
    pub classes: FxHashMap<SymbolId, Class>,
    pub props: FxHashMap<SymbolId, Prop>,
    pub props_objects: FxHashSet<SymbolId>,
    pub states: FxHashSet<SymbolId>,
}

pub fn binding(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(ident) => Some(ident.symbol_id()),
        _ => None,
    }
}

pub fn param_symbols(params: &FormalParameters<'_>) -> Option<Vec<SymbolId>> {
    if params.rest.is_some() {
        return None;
    }
    params
        .items
        .iter()
        .map(|param| binding(&param.pattern))
        .collect()
}

pub fn number(expr: &Expression<'_>) -> Option<u32> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(literal)
            if literal.value >= 0.0 && literal.value.fract() == 0.0 =>
        {
            Some(literal.value as u32)
        }
        Expression::UnaryExpression(unary) if unary.operator.as_str() == "-" => None,
        _ => None,
    }
}

pub fn string<'e>(expr: &'e Expression<'_>) -> Option<&'e str> {
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked)
            .and_then(|cooked| cooked.as_str()),
        _ => None,
    }
}

pub fn arg<'e, 'a>(call: &'e CallExpression<'a>, index: usize) -> Option<&'e Expression<'a>> {
    call.arguments.get(index).and_then(Argument::as_expression)
}

pub fn fn_params<'e, 'a>(expr: &'e Expression<'a>) -> Option<&'e FormalParameters<'a>> {
    match expr.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        Expression::FunctionExpression(function) => Some(&function.params),
        _ => None,
    }
}

pub fn appends_to(
    statements: &[Statement<'_>],
    anchor: SymbolId,
    runtime: &Runtime,
    scoping: &Scoping,
) -> bool {
    statements.iter().any(|statement| {
        let Statement::ExpressionStatement(stmt) = statement else {
            return false;
        };
        let Expression::CallExpression(call) = stmt.expression.without_parentheses() else {
            return false;
        };
        runtime.resolve(&call.callee, scoping) == Some(Rt::Append)
            && matches!(arg(call, 0), Some(Expression::Identifier(ident)) if symbol_of(ident, scoping) == Some(anchor))
    })
}

pub fn component_params(
    function: &Function<'_>,
    runtime: &Runtime,
    scoping: &Scoping,
) -> Option<(SymbolId, Option<SymbolId>)> {
    let params = param_symbols(&function.params)?;
    let body = function.body.as_ref()?;
    let (anchor, props) = match params.as_slice() {
        [anchor] => (*anchor, None),
        [anchor, props] => (*anchor, Some(*props)),
        _ => return None,
    };
    appends_to(&body.statements, anchor, runtime, scoping).then_some((anchor, props))
}

struct Classify<'f> {
    runtime: &'f Runtime,
    scoping: &'f Scoping,
    facts: Facts,
}

impl<'f> Classify<'f> {
    fn rt(&self, expr: &Expression<'_>) -> Option<Rt> {
        self.runtime.resolve(expr, self.scoping)
    }

    fn mark_params(&mut self, expr: Option<&Expression<'_>>, picks: &[(usize, Class)]) {
        let Some(params) = expr.and_then(fn_params) else {
            return;
        };
        for (index, class) in picks {
            if let Some(symbol) = params
                .items
                .get(*index)
                .and_then(|param| binding(&param.pattern))
            {
                self.facts.classes.insert(symbol, *class);
            }
        }
    }
}

impl<'f> Classify<'f> {
    fn on_function<'a>(&mut self, function: &Function<'a>) {
        if let Some((_, Some(props))) = component_params(function, self.runtime, self.scoping) {
            self.facts.props_objects.insert(props);
        }
    }

    fn on_variable_declarator<'a>(&mut self, declarator: &VariableDeclarator<'a>) {
        if let (Some(symbol), Some(init)) = (binding(&declarator.id), declarator.init.as_ref()) {
            match init.without_parentheses() {
                Expression::CallExpression(call) => match self.rt(&call.callee) {
                    Some(Rt::State | Rt::Derived) => {
                        self.facts.classes.insert(symbol, Class::Signal);
                        if self.rt(&call.callee) == Some(Rt::State) {
                            self.facts.states.insert(symbol);
                        }
                    }
                    Some(Rt::Proxy) => {
                        self.facts.states.insert(symbol);
                    }
                    Some(Rt::Prop) => {
                        let key = arg(call, 1).and_then(string);
                        let flags = arg(call, 2).and_then(number);
                        if let (Some(key), Some(flags)) = (key, flags) {
                            self.facts.classes.insert(symbol, Class::Getter);
                            self.facts.props.insert(
                                symbol,
                                Prop {
                                    key: key.to_string(),
                                    flags,
                                    fallback: arg(call, 3).map(GetSpan::span),
                                },
                            );
                        }
                    }
                    _ => {}
                },
                Expression::ArrowFunctionExpression(arrow) => {
                    for param in arrow.params.items.iter().skip(1) {
                        let noop = param
                            .initializer
                            .as_ref()
                            .is_some_and(|init| self.rt(init) == Some(Rt::Noop));
                        if let (true, Some(symbol)) = (noop, binding(&param.pattern)) {
                            self.facts.classes.insert(symbol, Class::Getter);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_call_expression<'a>(&mut self, call: &CallExpression<'a>) {
        match self.rt(&call.callee) {
            Some(Rt::Each) => {
                let flags = arg(call, 1).and_then(number).unwrap_or(0);
                let mut picks = Vec::new();
                if flags & 1 != 0 {
                    picks.push((1, Class::Signal));
                }
                if flags & 2 != 0 {
                    picks.push((2, Class::Signal));
                }
                self.mark_params(arg(call, 4), &picks);
                if let Some(Expression::ArrowFunctionExpression(render)) =
                    arg(call, 4).map(Expression::without_parentheses)
                {
                    for symbol in leading_getters(arrow_statements(render)) {
                        self.facts.classes.insert(symbol, Class::Getter);
                    }
                }
            }
            Some(Rt::Await) => {
                self.mark_params(arg(call, 3), &[(1, Class::Signal)]);
                self.mark_params(arg(call, 4), &[(1, Class::Signal)]);
            }
            Some(Rt::TemplateEffect) => {
                if let (Some(params), Some(Expression::ArrayExpression(deps))) =
                    (arg(call, 0).and_then(fn_params), arg(call, 1))
                {
                    for (param, dep) in params.items.iter().zip(deps.elements.iter()) {
                        let body = dep.as_expression().and_then(thunk_body);
                        if let (Some(symbol), Some(body)) = (binding(&param.pattern), body) {
                            self.facts.classes.insert(symbol, Class::Dep(body));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn arrow_statements<'e, 'a>(arrow: &'e ArrowFunctionExpression<'a>) -> &'e [Statement<'a>] {
    arrow
        .get_function_body()
        .map_or(&[], |body| body.statements.as_slice())
}

pub fn arrow_value<'e, 'a>(arrow: &'e ArrowFunctionExpression<'a>) -> Option<&'e Expression<'a>> {
    if let Some(expr) = arrow.get_expression() {
        return Some(expr);
    }
    match arrow_statements(arrow) {
        [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
        _ => None,
    }
}

pub fn thunk_body(expr: &Expression<'_>) -> Option<Span> {
    let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
        return None;
    };
    if !arrow.params.items.is_empty() || arrow.params.rest.is_some() {
        return None;
    }
    arrow_value(arrow).map(GetSpan::span)
}

struct Sites<'f> {
    runtime: &'f Runtime,
    scoping: &'f Scoping,
    facts: &'f Facts,
    source: &'f str,
    sites: Vec<Site>,
    refs: Vec<(Span, SymbolId)>,
    keys: FxHashMap<SymbolId, Vec<String>>,
    bare: Option<Span>,
}

fn lit(text: impl Into<String>) -> Piece {
    Piece::Lit(text.into())
}

impl<'f> Sites<'f> {
    fn class_of(&self, expr: &Expression<'_>) -> Option<(SymbolId, Class, String)> {
        let Expression::Identifier(ident) = expr.without_parentheses() else {
            return None;
        };
        let symbol = symbol_of(ident, self.scoping)?;
        let class = *self.facts.classes.get(&symbol)?;
        Some((symbol, class, ident.name.to_string()))
    }

    fn signal(&self, expr: Option<&Expression<'_>>) -> Option<String> {
        match self.class_of(expr?)? {
            (_, Class::Signal, name) => Some(name),
            _ => None,
        }
    }

    fn assign(&self, span: Span, target: String, value: Piece) -> Site {
        let bare = self.bare == Some(span);
        let mut pieces = vec![
            lit(if bare {
                format!("{target} = ")
            } else {
                format!("({target} = ")
            }),
            value,
        ];
        if !bare {
            pieces.push(lit(")"));
        }
        Site { span, pieces }
    }

    fn delta(&self, call: &CallExpression<'_>) -> Option<&'static str> {
        match arg(call, 1).map(Expression::without_parentheses) {
            None => Some("++"),
            Some(Expression::UnaryExpression(unary))
                if unary.operator.as_str() == "-" && number(&unary.argument) == Some(1) =>
            {
                Some("--")
            }
            Some(other) if number(other) == Some(1) => Some("++"),
            _ => None,
        }
    }

    fn rune(&self, call: &CallExpression<'_>) -> Option<Site> {
        let span = call.span;
        let site = |pieces: Vec<Piece>| Some(Site { span, pieces });
        match self.runtime.resolve(&call.callee, self.scoping)? {
            Rt::Get if call.arguments.len() == 1 => site(vec![lit(self.signal(arg(call, 0))?)]),
            Rt::Set if (2..=3).contains(&call.arguments.len()) => {
                let target = self.signal(arg(call, 0))?;
                Some(self.assign(span, target, Piece::Src(arg(call, 1)?.span())))
            }
            Rt::Update | Rt::UpdatePre if (1..=2).contains(&call.arguments.len()) => {
                let target = self.signal(arg(call, 0))?;
                let op = self.delta(call)?;
                match self.runtime.resolve(&call.callee, self.scoping)? {
                    Rt::Update => site(vec![lit(format!("{target}{op}"))]),
                    _ => site(vec![lit(format!("{op}{target}"))]),
                }
            }
            Rt::State if call.arguments.is_empty() => site(vec![lit("$state()")]),
            Rt::State if call.arguments.len() == 1 => {
                let value = arg(call, 0)?;
                match value.without_parentheses() {
                    Expression::CallExpression(proxy)
                        if self.runtime.resolve(&proxy.callee, self.scoping) == Some(Rt::Proxy)
                            && proxy.arguments.len() == 1 =>
                    {
                        site(vec![
                            lit("$state("),
                            Piece::Src(arg(proxy, 0)?.span()),
                            lit(")"),
                        ])
                    }
                    other if is_primitive(other) => {
                        site(vec![lit("$state("), Piece::Src(value.span()), lit(")")])
                    }
                    _ => site(vec![lit("$state.raw("), Piece::Src(value.span()), lit(")")]),
                }
            }
            Rt::Derived if call.arguments.len() == 1 => match arg(call, 0)?.without_parentheses() {
                Expression::ArrowFunctionExpression(arrow)
                    if arrow.params.items.is_empty() && arrow.params.rest.is_none() =>
                {
                    match thunk_body(arg(call, 0)?) {
                        Some(body) if arrow.is_expression() => {
                            site(vec![lit("$derived("), Piece::Src(body), lit(")")])
                        }
                        _ => site(vec![lit("$derived.by("), Piece::Src(arrow.span), lit(")")]),
                    }
                }
                other => site(vec![
                    lit("$derived.by("),
                    Piece::Src(other.span()),
                    lit(")"),
                ]),
            },
            Rt::UserEffect if call.arguments.len() == 1 => site(vec![
                lit("$effect("),
                Piece::Src(arg(call, 0)?.span()),
                lit(")"),
            ]),
            Rt::UserPreEffect if call.arguments.len() == 1 => site(vec![
                lit("$effect.pre("),
                Piece::Src(arg(call, 0)?.span()),
                lit(")"),
            ]),
            Rt::UpdateProp | Rt::UpdatePreProp if (1..=2).contains(&call.arguments.len()) => {
                let (_, class, target) = self.class_of(arg(call, 0)?)?;
                if class != Class::Getter {
                    return None;
                }
                let op = self.delta(call)?;
                match self.runtime.resolve(&call.callee, self.scoping)? {
                    Rt::UpdateProp => site(vec![lit(format!("{target}{op}"))]),
                    _ => site(vec![lit(format!("{op}{target}"))]),
                }
            }
            Rt::Snapshot if call.arguments.len() == 1 => site(vec![
                lit("$state.snapshot("),
                Piece::Src(arg(call, 0)?.span()),
                lit(")"),
            ]),
            _ => None,
        }
    }

    fn getter_call(&self, call: &CallExpression<'_>) -> Option<Site> {
        let (_, class, name) = self.class_of(&call.callee)?;
        if class != Class::Getter {
            return None;
        }
        match call.arguments.len() {
            0 => Some(Site {
                span: call.span,
                pieces: vec![lit(name)],
            }),
            1 => Some(self.assign(call.span, name, Piece::Src(arg(call, 0)?.span()))),
            _ => None,
        }
    }
}

impl<'f> Sites<'f> {
    fn on_variable_declaration<'a>(&mut self, declaration: &VariableDeclaration<'a>) {
        let holds_state = declaration.declarations.iter().any(|declarator| {
            binding(&declarator.id).is_some_and(|symbol| self.facts.states.contains(&symbol))
        });
        if holds_state && declaration.kind == VariableDeclarationKind::Const {
            self.sites.push(Site {
                span: Span::new(declaration.span.start, declaration.span.start + 5),
                pieces: vec![lit("let")],
            });
        }
        for declarator in &declaration.declarations {
            let Some(Expression::CallExpression(call)) = declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses)
            else {
                continue;
            };
            if self.runtime.resolve(&call.callee, self.scoping) == Some(Rt::Proxy)
                && call.arguments.len() == 1
            {
                if let Some(value) = arg(call, 0) {
                    self.sites.push(Site {
                        span: call.span,
                        pieces: vec![lit("$state("), Piece::Src(value.span()), lit(")")],
                    });
                }
            }
        }
    }

    fn on_call_expression<'a>(&mut self, call: &CallExpression<'a>) {
        if let Some(site) = self.rune(call).or_else(|| self.getter_call(call)) {
            self.sites.push(site);
        }
    }

    fn on_static_member_expression<'a>(&mut self, member: &StaticMemberExpression<'a>) {
        if let Expression::Identifier(object) = &member.object {
            if let Some(symbol) = symbol_of(object, self.scoping)
                .filter(|symbol| self.facts.props_objects.contains(symbol))
            {
                let key = member.property.name.to_string();
                self.keys.entry(symbol).or_default().push(key.clone());
                self.sites.push(Site {
                    span: member.span,
                    pieces: vec![lit(key)],
                });
            }
        }
    }

    fn on_identifier_reference<'a>(&mut self, ident: &IdentifierReference<'a>) {
        if let Some(symbol) = symbol_of(ident, self.scoping) {
            self.refs.push((ident.span, symbol));
            if let Some(Class::Dep(body)) = self.facts.classes.get(&symbol) {
                self.sites.push(Site {
                    span: ident.span,
                    pieces: vec![Piece::Src(*body)],
                });
            }
        }
        let _ = self.source;
    }
}

pub struct Collected {
    pub facts: Facts,
    pub sites: Vec<Site>,
    pub refs: Vec<(Span, SymbolId)>,
    pub keys: FxHashMap<SymbolId, Vec<String>>,
}

enum Pass<'f, 'g> {
    Classify(&'g mut Classify<'f>),
    Sites(&'g mut Sites<'f>),
}

impl<'f, 'g, 'a> Visit<'a> for Pass<'f, 'g> {
    fn visit_function(&mut self, function: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if let Pass::Classify(pass) = self {
            pass.on_function(function);
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Pass::Classify(pass) = self {
            pass.on_variable_declarator(declarator);
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        match self {
            Pass::Classify(pass) => pass.on_call_expression(call),
            Pass::Sites(pass) => pass.on_call_expression(call),
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_expression_statement(&mut self, stmt: &ExpressionStatement<'a>) {
        let previous = match self {
            Pass::Sites(pass) => Some(
                pass.bare
                    .replace(stmt.expression.without_parentheses().span()),
            ),
            Pass::Classify(_) => None,
        };
        walk::walk_expression_statement(self, stmt);
        if let (Pass::Sites(pass), Some(previous)) = (self, previous) {
            pass.bare = previous;
        }
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        if let Pass::Sites(pass) = self {
            pass.on_variable_declaration(declaration);
        }
        walk::walk_variable_declaration(self, declaration);
    }

    fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
        if let Pass::Sites(pass) = self {
            pass.on_static_member_expression(member);
        }
        walk::walk_static_member_expression(self, member);
    }

    fn visit_identifier_reference(&mut self, ident: &IdentifierReference<'a>) {
        if let Pass::Sites(pass) = self {
            pass.on_identifier_reference(ident);
        }
    }
}

pub fn collect(program: &Program<'_>, runtime: &Runtime, scoping: &Scoping) -> Collected {
    let mut classify = Classify {
        runtime,
        scoping,
        facts: Facts::default(),
    };
    Pass::Classify(&mut classify).visit_program(program);
    let facts = classify.facts;
    let mut sites = Sites {
        runtime,
        scoping,
        facts: &facts,
        source: program.source_text,
        sites: Vec::new(),
        refs: Vec::new(),
        keys: FxHashMap::default(),
        bare: None,
    };
    Pass::Sites(&mut sites).visit_program(program);
    let (sites_found, refs, keys) = (sites.sites, sites.refs, sites.keys);
    Collected {
        facts,
        sites: sites_found,
        refs,
        keys,
    }
}

pub fn is_primitive(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::Identifier(ident) => ident.name == "undefined",
        Expression::UnaryExpression(unary) => {
            matches!(unary.operator.as_str(), "-" | "+" | "void") && is_primitive(&unary.argument)
        }
        _ => false,
    }
}

pub fn leading_getters(statements: &[Statement<'_>]) -> Vec<SymbolId> {
    let mut out = Vec::new();
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            break;
        };
        let getters: Option<Vec<SymbolId>> = declaration
            .declarations
            .iter()
            .map(|declarator| {
                match declarator
                    .init
                    .as_ref()
                    .map(Expression::without_parentheses)
                {
                    Some(Expression::ArrowFunctionExpression(arrow))
                        if arrow.params.items.is_empty() && arrow.get_expression().is_some() =>
                    {
                        binding(&declarator.id)
                    }
                    _ => None,
                }
            })
            .collect();
        match getters {
            Some(getters) => out.extend(getters),
            None => break,
        }
    }
    out
}
