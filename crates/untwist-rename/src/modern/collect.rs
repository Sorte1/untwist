use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug)]
pub struct RefSite {
    pub start: u32,
    pub symbol: SymbolId,
    pub write: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    Function,
    Var {
        pure: bool,
        call_of: Option<SymbolId>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Decl {
    pub region: Span,
    pub kind: DeclKind,
    pub global: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callee {
    Symbol(SymbolId),
    Function(Span),
}

#[derive(Clone, Copy, Debug)]
pub struct Discarded {
    pub span: Span,
    pub callee: Callee,
}

#[derive(Default)]
pub struct Facts {
    pub refs: Vec<RefSite>,
    pub decls: FxHashMap<SymbolId, Decl>,
    pub discarded: Vec<Discarded>,
    pub function_binding: FxHashMap<Span, SymbolId>,
    pub trivial: Vec<Span>,
    pub trivial_true: rustc_hash::FxHashSet<Span>,
    pub dynamic: bool,
}

impl Facts {
    pub fn refs_in(&self, span: Span) -> &[RefSite] {
        let from = self.refs.partition_point(|site| site.start < span.start);
        let to = self.refs.partition_point(|site| site.start < span.end);
        &self.refs[from..to]
    }
}

pub fn collect(program: &Program<'_>, scoping: &Scoping) -> Facts {
    let mut collector = Collector {
        scoping,
        script: !program.source_type.is_module(),
        facts: Facts::default(),
        factories: rustc_hash::FxHashSet::default(),
    };
    collector.visit_program(program);
    collector.facts.refs.sort_by_key(|site| site.start);
    collector.facts.dynamic |= scoping.scope_descendants_from_root().any(|scope| {
        scoping
            .scope_flags(scope)
            .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
    });
    collector.facts
}

struct Collector<'s> {
    scoping: &'s Scoping,
    script: bool,
    facts: Facts,
    factories: rustc_hash::FxHashSet<SymbolId>,
}

fn returns_function(statements: &[Statement<'_>]) -> bool {
    matches!(statements, [Statement::ReturnStatement(ret)]
    if ret.argument.as_ref().is_some_and(|argument| {
        matches!(argument.without_parentheses(), Expression::FunctionExpression(_))
    }))
}

fn plain_params(params: &FormalParameters<'_>) -> bool {
    params.rest.is_none()
        && params.items.iter().all(|param| {
            param.initializer.is_none()
                && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
        })
}

fn is_factory(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => {
            plain_params(&function.params)
                && !function.generator
                && !function.r#async
                && function
                    .body
                    .as_ref()
                    .is_some_and(|body| returns_function(&body.statements))
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let body = match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => returns_function(&body.statements),
                ArrowFunctionBody::FunctionExpression(_) => true,
                _ => false,
            };
            plain_params(&arrow.params) && !arrow.r#async && body
        }
        _ => false,
    }
}

fn wrapped_function(expr: &Expression<'_>) -> Option<Span> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    let [argument] = call.arguments.as_slice() else {
        return None;
    };
    let Expression::ObjectExpression(object) = argument.as_expression()?.without_parentheses()
    else {
        return None;
    };
    let [ObjectPropertyKind::ObjectProperty(property)] = object.properties.as_slice() else {
        return None;
    };
    function_span(&property.value)
}

fn function_span(expr: &Expression<'_>) -> Option<Span> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function.span),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.span),
        _ => None,
    }
}

pub fn discarded_call<'e, 'a>(expr: &'e Expression<'a>) -> Option<&'e CallExpression<'a>> {
    match expr.without_parentheses() {
        Expression::CallExpression(call) if call.arguments.is_empty() && !call.optional => {
            Some(call)
        }
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::LogicalNot | UnaryOperator::Void
            ) =>
        {
            discarded_call(&unary.argument)
        }
        _ => None,
    }
}

fn is_pure(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_pure(&unary.argument)
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            other => other.as_expression().is_some_and(is_pure),
        }),
        Expression::ObjectExpression(object) => object.properties.iter().all(pure_property),
        _ => false,
    }
}

fn pure_property(property: &ObjectPropertyKind<'_>) -> bool {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return false;
    };
    let key_pure = match &property.key {
        PropertyKey::StaticIdentifier(_) | PropertyKey::PrivateIdentifier(_) => true,
        key => key.as_expression().is_some_and(|key| {
            matches!(
                key,
                Expression::StringLiteral(_) | Expression::NumericLiteral(_)
            )
        }),
    };
    key_pure && !property.shorthand && is_pure(&property.value)
}

impl Collector<'_> {
    fn global(&self, symbol: SymbolId) -> bool {
        self.script && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
    }

    fn callee(&self, call: &CallExpression<'_>) -> Option<Callee> {
        if let Some(span) = function_span(&call.callee) {
            return Some(Callee::Function(span));
        }
        let Expression::Identifier(id) = call.callee.without_parentheses() else {
            return None;
        };
        let reference = id.reference_id.get()?;
        self.scoping
            .get_reference(reference)
            .symbol_id()
            .map(Callee::Symbol)
    }

    fn bare_call(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if !call.arguments.is_empty() || call.optional {
            return None;
        }
        match self.callee(call)? {
            Callee::Symbol(symbol) => Some(symbol),
            Callee::Function(_) => None,
        }
    }

    fn factory_call(&self, expr: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return false;
        };
        let symbol = callee
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        symbol.is_some_and(|symbol| self.factories.contains(&symbol))
            && call
                .arguments
                .iter()
                .all(|argument| argument.as_expression().is_some_and(is_pure))
    }

    fn note_discarded(&mut self, expr: &Expression<'_>) {
        let Some(call) = discarded_call(expr) else {
            return;
        };
        if expr.span().is_empty() {
            return;
        }
        if let Some(callee) = self.callee(call) {
            self.facts.discarded.push(Discarded {
                span: expr.span(),
                callee,
            });
        }
    }

    fn note_decl(&mut self, symbol: SymbolId, region: Span, kind: DeclKind) {
        if region.is_empty() {
            return;
        }
        let global = self.global(symbol);
        self.facts.decls.insert(
            symbol,
            Decl {
                region,
                kind,
                global,
            },
        );
    }

    fn note_declarations(&mut self, declaration: &VariableDeclaration<'_>) {
        if declaration.kind.is_using() {
            return;
        }
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            if let Some(symbol) = id.symbol_id.get() {
                if declarator.init.as_ref().is_some_and(is_factory) {
                    self.factories.insert(symbol);
                }
                let pure = declarator
                    .init
                    .as_ref()
                    .is_none_or(|init| is_pure(init) || self.factory_call(init));
                let call_of = declarator
                    .init
                    .as_ref()
                    .and_then(|init| self.bare_call(init));
                self.note_decl(symbol, declarator.span, DeclKind::Var { pure, call_of });
            }
        }
    }

    fn note_statement(&mut self, statement: &Statement<'_>) {
        let statement = match statement {
            Statement::ExpressionStatement(statement) => statement,
            Statement::VariableDeclaration(declaration) => {
                return self.note_declarations(declaration)
            }
            Statement::FunctionDeclaration(function) => {
                if let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) {
                    self.note_decl(symbol, function.span, DeclKind::Function);
                    if let Some(value) =
                        super::trivial::wrapper(function).filter(|_| !function.span.is_empty())
                    {
                        self.facts.trivial.push(function.span);
                        if value == super::trivial::Value::True {
                            self.facts.trivial_true.insert(function.span);
                        }
                    }
                }
                return;
            }
            _ => return,
        };
        match statement.expression.without_parentheses() {
            Expression::SequenceExpression(sequence) => {
                for element in &sequence.expressions {
                    self.note_discarded(element);
                }
            }
            expr => self.note_discarded(expr),
        }
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        for statement in it {
            self.note_statement(statement);
        }
        walk::walk_statements(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        if let Some(symbol) = reference.symbol_id() {
            self.facts.refs.push(RefSite {
                start: it.span.start,
                symbol,
                write: reference.is_write(),
            });
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.is_declaration() && !it.span.is_empty() {
            if let Some(symbol) = it.id.as_ref().and_then(|id| id.symbol_id.get()) {
                self.facts.function_binding.insert(it.span, symbol);
            }
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            let span = it
                .init
                .as_ref()
                .and_then(|init| function_span(init).or_else(|| wrapped_function(init)));
            if let (Some(symbol), Some(span)) =
                (id.symbol_id.get(), span.filter(|span| !span.is_empty()))
            {
                self.facts.function_binding.insert(span, symbol);
            }
        }
        walk::walk_variable_declarator(self, it);
    }
}
