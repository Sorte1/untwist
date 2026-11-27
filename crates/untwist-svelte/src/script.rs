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
    pub aliases: FxHashMap<SymbolId, SymbolId>,
    pub handlers: FxHashSet<SymbolId>,
    pub written: FxHashSet<SymbolId>,
    pub state_kinds: FxHashMap<u32, StateKind>,
    pub renames: Vec<(SymbolId, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKind {
    Deep,
    Raw,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Proxyable {
    Yes,
    No,
    Unknown,
}

#[derive(Default)]
struct Writes {
    deep: bool,
    raw: bool,
    unknown: bool,
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
    arrows: FxHashMap<SymbolId, Vec<SymbolId>>,
    hoisted: Vec<(SymbolId, Vec<SymbolId>)>,
    inits: Vec<(SymbolId, u32, Option<Proxyable>)>,
    writes: FxHashMap<SymbolId, Writes>,
    binds: Vec<Span>,
}

impl<'f> Classify<'f> {
    fn rt(&self, expr: &Expression<'_>) -> Option<Rt> {
        self.runtime.resolve(expr, self.scoping)
    }

    fn proxyable(&self, expr: &Expression<'_>) -> Proxyable {
        match expr.without_parentheses() {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_) => Proxyable::No,
            Expression::Identifier(ident) if ident.name == "undefined" => Proxyable::No,
            Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::NewExpression(_) => Proxyable::Yes,
            Expression::CallExpression(call) => {
                let known = self.rt(&call.callee).is_some()
                    || matches!(call.callee.without_parentheses(), Expression::Identifier(ident) if symbol_of(ident, self.scoping).is_some_and(|symbol| self.facts.classes.contains_key(&symbol)));
                match known {
                    true => Proxyable::Unknown,
                    false => Proxyable::Yes,
                }
            }
            _ => Proxyable::Unknown,
        }
    }

    fn on_write(&mut self, call: &CallExpression<'_>) {
        let Some(Expression::Identifier(ident)) = arg(call, 0).map(Expression::without_parentheses)
        else {
            return;
        };
        let Some(symbol) = symbol_of(ident, self.scoping) else {
            return;
        };
        self.facts.written.insert(symbol);
        if self.rt(&call.callee) != Some(Rt::Set) {
            return;
        }
        let deep = arg(call, 2).is_some_and(is_true);
        let in_bind = self
            .binds
            .iter()
            .any(|bind| bind.start <= call.span.start && call.span.end <= bind.end);
        let value = match arg(call, 1) {
            Some(Expression::Identifier(_)) if in_bind => Proxyable::No,
            Some(value) => self.proxyable(value),
            None => Proxyable::Unknown,
        };
        let entry = self.writes.entry(symbol).or_default();
        match (deep, value) {
            (true, _) => entry.deep = true,
            (false, Proxyable::Yes) => entry.raw = true,
            (false, Proxyable::Unknown) => entry.unknown = true,
            (false, Proxyable::No) => {}
        }
    }

    fn settle_states(&mut self) {
        for (symbol, start, init) in std::mem::take(&mut self.inits) {
            let writes = self.writes.remove(&symbol).unwrap_or_default();
            let deep = writes.deep || init.is_none();
            let raw = writes.raw || init == Some(Proxyable::Yes);
            let kind = match (deep, raw) {
                (true, true) => StateKind::Unknown,
                (true, false) => StateKind::Deep,
                (false, true) => StateKind::Raw,
                (false, false) if !writes.unknown && init == Some(Proxyable::No) => StateKind::Deep,
                (false, false) => StateKind::Raw,
            };
            self.facts.state_kinds.insert(start, kind);
        }
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
                    Some(Rt::State | Rt::Derived | Rt::DerivedSafeEqual) => {
                        self.facts.classes.insert(symbol, Class::Signal);
                        if self.rt(&call.callee) == Some(Rt::State) {
                            self.facts.states.insert(symbol);
                            let init = match arg(call, 0) {
                                None => Some(Proxyable::No),
                                Some(Expression::CallExpression(proxy))
                                    if self.rt(&proxy.callee) == Some(Rt::Proxy) =>
                                {
                                    None
                                }
                                Some(value) => Some(self.proxyable(value)),
                            };
                            self.inits.push((symbol, call.span.start, init));
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
                    if let Some(params) = param_symbols(&arrow.params) {
                        self.arrows.insert(symbol, params);
                    }
                }
                _ => {}
            }
        }
    }

    fn item_getters(&self, statements: &[Statement<'_>]) -> Vec<SymbolId> {
        let mut out = Vec::new();
        for statement in statements {
            let Statement::VariableDeclaration(declaration) = statement else {
                break;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                break;
            };
            match declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses)
            {
                Some(Expression::ArrowFunctionExpression(arrow))
                    if arrow.params.items.is_empty() && arrow.get_expression().is_some() =>
                {
                    out.extend(binding(&declarator.id));
                }
                Some(Expression::CallExpression(call))
                    if matches!(
                        self.rt(&call.callee),
                        Some(Rt::Derived | Rt::DerivedSafeEqual)
                    ) => {}
                _ => break,
            }
        }
        out
    }

    fn on_arrow<'a>(&mut self, arrow: &ArrowFunctionExpression<'a>) {
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

    fn on_assignment<'a>(&mut self, assign: &oxc_ast::ast::AssignmentExpression<'a>) {
        let oxc_ast::ast::AssignmentTarget::StaticMemberExpression(member) = &assign.left else {
            return;
        };
        if !member.property.name.starts_with("__") {
            return;
        }
        let Expression::ArrayExpression(array) = assign.right.without_parentheses() else {
            return;
        };
        let symbols: Option<Vec<SymbolId>> = array
            .elements
            .iter()
            .map(
                |element| match element.as_expression().map(Expression::without_parentheses) {
                    Some(Expression::Identifier(ident)) => symbol_of(ident, self.scoping),
                    _ => None,
                },
            )
            .collect();
        if let Some((handler, args)) = symbols.as_deref().and_then(<[SymbolId]>::split_first) {
            self.hoisted.push((*handler, args.to_vec()));
        }
    }

    fn settle_hoisted(&mut self) {
        let mut seen: FxHashMap<SymbolId, Option<SymbolId>> = FxHashMap::default();
        let mut uses: FxHashMap<SymbolId, usize> = FxHashMap::default();
        for (handler, _) in &self.hoisted {
            *uses.entry(*handler).or_default() += 1;
        }
        for (handler, count) in uses {
            if self.arrows.contains_key(&handler)
                && self.scoping.get_resolved_reference_ids(handler).len() == count
            {
                self.facts.handlers.insert(handler);
            }
        }
        for (handler, args) in std::mem::take(&mut self.hoisted) {
            let Some(params) = self.arrows.get(&handler) else {
                continue;
            };
            if params.len() != args.len() + 1 {
                continue;
            }
            for (param, arg) in params.iter().skip(1).zip(args) {
                let entry = seen.entry(*param).or_insert(Some(arg));
                if *entry != Some(arg) {
                    *entry = None;
                }
            }
        }
        for (param, arg) in seen {
            let Some(arg) = arg else {
                continue;
            };
            if let Some(class) = self.facts.classes.get(&arg).copied() {
                self.facts.classes.insert(param, class);
            }
            if self.facts.props_objects.contains(&arg) {
                self.facts.props_objects.insert(param);
                self.facts.aliases.insert(param, arg);
            }
        }
    }

    fn on_call_expression<'a>(&mut self, call: &CallExpression<'a>) {
        match self.rt(&call.callee) {
            Some(Rt::Set | Rt::Update | Rt::UpdatePre) => self.on_write(call),
            Some(Rt::BindValue | Rt::BindChecked | Rt::BindGroup | Rt::BindThis) => {
                self.binds.push(call.span)
            }
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
                    for symbol in self.item_getters(arrow_statements(render)) {
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
    shorthands: Vec<u32>,
    keys: FxHashMap<SymbolId, Vec<String>>,
    bare: Option<Span>,
}

fn lit(text: impl Into<String>) -> Piece {
    Piece::Lit(text.into())
}

impl<'f> Sites<'f> {
    fn class_of(&self, expr: &Expression<'_>) -> Option<(SymbolId, Class, Piece)> {
        let Expression::Identifier(ident) = expr.without_parentheses() else {
            return None;
        };
        let symbol = symbol_of(ident, self.scoping)?;
        let class = *self.facts.classes.get(&symbol)?;
        Some((symbol, class, Piece::Name(symbol, ident.name.to_string())))
    }

    fn signal(&self, expr: Option<&Expression<'_>>) -> Option<Piece> {
        match self.class_of(expr?)? {
            (_, Class::Signal, name) => Some(name),
            _ => None,
        }
    }

    fn assign(&self, span: Span, target: Piece, value: &Expression<'_>) -> Site {
        let bare = self.bare == Some(span);
        let mut pieces = Vec::new();
        if !bare {
            pieces.push(lit("("));
        }
        pieces.extend([target, lit(" = ")]);
        match value.without_parentheses() {
            Expression::SequenceExpression(_) => {
                pieces.extend([lit("("), Piece::Src(value.span()), lit(")")])
            }
            _ => pieces.push(Piece::Src(value.span())),
        }
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
            Rt::Get if call.arguments.len() == 1 => site(vec![self.signal(arg(call, 0))?]),
            Rt::Set if (2..=3).contains(&call.arguments.len()) => {
                let target = self.signal(arg(call, 0))?;
                Some(self.assign(span, target, arg(call, 1)?))
            }
            Rt::Update | Rt::UpdatePre if (1..=2).contains(&call.arguments.len()) => {
                let target = self.signal(arg(call, 0))?;
                let op = self.delta(call)?;
                match self.runtime.resolve(&call.callee, self.scoping)? {
                    Rt::Update => site(vec![target, lit(op)]),
                    _ => site(vec![lit(op), target]),
                }
            }
            Rt::State if call.arguments.len() <= 1 => {
                let rune = match self.facts.state_kinds.get(&span.start)? {
                    StateKind::Deep => "$state(",
                    StateKind::Raw => "$state.raw(",
                    StateKind::Unknown => return None,
                };
                let Some(value) = arg(call, 0) else {
                    return site(vec![lit(rune), lit(")")]);
                };
                let value = match value.without_parentheses() {
                    Expression::CallExpression(proxy)
                        if self.runtime.resolve(&proxy.callee, self.scoping) == Some(Rt::Proxy)
                            && proxy.arguments.len() == 1 =>
                    {
                        arg(proxy, 0)?
                    }
                    _ => value,
                };
                site(vec![lit(rune), Piece::Src(value.span()), lit(")")])
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
                    Rt::UpdateProp => site(vec![target, lit(op)]),
                    _ => site(vec![lit(op), target]),
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
                pieces: vec![name],
            }),
            1 => Some(self.assign(call.span, name, arg(call, 0)?)),
            _ => None,
        }
    }
}

impl<'f> Sites<'f> {
    fn on_variable_declaration<'a>(&mut self, declaration: &VariableDeclaration<'a>) {
        let holds_state = declaration.declarations.iter().any(|declarator| {
            binding(&declarator.id).is_some_and(|symbol| {
                self.facts.states.contains(&symbol) || self.facts.written.contains(&symbol)
            })
        });
        if holds_state && declaration.kind == VariableDeclarationKind::Const {
            self.sites.push(Site {
                span: Span::new(declaration.span.start, declaration.span.start + 5),
                pieces: vec![lit("let")],
            });
        }
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(ident) = &declarator.id {
                let symbol = ident.symbol_id();
                if self
                    .facts
                    .renames
                    .iter()
                    .any(|(renamed, _)| *renamed == symbol)
                {
                    self.sites.push(Site {
                        span: ident.span,
                        pieces: vec![Piece::Name(symbol, ident.name.to_string())],
                    });
                }
            }
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
                let owner = self.facts.aliases.get(&symbol).copied().unwrap_or(symbol);
                self.keys.entry(owner).or_default().push(key.clone());
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
    pub shorthands: Vec<u32>,
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

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        if let Pass::Classify(pass) = self {
            pass.on_arrow(arrow);
        }
        walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_assignment_expression(&mut self, assign: &oxc_ast::ast::AssignmentExpression<'a>) {
        if let Pass::Classify(pass) = self {
            pass.on_assignment(assign);
        }
        walk::walk_assignment_expression(self, assign);
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

    fn visit_object_property(&mut self, property: &oxc_ast::ast::ObjectProperty<'a>) {
        if let (Pass::Sites(pass), true) = (&mut *self, property.shorthand) {
            pass.shorthands.push(property.value.span().start);
        }
        walk::walk_object_property(self, property);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        property: &oxc_ast::ast::AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if let Pass::Sites(pass) = self {
            pass.shorthands.push(property.binding.span.start);
        }
        walk::walk_assignment_target_property_identifier(self, property);
    }
}

pub fn collect(program: &Program<'_>, runtime: &Runtime, scoping: &Scoping) -> Collected {
    let mut classify = Classify {
        runtime,
        scoping,
        facts: Facts::default(),
        arrows: FxHashMap::default(),
        hoisted: Vec::new(),
        inits: Vec::new(),
        writes: FxHashMap::default(),
        binds: Vec::new(),
    };
    Pass::Classify(&mut classify).visit_program(program);
    classify.settle_hoisted();
    classify.settle_states();
    let mut facts = classify.facts;
    facts.renames = temporaries(&facts, scoping);
    let mut sites = Sites {
        runtime,
        scoping,
        facts: &facts,
        source: program.source_text,
        sites: Vec::new(),
        refs: Vec::new(),
        shorthands: Vec::new(),
        keys: FxHashMap::default(),
        bare: None,
    };
    Pass::Sites(&mut sites).visit_program(program);
    let (found, refs, shorthands, keys) = (sites.sites, sites.refs, sites.shorthands, sites.keys);
    Collected {
        facts,
        sites: found,
        refs,
        shorthands,
        keys,
    }
}

fn temporaries(facts: &Facts, scoping: &Scoping) -> Vec<(SymbolId, String)> {
    let mut symbols: Vec<SymbolId> = facts
        .classes
        .iter()
        .filter(|(symbol, class)| {
            **class == Class::Signal && scoping.symbol_name(**symbol).starts_with('$')
        })
        .map(|(symbol, _)| *symbol)
        .collect();
    symbols.sort();
    let mut taken: FxHashSet<String> = scoping
        .symbol_ids()
        .map(|symbol| scoping.symbol_name(symbol).to_string())
        .chain(
            scoping
                .root_unresolved_references()
                .keys()
                .map(|name| name.to_string()),
        )
        .collect();
    let mut out = Vec::new();
    for symbol in symbols {
        let trimmed = scoping.symbol_name(symbol).trim_start_matches('$');
        let base = match trimmed.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            true => trimmed.to_string(),
            false => "value".to_string(),
        };
        let mut name = base.clone();
        let mut counter = 1;
        while taken.contains(&name) {
            counter += 1;
            name = format!("{base}{counter}");
        }
        taken.insert(name.clone());
        out.push((symbol, name));
    }
    out
}

fn is_true(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(literal) => literal.value,
        Expression::UnaryExpression(unary) => {
            unary.operator.as_str() == "!"
                && matches!(unary.argument.without_parentheses(), Expression::NumericLiteral(n) if n.value == 0.0)
        }
        _ => false,
    }
}
