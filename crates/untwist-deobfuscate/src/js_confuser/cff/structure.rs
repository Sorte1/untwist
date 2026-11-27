use oxc_span::Span;

use super::walk::{Graph, NodeTerm};

pub(super) enum Out {
    Item(Span, usize),
    If(Span, usize, Vec<Out>, Vec<Out>),
    Ret(Span, usize),
}

fn successors(graph: &Graph, node: usize, exit: usize) -> Vec<usize> {
    match &graph.nodes[node].term {
        NodeTerm::Goto(next) => vec![*next],
        NodeTerm::Cond(_, _, a, b) => vec![*a, *b],
        NodeTerm::Ret(..) | NodeTerm::Exit => vec![exit],
    }
}

fn topological(graph: &Graph, exit: usize) -> Option<Vec<usize>> {
    let total = graph.nodes.len() + 1;
    let mut state = vec![0u8; total];
    let mut order = Vec::new();
    let mut stack: Vec<(usize, usize)> = vec![(graph.entry, 0)];
    state[graph.entry] = 1;
    while let Some((node, at)) = stack.pop() {
        let next = if node == exit {
            Vec::new()
        } else {
            successors(graph, node, exit)
        };
        match next.get(at) {
            Some(child) => {
                stack.push((node, at + 1));
                match state[*child] {
                    0 => {
                        state[*child] = 1;
                        stack.push((*child, 0));
                    }
                    1 => return None,
                    _ => {}
                }
            }
            None => {
                state[node] = 2;
                order.push(node);
            }
        }
    }
    Some(order)
}

struct Tree {
    ipdom: Vec<Option<usize>>,
    exit: usize,
}

fn post_dominators(graph: &Graph) -> Option<Tree> {
    let exit = graph.nodes.len();
    let order = topological(graph, exit)?;
    let total = exit + 1;
    let mut sets: Vec<Option<Vec<bool>>> = vec![None; total];
    let mut exit_set = vec![false; total];
    exit_set[exit] = true;
    sets[exit] = Some(exit_set);
    for node in order.iter().copied().filter(|n| *n != exit) {
        let mut set: Option<Vec<bool>> = None;
        for child in successors(graph, node, exit) {
            let child_set = sets[child].as_ref()?;
            set = Some(match set {
                None => child_set.clone(),
                Some(mut acc) => {
                    for (a, b) in acc.iter_mut().zip(child_set) {
                        *a &= *b;
                    }
                    acc
                }
            });
        }
        let mut set = set?;
        set[node] = true;
        sets[node] = Some(set);
    }
    let size = |n: usize| {
        sets[n]
            .as_ref()
            .map_or(0, |s| s.iter().filter(|b| **b).count())
    };
    let ipdom = (0..total)
        .map(|node| {
            let set = sets[node].as_ref()?;
            set.iter()
                .enumerate()
                .filter(|(other, member)| **member && *other != node)
                .map(|(other, _)| other)
                .max_by_key(|other| size(*other))
        })
        .collect();
    Some(Tree { ipdom, exit })
}

struct Emitter<'g> {
    graph: &'g Graph,
    tree: Tree,
    emitted: Vec<bool>,
}

impl Emitter<'_> {
    fn emit(&mut self, start: usize, stop: usize) -> Option<Vec<Out>> {
        let mut out = Vec::new();
        let mut node = start;
        while node != stop && node != self.tree.exit {
            let current = &self.graph.nodes[node];
            let bare_exit = current.items.is_empty() && matches!(current.term, NodeTerm::Exit);
            if !bare_exit && std::mem::replace(&mut self.emitted[node], true) {
                return None;
            }
            out.extend(current.items.iter().map(|(span, v)| Out::Item(*span, *v)));
            match &current.term {
                NodeTerm::Goto(next) => node = *next,
                NodeTerm::Exit => return (stop == self.tree.exit).then_some(out),
                NodeTerm::Ret(span, v) => {
                    out.push(Out::Ret(*span, *v));
                    return (stop == self.tree.exit).then_some(out);
                }
                NodeTerm::Cond(test, v, a, b) => {
                    let (test, v, a, b) = (*test, *v, *a, *b);
                    let join = self.tree.ipdom[node]?;
                    let then = self.emit(a, join)?;
                    let other = self.emit(b, join)?;
                    out.push(Out::If(test, v, then, other));
                    node = join;
                }
            }
        }
        Some(out)
    }
}

pub(super) fn structure(graph: &Graph) -> Option<Vec<Out>> {
    let tree = post_dominators(graph)?;
    let exit = tree.exit;
    let mut emitter = Emitter {
        graph,
        emitted: vec![false; graph.nodes.len()],
        tree,
    };
    emitter.emit(graph.entry, exit)
}
