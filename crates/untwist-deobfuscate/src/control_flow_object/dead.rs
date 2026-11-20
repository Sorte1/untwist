use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::global::{references_are_global, references_are_global_or};
use untwist_core::ast_util::inline::{is_plain_function, param_names, returned_expression};
use untwist_core::ast_util::prop::member_parts;
use untwist_core::ast_util::readonly::{
    is_readonly_object, is_readonly_object_outside, ObjectUses,
};
use untwist_core::pass::Cx;
use untwist_eval::fold::{fold_binary, fold_to_const, fold_unary, Const, FoldOptions};

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};

use super::assembled::{aliases_mutated_only_in_dead, assembled_alias, Assembled};
use super::nested_proof::{
    annex_b_written, builtins_proven_intact, proven_readonly_uses, reaches_its_receiver,
    static_property, touches_builtins,
};
use super::CffValue;

fn truthy(value: &Const) -> bool {
    match value {
        Const::Bool(flag) => *flag,
        Const::Num(number) => *number != 0.0 && !number.is_nan(),
        Const::Str(text) => !text.is_empty(),
        Const::Null | Const::Undefined => false,
    }
}

fn encloses(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

const MAX_TEST_DEPTH: usize = 16;
const MAX_DEAD_ROUNDS: usize = 4;

const FOLDED_GLOBALS: [&str; 3] = ["undefined", "NaN", "Infinity"];

fn receiver_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let (receiver, _) = member_parts(expr)?;
    let Expression::Identifier(id) = receiver else {
        return None;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn tested_receivers(
    test: &Expression<'_>,
    scoping: &Scoping,
    depth: usize,
    found: &mut Vec<SymbolId>,
) {
    if depth > MAX_TEST_DEPTH {
        return;
    }
    let inner = test.without_parentheses();
    if let Some(symbol) = receiver_symbol(inner, scoping) {
        found.push(symbol);
        return;
    }
    match inner {
        Expression::CallExpression(call) if !call.optional => {
            tested_receivers(&call.callee, scoping, depth + 1, found);
            for argument in &call.arguments {
                if let Some(argument) = argument.as_expression() {
                    tested_receivers(argument, scoping, depth + 1, found);
                }
            }
        }
        Expression::BinaryExpression(binary) => {
            tested_receivers(&binary.left, scoping, depth + 1, found);
            tested_receivers(&binary.right, scoping, depth + 1, found);
        }
        Expression::UnaryExpression(unary) => {
            tested_receivers(&unary.argument, scoping, depth + 1, found);
        }
        _ => {}
    }
}

struct Known<'k, 'p> {
    symbol: SymbolId,
    props: &'k [(Cow<'k, str>, CffValue<'k, 'p>)],
    scoping: &'k Scoping,
    branches: &'k Branches<'p>,
    builtins_unpatched: bool,
    assembled: bool,
    used: RefCell<FxHashSet<SymbolId>>,
}

enum Resolved<'k, 'p> {
    Value(Const),
    Function(&'k Function<'p>),
}

type Env = FxHashMap<SymbolId, Const>;

fn symbol_of(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn param_symbols(function: &Function<'_>) -> Option<Vec<SymbolId>> {
    if function.params.rest.is_some() {
        return None;
    }
    function
        .params
        .items
        .iter()
        .map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) if param.initializer.is_none() => {
                id.symbol_id.get()
            }
            _ => None,
        })
        .collect()
}

impl<'k, 'p> Known<'k, 'p> {
    fn own(&self, member: &Expression<'_>) -> Option<Resolved<'k, 'p>> {
        let (_, name) = member_parts(member)?;
        let value = self
            .props
            .iter()
            .find(|(key, _)| key.as_ref() == name.as_ref())
            .map(|(_, value)| value)?;
        match value {
            CffValue::Text(literal) => literal
                .value
                .as_str()
                .map(|text| Resolved::Value(Const::Str(text.to_string()))),
            CffValue::Function(function) => Some(Resolved::Function(function)),
        }
    }

    fn other(
        &self,
        receiver: SymbolId,
        member: &Expression<'_>,
        depth: usize,
    ) -> Option<Resolved<'k, 'p>> {
        let (_, name) = member_parts(member)?;
        let value = match self.assembled_property(receiver, name.as_ref()) {
            Some(value) => value,
            None => match self.branches.readonly_object(
                receiver,
                self.symbol,
                self.scoping,
                self.builtins_unpatched,
            ) {
                Some(object) => static_property(object, name.as_ref())?,
                None => self.optimistic_property(receiver, name.as_ref())?,
            },
        };
        match value.without_parentheses() {
            Expression::FunctionExpression(function) if is_plain_function(function) => {
                Some(Resolved::Function(function))
            }
            Expression::FunctionExpression(_) => None,
            value => self
                .value(value, &Env::default(), depth + 1)
                .map(Resolved::Value),
        }
    }

    fn assembled_property(&self, receiver: SymbolId, name: &str) -> Option<&'p Expression<'p>> {
        if !self.assembled {
            return None;
        }
        let value = self.branches.assembled.get(&receiver)?.property(name)?;
        self.used.borrow_mut().insert(receiver);
        Some(value)
    }

    fn optimistic_property(&self, receiver: SymbolId, name: &str) -> Option<&'p Expression<'p>> {
        if !self.assembled {
            return None;
        }
        if !self
            .branches
            .reaches_owner(receiver, self.symbol, self.scoping)
        {
            return None;
        }
        let value = self.branches.optimistic.get(&receiver)?.property(name)?;
        self.used.borrow_mut().insert(receiver);
        Some(value)
    }

    fn member(&self, member: &Expression<'_>, depth: usize) -> Option<Resolved<'k, 'p>> {
        let receiver = receiver_symbol(member, self.scoping)?;
        if receiver == self.symbol {
            return self.own(member);
        }
        self.other(receiver, member, depth)
    }

    fn value(&self, expr: &Expression<'_>, env: &Env, depth: usize) -> Option<Const> {
        if depth > MAX_TEST_DEPTH {
            return None;
        }
        let inner = expr.without_parentheses();
        if receiver_symbol(inner, self.scoping).is_some() {
            return match self.member(inner, depth)? {
                Resolved::Value(value) => Some(value),
                Resolved::Function(_) => None,
            };
        }
        match inner {
            Expression::Identifier(id) if !env.is_empty() => match symbol_of(id, self.scoping) {
                Some(symbol) => env.get(&symbol).cloned(),
                None => self.folded(inner, &FoldOptions::default()),
            },
            Expression::CallExpression(call) if !call.optional => self.call(call, env, depth),
            Expression::BinaryExpression(binary) => fold_binary(
                binary.operator,
                &self.value(&binary.left, env, depth + 1)?,
                &self.value(&binary.right, env, depth + 1)?,
            ),
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                fold_unary(
                    unary.operator,
                    &self.value(&unary.argument, env, depth + 1)?,
                )
            }
            _ if references_are_global(inner, self.scoping) => {
                self.folded(inner, &FoldOptions::default())
            }
            _ => None,
        }
    }

    fn folded(&self, expr: &Expression<'_>, options: &FoldOptions) -> Option<Const> {
        if !self.builtins_unpatched
            && touches_builtins(expr, self.scoping)
            && !self.branches.builtins_intact(self.scoping)
        {
            return None;
        }
        fold_to_const(expr, options)
    }

    fn folded_own_body(&self, function: &Function<'_>, arguments: &[Const]) -> Option<Const> {
        let names = param_names(function)?;
        if names.len() != arguments.len()
            || names.iter().any(|name| FOLDED_GLOBALS.contains(name))
            || !reads_only_own_or_globals(function, self.scoping)
        {
            return None;
        }
        let mut options = FoldOptions::default();
        for (name, argument) in names.iter().zip(arguments) {
            options
                .bindings
                .insert((*name).to_string(), argument.clone());
        }
        self.folded(returned_expression(function)?, &options)
    }

    fn call(&self, call: &CallExpression<'_>, env: &Env, depth: usize) -> Option<Const> {
        let Resolved::Function(function) = self.member(&call.callee, depth)? else {
            return None;
        };
        if !is_plain_function(function) {
            return None;
        }
        let mut arguments = Vec::with_capacity(call.arguments.len());
        for argument in &call.arguments {
            arguments.push(self.value(argument.as_expression()?, env, depth + 1)?);
        }
        if let Some(value) = self.folded_own_body(function, &arguments) {
            return Some(value);
        }
        let params = param_symbols(function)?;
        if params.len() != arguments.len() {
            return None;
        }
        let bound: Env = params.into_iter().zip(arguments).collect();
        if bound.is_empty() {
            return None;
        }
        self.value(returned_expression(function)?, &bound, depth + 1)
    }
}

fn decide(test: &Expression<'_>, known: &Known<'_, '_>) -> Option<bool> {
    known
        .value(test, &Env::default(), 0)
        .map(|value| truthy(&value))
}

fn reads_only_own_or_globals(function: &Function<'_>, scoping: &Scoping) -> bool {
    let own = function.scope_id.get();
    let is_own = |symbol| Some(scoping.symbol_scope_id(symbol)) == own;
    returned_expression(function)
        .is_some_and(|expr| references_are_global_or(expr, scoping, is_own))
}

struct Mentions<'s> {
    scoping: &'s Scoping,
    found: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found.extend(symbol_of(it, self.scoping));
    }
}

fn mentioned(object: &ObjectExpression<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut scan = Mentions {
        scoping,
        found: FxHashSet::default(),
    };
    scan.visit_object_expression(object);
    scan.found
}

#[derive(Clone, Copy)]
pub(super) struct DeadPart {
    node: usize,
    pub span: Span,
}

impl DeadPart {
    fn of_statement(stmt: &Statement<'_>) -> Self {
        Self {
            node: std::ptr::from_ref(stmt) as usize,
            span: covering_span(stmt),
        }
    }

    fn of_expression(expr: &Expression<'_>) -> Self {
        Self {
            node: std::ptr::from_ref(expr) as usize,
            span: expr.span(),
        }
    }
}

pub(super) struct Branch<'p> {
    test: &'p Expression<'p>,
    consequent: DeadPart,
    alternate: Option<DeadPart>,
}

#[derive(Default)]
pub(super) struct Branches<'p> {
    by_receiver: FxHashMap<SymbolId, Vec<Branch<'p>>>,
    statements: Vec<Span>,
    objects: FxHashMap<SymbolId, &'p ObjectExpression<'p>>,
    assembled: FxHashMap<SymbolId, Assembled<'p>>,
    optimistic: FxHashMap<SymbolId, Assembled<'p>>,
    hazard: bool,
    program: Option<&'p Program<'p>>,
    readonly: RefCell<FxHashMap<SymbolId, Option<ObjectUses>>>,
    dead: RefCell<Vec<Span>>,
    reached: RefCell<FxHashMap<(SymbolId, SymbolId), bool>>,
    builtins: OnceCell<bool>,
}

impl<'p> Branches<'p> {
    fn readonly_object(
        &self,
        receiver: SymbolId,
        owner: SymbolId,
        scoping: &Scoping,
        builtins_unpatched: bool,
    ) -> Option<&'p ObjectExpression<'p>> {
        let object = *self.objects.get(&receiver)?;
        if !self.reaches_owner(receiver, owner, scoping) {
            return None;
        }
        let program = self.program?;
        if !self.readonly.borrow().contains_key(&receiver) {
            let uses = proven_readonly_uses(program, object, scoping, receiver, builtins_unpatched);
            self.readonly.borrow_mut().insert(receiver, uses);
        }
        let readonly = self
            .readonly
            .borrow()
            .get(&receiver)
            .and_then(Option::as_ref)
            .is_some_and(|uses| {
                is_readonly_object(uses) || is_readonly_object_outside(uses, &self.dead.borrow())
            });
        readonly.then_some(object)
    }

    fn builtins_intact(&self, scoping: &Scoping) -> bool {
        *self.builtins.get_or_init(|| {
            self.program
                .is_some_and(|program| builtins_proven_intact(program, scoping))
        })
    }

    fn reaches_owner(&self, receiver: SymbolId, owner: SymbolId, scoping: &Scoping) -> bool {
        if let Some(known) = self.reached.borrow().get(&(receiver, owner)).copied() {
            return known;
        }
        let reached = self.search_owner(receiver, owner, scoping);
        self.reached.borrow_mut().insert((receiver, owner), reached);
        reached
    }

    fn search_owner(&self, receiver: SymbolId, owner: SymbolId, scoping: &Scoping) -> bool {
        let mut seen = FxHashSet::default();
        let mut pending = vec![receiver];
        while let Some(symbol) = pending.pop() {
            if !seen.insert(symbol) {
                continue;
            }
            let Some(object) = self.objects.get(&symbol) else {
                continue;
            };
            let found = mentioned(object, scoping);
            if found.contains(&owner) {
                return true;
            }
            pending.extend(
                found
                    .into_iter()
                    .filter(|next| self.objects.contains_key(next)),
            );
        }
        false
    }

    fn tested_by(&self, symbol: SymbolId) -> impl Iterator<Item = &Branch<'p>> {
        self.by_receiver
            .iter()
            .filter(move |(receiver, _)| {
                **receiver == symbol
                    || self.objects.contains_key(receiver)
                    || self.assembled.contains_key(receiver)
            })
            .flat_map(|(_, branches)| branches)
    }
}

struct BranchScan<'s, 'p> {
    scoping: &'s Scoping,
    found: Branches<'p>,
    bodies: Vec<&'p FunctionBody<'p>>,
}

impl<'p> BranchScan<'_, 'p> {
    fn branch(
        &mut self,
        test: &'p Expression<'p>,
        consequent: DeadPart,
        alternate: Option<DeadPart>,
    ) {
        let mut symbols = Vec::new();
        tested_receivers(test, self.scoping, 0, &mut symbols);
        symbols.sort_unstable();
        symbols.dedup();
        for symbol in symbols {
            self.found
                .by_receiver
                .entry(symbol)
                .or_default()
                .push(Branch {
                    test,
                    consequent,
                    alternate,
                });
        }
    }
}

impl<'p> BranchScan<'_, 'p> {
    fn never_rebound(&self, declarator: &VariableDeclarator<'p>) -> bool {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        id.symbol_id.get().is_some_and(|symbol| {
            !self.scoping.symbol_is_mutated(symbol)
                && self.scoping.symbol_redeclarations(symbol).is_empty()
        })
    }

    fn const_object(&mut self, declarator: &VariableDeclarator<'p>) {
        let (BindingPattern::BindingIdentifier(id), Some(Expression::ObjectExpression(object))) =
            (&declarator.id, &declarator.init)
        else {
            return;
        };
        if let Some(symbol) = id.symbol_id.get() {
            let object: &'p ObjectExpression<'p> = self.alloc(&**object);
            self.found.objects.insert(symbol, object);
            let root = self.bodies.last().copied();
            let plain = root
                .filter(|_| !reaches_its_receiver(object))
                .and_then(|_| Assembled::plain(symbol, object, self.scoping));
            if let Some(plain) = plain {
                self.found.optimistic.insert(symbol, plain.within(root));
            }
        }
    }
}

impl<'p> VisitJs<'p> for BranchScan<'_, 'p> {
    fn visit_function_body(&mut self, it: &FunctionBody<'p>) {
        let it: &'p FunctionBody<'p> = self.alloc(it);
        self.bodies.push(it);
        walk_js::walk_function_body(self, it);
        self.bodies.pop();
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'p>) {
        let it: &'p IfStatement<'p> = self.alloc(it);
        self.branch(
            &it.test,
            DeadPart::of_statement(&it.consequent),
            it.alternate.as_ref().map(DeadPart::of_statement),
        );
        walk_js::walk_if_statement(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'p>) {
        let it: &'p ConditionalExpression<'p> = self.alloc(it);
        self.branch(
            &it.test,
            DeadPart::of_expression(&it.consequent),
            Some(DeadPart::of_expression(&it.alternate)),
        );
        walk_js::walk_conditional_expression(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'p>) {
        for declarator in &it.declarations {
            if it.kind == VariableDeclarationKind::Const || self.never_rebound(declarator) {
                self.const_object(declarator);
            }
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_statements(&mut self, it: &AVec<'p, Statement<'p>>) {
        let it: &'p AVec<'p, Statement<'p>> = self.alloc(it);
        for start in 0..it.len() {
            let Some(root) = self.bodies.last().copied() else {
                break;
            };
            let found = assembled_alias(&it[start..], self.scoping)
                .filter(|(_, assembled)| !assembled.reaches_its_receiver());
            if let Some((alias, assembled)) = found {
                self.found
                    .assembled
                    .insert(alias, assembled.within(Some(root)));
            }
        }
        self.found
            .statements
            .extend(it.iter().filter_map(|statement| match statement {
                Statement::ExpressionStatement(expression) => Some(expression.span),
                _ => None,
            }));
        walk_js::walk_statements(self, it);
    }
}

fn covering_span(stmt: &Statement<'_>) -> Span {
    let span = stmt.span();
    if !span.is_empty() {
        return span;
    }
    let Statement::BlockStatement(block) = stmt else {
        return span;
    };
    let mut inner = block.body.iter().map(covering_span);
    let Some(first) = inner.next() else {
        return span;
    };
    if first.is_empty() {
        return span;
    }
    let mut covered = first;
    for next in inner {
        if next.is_empty() || next.start < covered.end {
            return span;
        }
        covered.end = next.end;
    }
    covered
}

pub(super) fn predicate_branches<'p>(program: &'p Program<'p>, scoping: &Scoping) -> Branches<'p> {
    let mut scan = BranchScan {
        scoping,
        found: Branches::default(),
        bodies: Vec::new(),
    };
    scan.visit_program(program);
    if let Some(writes) = annex_b_written(program, scoping) {
        scan.found
            .objects
            .retain(|symbol, _| !writes.may_write(*symbol));
        scan.found
            .optimistic
            .retain(|symbol, _| !writes.may_write(*symbol));
        scan.found.assembled.retain(|alias, assembled| {
            !writes.may_write(*alias) && !writes.may_write(assembled.base())
        });
    }
    scan.found.program = Some(program);
    scan.found.hazard = untwist_core::semantic::scope_hazard(scoping).is_some();
    scan.found
}

pub(super) fn dead_branch_parts<'p>(
    branches: &Branches<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    cx: &Cx<'_, '_>,
) -> Vec<DeadPart> {
    let unpatched = cx.opts.unsafe_transforms;
    let lanes = (!branches.hazard, unpatched);
    let (dead, used) = settled_parts(branches, symbol, props, lanes, cx.scoping());
    let proven = (unpatched, cx.scoping());
    if used.is_empty() || assembled_uses_dead(branches, &used, &dead, proven) {
        return dead;
    }
    settled_parts(branches, symbol, props, (false, unpatched), cx.scoping()).0
}

fn settled_parts<'p>(
    branches: &Branches<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    lanes: (bool, bool),
    scoping: &Scoping,
) -> (Vec<DeadPart>, FxHashSet<SymbolId>) {
    let (mut parts, mut used) = decided_parts(branches, symbol, props, lanes, scoping);
    for _ in 0..MAX_DEAD_ROUNDS {
        let spans: Vec<Span> = parts.iter().map(|part| part.span).collect();
        if spans.len() <= branches.dead.borrow().len() {
            break;
        }
        *branches.dead.borrow_mut() = spans;
        (parts, used) = decided_parts(branches, symbol, props, lanes, scoping);
    }
    branches.dead.borrow_mut().clear();
    (parts, used)
}

fn decided_parts<'p>(
    branches: &Branches<'p>,
    symbol: SymbolId,
    props: &[(Cow<'_, str>, CffValue<'_, 'p>)],
    (assembled, builtins_unpatched): (bool, bool),
    scoping: &Scoping,
) -> (Vec<DeadPart>, FxHashSet<SymbolId>) {
    let known = Known {
        symbol,
        props,
        scoping,
        branches,
        builtins_unpatched,
        assembled,
        used: RefCell::default(),
    };
    let mut seen = FxHashSet::default();
    let dead = branches
        .tested_by(symbol)
        .filter(|branch| seen.insert(std::ptr::from_ref(branch.test) as usize))
        .filter_map(|branch| {
            let taken = decide(branch.test, &known)?;
            if taken {
                branch.alternate
            } else {
                Some(branch.consequent)
            }
        })
        .collect();
    (dead, known.used.into_inner())
}

fn assembled_uses_dead(
    branches: &Branches<'_>,
    used: &FxHashSet<SymbolId>,
    dead: &[DeadPart],
    (builtins_unpatched, scoping): (bool, &Scoping),
) -> bool {
    let Some(program) = branches.program else {
        return false;
    };
    let nodes: FxHashSet<usize> = dead.iter().map(|part| part.node).collect();
    let objects: Option<Vec<_>> = used
        .iter()
        .map(|alias| {
            branches
                .assembled
                .get(alias)
                .or_else(|| branches.optimistic.get(alias))
                .map(|assembled| (*alias, assembled))
        })
        .collect();
    objects.is_some_and(|objects| {
        let proven = (scoping, builtins_unpatched);
        aliases_mutated_only_in_dead(program, proven, objects.into_iter(), &nodes)
    })
}

pub(super) struct DeadWrites {
    pub statements: Vec<Span>,
    pub kept: Vec<Span>,
}

pub(super) fn dead_write_statements(
    branches: &Branches<'_>,
    mutations: &[Span],
    dead: &[Span],
) -> DeadWrites {
    let mut found: Vec<Option<Span>> = vec![None; mutations.len()];
    let inside_dead = |span: &&Span| dead.iter().any(|range| encloses(*range, **span));
    for span in branches.statements.iter().filter(inside_dead) {
        for (slot, mutation) in found.iter_mut().zip(mutations) {
            if encloses(*span, *mutation) {
                *slot = Some(*span);
            }
        }
    }
    let mut statements = Vec::new();
    let mut kept = Vec::new();
    for (mutation, found) in mutations.iter().zip(&found) {
        match found {
            Some(span) => statements.push(*span),
            None => kept.push(*mutation),
        }
    }
    statements.sort_unstable_by_key(|span| (span.start, span.end));
    statements.dedup();
    DeadWrites { statements, kept }
}

struct DeadUses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    dead: FxHashSet<usize>,
    depth: usize,
    inside: FxHashSet<Span>,
    outside: FxHashSet<Span>,
}

impl DeadUses<'_> {
    fn record(&mut self, span: Span) {
        if self.depth > 0 {
            self.inside.insert(span);
        } else {
            self.outside.insert(span);
        }
    }

    fn resolves(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            == Some(self.symbol)
    }

    fn entering(&mut self, node: usize) -> bool {
        let dead = self.dead.contains(&node);
        if dead {
            self.depth += 1;
        }
        dead
    }
}

impl<'a> VisitJs<'a> for DeadUses<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        walk_js::walk_statement(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        if receiver_symbol(it, self.scoping) == Some(self.symbol) {
            self.record(it.span());
        }
        walk_js::walk_expression(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = it.as_member_expression() {
            if let Expression::Identifier(object) = member.object() {
                if self.resolves(object) {
                    self.record(member.span());
                }
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.resolves(it) {
            self.record(it.span);
        }
    }
}

pub(super) fn mutations_only_in_dead_parts(
    program: &Program<'_>,
    symbol: SymbolId,
    dead: &[DeadPart],
    mutations: &[Span],
    scoping: &Scoping,
) -> bool {
    let mut uses = DeadUses {
        scoping,
        symbol,
        dead: dead.iter().map(|part| part.node).collect(),
        depth: 0,
        inside: FxHashSet::default(),
        outside: FxHashSet::default(),
    };
    uses.visit_program(program);
    mutations
        .iter()
        .all(|span| uses.inside.contains(span) && !uses.outside.contains(span))
}
