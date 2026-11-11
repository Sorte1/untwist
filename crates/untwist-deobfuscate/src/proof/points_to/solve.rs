use oxc_span::Span;
use rustc_hash::{FxHashMap, FxHashSet};

use super::model::{self, Read};
use super::object::{Builtin, Method, Obj, ObjectId, Objects, SiteKind};
use crate::mutants::{
    back_routes_dropped, escaped_objects_kept, unbound_this_dropped, world_constructors_dropped,
};

const CHECK_EVERY: u32 = 4096;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(super) struct Node(u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(super) struct Field(u32);

impl Field {
    pub(super) const UNKNOWN: Field = Field(u32::MAX);
    pub(super) const ACCESSORS: Field = Field(u32::MAX - 1);
    pub(super) const ENTRIES: Field = Field(u32::MAX - 2);
    pub(super) const ELEMENTS: Field = Field(u32::MAX - 3);
    pub(super) const OWN: Field = Field(u32::MAX - 4);

    fn internal(self) -> bool {
        matches!(
            self,
            Field::ACCESSORS | Field::ENTRIES | Field::ELEMENTS | Field::OWN
        )
    }

    fn element_like(self, name: Option<&str>) -> bool {
        self == Field::UNKNOWN || name.is_some_and(|name| name.parse::<u32>().is_ok())
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Key {
    Static(Field),
    Dynamic(Node),
}

#[derive(Clone, Debug)]
pub(super) struct FnInfo {
    pub(super) params: Vec<Node>,
    pub(super) rest: Option<Node>,
    pub(super) this: Option<Node>,
    pub(super) ret: Node,
    pub(super) arguments: Option<Node>,
    pub(super) sloppy: bool,
    pub(super) supers: Option<Node>,
}

#[derive(Clone)]
pub(super) struct BoundInfo {
    pub(super) target: Node,
    pub(super) this: Node,
    pub(super) args: Vec<Node>,
    pub(super) loose: bool,
    pub(super) merged: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Call {
    pub(super) callee: Node,
    pub(super) this: Option<Node>,
    pub(super) args: Vec<Node>,
    pub(super) spread: Option<Node>,
    pub(super) result: Option<Node>,
    pub(super) construct: bool,
    pub(super) loose: bool,
    pub(super) firm: bool,
    pub(super) span: Span,
}

#[derive(Clone)]
pub(super) struct SimpleSite {
    pub(super) callee: Node,
    pub(super) this: Node,
    pub(super) args: Vec<Node>,
    pub(super) result: Node,
}

#[derive(Clone)]
pub(super) enum PatchKey {
    Node(Node),
    Fields(Vec<Node>),
    Whole,
    Absent,
}

#[derive(Clone)]
pub(super) struct Patch {
    pub(super) span: Span,
    pub(super) target: Node,
    pub(super) key: PatchKey,
}

#[derive(Clone)]
enum Init {
    Add(Node, ObjectId),
    Copy(Node, Node),
    Load(Node, Key, Node),
    Store(Node, Key, Node),
    Call(Call),
    Concat(Node, Node, Node),
    Keys(Node, Node),
    Assign(Node, Node),
    ProtoSet(Node, Node),
}

#[derive(Clone, Copy)]
enum Watch {
    LoadBase(u32),
    LoadKey(u32),
    StoreBase(u32),
    StoreKey(u32),
    Call(u32),
    Concat(u32),
    Keys(u32),
    Assign(u32),
    Define(u32),
    ProtoSet(u32),
    EscapeSites,
    Child(ObjectId),
    Heirs(u32),
    ExtraHeir(u32),
    Convert,
    WorldProto,
    WorldProtos,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Sub {
    CopyTo(Node),
    ElementsTo(Node),
    KeysTo(Node),
    StoreInto(Node),
    DefineInto(Node),
}

type CallKey = (
    Node,
    Option<Node>,
    Vec<Node>,
    Option<Node>,
    Option<Node>,
    Option<Span>,
    bool,
    bool,
);

#[derive(Default)]
struct Budget {
    steps: u64,
    elements: u64,
    steps_cap: u64,
    elements_cap: u64,
}

const CONCAT_LIMIT: usize = 64;
const CONCAT_TEXT_LIMIT: usize = 512;

pub(super) struct Graph {
    pub(super) objects: Objects,
    pts: Vec<Vec<ObjectId>>,
    pending: Vec<Vec<ObjectId>>,
    copies: Vec<Vec<Node>>,
    watches: Vec<Vec<Watch>>,
    edges: FxHashSet<(Node, Node)>,
    queue: Vec<Node>,
    replays: Vec<(Node, Watch)>,
    implicit_done: FxHashSet<Node>,
    converted: FxHashSet<ObjectId>,
    pub(super) builtin_conversions: FxHashSet<Node>,
    init: Vec<Init>,
    live: bool,
    loads: Vec<(Node, Key, Node)>,
    stores: Vec<(Node, Key, Node)>,
    pub(super) calls: Vec<Call>,
    concats: Vec<(Node, Node, Node, bool)>,
    keys: Vec<(Node, Node)>,
    assigns: Vec<(Node, Node)>,
    defines: Vec<(Node, Node)>,
    proto_sets: Vec<(Node, Node)>,
    fields: FxHashMap<(ObjectId, Field), Node>,
    field_list: FxHashMap<ObjectId, Vec<(Field, Node)>>,
    subs: FxHashMap<ObjectId, Vec<Sub>>,
    subscribed: FxHashSet<(ObjectId, Sub)>,
    keyed: FxHashSet<(ObjectId, Node)>,
    protos: FxHashMap<ObjectId, Node>,
    singletons: FxHashMap<ObjectId, Node>,
    escaped: FxHashSet<ObjectId>,
    pub(super) functions: FxHashMap<ObjectId, FnInfo>,
    pub(super) bound: FxHashMap<ObjectId, BoundInfo>,
    pub(super) bound_calls: FxHashMap<ObjectId, Vec<u32>>,
    pub(super) spills: FxHashMap<ObjectId, Node>,
    wired_loads: FxHashSet<(ObjectId, Field, Node)>,
    wired_stores: FxHashSet<(ObjectId, Field, Node)>,
    applied: FxHashSet<(u32, ObjectId)>,
    call_keys: FxHashSet<CallKey>,
    names: FxHashMap<Box<str>, Field>,
    name_list: Vec<Box<str>>,
    pub(super) esc: Node,
    pub(super) world: Node,
    pub(super) callers: Node,
    pub(super) empty: Node,
    pub(super) patches: Vec<Patch>,
    patch_seen: FxHashSet<(u32, Method, Node)>,
    pub(super) skip_definers: FxHashSet<Span>,
    pub(super) global_declared: FxHashSet<Field>,
    loaded: FxHashMap<(Node, Field), Node>,
    derived: FxHashMap<(u8, Vec<Node>, Span), Node>,
    simple_sites: FxHashMap<(Span, usize), SimpleSite>,
    returning: Vec<ObjectId>,
    returned: Vec<ObjectId>,
    children: FxHashMap<ObjectId, Vec<ObjectId>>,
    adopted: FxHashSet<(ObjectId, ObjectId)>,
    heirs: FxHashMap<ObjectId, Node>,
    heir_copies: Vec<Node>,
    extra_heirs: Vec<Node>,
    receivers: FxHashMap<Node, Node>,
    world_fields: Option<Node>,
    world_protos: Node,
    world_proto_sets: Node,
    world_loads: Vec<(Field, Node)>,
    budget: Budget,
    exhausted: bool,
}

impl Graph {
    pub(super) fn with_caps(program_bytes: usize) -> Self {
        let bytes = program_bytes as u64;
        let mut graph = Graph {
            objects: Objects::new(),
            pts: Vec::new(),
            pending: Vec::new(),
            copies: Vec::new(),
            watches: Vec::new(),
            edges: FxHashSet::default(),
            queue: Vec::new(),
            replays: Vec::new(),
            implicit_done: FxHashSet::default(),
            converted: FxHashSet::default(),
            builtin_conversions: FxHashSet::default(),
            init: Vec::new(),
            live: false,
            loads: Vec::new(),
            stores: Vec::new(),
            calls: Vec::new(),
            concats: Vec::new(),
            keys: Vec::new(),
            assigns: Vec::new(),
            defines: Vec::new(),
            proto_sets: Vec::new(),
            fields: FxHashMap::default(),
            field_list: FxHashMap::default(),
            subs: FxHashMap::default(),
            subscribed: FxHashSet::default(),
            keyed: FxHashSet::default(),
            protos: FxHashMap::default(),
            singletons: FxHashMap::default(),
            escaped: FxHashSet::default(),
            functions: FxHashMap::default(),
            bound: FxHashMap::default(),
            bound_calls: FxHashMap::default(),
            spills: FxHashMap::default(),
            wired_loads: FxHashSet::default(),
            wired_stores: FxHashSet::default(),
            applied: FxHashSet::default(),
            call_keys: FxHashSet::default(),
            names: FxHashMap::default(),
            name_list: Vec::new(),
            esc: Node(0),
            world: Node(0),
            callers: Node(0),
            empty: Node(0),
            patches: Vec::new(),
            patch_seen: FxHashSet::default(),
            skip_definers: FxHashSet::default(),
            global_declared: FxHashSet::default(),
            loaded: FxHashMap::default(),
            derived: FxHashMap::default(),
            simple_sites: FxHashMap::default(),
            returning: Vec::new(),
            returned: Vec::new(),
            children: FxHashMap::default(),
            adopted: FxHashSet::default(),
            heirs: FxHashMap::default(),
            heir_copies: Vec::new(),
            extra_heirs: Vec::new(),
            receivers: FxHashMap::default(),
            world_fields: None,
            world_protos: Node(0),
            world_proto_sets: Node(0),
            world_loads: Vec::new(),
            budget: Budget {
                steps_cap: bytes.saturating_mul(48).clamp(4_000_000, 400_000_000),
                elements_cap: bytes.saturating_mul(4).clamp(4_000_000, 24_000_000),
                ..Budget::default()
            },
            exhausted: false,
        };
        graph.esc = graph.node();
        graph.world = graph.node();
        graph.callers = graph.node();
        graph.empty = graph.node();
        graph.world_protos = graph.node();
        graph.world_proto_sets = graph.node();
        let world_protos = graph.world_protos;
        graph.watches[world_protos.0 as usize].push(Watch::WorldProtos);
        graph.insert(graph.world, ObjectId::WORLD);
        graph
    }

    pub(super) fn exhausted(&self) -> bool {
        self.exhausted
    }

    pub(super) fn node(&mut self) -> Node {
        let id = self.pts.len();
        self.pts.push(Vec::new());
        self.pending.push(Vec::new());
        self.copies.push(Vec::new());
        self.watches.push(Vec::new());
        Node(id as u32)
    }

    pub(super) fn points_to(&self, node: Node) -> Vec<ObjectId> {
        self.widened(&self.pts[node.0 as usize])
    }

    fn widened(&self, objects: &[ObjectId]) -> Vec<ObjectId> {
        let mut found = objects.to_vec();
        if objects.first() == Some(&ObjectId::WORLD) {
            for object in &self.returning {
                if objects.binary_search(object).is_err() {
                    found.push(*object);
                }
            }
        }
        found
    }

    pub(super) fn name(&mut self, text: &str) -> Field {
        if let Some(field) = self.names.get(text) {
            return *field;
        }
        let field = Field(self.name_list.len() as u32);
        self.name_list.push(text.into());
        self.names.insert(text.into(), field);
        field
    }

    pub(super) fn field_name(&self, field: Field) -> Option<&str> {
        self.name_list.get(field.0 as usize).map(|name| &**name)
    }

    pub(super) fn builtin(&mut self, builtin: Builtin) -> ObjectId {
        self.objects.builtin(builtin)
    }

    pub(super) fn is_escaped(&self, object: ObjectId) -> bool {
        self.escaped.contains(&object)
    }

    pub(super) fn named_fields(&self, object: ObjectId) -> Vec<Field> {
        self.field_list
            .get(&object)
            .map(|fields| fields.iter().map(|(field, _)| *field).collect())
            .unwrap_or_default()
    }

    fn insert(&mut self, node: Node, object: ObjectId) -> bool {
        if self.exhausted || self.covered_by_world(node, object) {
            return false;
        }
        let set = &mut self.pts[node.0 as usize];
        let Err(at) = set.binary_search(&object) else {
            return false;
        };
        set.insert(at, object);
        self.budget.elements += 1;
        if self.spent(0) {
            return false;
        }
        let pending = &mut self.pending[node.0 as usize];
        pending.push(object);
        if pending.len() == 1 {
            self.queue.push(node);
        }
        true
    }

    fn covered_by_world(&self, node: Node, object: ObjectId) -> bool {
        node != self.esc
            && self.pts[node.0 as usize].first() == Some(&ObjectId::WORLD)
            && self.objects.is_site(object)
            && self.escaped.contains(&object)
            && !escaped_objects_kept()
    }

    fn standard_proto(&self, object: ObjectId) -> bool {
        match self.objects.get(object) {
            Obj::Builtin(builtin) => matches!(
                builtin,
                Builtin::ObjectProto
                    | Builtin::FunctionProto
                    | Builtin::ArrayProto
                    | Builtin::StringProto
                    | Builtin::NumberProto
                    | Builtin::BooleanProto
                    | Builtin::SymbolProto
                    | Builtin::CollectionProto
                    | Builtin::IteratorProto
            ),
            _ => true,
        }
    }

    fn defer(&mut self, init: Init) -> bool {
        if self.live {
            return false;
        }
        self.init.push(init);
        true
    }

    pub(super) fn add(&mut self, node: Node, object: ObjectId) {
        if !self.defer(Init::Add(node, object)) {
            self.insert(node, object);
        }
    }

    pub(super) fn add_builtin(&mut self, node: Node, builtin: Builtin) {
        let object = self.builtin(builtin);
        self.add(node, object);
    }

    pub(super) fn copy(&mut self, from: Node, to: Node) {
        if from == to || self.defer(Init::Copy(from, to)) || self.exhausted {
            return;
        }
        if !self.edges.insert((from, to)) || self.spent(0) {
            return;
        }
        self.copies[from.0 as usize].push(to);
        let objects = self.pts[from.0 as usize].clone();
        for object in objects {
            self.insert(to, object);
        }
    }

    pub(super) fn escape(&mut self, node: Node) {
        let esc = self.esc;
        self.copy(node, esc);
    }

    pub(super) fn singleton(&mut self, object: ObjectId) -> Node {
        if let Some(node) = self.singletons.get(&object) {
            return *node;
        }
        let node = self.node();
        self.singletons.insert(object, node);
        self.add(node, object);
        node
    }

    pub(super) fn builtin_node(&mut self, builtin: Builtin) -> Node {
        let object = self.builtin(builtin);
        self.singleton(object)
    }

    fn watch(&mut self, node: Node, watch: Watch) {
        if self.spent(1) {
            return;
        }
        self.watches[node.0 as usize].push(watch);
        if !self.pts[node.0 as usize].is_empty() {
            self.replays.push((node, watch));
        }
    }

    pub(super) fn load(&mut self, base: Node, key: Key, to: Node) {
        if self.defer(Init::Load(base, key, to)) || self.exhausted {
            return;
        }
        self.add_builtin(to, Builtin::PrimUndefined);
        let index = self.loads.len() as u32;
        self.loads.push((base, key, to));
        self.watch(base, Watch::LoadBase(index));
        if let Key::Dynamic(node) = key {
            self.watch(node, Watch::LoadKey(index));
        }
    }

    pub(super) fn loaded(&mut self, base: Node, field: Field) -> Node {
        if let Some(node) = self.loaded.get(&(base, field)) {
            return *node;
        }
        let node = self.node();
        self.loaded.insert((base, field), node);
        self.load(base, Key::Static(field), node);
        node
    }

    pub(super) fn derived(&mut self, tag: u8, inputs: Vec<Node>, span: Span) -> (Node, bool) {
        if let Some(node) = self.derived.get(&(tag, inputs.clone(), span)) {
            return (*node, false);
        }
        let node = self.node();
        self.derived.insert((tag, inputs, span), node);
        (node, true)
    }

    pub(super) fn simple_site(&mut self, span: Span, arity: usize) -> SimpleSite {
        if let Some(site) = self.simple_sites.get(&(span, arity)) {
            return site.clone();
        }
        let site = SimpleSite {
            callee: self.node(),
            this: self.node(),
            args: (0..arity).map(|_| self.node()).collect(),
            result: self.node(),
        };
        self.simple_sites.insert((span, arity), site.clone());
        let live = self.live;
        self.live = true;
        self.call(Call {
            callee: site.callee,
            this: Some(site.this),
            args: site.args.clone(),
            spread: None,
            result: Some(site.result),
            construct: false,
            loose: true,
            firm: false,
            span,
        });
        self.live = live;
        site
    }

    pub(super) fn load_named(&mut self, base: Node, name: &str, to: Node) {
        let field = self.name(name);
        self.load(base, Key::Static(field), to);
    }

    pub(super) fn store(&mut self, base: Node, key: Key, from: Node) {
        if self.defer(Init::Store(base, key, from)) || self.exhausted {
            return;
        }
        let index = self.stores.len() as u32;
        self.stores.push((base, key, from));
        self.watch(base, Watch::StoreBase(index));
        if let Key::Dynamic(node) = key {
            self.watch(node, Watch::StoreKey(index));
        }
    }

    pub(super) fn store_named(&mut self, base: Node, name: &str, from: Node) {
        let field = self.name(name);
        self.store(base, Key::Static(field), from);
    }

    pub(super) fn call(&mut self, call: Call) {
        if self.defer(Init::Call(call.clone())) {
            return;
        }
        let key = (
            call.callee,
            call.this,
            call.args.clone(),
            call.spread,
            call.result,
            call.construct.then_some(call.span),
            call.loose,
            call.firm,
        );
        if self.exhausted || !self.call_keys.insert(key) {
            return;
        }
        if self.spent(call.args.len() as u64) {
            return;
        }
        let index = self.calls.len() as u32;
        let callee = call.callee;
        self.calls.push(call);
        self.watch(callee, Watch::Call(index));
    }

    pub(super) fn concat(&mut self, left: Node, right: Node, to: Node) {
        if self.defer(Init::Concat(left, right, to)) {
            return;
        }
        let index = self.concats.len() as u32;
        self.concats.push((left, right, to, false));
        self.watch(left, Watch::Concat(index));
        self.watch(right, Watch::Concat(index));
    }

    pub(super) fn keys_of(&mut self, source: Node, to: Node) {
        if self.defer(Init::Keys(source, to)) {
            return;
        }
        let index = self.keys.len() as u32;
        self.keys.push((source, to));
        self.watch(source, Watch::Keys(index));
    }

    pub(super) fn assign_fields(&mut self, source: Node, target: Node) {
        if self.defer(Init::Assign(source, target)) {
            return;
        }
        let index = self.assigns.len() as u32;
        self.assigns.push((source, target));
        self.watch(source, Watch::Assign(index));
    }

    pub(super) fn define_fields(&mut self, props: Node, target: Node) {
        let index = self.defines.len() as u32;
        self.defines.push((props, target));
        self.watch(props, Watch::Define(index));
    }

    pub(super) fn proto_set(&mut self, base: Node, from: Node) {
        if self.defer(Init::ProtoSet(base, from)) {
            return;
        }
        let index = self.proto_sets.len() as u32;
        self.proto_sets.push((base, from));
        self.watch(base, Watch::ProtoSet(index));
    }

    pub(super) fn field_node(&mut self, object: ObjectId, field: Field) -> Node {
        if let Some(node) = self.fields.get(&(object, field)) {
            return *node;
        }
        let node = self.node();
        self.fields.insert((object, field), node);
        self.field_list
            .entry(object)
            .or_default()
            .push((field, node));
        self.seed_field(object, field, node);
        if self.objects.is_site(object) && self.escaped.contains(&object) {
            self.insert(node, ObjectId::WORLD);
            let esc = self.esc;
            self.copy(node, esc);
        }
        let subs = self.subs.get(&object).cloned().unwrap_or_default();
        for sub in subs {
            self.apply_sub(sub, field, node);
        }
        node
    }

    fn seed_field(&mut self, object: ObjectId, field: Field, node: Node) {
        let Some(builtin) = self.objects.as_builtin(object) else {
            return;
        };
        let Some(name) = self.field_name(field).map(str::to_string) else {
            return;
        };
        let read =
            if builtin == Builtin::Global && back_routes_dropped() && model::is_back_route(&name) {
                Read::World
            } else {
                model::read(builtin, &name)
            };
        match read {
            Read::Values(values) => {
                for value in values {
                    let id = self.builtin(*value);
                    self.insert(node, id);
                }
            }
            Read::World => {
                if !self.global_declared.contains(&field) {
                    self.insert(node, ObjectId::WORLD);
                }
                if builtin == Builtin::Global
                    && model::is_back_route(&name)
                    && !back_routes_dropped()
                {
                    self.insert(node, object);
                }
            }
        }
    }

    pub(super) fn proto_node(&mut self, object: ObjectId) -> Node {
        if let Some(node) = self.protos.get(&object) {
            return *node;
        }
        let node = self.node();
        self.protos.insert(object, node);
        self.watch(node, Watch::Child(object));
        if let Some(builtin) = self.objects.as_builtin(object) {
            for proto in model::protos(builtin) {
                let id = self.builtin(*proto);
                self.insert(node, id);
            }
        }
        if self.escaped.contains(&object) {
            self.escape_sites(node);
        }
        node
    }

    fn escape_sites(&mut self, node: Node) {
        self.watch(node, Watch::EscapeSites);
    }

    pub(super) fn new_site(&mut self, span: Span, kind: SiteKind, proto: &[Builtin]) -> ObjectId {
        let (object, fresh) = self.objects.site(span, kind);
        if fresh {
            let node = self.proto_node(object);
            for builtin in proto {
                let id = self.builtin(*builtin);
                self.add(node, id);
            }
        }
        object
    }

    fn subscribe(&mut self, object: ObjectId, sub: Sub) {
        if !self.subscribed.insert((object, sub)) {
            return;
        }
        self.subs.entry(object).or_default().push(sub);
        let fields = self.field_list.get(&object).cloned().unwrap_or_default();
        for (field, node) in fields {
            self.apply_sub(sub, field, node);
        }
    }

    fn apply_sub(&mut self, sub: Sub, field: Field, node: Node) {
        match sub {
            Sub::CopyTo(to) => self.copy(node, to),
            Sub::ElementsTo(to) => {
                if field.element_like(self.field_name(field)) {
                    self.copy(node, to);
                }
            }
            Sub::KeysTo(to) => match field {
                Field::ACCESSORS | Field::ENTRIES => {}
                Field::UNKNOWN => self.add_builtin(to, Builtin::PrimString),
                named => {
                    let name = self.field_name(named).unwrap_or_default().to_string();
                    let id = self.objects.string(&name);
                    self.insert(to, id);
                }
            },
            Sub::StoreInto(target) => {
                if !field.internal() {
                    self.store(target, Key::Static(field), node);
                }
            }
            Sub::DefineInto(target) => {
                if !field.internal() {
                    self.define_one(target, Key::Static(field), node, Span::default());
                }
            }
        }
    }

    pub(super) fn define_one(&mut self, target: Node, key: Key, descriptor: Node, span: Span) {
        let names = [self.name("value"), self.name("get"), self.name("set")];
        let [value, getter, setter] = names.map(|name| self.loaded(descriptor, name));
        self.getter_call(target, getter, value, span);
        self.setter_call(target, key, setter, span);
        self.store(target, key, value);
        self.store(target, Key::Static(Field::ACCESSORS), getter);
        self.store(target, Key::Static(Field::ACCESSORS), setter);
    }

    pub(super) fn getter_call(&mut self, target: Node, getter: Node, result: Node, span: Span) {
        let this = self.receivers(target);
        self.call(Call {
            callee: getter,
            this: Some(this),
            args: Vec::new(),
            spread: None,
            result: Some(result),
            construct: false,
            loose: false,
            firm: false,
            span,
        });
    }

    pub(super) fn setter_call(&mut self, target: Node, key: Key, setter: Node, span: Span) {
        let this = self.receivers(target);
        let inputs = match key {
            Key::Dynamic(node) => (4, vec![this, setter, node]),
            Key::Static(Field(field)) => (5, vec![this, setter, Node(field)]),
        };
        let (value, fresh) = self.derived(inputs.0, inputs.1, span);
        if !fresh {
            return;
        }
        self.load(this, key, value);
        self.call(Call {
            callee: setter,
            this: Some(this),
            args: vec![value],
            spread: None,
            result: None,
            construct: false,
            loose: false,
            firm: false,
            span,
        });
    }

    pub(super) fn add_receiver(&mut self, target: Node, receiver: Node) {
        let index = self.extra_heirs.len() as u32;
        self.extra_heirs.push(receiver);
        self.watch(target, Watch::ExtraHeir(index));
    }

    fn receivers(&mut self, target: Node) -> Node {
        if let Some(node) = self.receivers.get(&target) {
            return *node;
        }
        let node = self.node();
        self.receivers.insert(target, node);
        let index = self.heir_copies.len() as u32;
        self.heir_copies.push(node);
        self.watch(target, Watch::Heirs(index));
        node
    }

    fn heirs_node(&mut self, object: ObjectId) -> Node {
        if let Some(node) = self.heirs.get(&object) {
            return *node;
        }
        let root = self.node();
        self.heirs.insert(object, root);
        self.insert(root, object);
        let mut work = vec![(object, root)];
        while let Some((holder, above)) = work.pop() {
            let children = self.children.get(&holder).cloned().unwrap_or_default();
            for child in children {
                let below = match self.heirs.get(&child) {
                    Some(node) => *node,
                    None => {
                        let node = self.node();
                        self.heirs.insert(child, node);
                        self.insert(node, child);
                        work.push((child, node));
                        node
                    }
                };
                self.copy(below, above);
            }
        }
        root
    }

    fn adopt(&mut self, parent: ObjectId, child: ObjectId) {
        match self.objects.get(parent) {
            Obj::World | Obj::Str(_) => return,
            Obj::Builtin(builtin) if builtin.is_primitive() => return,
            _ => {}
        }
        if !self.adopted.insert((parent, child)) {
            return;
        }
        self.children.entry(parent).or_default().push(child);
        if let Some(above) = self.heirs.get(&parent).copied() {
            let below = self.heirs_node(child);
            self.copy(below, above);
        }
    }

    fn heirs_into(&mut self, object: ObjectId, to: Node) {
        match self.objects.get(object).clone() {
            Obj::World => {
                self.insert(to, ObjectId::WORLD);
            }
            Obj::Str(_) => {}
            Obj::Builtin(builtin) if builtin.is_primitive() => {}
            Obj::Builtin(builtin) => {
                self.insert(to, ObjectId::WORLD);
                for prim in model::PRIMITIVES {
                    if builtin == Builtin::ObjectProto || model::protos(*prim).contains(&builtin) {
                        let id = self.builtin(*prim);
                        self.insert(to, id);
                    }
                }
                let heirs = self.heirs_node(object);
                self.copy(heirs, to);
            }
            Obj::Site(..) => {
                let heirs = self.heirs_node(object);
                self.copy(heirs, to);
            }
        }
    }

    fn key_fields(&mut self, key: Key, objects: Option<&[ObjectId]>) -> Vec<Field> {
        match key {
            Key::Static(field) => vec![field],
            Key::Dynamic(node) => {
                let objects = match objects {
                    Some(objects) => objects.to_vec(),
                    None => self.pts[node.0 as usize].clone(),
                };
                let mut found = Vec::new();
                for object in objects {
                    let field = match self.objects.key_text(object).map(str::to_string) {
                        Some(text) => self.name(&text),
                        None => Field::UNKNOWN,
                    };
                    if !found.contains(&field) {
                        found.push(field);
                    }
                }
                found
            }
        }
    }

    fn handle(&mut self, watch: Watch, delta: &[ObjectId]) {
        match watch {
            Watch::LoadBase(index) => {
                let (_, key, to) = self.loads[index as usize];
                let fields = self.key_fields(key, None);
                for object in delta {
                    for field in &fields {
                        self.wire_load(*object, *field, to);
                    }
                }
            }
            Watch::LoadKey(index) => {
                let (base, key, to) = self.loads[index as usize];
                let fields = self.key_fields(key, Some(delta));
                for object in self.points_to(base) {
                    for field in &fields {
                        self.wire_load(object, *field, to);
                    }
                }
            }
            Watch::StoreBase(index) => {
                let (_, key, from) = self.stores[index as usize];
                let fields = self.key_fields(key, None);
                for object in delta {
                    for field in &fields {
                        self.wire_store(*object, *field, from);
                    }
                }
            }
            Watch::StoreKey(index) => {
                let (base, key, from) = self.stores[index as usize];
                let fields = self.key_fields(key, Some(delta));
                for object in self.points_to(base) {
                    for field in &fields {
                        self.wire_store(object, *field, from);
                    }
                }
            }
            Watch::Call(index) => {
                for object in delta {
                    self.apply_call(index, *object);
                }
            }
            Watch::Concat(index) => self.run_concat(index),
            Watch::Keys(index) => {
                let (_, to) = self.keys[index as usize];
                for object in delta {
                    self.keys_into(*object, to);
                }
            }
            Watch::Assign(index) => {
                let (_, target) = self.assigns[index as usize];
                for object in delta {
                    self.fields_into(*object, Sub::StoreInto(target), target);
                }
            }
            Watch::Define(index) => {
                let (_, target) = self.defines[index as usize];
                for object in delta {
                    self.fields_into(*object, Sub::DefineInto(target), target);
                }
            }
            Watch::EscapeSites => {
                let esc = self.esc;
                for object in delta {
                    if self.objects.is_site(*object) {
                        self.insert(esc, *object);
                    }
                }
            }
            Watch::Child(child) => {
                for object in delta {
                    self.adopt(*object, child);
                }
            }
            Watch::Heirs(index) => {
                let to = self.heir_copies[index as usize];
                for object in delta {
                    self.heirs_into(*object, to);
                }
            }
            Watch::Convert => {
                for object in delta {
                    let object = match self.objects.get(*object) {
                        Obj::Str(_) => self.builtin(Builtin::PrimString),
                        _ => *object,
                    };
                    if self.converted.insert(object) {
                        self.convert(object);
                    }
                }
            }
            Watch::ExtraHeir(index) => {
                let receiver = self.extra_heirs[index as usize];
                for object in delta {
                    match self.objects.get(*object) {
                        Obj::World | Obj::Str(_) => {}
                        Obj::Builtin(builtin) if builtin.is_primitive() => {}
                        _ => {
                            let heirs = self.heirs_node(*object);
                            self.copy(receiver, heirs);
                        }
                    }
                }
            }
            Watch::WorldProto => {
                let world_protos = self.world_protos;
                for object in delta {
                    if !self.standard_proto(*object) {
                        self.insert(world_protos, *object);
                    }
                }
            }
            Watch::WorldProtos => {
                let loads = self.world_loads.clone();
                for object in delta {
                    for (field, to) in &loads {
                        if self.over_budget() {
                            return;
                        }
                        self.wire_load(*object, *field, *to);
                    }
                }
            }
            Watch::ProtoSet(index) => {
                let (_, from) = self.proto_sets[index as usize];
                for object in delta {
                    match self.objects.get(*object) {
                        Obj::World => self.world_proto_set(from),
                        Obj::Str(_) => {}
                        Obj::Builtin(builtin) if builtin.is_primitive() => {}
                        _ => {
                            let proto = self.proto_node(*object);
                            self.copy(from, proto);
                        }
                    }
                }
            }
        }
    }

    fn fields_into(&mut self, object: ObjectId, sub: Sub, target: Node) {
        match self.objects.get(object) {
            Obj::World => {
                let fields = self.world_fields();
                self.store(target, Key::Static(Field::UNKNOWN), fields);
            }
            Obj::Str(_) => {}
            Obj::Builtin(builtin) if builtin.is_primitive() => {}
            Obj::Builtin(_) => {
                self.subscribe(object, sub);
                let members = self.node();
                self.wire_load(object, Field::UNKNOWN, members);
                match sub {
                    Sub::DefineInto(_) => self.define_one(
                        target,
                        Key::Static(Field::UNKNOWN),
                        members,
                        Span::default(),
                    ),
                    _ => self.store(target, Key::Static(Field::UNKNOWN), members),
                }
            }
            _ => self.subscribe(object, sub),
        }
    }

    pub(super) fn world_fields(&mut self) -> Node {
        if let Some(node) = self.world_fields {
            return node;
        }
        let node = self.node();
        self.world_fields = Some(node);
        let world = self.world;
        let live = self.live;
        self.live = true;
        self.load(world, Key::Static(Field::UNKNOWN), node);
        self.live = live;
        node
    }

    fn keys_into(&mut self, object: ObjectId, to: Node) {
        if !self.keyed.insert((object, to)) {
            return;
        }
        match self.objects.get(object) {
            Obj::Site(..) => {
                if self.escaped.contains(&object) {
                    self.add_builtin(to, Builtin::PrimString);
                }
                self.subscribe(object, Sub::KeysTo(to));
                let proto = self.proto_node(object);
                self.keys_of(proto, to);
            }
            _ => self.add_builtin(to, Builtin::PrimString),
        }
    }

    fn run_concat(&mut self, index: u32) {
        let (left, right, to, saturated) = self.concats[index as usize];
        if saturated {
            return;
        }
        let texts = |graph: &Graph, node: Node| -> (Vec<String>, bool) {
            let mut texts = Vec::new();
            let mut other = false;
            for object in &graph.pts[node.0 as usize] {
                match graph.objects.key_text(*object) {
                    Some(text) => texts.push(text.to_string()),
                    None => other = true,
                }
            }
            (texts, other)
        };
        let (lefts, left_other) = texts(self, left);
        let (rights, right_other) = texts(self, right);
        if left_other || right_other {
            self.add_builtin(to, Builtin::PrimString);
            self.add_builtin(to, Builtin::PrimNumber);
        }
        if lefts.len().saturating_mul(rights.len()) > CONCAT_LIMIT {
            self.concats[index as usize].3 = true;
            self.add_builtin(to, Builtin::PrimString);
            return;
        }
        for l in &lefts {
            for r in &rights {
                if l.len() + r.len() > CONCAT_TEXT_LIMIT {
                    self.concats[index as usize].3 = true;
                    self.add_builtin(to, Builtin::PrimString);
                    return;
                }
                let joined = format!("{l}{r}");
                let id = self.objects.string(&joined);
                self.insert(to, id);
            }
        }
    }

    fn proto_name(&mut self) -> Field {
        self.name("__proto__")
    }

    fn wire_load(&mut self, object: ObjectId, field: Field, to: Node) {
        if self.exhausted {
            return;
        }
        if !self.wired_loads.insert((object, field, to)) {
            return;
        }
        if field == Field::ELEMENTS || field == Field::OWN {
            self.wire_elements(object, field, to);
            return;
        }
        let builtin = match self.objects.get(object) {
            Obj::World => {
                self.insert(to, ObjectId::WORLD);
                self.world_read(field, to);
                if !escaped_objects_kept() {
                    self.world_loads.push((field, to));
                    let protos = self.pts[self.world_protos.0 as usize].clone();
                    for proto in protos {
                        if self.over_budget() {
                            return;
                        }
                        self.wire_load(proto, field, to);
                    }
                }
                return;
            }
            Obj::Str(_) => Some(Builtin::PrimString),
            Obj::Builtin(builtin) => Some(*builtin),
            Obj::Site(..) => None,
        };
        if let Some(prim) = builtin.filter(|builtin| builtin.is_primitive()) {
            let reads_proto = field == self.proto_name();
            for proto in model::protos(prim) {
                let id = self.builtin(*proto);
                if reads_proto {
                    self.insert(to, id);
                } else {
                    self.wire_load(id, field, to);
                }
            }
            return;
        }
        if matches!(self.objects.get(object), Obj::Site(_, SiteKind::Function)) {
            self.frame_read(object, field, to);
        }
        let proto = self.proto_node(object);
        if field == self.proto_name() {
            self.copy(proto, to);
            return;
        }
        if field == Field::UNKNOWN {
            let unknown = self.field_node(object, Field::UNKNOWN);
            self.copy(unknown, to);
            self.subscribe(object, Sub::CopyTo(to));
            self.copy(proto, to);
            if let Some(builtin) = builtin {
                if builtin == Builtin::Global {
                    self.insert(to, ObjectId::WORLD);
                }
                let (values, _) = model::read_any(builtin);
                for value in values {
                    let id = self.builtin(value);
                    self.insert(to, id);
                }
            }
        } else {
            let own = self.field_node(object, field);
            self.copy(own, to);
            if !field.internal() {
                let unknown = self.field_node(object, Field::UNKNOWN);
                self.copy(unknown, to);
            }
        }
        let live = self.live;
        self.live = true;
        self.load(proto, Key::Static(field), to);
        self.live = live;
    }

    fn wire_elements(&mut self, object: ObjectId, field: Field, to: Node) {
        match self.objects.get(object).clone() {
            Obj::Str(_) => self.add_builtin(to, Builtin::PrimString),
            Obj::Builtin(Builtin::PrimString) => self.add_builtin(to, Builtin::PrimString),
            Obj::Builtin(builtin) if builtin.is_primitive() => {}
            Obj::Site(..) => {
                let unknown = self.field_node(object, Field::UNKNOWN);
                self.copy(unknown, to);
                if field == Field::OWN {
                    self.subscribe(object, Sub::CopyTo(to));
                    return;
                }
                self.subscribe(object, Sub::ElementsTo(to));
                let proto = self.proto_node(object);
                let live = self.live;
                self.live = true;
                self.load(proto, Key::Static(Field::ELEMENTS), to);
                self.live = live;
            }
            Obj::World | Obj::Builtin(_) if field == Field::OWN => {
                self.wire_load(object, Field::UNKNOWN, to);
            }
            Obj::World => {
                self.insert(to, ObjectId::WORLD);
                self.world_read(Field::UNKNOWN, to);
            }
            Obj::Builtin(builtin) => {
                if builtin == Builtin::Global {
                    self.insert(to, ObjectId::WORLD);
                    self.world_read(Field::UNKNOWN, to);
                }
                let unknown = self.field_node(object, Field::UNKNOWN);
                self.copy(unknown, to);
                self.subscribe(object, Sub::ElementsTo(to));
                let proto = self.proto_node(object);
                let live = self.live;
                self.live = true;
                self.load(proto, Key::Static(Field::ELEMENTS), to);
                self.live = live;
            }
        }
    }

    fn frame_read(&mut self, function: ObjectId, field: Field, to: Node) {
        let any = field == Field::UNKNOWN;
        let caller = self.field_name(field) == Some("caller");
        let arguments = self.field_name(field) == Some("arguments");
        if any || caller {
            let callers = self.callers;
            self.copy(callers, to);
            self.add_builtin(to, Builtin::PrimNull);
        }
        if !(any || arguments) {
            return;
        }
        self.insert(to, ObjectId::WORLD);
        let Some(info) = self.functions.get(&function).cloned() else {
            return;
        };
        for node in info.params.iter().chain(&info.rest).chain(&info.arguments) {
            self.escape(*node);
        }
    }

    fn world_read(&mut self, field: Field, to: Node) {
        if field.internal() {
            return;
        }
        let name = self.field_name(field).map(str::to_string);
        let any = field == Field::UNKNOWN;
        let route = any
            || name
                .as_deref()
                .is_some_and(|name| model::is_back_route(name) || model::is_index(name));
        if route && !back_routes_dropped() {
            let global = self.builtin(Builtin::Global);
            self.insert(to, global);
        }
        let inherited = any
            || name
                .as_deref()
                .is_some_and(|name| model::WORLD_CONSTRUCTOR_NAMES.contains(&name));
        if inherited && !world_constructors_dropped() {
            for builtin in model::WORLD_CONSTRUCTORS {
                let id = self.builtin(*builtin);
                self.insert(to, id);
            }
        }
    }

    fn wire_store(&mut self, object: ObjectId, field: Field, from: Node) {
        if self.exhausted {
            return;
        }
        if !self.wired_stores.insert((object, field, from)) {
            return;
        }
        let is_proto = field == self.proto_name();
        match self.objects.get(object) {
            Obj::World if is_proto => self.world_proto_set(from),
            Obj::World => self.escape(from),
            Obj::Str(_) => self.store_on_primitive(Builtin::PrimString, field, from),
            Obj::Builtin(builtin) if builtin.is_primitive() => {
                self.store_on_primitive(*builtin, field, from)
            }
            Obj::Builtin(builtin) if !builtin.patchable() => {}
            Obj::Builtin(_) => {
                let proto = self.proto_node(object);
                let target = if is_proto {
                    proto
                } else {
                    self.field_node(object, field)
                };
                self.copy(from, target);
                self.escape(from);
            }
            Obj::Site(..) => {
                if is_proto {
                    let proto = self.proto_node(object);
                    self.copy(from, proto);
                    return;
                }
                let own = self.field_node(object, field);
                self.copy(from, own);
                if field == Field::UNKNOWN {
                    let proto = self.proto_node(object);
                    self.copy(from, proto);
                }
            }
        }
    }

    fn store_on_primitive(&mut self, primitive: Builtin, field: Field, from: Node) {
        for proto in model::protos(primitive) {
            let id = self.builtin(*proto);
            self.wire_store(id, field, from);
        }
    }

    pub(super) fn record_patch(&mut self, index: u32, method: Method, patch: Patch) {
        if self.skip_definers.contains(&patch.span) {
            return;
        }
        if self.patch_seen.insert((index, method, patch.target)) {
            self.patches.push(patch);
        }
    }

    fn world_proto_set(&mut self, from: Node) {
        self.escape(from);
        if !escaped_objects_kept() {
            let sets = self.world_proto_sets;
            self.copy(from, sets);
        }
    }

    fn escape_object(&mut self, object: ObjectId) {
        match self.objects.get(object).clone() {
            Obj::World | Obj::Str(_) => {}
            Obj::Site(..) => {
                if !self.escaped.insert(object) {
                    return;
                }
                let fields = self.field_list.get(&object).cloned().unwrap_or_default();
                let esc = self.esc;
                for (_, node) in fields {
                    self.insert(node, ObjectId::WORLD);
                    self.copy(node, esc);
                }
                self.field_node(object, Field::UNKNOWN);
                let proto = self.proto_node(object);
                self.escape_sites(proto);
                if !escaped_objects_kept() {
                    self.watch(proto, Watch::WorldProto);
                    let sets = self.world_proto_sets;
                    self.copy(sets, proto);
                }
                if let Some(info) = self.functions.get(&object).cloned() {
                    let world = self.world;
                    for param in info.params.iter().chain(info.rest.iter()) {
                        self.copy(world, *param);
                    }
                    if let Some(this) = info.this {
                        self.copy(world, this);
                        if info.sloppy && !unbound_this_dropped() {
                            let global = self.builtin(Builtin::Global);
                            self.insert(this, global);
                        }
                    }
                    if let Some(arguments) = info.arguments {
                        self.copy(world, arguments);
                    }
                    self.copy(info.ret, esc);
                }
                if let Some(info) = self.bound.get(&object).cloned() {
                    self.copy(info.target, esc);
                    self.copy(info.this, esc);
                    for arg in info.args {
                        self.copy(arg, esc);
                    }
                    let spill = self.spills.get(&object).copied();
                    if let Some(spill) = spill {
                        self.copy(spill, esc);
                    }
                }
            }
            Obj::Builtin(builtin) => {
                if !builtin.is_primitive() && self.escaped.insert(object) {
                    self.returned.push(object);
                }
            }
        }
    }

    pub(super) fn watch_conversions(&mut self, value: Node) {
        self.watch(value, Watch::Convert);
    }

    pub(super) fn first_implicit(&mut self, value: Node) -> bool {
        self.implicit_done.insert(value)
    }

    pub(super) fn first_apply(&mut self, index: u32, object: ObjectId) -> bool {
        self.applied.insert((index, object))
    }

    pub(super) fn live_call(&mut self, call: Call) {
        let live = self.live;
        self.live = true;
        self.call(call);
        self.live = live;
    }

    pub(super) fn solve(&mut self, shuffle: Option<u64>) {
        let mut init = std::mem::take(&mut self.init);
        if let Some(seed) = shuffle {
            shuffle_in_place(&mut init, seed);
        }
        self.live = true;
        for item in init {
            self.apply_init(item);
            if self.over_budget() {
                return;
            }
        }
        let mut pops: u32 = 0;
        loop {
            pops = pops.wrapping_add(1);
            if pops.is_multiple_of(CHECK_EVERY) && untwist_core::budget::check().is_err() {
                self.give_up();
            }
            if self.over_budget() {
                return;
            }
            self.budget.steps += 1;
            if let Some(object) = self.returned.pop() {
                self.come_back(object);
                continue;
            }
            if let Some((node, watch)) = self.replays.pop() {
                let objects = self.points_to(node);
                self.handle(watch, &objects);
                continue;
            }
            let Some(node) = self.queue.pop() else {
                break;
            };
            let delta = std::mem::take(&mut self.pending[node.0 as usize]);
            if delta.is_empty() {
                continue;
            }
            let targets = self.copies[node.0 as usize].clone();
            let fan = (targets.len() + self.watches[node.0 as usize].len()) as u64;
            self.budget.steps += fan.saturating_mul(delta.len() as u64);
            for target in targets {
                for object in &delta {
                    self.insert(target, *object);
                }
            }
            let watches = self.watches[node.0 as usize].clone();
            if !watches.is_empty() {
                let mut seen = delta.clone();
                if delta.contains(&ObjectId::WORLD) {
                    let set = &self.pts[node.0 as usize];
                    for object in &self.returning {
                        if set.binary_search(object).is_err() {
                            seen.push(*object);
                        }
                    }
                }
                for watch in watches {
                    self.handle(watch, &seen);
                }
            }
            if node == self.esc {
                for object in &delta {
                    self.escape_object(*object);
                }
            }
        }
    }

    fn come_back(&mut self, object: ObjectId) {
        self.returning.push(object);
        for index in 0..self.pts.len() {
            let set = &self.pts[index];
            if set.first() != Some(&ObjectId::WORLD) || set.binary_search(&object).is_ok() {
                continue;
            }
            let watches = self.watches[index].clone();
            self.budget.steps += watches.len() as u64;
            for watch in watches {
                self.handle(watch, &[object]);
            }
        }
    }

    fn give_up(&mut self) {
        self.exhausted = true;
        self.queue.clear();
        self.replays.clear();
    }

    fn over_budget(&mut self) -> bool {
        self.spent(0)
    }

    fn spent(&mut self, charge: u64) -> bool {
        if self.exhausted {
            return true;
        }
        self.budget.elements += charge;
        let held = self.held();
        if self.budget.steps >= self.budget.steps_cap || held >= self.budget.elements_cap {
            self.exhausted = true;
            self.queue.clear();
            self.replays.clear();
        }
        self.exhausted
    }

    fn held(&self) -> u64 {
        self.budget.elements
            + (self.edges.len()
                + self.wired_loads.len()
                + self.wired_stores.len()
                + self.world_loads.len()
                + self.loads.len()
                + self.stores.len()
                + self.calls.len()
                + self.pts.len()) as u64
    }

    fn apply_init(&mut self, item: Init) {
        match item {
            Init::Add(node, object) => {
                self.insert(node, object);
            }
            Init::Copy(from, to) => self.copy(from, to),
            Init::Load(base, key, to) => self.load(base, key, to),
            Init::Store(base, key, from) => self.store(base, key, from),
            Init::Call(call) => self.call(call),
            Init::Concat(left, right, to) => self.concat(left, right, to),
            Init::Keys(source, to) => self.keys_of(source, to),
            Init::Assign(source, target) => self.assign_fields(source, target),
            Init::ProtoSet(base, from) => self.proto_set(base, from),
        }
    }

    #[cfg(test)]
    pub(super) fn cap_for_tests(&mut self, steps: u64) {
        self.budget.steps_cap = steps;
    }

    #[cfg(test)]
    pub(super) fn steps(&self) -> u64 {
        self.budget.steps
    }

    #[cfg(test)]
    pub(super) fn cap_elements_for_tests(&mut self, elements: u64) {
        self.budget.elements_cap = elements;
    }

    #[cfg(test)]
    pub(super) fn held_and_cap(&self) -> (u64, u64) {
        (self.held(), self.budget.elements_cap)
    }

    #[cfg(test)]
    pub(super) fn stores_issued(&self) -> usize {
        self.stores.len()
    }
}

fn shuffle_in_place<T>(items: &mut [T], seed: u64) {
    let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
    for index in (1..items.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let other = (state % (index as u64 + 1)) as usize;
        items.swap(index, other);
    }
}
