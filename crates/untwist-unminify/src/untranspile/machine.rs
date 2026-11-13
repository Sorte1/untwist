use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;

use untwist_core::pass::Cx;

const DEPTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Index(u16),
    Then,
    Else,
}

#[derive(Clone, Copy, Debug)]
pub struct Loc {
    pub case: u16,
    len: u8,
    steps: [Step; DEPTH],
}

impl Loc {
    pub fn root(case: u16) -> Self {
        Self {
            case,
            len: 0,
            steps: [Step::Index(0); DEPTH],
        }
    }

    pub fn then(self, step: Step) -> Option<Self> {
        let mut next = self;
        *next.steps.get_mut(usize::from(self.len))? = step;
        next.len += 1;
        Some(next)
    }

    fn steps(&self) -> &[Step] {
        self.steps.get(..usize::from(self.len)).unwrap_or(&[])
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Part {
    Test,
    Element,
    ElementArgument,
    Returned,
    ReturnedArgument(u8),
    Delegated,
}

#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub at: Loc,
    pub part: Part,
}

pub type NodeId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Label(u32),
    Node(NodeId),
}

#[derive(Clone, Copy, Debug)]
pub enum Term {
    Goto(Target),
    Branch {
        test: Slot,
        then: NodeId,
        otherwise: NodeId,
    },
    Return(Option<Slot>),
    Yield {
        value: Option<Slot>,
        delegate: bool,
        resume: u32,
    },
    EndFinally,
    Stop,
    Unreachable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Received {
    Nowhere,
    Statement,
    Test,
}

#[derive(Debug)]
pub struct Node {
    pub label: u32,
    pub body: Vec<Loc>,
    pub term: Term,
    pub received: Received,
    pub bound: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TryRegion {
    pub start: u32,
    pub catch: Option<u32>,
    pub finally: Option<u32>,
    pub end: u32,
}

impl TryRegion {
    pub fn contains(&self, label: u32) -> bool {
        self.start <= label && label < self.end
    }

    pub fn body_end(&self) -> u32 {
        self.catch.or(self.finally).unwrap_or(self.end)
    }

    pub fn catch_end(&self) -> u32 {
        self.finally.unwrap_or(self.end)
    }

    pub fn in_finally(&self, label: u32) -> bool {
        self.finally
            .is_some_and(|finally| finally <= label && label < self.end)
    }

    pub fn within(&self, outer: &TryRegion) -> bool {
        outer
            .parts()
            .iter()
            .flatten()
            .any(|(start, end)| *start <= self.start && self.end <= *end)
    }

    pub fn parts(&self) -> [Option<(u32, u32)>; 3] {
        [
            Some((self.start, self.body_end())),
            self.catch.map(|catch| (catch, self.catch_end())),
            self.finally.map(|finally| (finally, self.end)),
        ]
    }
}

#[derive(Debug, Default)]
pub struct Machine {
    pub nodes: Vec<Node>,
    pub entries: Vec<NodeId>,
    pub trys: Vec<TryRegion>,
}

impl Machine {
    pub fn entry(&self, label: u32) -> Option<NodeId> {
        self.entries.get(label as usize).copied()
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    pub fn target(&self, target: Target) -> Option<NodeId> {
        match target {
            Target::Label(label) => self.entry(label),
            Target::Node(node) => Some(node),
        }
    }

    pub fn successors(&self, id: NodeId) -> Option<[Option<NodeId>; 2]> {
        let node = self.node(id)?;
        Some(match node.term {
            Term::Goto(target) => [Some(self.target(target)?), None],
            Term::Branch {
                then, otherwise, ..
            } => [Some(then), Some(otherwise)],
            Term::Yield { resume, .. } => [Some(self.entry(resume)?), None],
            Term::Return(_) | Term::EndFinally | Term::Stop | Term::Unreachable => [None, None],
        })
    }

    pub fn innermost_try(&self, label: u32) -> Option<usize> {
        self.trys
            .iter()
            .enumerate()
            .filter(|(_, region)| region.contains(label))
            .min_by_key(|(_, region)| region.end - region.start)
            .map(|(index, _)| index)
    }
}

pub fn statement_at<'k, 'a>(cases: &[&'k [Statement<'a>]], loc: &Loc) -> Option<&'k Statement<'a>> {
    let mut list: &'k [Statement<'a>] = cases.get(usize::from(loc.case))?;
    let mut current: Option<&'k Statement<'a>> = None;
    for step in loc.steps() {
        current = Some(match (step, current) {
            (Step::Index(index), _) => list.get(usize::from(*index))?,
            (Step::Then, Some(Statement::IfStatement(it))) => &it.consequent,
            (Step::Else, Some(Statement::IfStatement(it))) => it.alternate.as_ref()?,
            _ => return None,
        });
        if let Some(Statement::BlockStatement(block)) = current {
            list = &block.body;
        }
    }
    current
}

fn statement_at_mut<'k, 'a>(
    cases: &'k mut [AVec<'a, Statement<'a>>],
    loc: &Loc,
) -> Option<&'k mut Statement<'a>> {
    let mut steps = loc.steps().iter();
    let Some(Step::Index(first)) = steps.next() else {
        return None;
    };
    let mut current = cases
        .get_mut(usize::from(loc.case))?
        .get_mut(usize::from(*first))?;
    for step in steps {
        current = match (step, current) {
            (Step::Index(index), Statement::BlockStatement(block)) => {
                block.body.get_mut(usize::from(*index))?
            }
            (Step::Then, Statement::IfStatement(it)) => &mut it.consequent,
            (Step::Else, Statement::IfStatement(it)) => it.alternate.as_mut()?,
            _ => return None,
        };
    }
    Some(current)
}

pub fn take_statement<'a>(
    cases: &mut [AVec<'a, Statement<'a>>],
    loc: &Loc,
    cx: &Cx<'a, '_>,
) -> Option<Statement<'a>> {
    Some(statement_at_mut(cases, loc)?.take_in(cx))
}

fn returned<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    match statement {
        Statement::ReturnStatement(it) => it.argument.as_mut(),
        _ => None,
    }
}

fn element<'k, 'a>(expression: &'k mut Expression<'a>) -> Option<&'k mut Expression<'a>> {
    match expression {
        Expression::ArrayExpression(array) => array.elements.get_mut(1)?.as_expression_mut(),
        _ => None,
    }
}

fn argument<'k, 'a>(
    expression: &'k mut Expression<'a>,
    index: u8,
) -> Option<&'k mut Expression<'a>> {
    match expression {
        Expression::CallExpression(call) => call
            .arguments
            .get_mut(usize::from(index))?
            .as_expression_mut(),
        _ => None,
    }
}

pub fn take_slot<'a>(
    cases: &mut [AVec<'a, Statement<'a>>],
    slot: &Slot,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let statement = statement_at_mut(cases, &slot.at)?;
    let expression = match slot.part {
        Part::Test => match statement {
            Statement::IfStatement(it) => &mut it.test,
            _ => return None,
        },
        Part::Element => element(returned(statement)?)?,
        Part::ElementArgument => argument(element(returned(statement)?)?, 0)?,
        Part::Returned => returned(statement)?,
        Part::ReturnedArgument(index) => argument(returned(statement)?, index)?,
        Part::Delegated => argument(argument(returned(statement)?, 0)?, 0)?,
    };
    Some(expression.take_in(cx))
}

fn returned_ref<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ReturnStatement(it) => it.argument.as_ref(),
        _ => None,
    }
}

fn element_ref<'k, 'a>(expression: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expression {
        Expression::ArrayExpression(array) => array.elements.get(1)?.as_expression(),
        _ => None,
    }
}

fn argument_ref<'k, 'a>(expression: &'k Expression<'a>, index: u8) -> Option<&'k Expression<'a>> {
    match expression {
        Expression::CallExpression(call) => call.arguments.get(usize::from(index))?.as_expression(),
        _ => None,
    }
}

pub fn slot_at<'k, 'a>(cases: &[&'k [Statement<'a>]], slot: &Slot) -> Option<&'k Expression<'a>> {
    let statement = statement_at(cases, &slot.at)?;
    match slot.part {
        Part::Test => match statement {
            Statement::IfStatement(it) => Some(&it.test),
            _ => None,
        },
        Part::Element => element_ref(returned_ref(statement)?),
        Part::ElementArgument => argument_ref(element_ref(returned_ref(statement)?)?, 0),
        Part::Returned => returned_ref(statement),
        Part::ReturnedArgument(index) => argument_ref(returned_ref(statement)?, index),
        Part::Delegated => argument_ref(argument_ref(returned_ref(statement)?, 0)?, 0),
    }
}
