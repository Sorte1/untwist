use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeId;

use super::machine::{
    slot_at, statement_at, Loc, Machine, Node, NodeId, Part, Received, Slot, Step, Target, Term,
    TryRegion,
};
use super::machine_check::{order, statement_order, Order, Plain, Reads, Uses};

#[derive(Clone, Copy, Debug)]
pub enum Read {
    Term(Term),
    LabelSet(u32),
    Try(TryRegion),
    Skip,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct Parsed {
    pub read: Read,
    pub consumed: usize,
    pub uses: usize,
}

enum Flow {
    Next(usize),
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cont {
    Fall(u32),
    Node(NodeId),
}

pub trait Machinery<'a> {
    fn reads(&self) -> &dyn Reads;

    fn scoping(&self) -> &Scoping;

    fn home(&self) -> Option<ScopeId>;

    fn falls_through(&self) -> bool;

    fn needs_received(&self) -> bool;

    fn statement<'k>(
        &self,
        statement: &'k Statement<'a>,
        next: Option<&'k Statement<'a>>,
        loc: Loc,
        next_loc: Option<Loc>,
        label: u32,
    ) -> Option<Parsed>;
}

pub struct Out<'a> {
    pub machine: Machine,
    pub uses: usize,
    pub labels: Vec<&'a str>,
}

#[derive(Clone, Copy)]
enum Base {
    Many(Loc),
    One(Loc),
}

impl Base {
    fn item(self, index: usize) -> Option<Loc> {
        match self {
            Base::Many(loc) => loc.then(Step::Index(u16::try_from(index).ok()?)),
            Base::One(loc) => (index == 0).then_some(loc),
        }
    }
}

struct Builder<'m, 'k, 'a, M> {
    cases: &'m [&'k [Statement<'a>]],
    machinery: &'m M,
    machine: Machine,
    uses: usize,
    plain: Plain<'a>,
}

fn is_empty(statement: &Statement<'_>) -> bool {
    matches!(statement, Statement::EmptyStatement(_))
}

fn arm<'k, 'a>(statement: &'k Statement<'a>, loc: Loc) -> (Base, &'k [Statement<'a>]) {
    match statement {
        Statement::BlockStatement(block) => (Base::Many(loc), block.body.as_slice()),
        other => (Base::One(loc), std::slice::from_ref(other)),
    }
}

struct Steady<'r, 'k, 'a> {
    reads: &'r dyn Reads,
    scoping: &'r Scoping,
    home: Option<ScopeId>,
    cases: &'r [&'k [Statement<'a>]],
    machine: &'r Machine,
    label: u32,
}

impl Steady<'_, '_, '_> {
    fn local(&self, name: &IdentifierReference<'_>) -> Option<()> {
        let symbol = self
            .scoping
            .get_reference(name.reference_id.get()?)
            .symbol_id()?;
        (Some(self.scoping.symbol_scope_id(symbol)) == self.home).then_some(())?;
        let mut uses = Uses::new(self.scoping, symbol);
        for statement in self.cases.iter().flat_map(|case| case.iter()) {
            uses.visit_statement(statement);
        }
        let total = self.scoping.get_resolved_reference_ids(symbol).len();
        (uses.inside == total && !uses.closed_over).then_some(())?;
        let mut operands = Uses::new(self.scoping, symbol);
        for node in &self.machine.nodes {
            if let Term::Yield {
                value: Some(slot),
                resume,
                ..
            } = node.term
            {
                if resume == self.label {
                    operands.visit_expression(slot_at(self.cases, &slot)?);
                }
            }
        }
        (operands.inside == 0).then_some(())
    }
}

impl Reads for Steady<'_, '_, '_> {
    fn is_read(&self, expression: &Expression<'_>) -> bool {
        self.reads.is_read(expression)
    }

    fn steady(&self, name: Option<&IdentifierReference<'_>>) -> bool {
        name.and_then(|name| self.local(name)).is_some()
    }
}

fn resumed(machine: &Machine, label: u32) -> bool {
    machine
        .nodes
        .iter()
        .any(|node| matches!(node.term, Term::Yield { resume, .. } if resume == label))
}

struct List<'k, 'a> {
    label: u32,
    base: Base,
    statements: &'k [Statement<'a>],
    top: bool,
}

impl<'k, 'a, M: Machinery<'a>> Builder<'_, 'k, 'a, M> {
    fn node(&mut self, label: u32) -> Option<NodeId> {
        let id = NodeId::try_from(self.machine.nodes.len()).ok()?;
        self.machine.nodes.push(Node {
            label,
            body: Vec::new(),
            term: Term::Stop,
            received: Received::Nowhere,
            bound: false,
        });
        Some(id)
    }

    fn set_term(&mut self, id: NodeId, term: Term) -> Option<()> {
        self.machine.nodes.get_mut(id as usize)?.term = term;
        Some(())
    }

    fn rest_empty(list: &List<'k, 'a>, from: usize) -> bool {
        list.statements
            .get(from..)
            .is_none_or(|rest| rest.iter().all(is_empty))
    }

    fn controls(&self, statement: &'k Statement<'a>) -> bool {
        let mut plain = Plain::default();
        plain.check(statement);
        plain.bad
    }

    fn lexical_free(&self, statement: &Statement<'_>) -> bool {
        match statement {
            Statement::BlockStatement(block) => block
                .scope_id
                .get()
                .is_some_and(|scope| self.machinery.scoping().get_bindings(scope).is_empty()),
            _ => true,
        }
    }

    fn split(
        &mut self,
        id: NodeId,
        list: &List<'k, 'a>,
        (at, loc): (usize, Loc),
        cont: Cont,
    ) -> Option<()> {
        let Statement::IfStatement(it) = list.statements.get(at)? else {
            return None;
        };
        let lexical_free = self.lexical_free(&it.consequent)
            && it
                .alternate
                .as_ref()
                .is_none_or(|alternate| self.lexical_free(alternate));
        if !lexical_free {
            return None;
        }
        self.plain.check_expression(&it.test);
        let rest = self.list(list, at + 1, cont)?;
        let (base, statements) = arm(&it.consequent, loc.then(Step::Then)?);
        let then = self.arm_list(list.label, base, statements, rest)?;
        let otherwise = match &it.alternate {
            Some(alternate) => {
                let (base, statements) = arm(alternate, loc.then(Step::Else)?);
                self.arm_list(list.label, base, statements, rest)?
            }
            None => rest,
        };
        let test = Slot {
            at: loc,
            part: Part::Test,
        };
        self.set_term(
            id,
            Term::Branch {
                test,
                then,
                otherwise,
            },
        )
    }

    fn arm_list(
        &mut self,
        label: u32,
        base: Base,
        statements: &'k [Statement<'a>],
        rest: NodeId,
    ) -> Option<NodeId> {
        let list = List {
            label,
            base,
            statements,
            top: false,
        };
        self.list(&list, 0, Cont::Node(rest))
    }

    fn check_term(&mut self, term: &Term) -> Option<()> {
        let slot = match term {
            Term::Return(slot) => *slot,
            Term::Yield { value, .. } => *value,
            _ => None,
        };
        if let Some(slot) = slot {
            self.plain.check_expression(slot_at(self.cases, &slot)?);
        }
        Some(())
    }

    fn step(
        &mut self,
        (id, list): (NodeId, &List<'k, 'a>),
        (at, first): (usize, bool),
        cont: Cont,
    ) -> Option<Flow> {
        let statement = list.statements.get(at)?;
        let loc = list.base.item(at)?;
        let next = list.statements.get(at + 1);
        let next_loc = next.and_then(|_| list.base.item(at + 1));
        let parsed = self
            .machinery
            .statement(statement, next, loc, next_loc, list.label)?;
        let used = parsed.consumed.max(1);
        self.uses += parsed.uses;
        match parsed.read {
            Read::Term(term) => {
                self.check_term(&term)?;
                Self::rest_empty(list, at + used).then_some(())?;
                self.set_term(id, term)?;
                Some(Flow::Done)
            }
            Read::LabelSet(target) => {
                let fits = list.top
                    && target == list.label + 1
                    && cont == Cont::Fall(target)
                    && Self::rest_empty(list, at + used);
                fits.then_some(())?;
                self.set_term(id, Term::Goto(Target::Label(target)))?;
                Some(Flow::Done)
            }
            Read::Try(region) => {
                (list.top && first && region.start == list.label).then_some(())?;
                self.machine.trys.push(region);
                Some(Flow::Next(used))
            }
            Read::Skip => {
                (list.top && first).then_some(())?;
                Some(Flow::Next(used))
            }
            Read::Other => self.other(id, list, (at, loc), cont),
        }
    }

    fn other(
        &mut self,
        id: NodeId,
        list: &List<'k, 'a>,
        (at, loc): (usize, Loc),
        cont: Cont,
    ) -> Option<Flow> {
        let statement = list.statements.get(at)?;
        if matches!(statement, Statement::IfStatement(_)) && self.controls(statement) {
            self.split(id, list, (at, loc), cont)?;
            return Some(Flow::Done);
        }
        self.plain.check(statement);
        if self.plain.bad {
            return None;
        }
        self.machine.nodes.get_mut(id as usize)?.body.push(loc);
        if matches!(statement, Statement::ThrowStatement(_)) {
            return Self::rest_empty(list, at + 1).then_some(Flow::Done);
        }
        Some(Flow::Next(1))
    }

    fn end_term(&self, list: &List<'k, 'a>, from: usize, cont: Cont) -> Option<Term> {
        match cont {
            Cont::Fall(next) if self.machinery.falls_through() || !list.top => {
                Some(Term::Goto(Target::Label(next)))
            }
            Cont::Fall(_) if from > 0 && Self::rest_empty(list, from) => Some(Term::Unreachable),
            Cont::Fall(_) => None,
            Cont::Node(node) => Some(Term::Goto(Target::Node(node))),
        }
    }

    fn list(&mut self, list: &List<'k, 'a>, from: usize, cont: Cont) -> Option<NodeId> {
        let id = self.node(list.label)?;
        let mut at = from;
        let mut first = from == 0;
        while let Some(statement) = list.statements.get(at) {
            if is_empty(statement) {
                at += 1;
                continue;
            }
            match self.step((id, list), (at, first), cont)? {
                Flow::Done => return Some(id),
                Flow::Next(used) => at += used,
            }
            first = false;
        }
        let term = self.end_term(list, from, cont)?;
        self.set_term(id, term).map(|()| id)
    }

    fn received(&self, id: NodeId) -> Option<(Received, bool)> {
        let node = self.machine.node(id)?;
        let reads = self.machinery.reads();
        let (received, found) = self.received_with(node, reads)?;
        let resumed =
            !is_catch_entry(&self.machine, node.label) || resumed(&self.machine, node.label);
        if received == Received::Nowhere || !resumed {
            return Some((received, false));
        }
        let steady = Steady {
            reads,
            scoping: self.machinery.scoping(),
            home: self.machinery.home(),
            cases: self.cases,
            machine: &self.machine,
            label: node.label,
        };
        let (_, steady_found) = self.received_with(node, &steady)?;
        Some((received, found && !steady_found))
    }

    fn received_with(&self, node: &Node, reads: &dyn Reads) -> Option<(Received, bool)> {
        if let Some(first) = node.body.first() {
            let statement = statement_at(self.cases, first)?;
            return Some(match statement_order(statement, reads) {
                Order::Found => (Received::Statement, true),
                _ => (Received::Nowhere, false),
            });
        }
        let slot = match node.term {
            Term::Branch { test, .. } => Some(test),
            Term::Return(slot) => slot,
            Term::Yield { value, .. } => value,
            _ => None,
        };
        let found = slot
            .and_then(|slot| slot_at(self.cases, &slot))
            .is_some_and(|expression| order(expression, reads) == Order::Found);
        Some(if found {
            (Received::Test, true)
        } else {
            (Received::Nowhere, false)
        })
    }
}

fn region_order(region: &TryRegion, cases: u32) -> bool {
    let mut points = vec![region.start];
    points.extend(region.catch);
    points.extend(region.finally);
    points.push(region.end);
    (region.catch.is_some() || region.finally.is_some())
        && points.windows(2).all(|pair| pair[0] < pair[1])
        && region.end < cases
}

fn regions_nest(trys: &[TryRegion], cases: u32) -> bool {
    trys.iter().all(|region| region_order(region, cases))
        && trys.iter().enumerate().all(|(index, a)| {
            trys.iter().skip(index + 1).all(|b| {
                let disjoint = a.end <= b.start || b.end <= a.start;
                disjoint || a.within(b) || b.within(a)
            })
        })
}

fn is_catch_entry(machine: &Machine, label: u32) -> bool {
    machine
        .trys
        .iter()
        .any(|region| region.catch == Some(label))
}

fn validate(machine: &Machine, needs_received: bool, cases: u32) -> bool {
    if !regions_nest(&machine.trys, cases) {
        return false;
    }
    let resumes: Vec<u32> = machine
        .nodes
        .iter()
        .filter_map(|node| match node.term {
            Term::Yield { resume, .. } => Some(resume),
            _ => None,
        })
        .collect();
    let labels_ok = machine.nodes.iter().all(|node| match node.term {
        Term::Goto(Target::Label(label)) => label < cases,
        Term::Yield { resume, .. } => {
            resume < cases
                && machine
                    .entry(resume)
                    .and_then(|id| machine.node(id))
                    .is_some_and(|entry| !needs_received || entry.received != Received::Nowhere)
        }
        Term::EndFinally => machine
            .innermost_try(node.label)
            .and_then(|index| machine.trys.get(index))
            .is_some_and(|region| region.in_finally(node.label)),
        _ => true,
    });
    let received_ok = machine.entries.iter().enumerate().all(|(label, id)| {
        let label = label as u32;
        machine.node(*id).is_some_and(|node| {
            node.received == Received::Nowhere
                || resumes.contains(&label)
                || is_catch_entry(machine, label)
        })
    });
    labels_ok && received_ok
}

pub fn read<'k, 'a, M: Machinery<'a>>(
    cases: &[&'k [Statement<'a>]],
    machinery: &M,
    uses: usize,
) -> Option<Out<'a>> {
    let mut builder = Builder {
        cases,
        machinery,
        machine: Machine::default(),
        uses,
        plain: Plain::default(),
    };
    let count = u32::try_from(cases.len()).ok()?;
    for (index, statements) in cases.iter().enumerate() {
        let label = u32::try_from(index).ok()?;
        let list = List {
            label,
            base: Base::Many(Loc::root(u16::try_from(index).ok()?)),
            statements,
            top: true,
        };
        let entry = builder.list(&list, 0, Cont::Fall(label + 1))?;
        let (received, bound) = builder.received(entry)?;
        if received != Received::Nowhere {
            builder.uses += 1;
        }
        let node = builder.machine.nodes.get_mut(entry as usize)?;
        node.received = received;
        node.bound = bound;
        builder.machine.entries.push(entry);
    }
    if builder.plain.bad || !validate(&builder.machine, machinery.needs_received(), count) {
        return None;
    }
    Some(Out {
        machine: builder.machine,
        uses: builder.uses,
        labels: builder.plain.seen_labels,
    })
}
