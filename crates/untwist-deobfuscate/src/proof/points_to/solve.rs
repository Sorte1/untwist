use rustc_hash::FxHashMap;

use super::object::ObjectId;

pub(super) const WORLD: ObjectId = ObjectId::WORLD;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Field(pub(super) u32);

impl Field {
    pub(super) const UNKNOWN: Field = Field(u32::MAX);
}

#[derive(Default)]
pub(super) struct PointsTo {
    sets: Vec<Vec<ObjectId>>,
}

impl PointsTo {
    pub(super) fn fresh(&mut self) -> Node {
        let id = self.sets.len();
        self.sets.push(Vec::new());
        Node(id as u32)
    }

    pub(super) fn get(&self, node: Node) -> &[ObjectId] {
        &self.sets[node.0 as usize]
    }

    fn insert(&mut self, node: Node, object: ObjectId) -> bool {
        let set = &mut self.sets[node.0 as usize];
        match set.binary_search(&object) {
            Ok(_) => false,
            Err(at) => {
                set.insert(at, object);
                true
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Node(u32);

enum Constraint {
    Copy(Node, Node),
    Load(Node, Node, Field),
    Store(Node, Field, Node),
}

#[derive(Default)]
pub(super) struct Graph {
    pts: PointsTo,
    constraints: Vec<Constraint>,
    copy_edges: FxHashMap<Node, Vec<Node>>,
    loads: FxHashMap<Node, Vec<(Node, Field)>>,
    stores: FxHashMap<Node, Vec<(Field, Node)>>,
    fields: FxHashMap<(ObjectId, u32), Node>,
    budget: Budget,
    exhausted: bool,
}

struct Budget {
    steps: u64,
    elements: u64,
    steps_cap: u64,
    elements_cap: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            steps: 0,
            elements: 0,
            steps_cap: 0,
            elements_cap: 0,
        }
    }
}

impl Graph {
    pub(super) fn with_caps(program_bytes: usize) -> Self {
        let bytes = program_bytes as u64;
        let mut graph = Self::default();
        graph.budget.steps_cap = bytes.saturating_mul(400).max(2_000_000);
        graph.budget.elements_cap = bytes.saturating_mul(64).max(1_000_000);
        graph
    }

    pub(super) fn exhausted(&self) -> bool {
        self.exhausted
    }

    pub(super) fn node(&mut self) -> Node {
        self.pts.fresh()
    }

    fn field_node(&mut self, object: ObjectId, field: Field) -> Node {
        if let Some(node) = self.fields.get(&(object, field.0)) {
            return *node;
        }
        let node = self.pts.fresh();
        self.fields.insert((object, field.0), node);
        node
    }

    pub(super) fn add_object(&mut self, node: Node, object: ObjectId) {
        if self.pts.insert(node, object) {
            self.budget.elements += 1;
        }
    }

    pub(super) fn copy(&mut self, to: Node, from: Node) {
        self.constraints.push(Constraint::Copy(to, from));
    }

    pub(super) fn load(&mut self, to: Node, object: Node, field: Field) {
        self.constraints.push(Constraint::Load(to, object, field));
    }

    pub(super) fn store(&mut self, object: Node, field: Field, from: Node) {
        self.constraints
            .push(Constraint::Store(object, field, from));
    }

    pub(super) fn solve(&mut self) {
        let constraints = std::mem::take(&mut self.constraints);
        let mut work: Vec<Node> = Vec::new();
        for constraint in &constraints {
            match *constraint {
                Constraint::Copy(to, from) => {
                    self.copy_edges.entry(from).or_default().push(to);
                    work.push(from);
                }
                Constraint::Load(to, object, field) => {
                    self.loads.entry(object).or_default().push((to, field));
                    work.push(object);
                }
                Constraint::Store(object, field, from) => {
                    self.stores.entry(object).or_default().push((field, from));
                    work.push(object);
                }
            }
        }
        work.sort_unstable_by_key(|node| node.0);
        work.dedup();
        let mut queue = work;
        while let Some(node) = queue.pop() {
            if self.budget.steps >= self.budget.steps_cap
                || self.budget.elements >= self.budget.elements_cap
            {
                self.exhausted = true;
                return;
            }
            self.budget.steps += 1;
            let objects = self.pts.get(node).to_vec();
            if let Some(targets) = self.copy_edges.get(&node).cloned() {
                for target in targets {
                    for object in &objects {
                        if self.pts.insert(target, *object) {
                            self.budget.elements += 1;
                            queue.push(target);
                        }
                    }
                }
            }
            if let Some(loads) = self.loads.get(&node).cloned() {
                for (to, field) in loads {
                    for object in &objects {
                        let source = self.field_node(*object, field);
                        self.copy_edges.entry(source).or_default().push(to);
                        queue.push(source);
                    }
                }
            }
            if let Some(stores) = self.stores.get(&node).cloned() {
                for (field, from) in stores {
                    for object in &objects {
                        let sink = self.field_node(*object, field);
                        self.copy_edges.entry(from).or_default().push(sink);
                        queue.push(from);
                    }
                }
            }
        }
    }

    pub(super) fn points_to(&self, node: Node) -> &[ObjectId] {
        self.pts.get(node)
    }

    pub(super) fn field_of(&self, object: ObjectId, field: Field) -> Option<Node> {
        self.fields.get(&(object, field.0)).copied()
    }
}
