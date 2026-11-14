use std::cell::RefCell;

use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::builtins::{self, Owner, Param};
use crate::facts::{Slot, SlotKind, Source};
use crate::infer::{global_name, resolved, Env};
use crate::stats::{Position, TypeStats};
use crate::ty::{Kind, Ty, Val};
use crate::view::{binary_view, static_name, Annotations, View, Viewer};
use crate::Mode;

const CONTEXT_DEPTH: usize = 12;
const DECIDE_ROUNDS: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Target {
    Symbol(SymbolId),
    Return(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Keep,
    Redundant,
    Refuse,
}

pub struct Decided {
    pub notes: Annotations,
    pub stats: TypeStats,
}

struct Decider<'e, 'f, 'a> {
    env: &'e Env<'f, 'a>,
    mode: Mode,
    visiting: RefCell<FxHashSet<SymbolId>>,
    reason: RefCell<Option<String>>,
}

fn position_of(slot: &Slot<'_>) -> Position {
    match slot.kind {
        SlotKind::Binding { .. } => Position::Binding,
        SlotKind::Param { .. } => Position::Param,
    }
}

pub fn decide(env: &Env<'_, '_>, mode: Mode) -> Decided {
    let decider = Decider {
        env,
        mode,
        visiting: RefCell::default(),
        reason: RefCell::default(),
    };
    let mut stats = TypeStats::default();
    let mut notes = Annotations {
        symbols: FxHashMap::default(),
        returns: FxHashMap::default(),
    };
    let mut kinds: FxHashMap<Target, Position> = FxHashMap::default();
    for (symbol, slot) in &env.facts.slots {
        let position = position_of(slot);
        stats.count(position).eligible += 1;
        let Some(ty) = env.vals.get(symbol).and_then(Val::ty) else {
            continue;
        };
        stats.count(position).proven += 1;
        match (decider.placeable(slot), admissible(ty)) {
            (true, Ok(())) => {
                notes.symbols.insert(*symbol, ty.clone());
                kinds.insert(Target::Symbol(*symbol), position);
            }
            (false, _) => {
                stats.count(position).refused += 1;
                stats.refuse("placement".to_string());
            }
            (true, Err(reason)) => {
                stats.count(position).refused += 1;
                stats.refuse(reason.to_string());
            }
        }
    }
    for (index, fact) in env.facts.functions.iter().enumerate() {
        if !fact.returns_typed {
            continue;
        }
        stats.count(Position::Return).eligible += 1;
        let Some(ty) = env.returns[index].ty() else {
            continue;
        };
        stats.count(Position::Return).proven += 1;
        let inner = match fact.func.is_async() {
            true => promised(ty),
            false => Some(ty.clone()),
        };
        let fine = match inner {
            _ if !fact.closed => Err("return of a function with unknown callers"),
            Some(inner) => admissible(&inner),
            None => Err("promise"),
        };
        match fine {
            Ok(()) => {
                notes.returns.insert(index, ty.clone());
                kinds.insert(Target::Return(index), Position::Return);
            }
            Err(_) if decider.inferred_alike(index, ty) => {
                stats.count(Position::Return).redundant += 1;
            }
            Err(reason) => {
                stats.count(Position::Return).refused += 1;
                stats.refuse(reason.to_string());
            }
        }
    }
    let mut redundant: FxHashSet<Target> = FxHashSet::default();
    let mut reasons: FxHashMap<Target, String> = FxHashMap::default();
    let mut settled = false;
    for _ in 0..DECIDE_ROUNDS {
        let mut changed = false;
        let mut targets: Vec<Target> = notes
            .symbols
            .keys()
            .map(|symbol| Target::Symbol(*symbol))
            .chain(notes.returns.keys().map(|index| Target::Return(*index)))
            .collect();
        targets.sort_by_key(|target| match target {
            Target::Symbol(symbol) => (0, symbol.index()),
            Target::Return(index) => (1, *index),
        });
        for target in targets {
            let held = match target {
                Target::Symbol(symbol) => notes.symbols.get(&symbol).cloned(),
                Target::Return(index) => notes.returns.get(&index).cloned(),
            };
            let Some(ty) = held else {
                continue;
            };
            match decider.judge(target, &ty, &notes) {
                Verdict::Keep => {}
                verdict => {
                    let wider = ty.widened();
                    let widen = verdict == Verdict::Refuse
                        && wider != ty
                        && decider.judge(target, &wider, &notes) == Verdict::Keep;
                    match (target, widen) {
                        (Target::Symbol(symbol), true) => {
                            notes.symbols.insert(symbol, wider);
                        }
                        (Target::Return(index), true) => {
                            notes.returns.insert(index, wider);
                        }
                        (Target::Symbol(symbol), false) => {
                            notes.symbols.remove(&symbol);
                        }
                        (Target::Return(index), false) => {
                            notes.returns.remove(&index);
                        }
                    }
                    if !widen && verdict == Verdict::Redundant {
                        redundant.insert(target);
                    }
                    if !widen && verdict == Verdict::Refuse {
                        let reason = decider.reason.borrow_mut().take();
                        reasons.insert(target, reason.unwrap_or_else(|| "refused".to_string()));
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            settled = true;
            break;
        }
    }
    if !settled {
        notes.symbols.clear();
        notes.returns.clear();
        stats.refuse("decision did not settle".to_string());
    }
    for (target, position) in kinds {
        let annotated = match target {
            Target::Symbol(symbol) => notes.symbols.contains_key(&symbol),
            Target::Return(index) => notes.returns.contains_key(&index),
        };
        let row = stats.count(position);
        match (annotated, redundant.contains(&target)) {
            (true, _) => row.annotated += 1,
            (false, true) => row.redundant += 1,
            (false, false) => {
                row.refused += 1;
                let reason = reasons
                    .remove(&target)
                    .unwrap_or_else(|| "refused".to_string());
                stats.refuse(reason);
            }
        }
    }
    Decided { notes, stats }
}

fn promised(ty: &Ty) -> Option<Ty> {
    match ty.members() {
        [crate::ty::Member::Promise(inner)] => Some((**inner).clone()),
        _ => None,
    }
}

fn admissible(ty: &Ty) -> Result<(), &'static str> {
    match ty.promise_free() {
        true => Ok(()),
        false => Err("promise"),
    }
}

impl<'e, 'f, 'a> Decider<'e, 'f, 'a> {
    fn cx(&self) -> &'f crate::facts::Ctx<'f, 'a> {
        self.env.cx
    }

    fn placeable(&self, slot: &Slot<'a>) -> bool {
        match slot.kind {
            SlotKind::Binding { in_loop_head, .. } => {
                if in_loop_head {
                    return false;
                }
                let Some(declarator) = slot.declarator else {
                    return false;
                };
                let nodes = self.cx().nodes;
                let declaration = nodes.parent_id(declarator.node_id.get());
                let exported = matches!(
                    nodes.parent_kind(declaration),
                    AstKind::ExportNamedDeclaration(_)
                );
                if exported {
                    return false;
                }
                match self.mode {
                    Mode::Ts => true,
                    Mode::Jsdoc => jsdoc_placeable(nodes, declaration),
                }
            }
            SlotKind::Param { .. } => true,
        }
    }

    fn judge(&self, target: Target, ty: &Ty, notes: &Annotations) -> Verdict {
        let viewer = Viewer::new(self.env, notes);
        self.visiting.borrow_mut().clear();
        self.reason.borrow_mut().take();
        match target {
            Target::Symbol(symbol) => self.judge_symbol(&viewer, symbol, ty),
            Target::Return(index) => self.judge_return(&viewer, index, ty),
        }
    }

    fn judge_symbol(&self, viewer: &Viewer<'_, 'f, 'a>, symbol: SymbolId, ty: &Ty) -> Verdict {
        let Some(slot) = self.env.facts.slots.get(&symbol) else {
            return Verdict::Refuse;
        };
        if let SlotKind::Binding { .. } = slot.kind {
            if viewer.declared_without(symbol) == View::Known(ty.clone()) {
                return Verdict::Redundant;
            }
        }
        if !slot
            .sources
            .iter()
            .all(|source| self.source_fits(viewer, source, ty))
        {
            return self.refuse("a write the checker would type differently");
        }
        if let SlotKind::Param { function, .. } = slot.kind {
            if !self.param_leaves_calls_alone(viewer, function) {
                return self.refuse("the function's call results would change type");
            }
            if self.mode == Mode::Jsdoc && !self.calls_pass_every_argument(function) {
                return self.refuse("jsdoc: a call passes fewer arguments");
            }
        }
        self.visiting.borrow_mut().insert(symbol);
        match self.symbol_reads_ok(viewer, symbol, &View::Known(ty.clone()), 0) {
            true => Verdict::Keep,
            false => Verdict::Refuse,
        }
    }

    fn inferred_alike(&self, index: usize, ty: &Ty) -> bool {
        let empty = Annotations {
            symbols: FxHashMap::default(),
            returns: FxHashMap::default(),
        };
        let viewer = Viewer::new(self.env, &empty);
        let fact = &self.env.facts.functions[index];
        let mut joined = match fact.falls_through {
            true => View::Known(Ty::undefined()),
            false => View::Known(Ty::from_members(Vec::new())),
        };
        for operand in &fact.returns {
            let view = match operand {
                Some(operand) => viewer.view(operand).widened(),
                None => View::Known(Ty::undefined()),
            };
            joined = joined.join(&view);
        }
        if let Some(body) = fact.func.expression_body() {
            joined = joined.join(&viewer.view(body).widened());
        }
        let expected = match fact.func.is_async() {
            true => promised(ty),
            false => Some(ty.clone()),
        };
        matches!((joined, expected), (View::Known(seen), Some(expected)) if seen == expected.widened())
    }

    fn calls_pass_every_argument(&self, function: usize) -> bool {
        let fact = &self.env.facts.functions[function];
        let items = &fact.func.params().items;
        let required = items
            .iter()
            .rposition(|param| param.initializer.is_none())
            .map_or(0, |last| last + 1);
        fact.calls
            .iter()
            .all(|call| call.arguments.len() >= required)
    }

    fn param_leaves_calls_alone(&self, viewer: &Viewer<'_, 'f, 'a>, function: usize) -> bool {
        if viewer.notes.returns.contains_key(&function) {
            return true;
        }
        let fact = &self.env.facts.functions[function];
        let returns_value =
            fact.func.expression_body().is_some() || fact.returns.iter().any(Option::is_some);
        if !returns_value {
            return true;
        }
        fact.calls
            .iter()
            .all(|call| self.context_ok(viewer, call.node_id.get(), &View::Unknown, 0))
    }

    fn source_fits(&self, viewer: &Viewer<'_, 'f, 'a>, source: &Source<'a>, ty: &Ty) -> bool {
        match source {
            Source::Expr(expression) => viewer.view(expression).fits(ty),
            Source::Undefined => ty.any_of(Kind::Undefined),
            Source::String => ty.contains(&Ty::string()),
            Source::Top => false,
            Source::Compound(operator, _, right) => {
                compound_ok(*operator, ty, &viewer.view(right))
                    && binary_view(*operator, &View::Known(ty.clone()), &viewer.view(right))
                        .fits(ty)
            }
            Source::Update(_) => ty.only(Kind::Number) || ty.only(Kind::BigInt),
            Source::Defaulted(argument, default) => {
                let with_undefined = ty.join(&Ty::undefined());
                viewer.view(default).fits(ty)
                    && argument.is_none_or(|argument| viewer.view(argument).fits(&with_undefined))
            }
        }
    }

    fn judge_return(&self, viewer: &Viewer<'_, 'f, 'a>, index: usize, ty: &Ty) -> Verdict {
        let fact = &self.env.facts.functions[index];
        let inner = match fact.func.is_async() {
            true => match promised(ty) {
                Some(inner) => inner,
                None => return Verdict::Refuse,
            },
            false => ty.clone(),
        };
        let returns_nothing =
            fact.func.expression_body().is_none() && fact.returns.iter().all(Option::is_none);
        if returns_nothing && inner.is_exactly_undefined() {
            return Verdict::Redundant;
        }
        let operands = fact
            .returns
            .iter()
            .flatten()
            .copied()
            .chain(fact.func.expression_body());
        for operand in operands {
            if !viewer.view(operand).fits(&inner) {
                return self.refuse("a return the checker would type differently");
            }
        }
        let class = View::Known(ty.clone());
        let ok = fact
            .calls
            .iter()
            .all(|call| self.context_ok(viewer, call.node_id.get(), &class, 0));
        match ok {
            true => Verdict::Keep,
            false => Verdict::Refuse,
        }
    }

    fn symbol_reads_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        symbol: SymbolId,
        class: &View,
        depth: usize,
    ) -> bool {
        let cx = self.cx();
        for reference in cx.scoping.get_resolved_references(symbol) {
            let node = reference.node_id();
            if reference.flags().is_write() {
                if !self.write_use_ok(viewer, node, class) {
                    return false;
                }
                continue;
            }
            if !self.context_ok(viewer, node, class, depth) {
                return false;
            }
        }
        true
    }

    fn write_use_ok(&self, viewer: &Viewer<'_, 'f, 'a>, node: NodeId, class: &View) -> bool {
        let View::Known(ty) = class else {
            return false;
        };
        match self.cx().nodes.parent_kind(node) {
            AstKind::UpdateExpression(update) => {
                (ty.only(Kind::Number) || ty.only(Kind::BigInt))
                    && self.context_ok(viewer, update.node_id.get(), &View::Known(ty.widened()), 0)
            }
            AstKind::AssignmentExpression(assign) => {
                let value_ok = |cx: &Self| {
                    cx.context_ok(viewer, assign.node_id.get(), &View::Known(ty.clone()), 0)
                };
                match assign.operator {
                    AssignmentOperator::Assign => value_ok(self),
                    operator => match operator.to_binary_operator() {
                        Some(binary) => {
                            compound_ok(binary, ty, &viewer.view(&assign.right)) && value_ok(self)
                        }
                        None => value_ok(self),
                    },
                }
            }
            AstKind::ForInStatement(_) => true,
            _ => false,
        }
    }

    fn refuse(&self, reason: &str) -> Verdict {
        let mut held = self.reason.borrow_mut();
        if held.is_none() {
            *held = Some(reason.to_string());
        }
        Verdict::Refuse
    }

    fn context_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        node: NodeId,
        class: &View,
        depth: usize,
    ) -> bool {
        let ok = self.context_inner(viewer, node, class, depth);
        if !ok {
            let parent = self.cx().nodes.parent_kind(node).ty();
            let mut held = self.reason.borrow_mut();
            if held.is_none() {
                *held = Some(format!("used in {parent:?}"));
            }
        }
        ok
    }

    fn context_inner(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        node: NodeId,
        class: &View,
        depth: usize,
    ) -> bool {
        if depth > CONTEXT_DEPTH {
            return false;
        }
        let nodes = self.cx().nodes;
        let child = nodes.kind(node).span();
        let parent_id = nodes.parent_id(node);
        let deeper = depth + 1;
        match nodes.kind(parent_id) {
            AstKind::ExpressionStatement(_)
            | AstKind::IfStatement(_)
            | AstKind::WhileStatement(_)
            | AstKind::DoWhileStatement(_)
            | AstKind::ForStatement(_)
            | AstKind::ThrowStatement(_)
            | AstKind::TemplateLiteral(_) => true,
            AstKind::ParenthesizedExpression(_) | AstKind::AwaitExpression(_) => {
                self.context_ok(viewer, parent_id, class, deeper)
            }
            AstKind::ConditionalExpression(conditional) => {
                if conditional.test.span() == child {
                    return true;
                }
                let other = match conditional.consequent.span() == child {
                    true => &conditional.alternate,
                    false => &conditional.consequent,
                };
                let joined = class.join(&viewer.view(other));
                self.context_ok(viewer, parent_id, &joined, deeper)
            }
            AstKind::LogicalExpression(logical) => {
                let other = match logical.left.span() == child {
                    true => &logical.right,
                    false => &logical.left,
                };
                let joined = class.join(&viewer.view(other));
                self.context_ok(viewer, parent_id, &joined, deeper)
            }
            AstKind::SequenceExpression(sequence) => {
                match sequence.expressions.last().map(GetSpan::span) == Some(child) {
                    true => self.context_ok(viewer, parent_id, class, deeper),
                    false => true,
                }
            }
            AstKind::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::LogicalNot | UnaryOperator::Typeof | UnaryOperator::Void => true,
                UnaryOperator::Delete => false,
                UnaryOperator::UnaryPlus => {
                    matches!(class, View::Known(ty) if !ty.any_of(Kind::BigInt))
                }
                _ => class.only(Kind::Number),
            },
            AstKind::BinaryExpression(binary) => {
                let other = match binary.left.span() == child {
                    true => &binary.right,
                    false => &binary.left,
                };
                let on_left = binary.left.span() == child;
                self.binary_ok(viewer, (binary, parent_id), class, (other, on_left), deeper)
            }
            AstKind::AssignmentExpression(assign) => {
                if assign.right.span() != child {
                    return false;
                }
                self.assigned_ok(viewer, assign, parent_id, class, deeper)
            }
            AstKind::VariableDeclarator(declarator) => {
                self.declared_ok(viewer, declarator, class, deeper)
            }
            AstKind::ReturnStatement(_) => self.returned_ok(viewer, parent_id, class, deeper),
            AstKind::ArrowFunctionExpression(arrow) => {
                match arrow.body.as_expression().map(GetSpan::span) == Some(child) {
                    true => self.returned_from(viewer, arrow.node_id.get(), class, deeper),
                    false => false,
                }
            }
            AstKind::CallExpression(call) => {
                if call.callee.without_parentheses().span() == child {
                    return false;
                }
                let Some(position) = call
                    .arguments
                    .iter()
                    .position(|argument| argument.span() == child)
                else {
                    return false;
                };
                if call.arguments[position].is_spread() {
                    return false;
                }
                self.argument_ok(viewer, call, position, class)
            }
            AstKind::NewExpression(new) => {
                new.callee.span() != child && self.constructs_local_class(&new.callee)
            }
            AstKind::StaticMemberExpression(member) => {
                if member.object.span() != child || member.optional {
                    return false;
                }
                self.member_ok(viewer, member, parent_id, class, deeper)
            }
            AstKind::ComputedMemberExpression(member) => {
                if self.is_write_target(parent_id) {
                    return false;
                }
                if member.object.span() == child {
                    return class.only(Kind::String)
                        && !member.optional
                        && self.context_ok(viewer, parent_id, &View::Known(Ty::string()), deeper);
                }
                matches!(class, View::Known(ty) if ty.kinds().all(|kind| matches!(kind, Kind::String | Kind::Number)))
            }
            AstKind::SwitchStatement(switch) => switch.cases.iter().all(|case| {
                case.test
                    .as_ref()
                    .is_none_or(|test| comparable(class, &viewer.view(test)))
            }),
            AstKind::SwitchCase(_) => {
                let switch = nodes.parent_kind(parent_id);
                match switch {
                    AstKind::SwitchStatement(switch) => {
                        comparable(class, &viewer.view(&switch.discriminant))
                    }
                    _ => false,
                }
            }
            AstKind::ObjectProperty(property) => {
                property.computed
                    && property.key.span() == child
                    && matches!(class, View::Known(ty) if ty.kinds().all(|kind| matches!(kind, Kind::String | Kind::Number)))
            }
            _ => false,
        }
    }

    fn is_write_target(&self, member: NodeId) -> bool {
        let nodes = self.cx().nodes;
        let span = nodes.kind(member).span();
        match nodes.parent_kind(member) {
            AstKind::AssignmentExpression(assign) => assign.left.span() == span,
            AstKind::UpdateExpression(_) => true,
            AstKind::UnaryExpression(unary) => unary.operator == UnaryOperator::Delete,
            AstKind::ForInStatement(_)
            | AstKind::ForOfStatement(_)
            | AstKind::ArrayAssignmentTarget(_)
            | AstKind::ObjectAssignmentTarget(_)
            | AstKind::AssignmentTargetWithDefault(_)
            | AstKind::AssignmentTargetPropertyProperty(_) => true,
            _ => false,
        }
    }

    fn binary_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        (binary, node): (&BinaryExpression<'a>, NodeId),
        class: &View,
        (other, on_left): (&Expression<'a>, bool),
        depth: usize,
    ) -> bool {
        let View::Known(ty) = class else {
            return false;
        };
        let theirs = viewer.view(other);
        let operator = binary.operator;
        if operator.is_equality() {
            return comparable(class, &theirs);
        }
        if operator.is_compare() {
            let numeric = |view: &View| match view {
                View::Any => true,
                View::Known(ty) => ty
                    .kinds()
                    .all(|kind| matches!(kind, Kind::Number | Kind::BigInt)),
                View::Unknown => false,
            };
            let textual = |view: &View| matches!(view, View::Any) || view.only(Kind::String);
            return (numeric(class) && numeric(&theirs)) || (textual(class) && textual(&theirs));
        }
        match operator {
            BinaryOperator::In => {
                on_left
                    && ty
                        .kinds()
                        .all(|kind| matches!(kind, Kind::String | Kind::Number))
            }
            BinaryOperator::Instanceof => false,
            BinaryOperator::Addition => {
                let result = if ty.only(Kind::String) {
                    View::Known(Ty::string())
                } else if ty.only(Kind::Number) && theirs.only(Kind::Number) {
                    View::Known(Ty::number())
                } else if ty.only(Kind::BigInt) && theirs.only(Kind::BigInt) {
                    View::Known(Ty::bigint())
                } else {
                    return ty.only(Kind::Number)
                        && (theirs.only(Kind::String) || matches!(theirs, View::Any));
                };
                self.context_ok(viewer, node, &result, depth)
            }
            _ => {
                if ty.only(Kind::Number) {
                    return matches!(theirs, View::Any)
                        || matches!(&theirs, View::Known(other) if !other.any_of(Kind::BigInt));
                }
                ty.only(Kind::BigInt)
                    && theirs.only(Kind::BigInt)
                    && self.context_ok(viewer, node, &View::Known(Ty::bigint()), depth)
            }
        }
    }

    fn assigned_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        assign: &AssignmentExpression<'a>,
        node: NodeId,
        class: &View,
        depth: usize,
    ) -> bool {
        let View::Known(ty) = class else {
            return false;
        };
        let target_ok = match &assign.left {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                let declared = match resolved(self.cx(), ident) {
                    Some(symbol) => viewer.symbol(symbol, 0),
                    None => View::Unknown,
                };
                let result = match assign.operator.to_binary_operator() {
                    None if assign.operator.is_assign() => View::Known(ty.clone()),
                    None => declared.join(class),
                    Some(operator) => match &declared {
                        View::Known(left) if !compound_ok(operator, left, class) => return false,
                        _ => binary_view(operator, &declared, class),
                    },
                };
                match (&declared, &result) {
                    (View::Any, _) => true,
                    (View::Known(held), View::Known(value)) => held.contains(value),
                    _ => false,
                }
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                matches!(viewer.view(&member.object), View::Any)
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                matches!(viewer.view(&member.object), View::Any)
            }
            _ => false,
        };
        target_ok && self.context_ok(viewer, node, class, depth)
    }

    fn declared_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        declarator: &VariableDeclarator<'a>,
        class: &View,
        depth: usize,
    ) -> bool {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        let View::Known(ty) = class else {
            return false;
        };
        if let Some(held) = viewer.notes.symbols.get(&symbol) {
            return held.contains(ty);
        }
        if !self.visiting.borrow_mut().insert(symbol) {
            return true;
        }
        let declared = match crate::facts::declaration_kind(self.cx().nodes, declarator).is_const()
        {
            true => ty.clone(),
            false => ty.widened(),
        };
        let Some(slot) = self.env.facts.slots.get(&symbol) else {
            return false;
        };
        let init = declarator.init.as_ref().map(GetSpan::span);
        let writes_fit = slot.sources.iter().all(|source| match source {
            Source::Expr(expression) if Some(expression.span()) == init => true,
            source => self.source_fits(viewer, source, &declared),
        });
        writes_fit && self.symbol_reads_ok(viewer, symbol, &View::Known(declared), depth)
    }

    fn returned_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        statement: NodeId,
        class: &View,
        depth: usize,
    ) -> bool {
        let nodes = self.cx().nodes;
        let owner = nodes.ancestor_ids(statement).skip(1).find(|id| {
            matches!(
                nodes.kind(*id),
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
            )
        });
        match owner {
            Some(owner) => self.returned_from(viewer, owner, class, depth),
            None => false,
        }
    }

    fn returned_from(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        function: NodeId,
        class: &View,
        depth: usize,
    ) -> bool {
        let Some(&index) = self.env.facts.by_node.get(&function) else {
            return false;
        };
        let fact = &self.env.facts.functions[index];
        if let Some(held) = viewer.notes.returns.get(&index) {
            let inner = match fact.func.is_async() {
                true => promised(held),
                false => Some(held.clone()),
            };
            return inner.is_some_and(|inner| class.fits(&inner));
        }
        if !fact.closed || fact.func.is_async() {
            return false;
        }
        let joined = fact
            .returns
            .iter()
            .map(|operand| match operand {
                Some(operand) => viewer.view(operand),
                None => View::Known(Ty::undefined()),
            })
            .chain(fact.func.expression_body().map(|body| viewer.view(body)))
            .fold(class.clone(), |held, view| held.join(&view));
        fact.calls
            .iter()
            .all(|call| self.context_ok(viewer, call.node_id.get(), &joined, depth))
    }

    fn argument_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        call: &CallExpression<'a>,
        position: usize,
        class: &View,
    ) -> bool {
        let View::Known(ty) = class else {
            return false;
        };
        match call.callee.without_parentheses() {
            Expression::Identifier(ident) => match resolved(self.cx(), ident) {
                Some(symbol) => self.local_param_ok(viewer, symbol, position, ty),
                None => {
                    let row = global_name(self.cx(), ident)
                        .and_then(|name| builtins::find(Owner::Global, name, true));
                    row.is_some_and(|row| param_accepts(row.params.get(position), class))
                }
            },
            Expression::FunctionExpression(function) => {
                self.function_param_ok(viewer, function.node_id.get(), position, ty)
            }
            Expression::ArrowFunctionExpression(arrow) => {
                self.function_param_ok(viewer, arrow.node_id.get(), position, ty)
            }
            Expression::StaticMemberExpression(member) => {
                let key = member.property.name.as_str();
                if let Some(root) = self.env.static_owner(&member.object) {
                    if builtins::ANY_ARGUMENT_CALLEES.contains(&(root, key)) {
                        return true;
                    }
                    return builtins::find(Owner::Static(static_name(root)), key, true)
                        .is_some_and(|row| param_accepts(row.params.get(position), class));
                }
                match viewer.view(&member.object) {
                    View::Any => true,
                    View::Known(receiver) => receiver.single_kind().is_some_and(|kind| {
                        builtins::find(Owner::Proto(kind), key, true)
                            .is_some_and(|row| param_accepts(row.params.get(position), class))
                    }),
                    View::Unknown => false,
                }
            }
            _ => false,
        }
    }

    fn local_param_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        symbol: SymbolId,
        position: usize,
        ty: &Ty,
    ) -> bool {
        let cx = self.cx();
        let declaration = cx.scoping.symbol_declaration(symbol);
        let function = match cx.nodes.kind(declaration) {
            AstKind::Function(function) => Some(function.node_id.get()),
            AstKind::VariableDeclarator(declarator)
                if crate::facts::declaration_kind(cx.nodes, declarator).is_const() =>
            {
                match declarator
                    .init
                    .as_ref()
                    .map(Expression::without_parentheses)
                {
                    Some(Expression::FunctionExpression(function)) => Some(function.node_id.get()),
                    Some(Expression::ArrowFunctionExpression(arrow)) => Some(arrow.node_id.get()),
                    _ => None,
                }
            }
            _ => None,
        };
        match function {
            Some(node) => self.function_param_ok(viewer, node, position, ty),
            None => matches!(viewer.symbol(symbol, 0), View::Any),
        }
    }

    fn function_param_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        node: NodeId,
        position: usize,
        ty: &Ty,
    ) -> bool {
        let Some(&index) = self.env.facts.by_node.get(&node) else {
            return false;
        };
        let fact = &self.env.facts.functions[index];
        let params = fact.func.params();
        let Some(param) = params.items.get(position) else {
            return true;
        };
        let BindingPattern::BindingIdentifier(id) = &param.pattern else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        match viewer.notes.symbols.get(&symbol) {
            Some(held) => held.contains(ty),
            None => match &param.initializer {
                Some(default) => matches!(
                    viewer.view(default).widened(),
                    View::Known(declared) if declared.contains(ty)
                ),
                None => {
                    let iife = fact.closed && fact.symbol.is_none();
                    !iife || matches!(viewer.symbol(symbol, 0), View::Any)
                }
            },
        }
    }

    fn constructs_local_class(&self, callee: &Expression<'a>) -> bool {
        let Expression::Identifier(ident) = callee.without_parentheses() else {
            return false;
        };
        let Some(symbol) = resolved(self.cx(), ident) else {
            return false;
        };
        let declaration = self.cx().scoping.symbol_declaration(symbol);
        match self.cx().nodes.kind(declaration) {
            AstKind::Class(class) => class.body.body.iter().all(|element| match element {
                ClassElement::MethodDefinition(method) => {
                    method.kind != MethodDefinitionKind::Constructor
                        || method
                            .value
                            .params
                            .items
                            .iter()
                            .all(|param| param.type_annotation.is_none())
                }
                _ => true,
            }),
            _ => false,
        }
    }

    fn member_ok(
        &self,
        viewer: &Viewer<'_, 'f, 'a>,
        member: &StaticMemberExpression<'a>,
        node: NodeId,
        class: &View,
        depth: usize,
    ) -> bool {
        let View::Known(ty) = class else {
            return false;
        };
        if self.is_write_target(node) || ty.is_nullable() {
            return false;
        }
        let Some(kind) = ty.single_kind() else {
            return false;
        };
        let key = member.property.name.as_str();
        let nodes = self.cx().nodes;
        let span = member.span;
        if let AstKind::CallExpression(call) = nodes.parent_kind(node) {
            if call.callee.span() == span {
                let Some(row) = builtins::find(Owner::Proto(kind), key, true) else {
                    return false;
                };
                let arguments_fit = call.arguments.iter().enumerate().all(|(index, argument)| {
                    !argument.is_spread()
                        && argument.as_expression().is_some_and(|expression| {
                            param_accepts(row.params.get(index), &viewer.view(expression))
                        })
                });
                return arguments_fit
                    && !call.optional
                    && self.context_ok(
                        viewer,
                        call.node_id.get(),
                        &View::Known(row.lib.ty()),
                        depth,
                    );
            }
        }
        key == "length"
            && kind == Kind::String
            && self.context_ok(viewer, node, &View::Known(Ty::number()), depth)
    }
}

fn compound_ok(operator: BinaryOperator, ty: &Ty, right: &View) -> bool {
    match operator {
        BinaryOperator::Addition => {
            ty.only(Kind::String)
                || (ty.only(Kind::Number)
                    && (right.only(Kind::Number) || matches!(right, View::Any)))
                || (ty.only(Kind::BigInt) && right.only(Kind::BigInt))
        }
        _ => {
            (ty.only(Kind::Number)
                && (matches!(right, View::Any)
                    || matches!(right, View::Known(other) if !other.any_of(Kind::BigInt))))
                || (ty.only(Kind::BigInt) && right.only(Kind::BigInt))
        }
    }
}

fn comparable(class: &View, theirs: &View) -> bool {
    let View::Known(ty) = class else {
        return false;
    };
    if ty.has_literals() {
        return false;
    }
    match theirs {
        View::Any => true,
        View::Known(other) => {
            ty.overlaps(&other.widened())
                || other
                    .kinds()
                    .all(|kind| matches!(kind, Kind::Null | Kind::Undefined))
        }
        View::Unknown => false,
    }
}

fn param_accepts(param: Option<&Param>, class: &View) -> bool {
    let Some(param) = param else {
        return true;
    };
    if matches!(class, View::Any) {
        return true;
    }
    let View::Known(ty) = class else {
        return false;
    };
    match param {
        Param::Any => true,
        Param::Num => ty.only(Kind::Number),
        Param::Str | Param::Pattern | Param::Replacement => ty.only(Kind::String),
        Param::Primitive => ty
            .kinds()
            .all(|kind| matches!(kind, Kind::String | Kind::Number | Kind::Boolean)),
    }
}

fn jsdoc_placeable(nodes: &oxc_semantic::AstNodes<'_>, declaration: NodeId) -> bool {
    let AstKind::VariableDeclaration(declaration_node) = nodes.kind(declaration) else {
        return false;
    };
    declaration_node.declarations.len() == 1
        && matches!(
            nodes.parent_kind(declaration),
            AstKind::Program(_)
                | AstKind::FunctionBody(_)
                | AstKind::BlockStatement(_)
                | AstKind::SwitchCase(_)
                | AstKind::StaticBlock(_)
        )
}
