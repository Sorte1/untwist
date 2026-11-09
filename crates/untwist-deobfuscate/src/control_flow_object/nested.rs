use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashMap;
use untwist_core::ast_util::edit::ExprEdits;
use untwist_core::pass::Cx;

use super::{forwards_a_binary_pair, inline_site};

pub(super) struct Recipe<'p> {
    pub function: &'p Function<'p>,
    pub arguments: &'p AVec<'p, Argument<'p>>,
}

pub(super) struct PlannedEdit<'a, 'p> {
    pub span: Span,
    pub expr: Expression<'a>,
    pub recipe: Option<Recipe<'p>>,
}

struct Node<'a, 'p> {
    span: Span,
    recipe: Option<Recipe<'p>>,
    parent: Option<usize>,
    children: Vec<usize>,
    built: Option<Expression<'a>>,
}

struct Ancestry<'n> {
    at: &'n FxHashMap<Span, usize>,
    seen: Vec<bool>,
    parents: Vec<Option<usize>>,
    open: Vec<usize>,
    order: Vec<usize>,
}

impl<'a> VisitJs<'a> for Ancestry<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        let found = self
            .at
            .get(&it.span())
            .copied()
            .filter(|&index| !self.seen[index]);
        let Some(index) = found else {
            walk_js::walk_expression(self, it);
            return;
        };
        self.seen[index] = true;
        self.parents[index] = self.open.last().copied();
        self.open.push(index);
        walk_js::walk_expression(self, it);
        self.open.pop();
        self.order.push(index);
    }
}

fn link_parents(program: &Program<'_>, nodes: &mut [Node<'_, '_>]) -> Vec<usize> {
    if nodes.len() < 2 {
        return Vec::new();
    }
    let mut at: FxHashMap<Span, usize> = FxHashMap::default();
    for (index, node) in nodes.iter().enumerate() {
        at.entry(node.span).or_insert(index);
    }
    let mut ancestry = Ancestry {
        at: &at,
        seen: vec![false; nodes.len()],
        parents: vec![None; nodes.len()],
        open: Vec::new(),
        order: Vec::new(),
    };
    ancestry.visit_program(program);
    let Ancestry { parents, order, .. } = ancestry;
    for (index, parent) in parents.into_iter().enumerate() {
        if let Some(parent) = parent {
            nodes[index].parent = Some(parent);
            nodes[parent].children.push(index);
        }
    }
    order
}

struct Substitute<'a, 'c> {
    pieces: FxHashMap<Span, &'c Expression<'a>>,
    placed: usize,
    alloc: &'a oxc_allocator::Allocator,
}

impl<'a> VisitMut<'a> for Substitute<'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(piece) = self.pieces.remove(&it.span()) {
            *it = piece.clone_in(self.alloc);
            self.placed += 1;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

fn compose<'a>(nodes: &[Node<'a, '_>], index: usize, cx: &Cx<'a, '_>) -> Option<Expression<'a>> {
    let node = nodes.get(index)?;
    let recipe = node.recipe.as_ref()?;
    if !forwards_a_binary_pair(recipe.function) {
        return None;
    }
    let mut pieces: FxHashMap<Span, &Expression<'a>> = FxHashMap::default();
    for &child in &node.children {
        let child = nodes.get(child)?;
        pieces.insert(child.span, child.built.as_ref()?);
    }
    let wanted = node.children.len();
    if pieces.len() != wanted {
        return None;
    }
    let alloc = cx.allocator();
    let mut arguments = recipe.arguments.clone_in(alloc);
    let mut substitute = Substitute {
        pieces,
        placed: 0,
        alloc,
    };
    substitute.visit_arguments(&mut arguments);
    if substitute.placed != wanted {
        return None;
    }
    inline_site(recipe.function, &arguments, cx)
}

fn build_nested<'a>(nodes: &mut [Node<'a, '_>], order: &[usize], cx: &Cx<'a, '_>) {
    for &index in order {
        if nodes[index].children.is_empty() {
            continue;
        }
        nodes[index].built = compose(nodes, index, cx);
    }
}

fn mark_done(nodes: &[Node<'_, '_>], index: usize, done: &mut [bool]) {
    done[index] = true;
    for &child in &nodes[index].children {
        mark_done(nodes, child, done);
    }
}

fn insert_root<'a>(
    nodes: &mut [Node<'a, '_>],
    index: usize,
    edits: &mut ExprEdits<'a>,
    done: &mut [bool],
) {
    let Some(expr) = nodes[index].built.take() else {
        return;
    };
    if edits.insert(nodes[index].span, expr) {
        mark_done(nodes, index, done);
        return;
    }
    let children = nodes[index].children.clone();
    for child in children {
        insert_root(nodes, child, edits, done);
    }
}

fn is_root(nodes: &[Node<'_, '_>], index: usize) -> bool {
    nodes[index]
        .parent
        .is_none_or(|parent| nodes[parent].built.is_none())
}

pub(super) fn insert_nested<'a, 'p>(
    program: &Program<'_>,
    planned: Vec<Vec<PlannedEdit<'a, 'p>>>,
    edits: &mut ExprEdits<'a>,
    cx: &Cx<'a, '_>,
) -> Vec<usize> {
    let plans = planned.len();
    let owners: Vec<usize> = planned
        .iter()
        .enumerate()
        .flat_map(|(plan, list)| std::iter::repeat_n(plan, list.len()))
        .collect();
    let mut nodes: Vec<Node<'a, 'p>> = planned
        .into_iter()
        .flatten()
        .map(|edit| Node {
            span: edit.span,
            recipe: edit.recipe,
            parent: None,
            children: Vec::new(),
            built: Some(edit.expr),
        })
        .collect();
    let order = link_parents(program, &mut nodes);
    build_nested(&mut nodes, &order, cx);
    let roots: Vec<bool> = (0..nodes.len())
        .map(|index| is_root(&nodes, index))
        .collect();
    let mut done = vec![false; nodes.len()];
    for (index, root) in roots.into_iter().enumerate() {
        if root {
            insert_root(&mut nodes, index, edits, &mut done);
        }
    }
    let mut substituted = vec![0usize; plans];
    for (owner, done) in owners.into_iter().zip(done) {
        if done {
            substituted[owner] += 1;
        }
    }
    substituted
}
