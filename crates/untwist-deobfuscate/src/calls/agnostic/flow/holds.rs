use std::cell::{Cell, RefCell};
use std::ops::BitOr;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::calls::agnostic::bind::{reads_arguments, Bindings};
use crate::value_flow::{flow_rule_disabled, FUNCTION_VALUES};

const HOLD_DEPTH: u8 = 6;
const HOLD_WORK: u32 = 400_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::calls::agnostic) struct Tags(u8);

impl Tags {
    pub(in crate::calls::agnostic) const OUTSIDE: Self = Self(1);
    pub(in crate::calls::agnostic) const MACHINERY: Self = Self(1 << 2);
    pub(in crate::calls::agnostic) const FUNC: Self = Self(1 << 3);

    fn has(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for Tags {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Clone, Copy)]
pub(in crate::calls::agnostic) enum UserFunction<'k, 'a> {
    Function(&'k Function<'a>),
    Arrow(&'k ArrowFunctionExpression<'a>),
    Class(Span),
}

impl UserFunction<'_, '_> {
    fn start(self) -> u32 {
        match self {
            Self::Function(func) => func.span.start,
            Self::Arrow(arrow) => arrow.span.start,
            Self::Class(span) => span.start,
        }
    }
}

type Values<'k, 'a> = Option<Vec<UserFunction<'k, 'a>>>;

fn plain_param(param: &FormalParameter<'_>, symbol: SymbolId) -> bool {
    param.initializer.is_none()
        && matches!(&param.pattern, BindingPattern::BindingIdentifier(id) if id.symbol_id.get() == Some(symbol))
}

fn union<'k, 'a>(left: Values<'k, 'a>, right: Values<'k, 'a>) -> Values<'k, 'a> {
    let mut all = left?;
    for value in right? {
        if all.iter().all(|seen| seen.start() != value.start()) {
            all.push(value);
        }
    }
    Some(all)
}

#[derive(Default)]
struct Returns<'a> {
    exprs: Vec<&'a Expression<'a>>,
}

impl<'a> VisitJs<'a> for Returns<'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _it: &Class<'a>) {}
    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if let Some(argument) = &it.argument {
            self.exprs.push(self.alloc(argument));
        }
        walk_js::walk_return_statement(self, it);
    }
}

fn returned_in<'a>(statements: &[Statement<'a>]) -> Vec<&'a Expression<'a>> {
    let mut returns = Returns::default();
    for statement in statements {
        returns.visit_statement(statement);
    }
    returns.exprs
}

fn returned_expressions<'a>(func: &Function<'a>) -> Vec<&'a Expression<'a>> {
    func.body
        .as_ref()
        .map_or_else(Vec::new, |body| returned_in(&body.statements))
}

pub(in crate::calls::agnostic) struct Holds<'s, 'a> {
    scoping: &'s Scoping,
    bindings: &'s Bindings<'s, 'a>,
    open: bool,
    script: bool,
    visiting: RefCell<FxHashSet<SymbolId>>,
    sites: RefCell<FxHashMap<SymbolId, (u8, Vec<UserFunction<'a, 'a>>)>>,
    work: Cell<u32>,
}

impl<'s, 'a> Holds<'s, 'a> {
    pub(in crate::calls::agnostic) fn new(
        scoping: &'s Scoping,
        bindings: &'s Bindings<'s, 'a>,
        script: bool,
    ) -> Self {
        let open = bindings.scoped_lookup
            || !bindings.tracks_calls()
            || scoping.root_unresolved_references().contains_key("eval");
        Self {
            scoping,
            bindings,
            open,
            script,
            visiting: RefCell::default(),
            sites: RefCell::default(),
            work: Cell::new(0),
        }
    }

    pub(in crate::calls::agnostic) fn may_hold(&self, expr: &Expression<'a>, tags: Tags) -> bool {
        if flow_rule_disabled(FUNCTION_VALUES) {
            return false;
        }
        match self.user_functions(expr) {
            None => true,
            Some(functions) => tags.has(Tags::FUNC | Tags::MACHINERY) && !functions.is_empty(),
        }
    }

    pub(in crate::calls::agnostic) fn calls_only_user_functions(
        &self,
        callee: &Expression<'a>,
        arguments: &[Argument<'a>],
    ) -> bool {
        if flow_rule_disabled(FUNCTION_VALUES) {
            return true;
        }
        !self.may_hold(callee, Tags::OUTSIDE) || self.correlated(callee, arguments)
    }

    pub(in crate::calls::agnostic) fn user_functions<'k>(
        &self,
        expr: &'k Expression<'a>,
    ) -> Values<'k, 'a> {
        if self.open {
            return None;
        }
        self.values(expr, HOLD_DEPTH)
    }

    fn spend(&self) -> Option<()> {
        let spent = self.work.get();
        if spent >= HOLD_WORK {
            return None;
        }
        self.work.set(spent + 1);
        Some(())
    }

    fn values<'k>(&self, expr: &'k Expression<'a>, depth: u8) -> Values<'k, 'a> {
        let next = depth.checked_sub(1)?;
        self.spend()?;
        match expr.without_parentheses() {
            Expression::FunctionExpression(func) => Some(vec![UserFunction::Function(func)]),
            Expression::ArrowFunctionExpression(arrow) => Some(vec![UserFunction::Arrow(arrow)]),
            Expression::ClassExpression(class) => Some(vec![UserFunction::Class(class.span)]),
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_) => Some(Vec::new()),
            Expression::Identifier(id) => self.identifier(id, next),
            Expression::ConditionalExpression(cond) => union(
                self.values(&cond.consequent, next),
                self.values(&cond.alternate, next),
            ),
            Expression::LogicalExpression(logical) => union(
                self.values(&logical.left, next),
                self.values(&logical.right, next),
            ),
            Expression::SequenceExpression(seq) => self.values(seq.expressions.last()?, next),
            Expression::AssignmentExpression(assign) => match assign.operator {
                AssignmentOperator::Assign => self.values(&assign.right, next),
                operator if operator.is_arithmetic() || operator.is_bitwise() => Some(Vec::new()),
                _ => None,
            },
            Expression::CallExpression(call) => self.call(call, next),
            _ => None,
        }
    }

    fn symbol(&self, id: &IdentifierReference<'a>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn correlated(&self, callee: &Expression<'a>, arguments: &[Argument<'a>]) -> bool {
        if self.open {
            return false;
        }
        let Expression::Identifier(id) = callee.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.symbol(id) else {
            return false;
        };
        let Some((owner, index)) = self.bindings.param_of(symbol) else {
            return false;
        };
        if !self.outside_callable(owner) {
            return false;
        }
        let Some(func) = self.tracked_param(symbol, owner, index) else {
            return false;
        };
        if func.r#async || func.generator {
            return false;
        }
        let closures = returned_expressions(func);
        let bound = arguments.iter().all(|argument| {
            self.argument_symbol(argument).is_some_and(|arg| {
                self.bindings.never_reassigned(arg)
                    && (func
                        .params
                        .items
                        .iter()
                        .any(|param| plain_param(param, arg))
                        || closures
                            .iter()
                            .any(|closure| self.closure_param(closure, arg)))
            })
        });
        bound
            && self
                .call_site_values(symbol, owner, index, HOLD_DEPTH)
                .is_some()
    }

    fn argument_symbol(&self, argument: &Argument<'a>) -> Option<SymbolId> {
        match argument.as_expression()?.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id),
            _ => None,
        }
    }

    fn closure_param(&self, closure: &Expression<'a>, arg: SymbolId) -> bool {
        match closure.without_parentheses() {
            Expression::FunctionExpression(func) => {
                let unnamed = func.id.as_ref().is_none_or(|id| {
                    id.symbol_id.get().is_some_and(|name| {
                        self.scoping.get_resolved_reference_ids(name).is_empty()
                    })
                });
                unnamed
                    && !reads_arguments(func)
                    && func
                        .params
                        .items
                        .iter()
                        .any(|param| plain_param(param, arg))
            }
            Expression::ArrowFunctionExpression(arrow) => arrow
                .params
                .items
                .iter()
                .any(|param| plain_param(param, arg)),
            _ => false,
        }
    }

    fn identifier<'k>(&self, id: &IdentifierReference<'a>, depth: u8) -> Values<'k, 'a>
    where
        'a: 'k,
    {
        let Some(symbol) = self.symbol(id) else {
            return (id.name == "undefined").then(Vec::new);
        };
        if !self.visiting.borrow_mut().insert(symbol) {
            return None;
        }
        let found = match self.bindings.param_of(symbol) {
            Some((owner, index)) => self.parameter(symbol, owner, index, depth),
            None => self.binding(symbol, depth),
        };
        self.visiting.borrow_mut().remove(&symbol);
        found
    }

    fn binding<'k>(&self, symbol: SymbolId, depth: u8) -> Values<'k, 'a> {
        let defs = self.bindings.defs.get(symbol)?;
        if defs.opaque || defs.mutated || !defs.writes.is_empty() {
            return None;
        }
        let mut found: Values<'k, 'a> = Some(Vec::new());
        if let Some(func) = defs.func {
            found = union(found, Some(vec![UserFunction::Function(func)]));
        } else if defs.callable {
            found = union(
                found,
                Some(vec![UserFunction::Class(self.scoping.symbol_span(symbol))]),
            );
        } else if defs.inits.is_empty() {
            return None;
        }
        for init in defs.inits {
            if let Some(init) = init {
                found = union(found, self.values(init, depth));
            }
            found.as_ref()?;
        }
        found
    }

    fn parameter<'k>(
        &self,
        symbol: SymbolId,
        owner: SymbolId,
        index: usize,
        depth: u8,
    ) -> Values<'k, 'a>
    where
        'a: 'k,
    {
        if self.outside_callable(owner) {
            return None;
        }
        self.call_site_values(symbol, owner, index, depth)
    }

    fn outside_callable(&self, owner: SymbolId) -> bool {
        self.script && self.scoping.symbol_scope_id(owner) == self.scoping.root_scope_id()
    }

    fn tracked_param(
        &self,
        symbol: SymbolId,
        owner: SymbolId,
        index: usize,
    ) -> Option<&'a Function<'a>> {
        let bindings = self.bindings;
        let func = bindings.defs.func(owner)?;
        let param = func.params.items.get(index)?;
        let tracked = plain_param(param, symbol)
            && func.params.rest.is_none()
            && bindings.does_not_escape(owner)
            && bindings.never_reassigned(symbol)
            && !reads_arguments(func);
        tracked.then_some(func)
    }

    fn call_site_values(
        &self,
        symbol: SymbolId,
        owner: SymbolId,
        index: usize,
        depth: u8,
    ) -> Values<'a, 'a> {
        if let Some((known, values)) = self.sites.borrow().get(&symbol) {
            if *known <= depth {
                return Some(values.clone());
            }
        }
        let found = self.site_values(symbol, owner, index, depth)?;
        self.sites
            .borrow_mut()
            .insert(symbol, (depth, found.clone()));
        Some(found)
    }

    fn site_values(
        &self,
        symbol: SymbolId,
        owner: SymbolId,
        index: usize,
        depth: u8,
    ) -> Values<'a, 'a> {
        let bindings = self.bindings;
        self.tracked_param(symbol, owner, index)?;
        let mut found: Values<'a, 'a> = Some(Vec::new());
        for call in bindings.call_sites(owner) {
            let value = match call.arguments.get(index) {
                None => Some(Vec::new()),
                Some(argument) => self.values(argument.as_expression()?, depth),
            };
            found = union(found, value);
            found.as_ref()?;
        }
        found
    }

    fn call<'k>(&self, call: &'k CallExpression<'a>, depth: u8) -> Values<'k, 'a> {
        let callees = self.values(&call.callee, depth)?;
        let mut found: Values<'k, 'a> = Some(Vec::new());
        for callee in callees {
            let returned = match callee {
                UserFunction::Function(func) if func.r#async || func.generator => Some(Vec::new()),
                UserFunction::Function(func) => {
                    self.returned(&func.body.as_ref()?.statements, depth)
                }
                UserFunction::Arrow(arrow) if arrow.r#async => Some(Vec::new()),
                UserFunction::Arrow(arrow) => match &arrow.body {
                    ArrowFunctionBody::FunctionBody(body) => self.returned(&body.statements, depth),
                    other => self.values(other.as_expression()?, depth),
                },
                UserFunction::Class(_) => Some(Vec::new()),
            };
            found = union(found, returned);
            found.as_ref()?;
        }
        found
    }

    fn returned<'k>(&self, statements: &[Statement<'a>], depth: u8) -> Values<'k, 'a> {
        let mut found: Values<'k, 'a> = Some(Vec::new());
        for expr in returned_in(statements) {
            found = union(found, self.values(expr, depth));
            found.as_ref()?;
        }
        found
    }
}
