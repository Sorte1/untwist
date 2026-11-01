use rustc_hash::FxHashMap;

use super::machine::{Machine, NodeId, Received, Slot, Term, TryRegion};

pub type LabelId = u32;

const NODE_CAP: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Nothing,
    Yield(u32),
    Bound(u32),
    Caught(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct YieldSpec {
    pub value: Option<Slot>,
    pub delegate: bool,
    pub fill: Fill,
}

#[derive(Debug)]
pub enum Code {
    Node(NodeId, Fill),
    Yield(u32),
    Bind(u32),
    Return(Option<Slot>, Fill),
    If {
        test: Slot,
        fill: Fill,
        negated: bool,
        then: Vec<Code>,
        otherwise: Vec<Code>,
    },
    Loop(LabelId, Vec<Code>),
    While(LabelId, Slot, bool, Vec<Code>),
    Block(LabelId, Vec<Code>),
    Break(LabelId),
    Continue(LabelId),
    Try {
        label: LabelId,
        region: u32,
        block: Vec<Code>,
        handler: Option<Vec<Code>>,
        finalizer: Option<Vec<Code>>,
    },
    Stop,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum R {
    Node(NodeId),
    Try(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Key {
    Node(NodeId),
    FinallyEnd,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Frame {
    Loop(Key, LabelId),
    Follow(Key, LabelId),
}

enum Mapped {
    Inside(R),
    Exit,
}

struct Graph {
    order: Vec<R>,
    index: FxHashMap<R, usize>,
    succ: Vec<Vec<usize>>,
    idom: Vec<usize>,
    header: Vec<bool>,
    merge: Vec<bool>,
    children: Vec<Vec<usize>>,
}

pub struct Structured {
    pub code: Vec<Code>,
    pub yields: Vec<YieldSpec>,
}

struct Structurer<'m> {
    machine: &'m Machine,
    ctx: Vec<Frame>,
    yields: Vec<YieldSpec>,
    labels: LabelId,
}

fn rank(order: &[usize], node: usize) -> usize {
    order.get(node).copied().unwrap_or(usize::MAX)
}

fn intersect(idom: &[usize], rpo: &[usize], mut a: usize, mut b: usize) -> Option<usize> {
    while a != b {
        while rank(rpo, a) > rank(rpo, b) {
            a = *idom.get(a)?;
        }
        while rank(rpo, b) > rank(rpo, a) {
            b = *idom.get(b)?;
        }
    }
    Some(a)
}

fn dominators(succ: &[Vec<usize>]) -> Option<Vec<usize>> {
    let count = succ.len();
    let rpo: Vec<usize> = (0..count).collect();
    let mut preds = vec![Vec::new(); count];
    for (from, targets) in succ.iter().enumerate() {
        for target in targets {
            preds.get_mut(*target)?.push(from);
        }
    }
    let mut idom = vec![usize::MAX; count];
    *idom.first_mut()? = 0;
    let mut changed = true;
    while changed {
        changed = false;
        for node in 1..count {
            let mut chosen: Option<usize> = None;
            for pred in preds.get(node)? {
                if *idom.get(*pred)? == usize::MAX {
                    continue;
                }
                chosen = Some(match chosen {
                    None => *pred,
                    Some(current) => intersect(&idom, &rpo, *pred, current)?,
                });
            }
            let chosen = chosen?;
            if *idom.get(node)? != chosen {
                *idom.get_mut(node)? = chosen;
                changed = true;
            }
        }
    }
    Some(idom)
}

fn dominates(idom: &[usize], dominator: usize, mut node: usize) -> bool {
    loop {
        if node == dominator {
            return true;
        }
        match idom.get(node) {
            Some(&parent) if parent != node => node = parent,
            _ => return false,
        }
    }
}

impl Graph {
    fn rpo(&self, r: R) -> Option<usize> {
        self.index.get(&r).copied()
    }

    fn in_loop(&self, header: usize, node: usize) -> bool {
        let mut seen = vec![false; self.order.len()];
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            for &next in self.succ.get(current).map_or(&[][..], Vec::as_slice) {
                if next == header {
                    return true;
                }
                let fresh = seen.get(next).is_some_and(|seen| !seen);
                if fresh && dominates(&self.idom, header, next) {
                    if let Some(mark) = seen.get_mut(next) {
                        *mark = true;
                    }
                    stack.push(next);
                }
            }
        }
        false
    }
}

impl<'m> Structurer<'m> {
    fn label(&mut self) -> LabelId {
        self.labels += 1;
        self.labels
    }

    fn region_of(&self, label: u32, range: (u32, u32)) -> Option<Option<u32>> {
        let mut best: Option<(u32, &TryRegion)> = None;
        for (index, region) in self.machine.trys.iter().enumerate() {
            let inside = range.0 <= region.start && region.end <= range.1;
            if !inside || !region.contains(label) {
                continue;
            }
            let wider =
                best.is_none_or(|(_, other)| region.end - region.start > other.end - other.start);
            if wider {
                best = Some((u32::try_from(index).ok()?, region));
            }
        }
        Some(best.map(|(index, _)| index))
    }

    fn map(&self, node: NodeId, range: (u32, u32)) -> Option<Mapped> {
        let label = self.machine.node(node)?.label;
        if label < range.0 || label >= range.1 {
            return Some(Mapped::Exit);
        }
        match self.region_of(label, range)? {
            None => Some(Mapped::Inside(R::Node(node))),
            Some(index) => {
                let region = self.machine.trys.get(index as usize)?;
                (self.machine.entry(region.start)? == node).then_some(Mapped::Inside(R::Try(index)))
            }
        }
    }

    fn escapes(&self, index: u32) -> Option<Vec<NodeId>> {
        let region = *self.machine.trys.get(index as usize)?;
        let end = self.machine.entry(region.end)?;
        let mut out = vec![end];
        for (id, node) in self.machine.nodes.iter().enumerate() {
            if !region.contains(node.label) {
                continue;
            }
            for target in self
                .machine
                .successors(u32::try_from(id).ok()?)?
                .into_iter()
                .flatten()
            {
                let label = self.machine.node(target)?.label;
                if !region.contains(label) && !out.contains(&target) {
                    out.push(target);
                }
            }
        }
        Some(out)
    }

    fn raw_successors(&self, r: R) -> Option<Vec<NodeId>> {
        match r {
            R::Node(node) => Some(
                self.machine
                    .successors(node)?
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
            R::Try(index) => self.escapes(index),
        }
    }

    fn label_of(&self, r: R) -> u32 {
        match r {
            R::Node(node) => self.machine.node(node).map_or(0, |node| node.label),
            R::Try(index) => self
                .machine
                .trys
                .get(index as usize)
                .map_or(0, |it| it.start),
        }
    }

    fn inside(&self, targets: &[NodeId], range: (u32, u32)) -> Option<Vec<R>> {
        let mut out = Vec::new();
        for target in targets {
            if let Mapped::Inside(r) = self.map(*target, range)? {
                out.push(r);
            }
        }
        Some(out)
    }

    fn postorder(&self, entry: R, range: (u32, u32)) -> Option<Vec<R>> {
        let mut seen: FxHashMap<R, ()> = FxHashMap::default();
        let mut post = Vec::new();
        let mut stack: Vec<(R, Vec<R>)> = Vec::new();
        let mut first = self.inside(&self.raw_successors(entry)?, range)?;
        first.sort_by_key(|r| std::cmp::Reverse(self.label_of(*r)));
        seen.insert(entry, ());
        stack.push((entry, first));
        while let Some((top, pending)) = stack.last_mut() {
            match pending.pop() {
                Some(next) if !seen.contains_key(&next) => {
                    seen.insert(next, ());
                    let mut succ = self.inside(&self.raw_successors(next)?, range)?;
                    succ.sort_by_key(|r| std::cmp::Reverse(self.label_of(*r)));
                    stack.push((next, succ));
                }
                Some(_) => {}
                None => {
                    post.push(*top);
                    stack.pop();
                }
            }
            if post.len() + stack.len() > NODE_CAP {
                return None;
            }
        }
        Some(post)
    }

    #[allow(clippy::type_complexity)]
    fn edges(
        &self,
        order: &[R],
        index: &FxHashMap<R, usize>,
        range: (u32, u32),
    ) -> Option<(Vec<Vec<usize>>, Vec<bool>)> {
        let mut succ = Vec::with_capacity(order.len());
        let mut forced = vec![false; order.len()];
        for r in order {
            let targets = self.inside(&self.raw_successors(*r)?, range)?;
            let normal = self.normal_end(*r, range)?;
            let mut list = Vec::with_capacity(targets.len());
            for target in &targets {
                let position = *index.get(target)?;
                if matches!(r, R::Try(_)) && Some(*target) != normal {
                    *forced.get_mut(position)? = true;
                }
                list.push(position);
            }
            succ.push(list);
        }
        Some((succ, forced))
    }

    fn graph(&self, entry: R, range: (u32, u32)) -> Option<Graph> {
        let mut order = self.postorder(entry, range)?;
        order.reverse();
        let index: FxHashMap<R, usize> = order.iter().enumerate().map(|(at, r)| (*r, at)).collect();
        let (succ, forced) = self.edges(&order, &index, range)?;
        let idom = dominators(&succ)?;
        let count = order.len();
        let mut header = vec![false; count];
        let mut forward = vec![0u32; count];
        for (from, targets) in succ.iter().enumerate() {
            for &target in targets {
                if target > from {
                    *forward.get_mut(target)? += 1;
                } else if dominates(&idom, target, from) {
                    *header.get_mut(target)? = true;
                } else {
                    return None;
                }
            }
        }
        let merge = forward
            .iter()
            .zip(&forced)
            .map(|(count, forced)| *count >= 2 || *forced)
            .collect();
        let mut children = vec![Vec::new(); count];
        for node in 1..count {
            children.get_mut(*idom.get(node)?)?.push(node);
        }
        Some(Graph {
            order,
            index,
            succ,
            idom,
            header,
            merge,
            children,
        })
    }

    fn normal_end(&self, r: R, range: (u32, u32)) -> Option<Option<R>> {
        let R::Try(index) = r else {
            return Some(None);
        };
        let region = self.machine.trys.get(index as usize)?;
        Some(match self.map(self.machine.entry(region.end)?, range)? {
            Mapped::Inside(end) => Some(end),
            Mapped::Exit => None,
        })
    }

    fn find(&self, key: Key, looped: bool) -> Option<Code> {
        self.ctx.iter().rev().find_map(|frame| match *frame {
            Frame::Loop(found, label) if found == key && looped => Some(Code::Continue(label)),
            Frame::Follow(found, label) if found == key && !looped => Some(Code::Break(label)),
            _ => None,
        })
    }

    fn key(&self, r: R) -> Option<Key> {
        match r {
            R::Node(node) => Some(Key::Node(node)),
            R::Try(index) => {
                let region = self.machine.trys.get(index as usize)?;
                Some(Key::Node(self.machine.entry(region.start)?))
            }
        }
    }

    fn branch(
        &mut self,
        graph: &Graph,
        (from, target): (usize, NodeId),
        range: (u32, u32),
        fill: Fill,
    ) -> Option<Vec<Code>> {
        let r = match self.map(target, range)? {
            Mapped::Exit => {
                if fill != Fill::Nothing {
                    return None;
                }
                let code = self
                    .find(Key::Node(target), true)
                    .or_else(|| self.find(Key::Node(target), false))?;
                return Some(vec![code]);
            }
            Mapped::Inside(r) => r,
        };
        let at = graph.rpo(r)?;
        let key = self.key(r)?;
        if at <= from {
            return (fill == Fill::Nothing).then(|| self.find(key, true).map(|code| vec![code]))?;
        }
        if *graph.merge.get(at)? {
            return (fill == Fill::Nothing)
                .then(|| self.find(key, false).map(|code| vec![code]))?;
        }
        if fill != Fill::Nothing && *graph.header.get(at)? {
            return None;
        }
        self.tree(graph, at, range, fill)
    }

    fn tree(
        &mut self,
        graph: &Graph,
        at: usize,
        range: (u32, u32),
        fill: Fill,
    ) -> Option<Vec<Code>> {
        let mut merges: Vec<usize> = graph
            .children
            .get(at)?
            .iter()
            .copied()
            .filter(|child| graph.merge.get(*child).copied().unwrap_or(false))
            .collect();
        merges.sort_unstable_by(|a, b| b.cmp(a));
        let r = *graph.order.get(at)?;
        if !*graph.header.get(at)? {
            return self.within(graph, at, &merges, range, fill);
        }
        let exits: Vec<usize> = merges
            .iter()
            .copied()
            .filter(|merge| !graph.in_loop(at, *merge))
            .collect();
        let label = self.label();
        self.ctx.push(Frame::Loop(self.key(r)?, label));
        let follow = match exits.as_slice() {
            [exit] => {
                self.ctx
                    .push(Frame::Follow(self.key(*graph.order.get(*exit)?)?, label));
                merges.retain(|merge| merge != exit);
                Some(*exit)
            }
            _ => None,
        };
        let body = self.within(graph, at, &merges, range, fill);
        if follow.is_some() {
            self.ctx.pop();
        }
        self.ctx.pop();
        let mut code = vec![Code::Loop(label, body?)];
        if let Some(exit) = follow {
            code.extend(self.tree(graph, exit, range, Fill::Nothing)?);
        }
        Some(code)
    }

    fn within(
        &mut self,
        graph: &Graph,
        at: usize,
        merges: &[usize],
        range: (u32, u32),
        fill: Fill,
    ) -> Option<Vec<Code>> {
        let Some((&first, rest)) = merges.split_first() else {
            return self.emit(graph, at, range, fill);
        };
        let label = self.label();
        let key = self.key(*graph.order.get(first)?)?;
        self.ctx.push(Frame::Follow(key, label));
        let inner = self.within(graph, at, rest, range, fill);
        self.ctx.pop();
        let mut code = vec![Code::Block(label, inner?)];
        code.extend(self.tree(graph, first, range, Fill::Nothing)?);
        Some(code)
    }

    fn emit(
        &mut self,
        graph: &Graph,
        at: usize,
        range: (u32, u32),
        fill: Fill,
    ) -> Option<Vec<Code>> {
        match *graph.order.get(at)? {
            R::Node(node) => self.emit_node(graph, (at, node), range, fill),
            R::Try(index) => {
                if fill != Fill::Nothing {
                    return None;
                }
                let region = *self.machine.trys.get(index as usize)?;
                let mut code = vec![self.try_code(index, &region)?];
                code.extend(self.branch(
                    graph,
                    (at, self.machine.entry(region.end)?),
                    range,
                    Fill::Nothing,
                )?);
                Some(code)
            }
        }
    }

    fn emit_yield(
        &mut self,
        graph: &Graph,
        (at, spec): (usize, YieldSpec),
        resume: u32,
        range: (u32, u32),
    ) -> Option<Vec<Code>> {
        let index = u32::try_from(self.yields.len()).ok()?;
        self.yields.push(spec);
        let next = self.machine.entry(resume)?;
        let entry = self.machine.node(next)?;
        if entry.received == Received::Nowhere {
            let mut code = vec![Code::Yield(index)];
            code.extend(self.branch(graph, (at, next), range, Fill::Nothing)?);
            return Some(code);
        }
        if entry.bound {
            let mut code = vec![Code::Bind(index)];
            code.extend(self.branch(graph, (at, next), range, Fill::Bound(index))?);
            return Some(code);
        }
        self.branch(graph, (at, next), range, Fill::Yield(index))
    }

    fn emit_term(
        &mut self,
        graph: &Graph,
        (at, term): (usize, Term),
        range: (u32, u32),
        fill: Fill,
    ) -> Option<Vec<Code>> {
        Some(match term {
            Term::Goto(target) => {
                let target = self.machine.target(target)?;
                self.branch(graph, (at, target), range, Fill::Nothing)?
            }
            Term::Branch {
                test,
                then,
                otherwise,
            } => {
                let then = self.branch(graph, (at, then), range, Fill::Nothing)?;
                let otherwise = self.branch(graph, (at, otherwise), range, Fill::Nothing)?;
                vec![Code::If {
                    test,
                    fill,
                    negated: false,
                    then,
                    otherwise,
                }]
            }
            Term::Return(slot) => vec![Code::Return(slot, fill)],
            Term::Yield {
                value,
                delegate,
                resume,
            } => {
                let spec = YieldSpec {
                    value,
                    delegate,
                    fill,
                };
                self.emit_yield(graph, (at, spec), resume, range)?
            }
            Term::EndFinally => vec![self.find(Key::FinallyEnd, false)?],
            Term::Stop => vec![Code::Stop],
            Term::Unreachable => return None,
        })
    }

    fn emit_node(
        &mut self,
        graph: &Graph,
        (at, node): (usize, NodeId),
        range: (u32, u32),
        fill: Fill,
    ) -> Option<Vec<Code>> {
        let current = self.machine.node(node)?;
        let (body_fill, term_fill) = match current.received {
            Received::Statement => (fill, Fill::Nothing),
            Received::Test => (Fill::Nothing, fill),
            Received::Nowhere => (Fill::Nothing, Fill::Nothing),
        };
        let mut code = Vec::new();
        if let (Received::Nowhere, Fill::Yield(spec)) = (current.received, fill) {
            code.push(Code::Yield(spec));
        }
        code.push(Code::Node(node, body_fill));
        code.extend(self.emit_term(graph, (at, current.term), range, term_fill)?);
        Some(code)
    }

    fn try_code(&mut self, index: u32, region: &TryRegion) -> Option<Code> {
        let label = self.label();
        let end = self.machine.entry(region.end)?;
        self.ctx.push(Frame::Follow(Key::Node(end), label));
        let parts = self.try_parts(index, region);
        self.ctx.pop();
        let (block, handler, finalizer) = parts?;
        Some(Code::Try {
            label,
            region: index,
            block,
            handler,
            finalizer,
        })
    }

    #[allow(clippy::type_complexity)]
    fn try_parts(
        &mut self,
        index: u32,
        region: &TryRegion,
    ) -> Option<(Vec<Code>, Option<Vec<Code>>, Option<Vec<Code>>)> {
        let block = self.region(
            region.start,
            (region.start, region.body_end()),
            Fill::Nothing,
        )?;
        let handler = match region.catch {
            Some(catch) => {
                Some(self.region(catch, (catch, region.catch_end()), Fill::Caught(index))?)
            }
            None => None,
        };
        let finalizer = match region.finally {
            Some(finally) => {
                let label = self.label();
                self.ctx.push(Frame::Follow(Key::FinallyEnd, label));
                let code = self.region(finally, (finally, region.end), Fill::Nothing);
                self.ctx.pop();
                Some(vec![Code::Block(label, code?)])
            }
            None => None,
        };
        Some((block, handler, finalizer))
    }

    fn region(&mut self, start: u32, range: (u32, u32), fill: Fill) -> Option<Vec<Code>> {
        let entry = self.machine.entry(start)?;
        let Mapped::Inside(r) = self.map(entry, range)? else {
            return None;
        };
        let graph = self.graph(r, range)?;
        if *graph.header.first()? && fill != Fill::Nothing {
            return None;
        }
        self.tree(&graph, 0, range, fill)
    }
}

pub fn structure(machine: &Machine) -> Option<Structured> {
    if machine.nodes.len() > NODE_CAP {
        return None;
    }
    let mut structurer = Structurer {
        machine,
        ctx: Vec::new(),
        yields: Vec::new(),
        labels: 0,
    };
    let count = u32::try_from(machine.entries.len()).ok()?;
    let code = structurer.region(0, (0, count), Fill::Nothing)?;
    Some(Structured {
        code,
        yields: structurer.yields,
    })
}
