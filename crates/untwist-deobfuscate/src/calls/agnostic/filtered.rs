mod machine;
mod rewrite;
mod shape;

use std::cell::{Cell, RefCell};

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::bind::function_uses_this;
use super::bootstrap::Resolve;
use super::bound_once::Stable;
use super::defs::DefTable;
use super::roles::{
    Origin, BATCH, CONSTRUCTS, DEFINE_KEYED, GETTERS, MARKUP_CALLS, MUTATORS, READERS, RUNNERS,
};
use super::stored::Facts;
use super::AKey;
use crate::value_flow::{
    flow_rule_disabled, FILTERED_ENUMERATION, FILTERED_REWRITE_ORDER, STORAGE_CALLS,
    TRUSTED_STORAGE,
};
use machine::{is_binary_compare, literal, Machine, V};
use rewrite::{check_loop, is_rewrite_loop, rewrite_loop, Rewrite};
use shape::{closed, Sealed, Shape, Target};
pub(super) use shape::{Body, STRING_METHODS};

#[derive(Clone, Copy)]
pub(super) struct Candidate<'a> {
    span: Span,
    key: SymbolId,
    found: SymbolId,
    test: &'a CallExpression<'a>,
    assignment: Span,
    within: Body<'a>,
}

fn identifier_symbol(scoping: &Scoping, id: &IdentifierReference<'_>) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn single_if<'k, 'a>(body: &'k Statement<'a>) -> Option<&'k IfStatement<'a>> {
    match body {
        Statement::IfStatement(it) => Some(it),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [Statement::IfStatement(it)] => Some(it),
            _ => None,
        },
        _ => None,
    }
}

fn found_assignment<'k, 'a>(consequent: &'k Statement<'a>) -> Option<&'k AssignmentExpression<'a>> {
    let Statement::BlockStatement(block) = consequent else {
        return None;
    };
    let [Statement::ExpressionStatement(store), Statement::BreakStatement(stop)] =
        block.body.as_slice()
    else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &store.expression else {
        return None;
    };
    (stop.label.is_none() && assignment.operator == AssignmentOperator::Assign)
        .then_some(assignment)
}

pub(super) type Loop<'a> = (&'a ForInStatement<'a>, Option<Body<'a>>);

fn candidate<'a>(
    scoping: &Scoping,
    (it, within): Loop<'a>,
    facts: Facts<'_, 'a>,
) -> Option<Candidate<'a>> {
    let within = within?;
    let ForStatementLeft::VariableDeclaration(declaration) = &it.left else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let (BindingPattern::BindingIdentifier(binding), None) = (&declarator.id, &declarator.init)
    else {
        return None;
    };
    let key = binding.symbol_id.get()?;
    let test_if = single_if(super::opaque::live(&it.body, facts)?)?;
    if test_if.alternate.is_some() {
        return None;
    }
    let Expression::CallExpression(test) = test_if.test.without_parentheses() else {
        return None;
    };
    let assignment = found_assignment(super::opaque::live(&test_if.consequent, facts)?)?;
    let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) =
        assignment.left.as_simple_assignment_target()?
    else {
        return None;
    };
    let Expression::Identifier(value) = assignment.right.without_parentheses() else {
        return None;
    };
    (identifier_symbol(scoping, value)? == key).then_some(())?;
    let found = identifier_symbol(scoping, target)?;
    Some(Candidate {
        span: it.span,
        key,
        found,
        test,
        assignment: assignment.span,
        within,
    })
}

struct Pred<'a> {
    func: Body<'a>,
    args: Vec<Option<V>>,
    callees: FxHashMap<SymbolId, Body<'a>>,
    storage: FxHashMap<Span, Target<'a>>,
    resolved: FxHashMap<Span, String>,
}

#[derive(Default)]
pub(super) struct Filters<'a> {
    keys: FxHashMap<SymbolId, Span>,
    rewritten: FxHashMap<SymbolId, Span>,
    preds: FxHashMap<Span, Option<Pred<'a>>>,
    accepted: RefCell<FxHashMap<(Span, String), bool>>,
    open: Cell<bool>,
    source: &'a str,
}

fn rewritten_through_this(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().any(|property| match property {
        ObjectPropertyKind::ObjectProperty(prop) => {
            !matches!(prop.kind, PropertyKind::Init)
                || matches!(prop.value.without_parentheses(),
                    Expression::FunctionExpression(function) if function_uses_this(function))
        }
        ObjectPropertyKind::SpreadProperty(_) => true,
    })
}

impl<'a> Filters<'a> {
    pub(super) fn new(
        source: &'a str,
        scoping: &Scoping,
        (defs, stable): (&DefTable<'a>, &Stable<'_>),
        resolve: Resolve<'_>,
        loops: &[Loop<'a>],
    ) -> Self {
        let mut filters = Self {
            source,
            ..Self::default()
        };
        let objects = |symbol: SymbolId| {
            if flow_rule_disabled(STORAGE_CALLS) {
                return None;
            }
            let found = defs.get(symbol)?;
            let trusted = flow_rule_disabled(TRUSTED_STORAGE);
            let untouched = trusted
                || (stable.holds(symbol)
                    && found.writes.is_empty()
                    && !found.opaque
                    && !found.mutated
                    && !found.fields_written
                    && !found.escapes);
            match (untouched, found.inits) {
                (true, [Some(Expression::ObjectExpression(object))])
                    if trusted || !rewritten_through_this(object) =>
                {
                    Some(&**object)
                }
                _ => None,
            }
        };
        let facts = Facts {
            scoping,
            resolve,
            objects: &objects,
        };
        let candidates: Vec<Candidate<'a>> = loops
            .iter()
            .filter_map(|found| candidate(scoping, *found, facts))
            .collect();
        if candidates.is_empty() {
            return filters;
        }
        let mutant = flow_rule_disabled(FILTERED_ENUMERATION);
        let ordered = !flow_rule_disabled(FILTERED_REWRITE_ORDER);
        let sealed = |symbol: SymbolId| {
            if !stable.holds(symbol) {
                return None;
            }
            let found = defs.get(symbol)?;
            if !found.writes.is_empty() || found.opaque || found.mutated {
                return None;
            }
            match (found.func, found.inits) {
                (Some(func), [] | [Some(_)]) => Body::of_function(func),
                (None, [Some(init)]) => Body::of_value(init),
                _ => None,
            }
        };
        for candidate in &candidates {
            let shared = candidates
                .iter()
                .filter(|other| {
                    other.found == candidate.found
                        || other.key == candidate.key
                        || other.found == candidate.key
                        || other.key == candidate.found
                })
                .count();
            let rewrite = rewrite_loop(candidate, ordered);
            let valid = mutant
                || (shared == 1
                    && uses_are_keys(candidate, (rewrite, facts))
                    && found_declared(scoping, candidate));
            if !valid {
                continue;
            }
            let pred = if mutant {
                None
            } else {
                match predicate(scoping, &sealed, facts, candidate) {
                    Some(pred) => Some(pred),
                    None => continue,
                }
            };
            filters.keys.insert(candidate.found, candidate.span);
            if mutant {
                filters.keys.insert(candidate.key, candidate.span);
            }
            if let Some(rewrite) = rewrite.filter(|rewrite| rewrite.ordered && !mutant) {
                filters.rewritten.insert(candidate.found, rewrite.span);
            }
            filters.preds.insert(candidate.span, pred);
        }
        filters
    }

    pub(super) fn source(&self) -> &'a str {
        self.source
    }

    pub(super) fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub(super) fn key(&self, symbol: SymbolId, at: Span) -> Option<AKey<'static>> {
        let rewritten = self
            .rewritten
            .get(&symbol)
            .is_some_and(|region| region.start <= at.start && at.end <= region.end);
        if rewritten {
            return None;
        }
        self.keys.get(&symbol).map(|span| AKey::Filtered(*span))
    }

    pub(super) fn filters_loop(&self, span: Span) -> bool {
        self.preds.contains_key(&span)
    }

    pub(super) fn open(&self) {
        self.open.set(true);
    }

    pub(super) fn accepts(&self, scoping: &Scoping, filter: Span, name: &str) -> bool {
        let Some(slot) = self.preds.get(&filter) else {
            return true;
        };
        let Some(pred) = slot else {
            return false;
        };
        if self.open.get() {
            return true;
        }
        if name == "undefined" {
            return true;
        }
        let cached = (filter, name.to_string());
        if let Some(known) = self.accepted.borrow().get(&cached) {
            return *known;
        }
        let mut machine = Machine {
            scoping,
            callees: &pred.callees,
            storage: &pred.storage,
            resolved: &pred.resolved,
            steps: 0,
            calls: 0,
        };
        let args = pred
            .args
            .iter()
            .map(|arg| {
                arg.clone()
                    .unwrap_or_else(|| V::Prim(untwist_eval::fold::Const::Str(name.to_string())))
            })
            .collect();
        let rejected = machine
            .call(pred.func, args)
            .and_then(|value| value.truthy())
            == Some(false);
        self.accepted.borrow_mut().insert(cached, !rejected);
        !rejected
    }

    pub(super) fn origin(&self, scoping: &Scoping, filter: Span) -> Origin {
        let any = |names: &[&str]| names.iter().any(|name| self.accepts(scoping, filter, name));
        Origin {
            definer: any(&DEFINE_KEYED)
                || any(&BATCH)
                || any(&GETTERS)
                || any(&MUTATORS)
                || any(&["set", "setPrototypeOf"])
                || any(&READERS),
            runner: any(&RUNNERS)
                || any(&MARKUP_CALLS)
                || self.accepts(scoping, filter, "constructor"),
            bind: self.accepts(scoping, filter, "bind"),
            apply: any(&["apply", "call"]),
            construct: any(&CONSTRUCTS),
            markup: any(&MARKUP_CALLS),
        }
    }
}

fn predicate<'a>(
    scoping: &Scoping,
    sealed: Sealed<'_, 'a>,
    facts: Facts<'_, 'a>,
    candidate: &Candidate<'a>,
) -> Option<Pred<'a>> {
    let mut shape = Shape::new(scoping, sealed, facts);
    let (callee, arguments) = match candidate.test.callee.without_parentheses() {
        Expression::Identifier(callee) => (&**callee, &candidate.test.arguments[..]),
        _ if facts.forwards(candidate.test) => {
            let (first, rest) = candidate.test.arguments.split_first()?;
            let Expression::Identifier(callee) = first.as_expression()?.without_parentheses()
            else {
                return None;
            };
            (&**callee, rest)
        }
        _ => return None,
    };
    let symbol = identifier_symbol(scoping, callee)?;
    if !shape.callee(symbol) {
        return None;
    }
    let func = *shape.callees.get(&symbol)?;
    let mut keyed = 0;
    let mut args = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let expr = argument.as_expression()?;
        match expr.without_parentheses() {
            Expression::Identifier(id) if identifier_symbol(scoping, id) == Some(candidate.key) => {
                keyed += 1;
                args.push(None);
            }
            other if closed(other) => args.push(Some(literal(other)?)),
            _ => return None,
        }
    }
    (keyed == 1).then_some(Pred {
        func,
        args,
        callees: shape.callees,
        storage: shape.storage,
        resolved: shape.resolved,
    })
}

fn found_declared(scoping: &Scoping, candidate: &Candidate<'_>) -> bool {
    let scope = candidate.within.scope;
    let declared = scoping.symbol_scope_id(candidate.found);
    scoping
        .scope_ancestors(declared)
        .any(|ancestor| ancestor == scope)
}

fn uses_are_keys<'a>(
    candidate: &Candidate<'a>,
    (rewrite, facts): (Option<Rewrite>, Facts<'_, 'a>),
) -> bool {
    let scoping = facts.scoping;
    let mut check = Uses {
        facts,
        scoping,
        candidate,
        bad: false,
        declared: false,
        rewrite,
        depth: 0,
        loop_writes: 0,
    };
    match (candidate.within.block, candidate.within.value) {
        (Some(block), _) => check.visit_function_body(block),
        (None, Some(value)) => check.visit_expression(value),
        (None, None) => return false,
    }
    let writes = scoping
        .get_resolved_references(candidate.found)
        .filter(|reference| reference.is_write())
        .count();
    let key_written = scoping
        .get_resolved_references(candidate.key)
        .any(|reference| reference.is_write());
    !check.bad && check.declared && writes == 1 + check.loop_writes && !key_written
}

struct Uses<'c, 'a> {
    facts: Facts<'c, 'a>,
    scoping: &'c Scoping,
    candidate: &'c Candidate<'a>,
    bad: bool,
    declared: bool,
    rewrite: Option<Rewrite>,
    depth: usize,
    loop_writes: usize,
}

impl<'a> Uses<'_, 'a> {
    fn out_of_order(&self, at: Span) -> bool {
        self.rewrite
            .is_some_and(|rewrite| rewrite.ordered && (self.depth > 0 || rewrite.after(at)))
    }

    fn tracked(&self, expr: &Expression<'a>) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) if self.out_of_order(id.span) => false,
            Expression::Identifier(id) => {
                let symbol = identifier_symbol(self.scoping, id);
                symbol == Some(self.candidate.found) || symbol == Some(self.candidate.key)
            }
            _ => false,
        }
    }
}

impl<'a> Uses<'_, 'a> {
    fn stored_compare(&self, call: &CallExpression<'a>) -> bool {
        if self.facts.compare(call).is_none() {
            return false;
        }
        let [left, right] = call.arguments.as_slice() else {
            return false;
        };
        let (Some(left), Some(right)) = (left.as_expression(), right.as_expression()) else {
            return false;
        };
        (self.tracked(left) && literal_side(right)) || (self.tracked(right) && literal_side(left))
    }
}

fn literal_side(side: &Expression<'_>) -> bool {
    matches!(
        side.without_parentheses(),
        Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    )
}

impl<'a> VisitJs<'a> for Uses<'_, 'a> {
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        let Some(taken) = super::opaque::choice(&it.test, self.facts) else {
            return walk_js::walk_if_statement(self, it);
        };
        self.visit_expression(&it.test);
        let branch = if taken {
            Some(&it.consequent)
        } else {
            it.alternate.as_ref()
        };
        if let Some(branch) = branch {
            self.visit_statement(branch);
        }
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        match super::opaque::choice(&it.test, self.facts) {
            Some(taken) => {
                self.visit_expression(&it.test);
                self.visit_expression(if taken { &it.consequent } else { &it.alternate });
            }
            None => walk_js::walk_conditional_expression(self, it),
        }
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        let Some(rewrite) = self.rewrite.filter(|_| is_rewrite_loop(self.rewrite, it)) else {
            return walk_js::walk_for_statement(self, it);
        };
        let checked = check_loop(self.scoping, self.candidate, it, rewrite.ordered);
        self.bad |= checked.bad;
        self.loop_writes += checked.writes;
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = identifier_symbol(self.scoping, it);
        if symbol == Some(self.candidate.found) || symbol == Some(self.candidate.key) {
            self.bad = true;
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            let symbol = id.symbol_id.get();
            if symbol == Some(self.candidate.found) {
                self.declared = true;
                self.bad |= it.init.is_some();
            }
            if symbol == Some(self.candidate.key) {
                self.bad |= it.init.is_some();
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.pattern {
            let symbol = id.symbol_id.get();
            self.bad |= symbol == Some(self.candidate.found) || symbol == Some(self.candidate.key);
        }
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.span == self.candidate.assignment {
            return;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.stored_compare(it) {
            return self.visit_expression(&it.callee);
        }
        if it.span != self.candidate.test.span {
            return walk_js::walk_call_expression(self, it);
        }
        self.visit_expression(&it.callee);
        for argument in &it.arguments {
            match argument.as_expression() {
                Some(expr) if self.tracked(expr) => {}
                _ => self.visit_argument(argument),
            }
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.visit_expression(&it.object);
        if !self.tracked(&it.expression) {
            self.visit_expression(&it.expression);
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let harmless = matches!(
            it.operator,
            UnaryOperator::LogicalNot | UnaryOperator::Typeof
        );
        if !(harmless && self.tracked(&it.argument)) {
            walk_js::walk_unary_expression(self, it);
        }
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        let compare = is_binary_compare(it.operator);
        let harmless = compare
            && ((self.tracked(&it.left) && literal_side(&it.right))
                || (self.tracked(&it.right) && literal_side(&it.left)));
        if !harmless {
            walk_js::walk_binary_expression(self, it);
        }
    }
}
