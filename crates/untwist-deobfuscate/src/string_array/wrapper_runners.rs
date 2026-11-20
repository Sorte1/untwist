use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::literal::is_true_literal;
use untwist_core::ast_util::matcher::{call_parts, iife};
use untwist_core::pass::Cx;

use crate::bindings::{reference_symbol, resolved_symbol};
use crate::self_defending::is_loose_call_controller;

fn is_controller_init(init: Option<&Expression<'_>>) -> bool {
    init.is_some_and(|init| is_loose_call_controller(init.without_parentheses()))
}

struct Controllers<'s> {
    scoping: &'s Scoping,
    found: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Controllers<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), true) =
            (&it.id, is_controller_init(it.init.as_ref()))
        {
            if let Some(symbol) = id.symbol_id.get() {
                if !self.scoping.symbol_is_mutated(symbol) {
                    self.found.insert(symbol);
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

fn is_callback(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(function) => {
            function.params.items.is_empty()
                && function.params.rest.is_none()
                && !function.r#async
                && !function.generator
        }
        Expression::ArrowFunctionExpression(arrow) => {
            arrow.params.items.is_empty() && arrow.params.rest.is_none() && !arrow.r#async
        }
        _ => false,
    }
}

struct Head<'s> {
    scoping: &'s Scoping,
    controllers: &'s FxHashSet<SymbolId>,
    local: FxHashSet<SymbolId>,
    runners: Vec<SymbolId>,
    called: FxHashSet<SymbolId>,
}

impl Head<'_> {
    fn is_controller(&self, expr: &Expression<'_>) -> bool {
        reference_symbol(expr.without_parentheses(), self.scoping).is_some_and(|symbol| {
            self.controllers.contains(&symbol) || self.local.contains(&symbol)
        })
    }

    fn controller_call(&self, expr: &Expression<'_>) -> bool {
        let Some((callee, arguments)) = call_parts(expr.without_parentheses()) else {
            return false;
        };
        let [Argument::ThisExpression(_), callback] = arguments.as_slice() else {
            return false;
        };
        self.is_controller(callee) && callback.as_expression().is_some_and(is_callback)
    }

    fn immediate_run(&self, expr: &Expression<'_>) -> bool {
        let Some((callee, arguments)) = call_parts(expr.without_parentheses()) else {
            return false;
        };
        arguments.is_empty() && self.controller_call(callee)
    }

    fn runner_shell(&self, expr: &Expression<'_>) -> bool {
        let Some((function, arguments)) = iife(expr.without_parentheses()) else {
            return false;
        };
        if !arguments.is_empty()
            || !function.params.items.is_empty()
            || function.params.rest.is_some()
            || function.r#async
            || function.generator
        {
            return false;
        }
        let Some(body) = function.body.as_ref() else {
            return false;
        };
        let [Statement::ExpressionStatement(only)] = body.statements.as_slice() else {
            return false;
        };
        body.directives.is_empty() && self.immediate_run(&only.expression)
    }

    fn bare_runner_call(&mut self, expr: &Expression<'_>) -> bool {
        let Some((callee, arguments)) = call_parts(expr.without_parentheses()) else {
            return false;
        };
        let Some(symbol) = reference_symbol(callee.without_parentheses(), self.scoping) else {
            return false;
        };
        arguments.is_empty() && self.runners.contains(&symbol) && self.called.insert(symbol)
    }

    fn expression(&mut self, expr: &Expression<'_>) -> bool {
        self.runner_shell(expr) || self.immediate_run(expr) || self.bare_runner_call(expr)
    }

    fn statement(&mut self, stmt: &Statement<'_>) -> bool {
        match stmt {
            Statement::ExpressionStatement(node) => self.expression(&node.expression),
            Statement::VariableDeclaration(declaration) => declaration
                .declarations
                .iter()
                .all(|declarator| self.declarator(declarator)),
            _ => false,
        }
    }

    fn declarator(&mut self, declarator: &VariableDeclarator<'_>) -> bool {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        if self.scoping.symbol_is_mutated(symbol) {
            return false;
        }
        let init = declarator.init.as_ref();
        if is_controller_init(init) {
            return self.local.insert(symbol);
        }
        if init.is_some_and(|init| self.controller_call(init)) {
            self.runners.push(symbol);
            return true;
        }
        false
    }

    fn complete(&self) -> bool {
        self.runners
            .iter()
            .all(|symbol| self.called.contains(symbol))
    }
}

fn simple_params(function: &Function<'_>) -> Option<Vec<SymbolId>> {
    if function.params.rest.is_some() {
        return None;
    }
    function
        .params
        .items
        .iter()
        .map(|item| match (&item.pattern, &item.initializer) {
            (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn forwarded_call(expr: &Expression<'_>) -> bool {
    call_parts(expr.without_parentheses()).is_some_and(|(callee, arguments)| {
        matches!(callee.without_parentheses(), Expression::Identifier(_)) && !arguments.is_empty()
    })
}

struct Candidate {
    function: Span,
    watched: Option<Span>,
    removed: Vec<Span>,
    statements: usize,
    sequence: bool,
    tracked: Vec<SymbolId>,
    params: Vec<SymbolId>,
}

fn returned_prefix<'k, 'a>(statements: &'k [Statement<'a>]) -> Vec<&'k Expression<'a>> {
    let [Statement::ReturnStatement(returned)] = statements else {
        return Vec::new();
    };
    let Some(Expression::SequenceExpression(sequence)) = returned
        .argument
        .as_ref()
        .map(Expression::without_parentheses)
    else {
        return Vec::new();
    };
    match sequence.expressions.split_last() {
        Some((call, prefix)) if forwarded_call(call) => prefix.iter().collect(),
        _ => Vec::new(),
    }
}

fn forwarded_target(statements: &[Statement<'_>], scoping: &Scoping) -> Option<SymbolId> {
    let [Statement::ReturnStatement(returned)] = statements else {
        return None;
    };
    let call = match returned.argument.as_ref()?.without_parentheses() {
        Expression::SequenceExpression(sequence) => sequence.expressions.last()?,
        other => other,
    };
    if !forwarded_call(call) {
        return None;
    }
    let (callee, _) = call_parts(call.without_parentheses())?;
    reference_symbol(callee.without_parentheses(), scoping)
}

fn builds_a_controller(statements: &[Statement<'_>]) -> bool {
    let [Statement::VariableDeclaration(flag), Statement::ReturnStatement(returned)] = statements
    else {
        return false;
    };
    let [declarator] = flag.declarations.as_slice() else {
        return false;
    };
    let returns_a_runner = matches!(
        returned.argument.as_ref().map(Expression::without_parentheses),
        Some(Expression::FunctionExpression(inner)) if inner.params.items.len() == 2
    );
    declarator.init.as_ref().is_some_and(is_true_literal) && returns_a_runner
}

fn shifts_its_index(statements: &[Statement<'_>], params: &[SymbolId], scoping: &Scoping) -> bool {
    let Some(Statement::ExpressionStatement(first)) = statements.first() else {
        return false;
    };
    let Expression::AssignmentExpression(assigned) = &first.expression else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assigned.left else {
        return false;
    };
    let Expression::BinaryExpression(shifted) = &assigned.right else {
        return false;
    };
    let index = resolved_symbol(target, scoping);
    assigned.operator == AssignmentOperator::Assign
        && index.is_some_and(|index| params.first() == Some(&index))
        && shifted.operator == BinaryOperator::Subtraction
        && matches!(&shifted.left, Expression::Identifier(id) if resolved_symbol(id, scoping) == index)
        && matches!(&shifted.right, Expression::NumericLiteral(_))
}

fn candidate(
    function: &Function<'_>,
    scoping: &Scoping,
    controllers: &FxHashSet<SymbolId>,
) -> Option<Candidate> {
    let params = simple_params(function)?;
    let body = function.body.as_ref()?;
    let mut scan = Head {
        scoping,
        controllers,
        local: FxHashSet::default(),
        runners: Vec::new(),
        called: FxHashSet::default(),
    };
    let head = body
        .statements
        .iter()
        .take_while(|stmt| scan.statement(stmt))
        .count();
    let rest = &body.statements[head..];
    let watched = match forwarded_target(rest, scoping) {
        Some(target) => Some(scoping.symbol_span(target)),
        None if shifts_its_index(rest, &params, scoping) => function.id.as_ref().map(|id| id.span),
        None if builds_a_controller(rest) => None,
        None => return None,
    };
    let prefix = returned_prefix(rest);
    let fits = prefix.iter().all(|expr| scan.expression(expr));
    if rest.is_empty() || !fits || (head == 0 && prefix.is_empty()) || !scan.complete() {
        return None;
    }
    let mut removed: Vec<Span> = body.statements[..head].iter().map(GetSpan::span).collect();
    removed.extend(prefix.iter().map(|expr| expr.span()));
    let mut tracked: Vec<SymbolId> = scan.local.into_iter().collect();
    tracked.extend(scan.runners);
    Some(Candidate {
        function: function.span,
        watched,
        removed,
        statements: head,
        sequence: !prefix.is_empty(),
        tracked,
        params,
    })
}

struct Find<'s> {
    scoping: &'s Scoping,
    controllers: &'s FxHashSet<SymbolId>,
    found: Vec<Candidate>,
}

impl<'a> VisitJs<'a> for Find<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(found) = candidate(it, self.scoping, self.controllers) {
            self.found.push(found);
        }
        walk_js::walk_function(self, it, flags);
    }
}

struct Escapes<'s> {
    scoping: &'s Scoping,
    owner: FxHashMap<SymbolId, usize>,
    params: FxHashSet<SymbolId>,
    candidates: &'s [Candidate],
    spoiled: FxHashSet<usize>,
}

impl<'a> VisitJs<'a> for Escapes<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = resolved_symbol(it, self.scoping) else {
            return;
        };
        let Some(&index) = self.owner.get(&symbol) else {
            return;
        };
        let inside = self.candidates[index]
            .removed
            .iter()
            .any(|span| span.start <= it.span.start && it.span.end <= span.end);
        if inside == self.params.contains(&symbol) {
            self.spoiled.insert(index);
        }
    }
}

struct Strip<'s> {
    candidates: &'s [Candidate],
    stripped: usize,
}

impl<'a> VisitMut<'a> for Strip<'_> {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        walk_mut::walk_function(self, it, flags);
        let Some(found) = self
            .candidates
            .iter()
            .find(|found| found.function == it.span)
        else {
            return;
        };
        let Some(body) = it.body.as_mut() else {
            return;
        };
        body.statements.drain(..found.statements);
        if found.sequence {
            if let Some(Statement::ReturnStatement(returned)) = body.statements.last_mut() {
                if let Some(Expression::SequenceExpression(sequence)) = returned.argument.as_mut() {
                    if let Some(call) = sequence.expressions.pop() {
                        returned.argument = Some(call);
                    }
                }
            }
        }
        self.stripped += 1;
    }
}

pub(super) fn strip_wrapper_runners<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    if !crate::legacy_reach::strips_allowed() {
        return 0;
    }
    let scoping = cx.scoping();
    let mut controllers = Controllers {
        scoping,
        found: FxHashSet::default(),
    };
    controllers.visit_program(program);
    let mut find = Find {
        scoping,
        controllers: &controllers.found,
        found: Vec::new(),
    };
    find.visit_program(program);
    if find.found.is_empty() {
        return 0;
    }
    let mut owner = FxHashMap::default();
    let mut params = FxHashSet::default();
    for (index, found) in find.found.iter().enumerate() {
        for symbol in found.tracked.iter().chain(&found.params) {
            owner.insert(*symbol, index);
        }
        params.extend(found.params.iter().copied());
    }
    let mut escapes = Escapes {
        scoping,
        owner,
        params,
        candidates: &find.found,
        spoiled: FxHashSet::default(),
    };
    escapes.visit_program(program);
    let spoiled = escapes.spoiled;
    let kept: Vec<Candidate> = find
        .found
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !spoiled.contains(index))
        .map(|(_, found)| found)
        .collect();
    if kept.is_empty() {
        return 0;
    }
    let mut strip = Strip {
        candidates: &kept,
        stripped: 0,
    };
    strip.visit_program(program);
    let watched: Vec<Span> = kept.iter().filter_map(|found| found.watched).collect();
    crate::legacy_reach::watch_strips(&watched);
    strip.stripped
}
