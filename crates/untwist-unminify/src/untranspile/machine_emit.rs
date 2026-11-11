use oxc_allocator::{Box as ABox, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolFlags;
use oxc_traverse::BoundIdentifier;
use rustc_hash::{FxHashMap, FxHashSet};

use untwist_core::pass::Cx;

use super::machine::{take_slot, take_statement, Machine, Received, Slot};
use super::machine_check::Reads;
use super::structure::{Code, Fill, LabelId, YieldSpec};

pub struct Emitter<'e, 'a, 'c, 'x> {
    pub cases: Vec<AVec<'a, Statement<'a>>>,
    pub machine: &'e Machine,
    pub yields: &'e [YieldSpec],
    pub reads: &'e dyn Reads,
    pub scope: ScopeId,
    pub cx: &'x mut Cx<'a, 'c>,
    names: FxHashMap<LabelId, &'a str>,
    caught: FxHashMap<u32, BoundIdentifier<'a>>,
    received: FxHashMap<u32, BoundIdentifier<'a>>,
    pub rebound: FxHashMap<u32, BoundIdentifier<'a>>,
    loops: Vec<LabelId>,
    failed: bool,
}

struct Replace<'r, 'a> {
    reads: &'r dyn Reads,
    with: Option<Expression<'a>>,
}

impl<'a> VisitMut<'a> for Replace<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.with.is_none() {
            return;
        }
        if self.reads.is_read(it) {
            if let Some(with) = self.with.take() {
                *it = with;
            }
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

fn needs_names(code: &[Code], loops: &mut Vec<LabelId>, named: &mut FxHashSet<LabelId>) {
    for item in code {
        match item {
            Code::Break(label) | Code::Continue(label) => {
                if loops.last() != Some(label) {
                    named.insert(*label);
                }
            }
            Code::If {
                then, otherwise, ..
            } => {
                needs_names(then, loops, named);
                needs_names(otherwise, loops, named);
            }
            Code::Loop(label, body) | Code::While(label, _, _, body) => {
                loops.push(*label);
                needs_names(body, loops, named);
                loops.pop();
            }
            Code::Block(_, body) => needs_names(body, loops, named),
            Code::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                needs_names(block, loops, named);
                for part in [handler, finalizer].into_iter().flatten() {
                    needs_names(part, loops, named);
                }
            }
            _ => {}
        }
    }
}

fn label_order(code: &[Code], out: &mut Vec<LabelId>) {
    for item in code {
        match item {
            Code::If {
                then, otherwise, ..
            } => {
                label_order(then, out);
                label_order(otherwise, out);
            }
            Code::Loop(label, body) | Code::While(label, _, _, body) | Code::Block(label, body) => {
                out.push(*label);
                label_order(body, out);
            }
            Code::Try {
                label,
                block,
                handler,
                finalizer,
                ..
            } => {
                out.push(*label);
                label_order(block, out);
                for part in [handler, finalizer].into_iter().flatten() {
                    label_order(part, out);
                }
            }
            _ => {}
        }
    }
}

impl<'e, 'a, 'c, 'x> Emitter<'e, 'a, 'c, 'x> {
    pub fn new(
        cases: Vec<AVec<'a, Statement<'a>>>,
        (machine, yields): (&'e Machine, &'e [YieldSpec]),
        reads: &'e dyn Reads,
        scope: ScopeId,
        cx: &'x mut Cx<'a, 'c>,
    ) -> Self {
        Self {
            cases,
            machine,
            yields,
            reads,
            scope,
            cx,
            names: FxHashMap::default(),
            caught: FxHashMap::default(),
            received: FxHashMap::default(),
            rebound: FxHashMap::default(),
            loops: Vec::new(),
            failed: false,
        }
    }

    pub fn failed(&self) -> bool {
        self.failed
    }

    pub fn generated(&self) -> impl Iterator<Item = &BoundIdentifier<'a>> {
        self.caught.values().chain(self.received.values())
    }

    fn fail<T>(&mut self) -> Option<T> {
        self.failed = true;
        None
    }

    pub fn name_labels(&mut self, code: &[Code], taken: &[&str]) {
        let mut named = FxHashSet::default();
        needs_names(code, &mut Vec::new(), &mut named);
        let mut order = Vec::new();
        label_order(code, &mut order);
        let mut next = 0u32;
        for label in order.into_iter().filter(|label| named.contains(label)) {
            let name = loop {
                next += 1;
                let candidate = format!("L{next}");
                if !taken.contains(&candidate.as_str()) {
                    break candidate;
                }
            };
            let name: &'a str = oxc_allocator::GetAllocator::allocator(&*self.cx).alloc_str(&name);
            self.names.insert(label, name);
        }
    }

    fn label_identifier(&self, label: LabelId) -> Option<LabelIdentifier<'a>> {
        let name = *self.names.get(&label)?;
        Some(LabelIdentifier::new(SPAN, name, &*self.cx))
    }

    fn labelled(&mut self, label: LabelId, statement: Statement<'a>) -> Statement<'a> {
        match self.label_identifier(label) {
            Some(name) => Statement::new_labeled_statement(SPAN, name, statement, &*self.cx),
            None => statement,
        }
    }

    fn block(
        &mut self,
        body: AVec<'a, Statement<'a>>,
        flags: ScopeFlags,
    ) -> ABox<'a, BlockStatement<'a>> {
        let scope = self.cx.tcx.create_child_scope(self.scope, flags);
        BlockStatement::boxed_with_scope_id(SPAN, body, scope, &*self.cx)
    }

    fn fill_expression(&mut self, fill: Fill) -> Option<Expression<'a>> {
        match fill {
            Fill::Nothing => None,
            Fill::Yield(index) => {
                let spec = *self.yields.get(index as usize)?;
                self.yield_expression(&spec)
            }
            Fill::Bound(index) => {
                let bound = self.received.get(&index)?.clone();
                Some(bound.create_read_expression(self.cx.tcx))
            }
            Fill::Caught(region) => {
                let bound = self.caught.get(&region)?.clone();
                Some(bound.create_read_expression(self.cx.tcx))
            }
        }
    }

    fn filling(&mut self, fill: Fill) -> Option<Replace<'e, 'a>> {
        let with = self.fill_expression(fill);
        if with.is_none() && fill != Fill::Nothing {
            return self.fail();
        }
        Some(Replace {
            reads: self.reads,
            with,
        })
    }

    fn filled_slot(&mut self, slot: &Slot, fill: Fill) -> Option<Expression<'a>> {
        let Some(mut expression) = take_slot(&mut self.cases, slot, &*self.cx) else {
            return self.fail();
        };
        self.filling(fill)?.visit_expression(&mut expression);
        Some(expression)
    }

    fn yield_expression(&mut self, spec: &YieldSpec) -> Option<Expression<'a>> {
        let value = match spec.value {
            Some(slot) => Some(self.filled_slot(&slot, spec.fill)?),
            None => None,
        };
        Some(Expression::new_yield_expression(
            SPAN,
            spec.delegate,
            value,
            &*self.cx,
        ))
    }

    fn node(&mut self, node: u32, fill: Fill, out: &mut AVec<'a, Statement<'a>>) -> Option<()> {
        let Some(current) = self.machine.node(node) else {
            return self.fail();
        };
        let filled = current.received == Received::Statement;
        let skip_first = matches!(fill, Fill::Caught(region) if self.rebound.contains_key(&region));
        for (at, loc) in current.body.iter().enumerate() {
            let Some(mut statement) = take_statement(&mut self.cases, loc, &*self.cx) else {
                return self.fail();
            };
            if at == 0 && filled && skip_first {
                continue;
            }
            if at == 0 && filled {
                self.filling(fill)?.visit_statement(&mut statement);
            }
            out.push(statement);
        }
        Some(())
    }

    fn jump(&self, label: LabelId, looped: bool) -> Statement<'a> {
        let name = if self.loops.last() == Some(&label) {
            None
        } else {
            self.label_identifier(label)
        };
        if looped {
            Statement::new_continue_statement(SPAN, name, &*self.cx)
        } else {
            Statement::new_break_statement(SPAN, name, &*self.cx)
        }
    }

    fn if_statement(
        &mut self,
        (test, fill, negated): (Slot, Fill, bool),
        then: Vec<Code>,
        otherwise: Vec<Code>,
    ) -> Option<Statement<'a>> {
        let mut test = self.filled_slot(&test, fill)?;
        let mut then = self.sequence(then);
        let mut otherwise = self.sequence(otherwise);
        let mut negate = negated;
        if then.is_empty() && !otherwise.is_empty() {
            std::mem::swap(&mut then, &mut otherwise);
            negate = !negate;
        }
        if negate {
            test = self.negated(test);
        }
        if then.is_empty() {
            return Some(Statement::new_expression_statement(SPAN, test, &*self.cx));
        }
        let consequent = Statement::BlockStatement(self.block(then, ScopeFlags::empty()));
        let alternate = (!otherwise.is_empty())
            .then(|| Statement::BlockStatement(self.block(otherwise, ScopeFlags::empty())));
        Some(Statement::new_if_statement(
            SPAN, test, consequent, alternate, &*self.cx,
        ))
    }

    fn negated(&self, test: Expression<'a>) -> Expression<'a> {
        match test {
            Expression::UnaryExpression(mut unary)
                if unary.operator == UnaryOperator::LogicalNot =>
            {
                unary.argument.take_in(&*self.cx)
            }
            other => {
                Expression::new_unary_expression(SPAN, UnaryOperator::LogicalNot, other, &*self.cx)
            }
        }
    }

    fn while_statement(
        &mut self,
        label: LabelId,
        test: Expression<'a>,
        body: Vec<Code>,
    ) -> Statement<'a> {
        self.loops.push(label);
        let body = self.sequence(body);
        self.loops.pop();
        let body = Statement::BlockStatement(self.block(body, ScopeFlags::empty()));
        let statement = Statement::new_while_statement(SPAN, test, body, &*self.cx);
        self.labelled(label, statement)
    }

    fn try_statement(
        &mut self,
        (label, region): (LabelId, u32),
        block: Vec<Code>,
        handler: Option<Vec<Code>>,
        finalizer: Option<Vec<Code>>,
    ) -> Option<Statement<'a>> {
        let block = self.sequence(block);
        let block = self.block(block, ScopeFlags::empty());
        let handler = match handler {
            Some(code) => Some(self.catch_clause(region, code)?),
            None => None,
        };
        let finalizer = finalizer.map(|code| {
            let body = self.sequence(code);
            self.block(body, ScopeFlags::empty())
        });
        let statement = Statement::new_try_statement(SPAN, block, handler, finalizer, &*self.cx);
        Some(self.labelled(label, statement))
    }

    fn catch_clause(&mut self, region: u32, code: Vec<Code>) -> Option<ABox<'a, CatchClause<'a>>> {
        let try_region = self.machine.trys.get(region as usize)?;
        let entry = self.machine.entry(try_region.catch?)?;
        let received = self.machine.node(entry)?.received != Received::Nowhere;
        let scope = self
            .cx
            .tcx
            .create_child_scope(self.scope, ScopeFlags::CatchClause);
        let param = received.then(|| {
            let bound = match self.rebound.get(&region) {
                Some(bound) => bound.clone(),
                None => {
                    let bound = self
                        .cx
                        .tcx
                        .generate_uid("e", scope, SymbolFlags::CatchVariable);
                    self.caught.insert(region, bound.clone());
                    bound
                }
            };
            let pattern = bound.create_binding_pattern(self.cx.tcx);
            CatchParameter::new(SPAN, pattern, None, &*self.cx)
        });
        let body = self.sequence(code);
        let body = self.block(body, ScopeFlags::empty());
        Some(CatchClause::boxed_with_scope_id(
            SPAN, param, body, scope, &*self.cx,
        ))
    }

    fn bind(&mut self, index: u32) -> Option<Statement<'a>> {
        let spec = *self.yields.get(index as usize)?;
        let value = self.yield_expression(&spec)?;
        let bound = self.cx.tcx.generate_uid(
            "t",
            self.scope,
            SymbolFlags::BlockScopedVariable | SymbolFlags::ConstVariable,
        );
        let id = bound.create_binding_pattern(self.cx.tcx);
        let declarator = VariableDeclarator::new(SPAN, id, None, Some(value), false, &*self.cx);
        self.received.insert(index, bound);
        let declarations = AVec::from_iter_in([declarator], &*self.cx);
        Some(Statement::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            declarations,
            false,
            &*self.cx,
        ))
    }

    fn statement(&mut self, item: Code) -> Option<Statement<'a>> {
        Some(match item {
            Code::Bind(index) => self.bind(index)?,
            Code::Yield(index) => {
                let spec = *self.yields.get(index as usize)?;
                let expression = self.yield_expression(&spec)?;
                Statement::new_expression_statement(SPAN, expression, &*self.cx)
            }
            Code::Return(slot, fill) => {
                let argument = match slot {
                    Some(slot) => Some(self.filled_slot(&slot, fill)?),
                    None => None,
                };
                Statement::new_return_statement(SPAN, argument, &*self.cx)
            }
            Code::If {
                test,
                fill,
                negated,
                then,
                otherwise,
            } => self.if_statement((test, fill, negated), then, otherwise)?,
            Code::Loop(label, body) => {
                let test = Expression::new_boolean_literal(SPAN, true, &*self.cx);
                self.while_statement(label, test, body)
            }
            Code::While(label, test, negate, body) => {
                let mut test = take_slot(&mut self.cases, &test, &*self.cx)?;
                if negate {
                    test = self.negated(test);
                }
                self.while_statement(label, test, body)
            }
            Code::Block(label, body) => {
                let body = self.sequence(body);
                let block = Statement::BlockStatement(self.block(body, ScopeFlags::empty()));
                self.labelled(label, block)
            }
            Code::Break(label) => self.jump(label, false),
            Code::Continue(label) => self.jump(label, true),
            Code::Try {
                label,
                region,
                block,
                handler,
                finalizer,
            } => self.try_statement((label, region), block, handler, finalizer)?,
            Code::Node(..) | Code::Stop => return None,
        })
    }

    fn item(&mut self, item: Code, out: &mut AVec<'a, Statement<'a>>) -> Option<()> {
        match item {
            Code::Node(node, fill) => self.node(node, fill, out)?,
            Code::Stop => {}
            other => out.push(self.statement(other)?),
        }
        Some(())
    }

    pub fn sequence(&mut self, code: Vec<Code>) -> AVec<'a, Statement<'a>> {
        let mut out = AVec::with_capacity_in(code.len(), &*self.cx);
        for item in code {
            if self.item(item, &mut out).is_none() {
                self.failed = true;
                out.clear();
                break;
            }
        }
        out
    }
}
