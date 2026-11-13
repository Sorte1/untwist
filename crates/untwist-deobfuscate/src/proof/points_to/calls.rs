use oxc_span::Span;

use super::object::{Builtin, Each, Method, Obj, ObjectId, SiteKind};
use super::solve::{BoundInfo, Call, Field, Graph, Key, Node, Patch, PatchKey};
use crate::mutants::unbound_this_dropped;

const UNKNOWN: Key = Key::Static(Field::UNKNOWN);
const ACCESSORS: Key = Key::Static(Field::ACCESSORS);
const ENTRIES: Key = Key::Static(Field::ENTRIES);
const ELEMENTS: Key = Key::Static(Field::ELEMENTS);
const OWN: Key = Key::Static(Field::OWN);
const INDEX: Key = Key::Static(Field::INDEX);
const BOUND_ARGS_LIMIT: usize = 16;

impl Graph {
    pub(super) fn apply_call(&mut self, index: u32, callee: ObjectId) {
        if !self.first_apply(index, callee) {
            return;
        }
        let call = self.calls[index as usize].clone();
        let standard_conversion = self.builtin_conversions.contains(&call.callee);
        match self.objects.get(callee).clone() {
            Obj::World | Obj::Builtin(Builtin::Other) if standard_conversion => {}
            Obj::World => self.generic(&call),
            Obj::Functions => self.call_functions(&call),
            Obj::Str(_) => {}
            Obj::Site(_, SiteKind::Function) => self.bind_function(callee, &call),
            Obj::Site(_, SiteKind::Bound) => {
                self.bound_calls.entry(callee).or_default().push(index);
                self.call_bound(callee, &call);
            }
            Obj::Site(..) => {}
            Obj::Builtin(Builtin::Method(method)) => self.method(index, method, &call),
            Obj::Builtin(builtin) => self.root_call(builtin, &call),
        }
    }

    fn call_functions(&mut self, call: &Call) {
        let slot = usize::from(call.construct);
        let (this, args, ret) = match self.function_calls[slot] {
            Some(summary) => summary,
            None => {
                let summary = (self.node(), self.node(), self.node());
                self.function_calls[slot] = Some(summary);
                let (this, args, ret) = summary;
                let shared = Call {
                    callee: self.empty,
                    this: Some(this),
                    args: Vec::new(),
                    spread: Some(args),
                    result: Some(ret),
                    construct: call.construct,
                    loose: true,
                    firm: false,
                    span: Span::new(u32::MAX, u32::MAX),
                };
                for function in self.function_sites().iter() {
                    self.bind_function(*function, &shared);
                }
                summary
            }
        };
        if let Some(receiver) = call.this {
            self.copy(receiver, this);
        }
        for arg in call.args.iter().chain(&call.spread) {
            self.copy(*arg, args);
        }
        if let Some(result) = call.result {
            self.copy(ret, result);
        }
    }

    fn generic(&mut self, call: &Call) {
        for node in call.this.iter().chain(&call.args).chain(&call.spread) {
            self.escape(*node);
        }
        if let Some(result) = call.result {
            self.copy_world(result);
        }
    }

    fn copy_world(&mut self, to: Node) {
        let world = self.world;
        self.copy(world, to);
    }

    fn bind_function(&mut self, function: ObjectId, call: &Call) {
        let Some(info) = self.functions.get(&function).cloned() else {
            return;
        };
        for (position, arg) in call.args.iter().enumerate() {
            match info.params.get(position) {
                Some(param) => self.copy(*arg, *param),
                None => {
                    if let Some(rest) = info.rest {
                        self.copy(*arg, rest);
                    }
                }
            }
            if let Some(arguments) = info.arguments {
                self.copy(*arg, arguments);
            }
        }
        if let Some(spread) = call.spread {
            for param in info.params.iter().skip(call.args.len()).chain(&info.rest) {
                self.copy(spread, *param);
            }
            if let Some(arguments) = info.arguments {
                self.copy(spread, arguments);
            }
        }
        if let Some(this) = info.this {
            if call.construct {
                let instance = self.new_site(call.span, SiteKind::Instance, &[]);
                let proto = self.proto_node(instance);
                let constructor = self.singleton(function);
                self.load_named(constructor, "prototype", proto);
                self.add(this, instance);
                if let Some(result) = call.result {
                    self.add(result, instance);
                    if let Some(supers) = info.supers {
                        self.copy(supers, result);
                    }
                }
            } else {
                if let Some(receiver) = call.this {
                    self.copy(receiver, this);
                }
                let unbound = call.this.is_none() || call.loose;
                if info.sloppy && unbound && !unbound_this_dropped() {
                    self.add_builtin(this, Builtin::Global);
                }
            }
        }
        if let Some(result) = call.result {
            self.copy(info.ret, result);
        }
    }

    fn call_bound(&mut self, bound: ObjectId, call: &Call) {
        let Some(info) = self.bound.get(&bound).cloned() else {
            return;
        };
        let mut args = info.args.clone();
        args.extend(call.args.iter().copied());
        let spread = if info.merged {
            let spill = self.spill(bound);
            for extra in std::mem::take(&mut args) {
                self.copy(extra, spill);
            }
            if let Some(spread) = call.spread {
                self.copy(spread, spill);
            }
            Some(spill)
        } else if args.len() > BOUND_ARGS_LIMIT
            || (call.spread.is_some() && self.spills.contains_key(&bound))
        {
            let spill = self.spill(bound);
            for extra in args.split_off(BOUND_ARGS_LIMIT.min(args.len())) {
                self.copy(extra, spill);
            }
            if let Some(spread) = call.spread {
                self.copy(spread, spill);
            }
            Some(spill)
        } else {
            call.spread
        };
        self.call(Call {
            callee: info.target,
            this: if call.construct {
                call.this
            } else {
                Some(info.this)
            },
            args,
            spread,
            result: call.result,
            construct: call.construct,
            loose: info.loose,
            firm: false,
            span: call.span,
        });
    }

    fn bind_into(
        &mut self,
        bound: ObjectId,
        target: Node,
        this: Option<Node>,
        args: Vec<Node>,
        loose: bool,
    ) {
        if let Some(info) = self.bound.get(&bound) {
            let same_this = this.unwrap_or(self.empty) == info.this;
            if info.target == target && same_this && info.args == args && info.loose == loose {
                return;
            }
        }
        let Some(info) = self.bound.get(&bound).cloned() else {
            let empty = self.empty;
            self.bound.insert(
                bound,
                BoundInfo {
                    target,
                    this: this.unwrap_or(empty),
                    args,
                    loose,
                    merged: false,
                },
            );
            return;
        };
        if info.merged {
            self.copy(target, info.target);
            if let Some(this) = this {
                self.copy(this, info.this);
            }
            let spill = self.spill(bound);
            for arg in &args {
                self.copy(*arg, spill);
            }
            if !loose || info.loose {
                return;
            }
            if let Some(stored) = self.bound.get_mut(&bound) {
                stored.loose = true;
            }
        } else {
            self.merge_bound(bound, info, (target, this), args, loose);
        }
        let indices = self.bound_calls.get(&bound).cloned().unwrap_or_default();
        for index in indices {
            let call = self.calls[index as usize].clone();
            self.call_bound(bound, &call);
        }
    }

    fn merge_bound(
        &mut self,
        bound: ObjectId,
        info: BoundInfo,
        (target, this): (Node, Option<Node>),
        args: Vec<Node>,
        loose: bool,
    ) {
        let (merged_target, merged_this) = (self.node(), self.node());
        self.copy(info.target, merged_target);
        self.copy(target, merged_target);
        self.copy(info.this, merged_this);
        if let Some(this) = this {
            self.copy(this, merged_this);
        }
        let spill = self.spill(bound);
        for arg in info.args.iter().chain(&args) {
            self.copy(*arg, spill);
        }
        self.bound.insert(
            bound,
            BoundInfo {
                target: merged_target,
                this: merged_this,
                args: Vec::new(),
                loose: info.loose || loose,
                merged: true,
            },
        );
        if self.is_escaped(bound) {
            for node in [merged_target, merged_this, spill] {
                self.escape(node);
            }
        }
    }

    fn spill(&mut self, bound: ObjectId) -> Node {
        if let Some(node) = self.spills.get(&bound) {
            return *node;
        }
        let node = self.node();
        self.spills.insert(bound, node);
        node
    }

    fn fresh(&mut self, span: Span, kind: SiteKind, proto: &[Builtin]) -> (ObjectId, Node) {
        let object = self.new_site(span, kind, proto);
        (object, self.singleton(object))
    }

    fn fresh_array(&mut self, span: Span, result: Option<Node>) -> Node {
        let (object, node) = self.fresh(span, SiteKind::FreshArray, &[Builtin::ArrayProto]);
        if let Some(result) = result {
            self.copy(node, result);
        }
        self.field_node(object, Field::INDEX)
    }

    fn prims(&mut self, result: Option<Node>) {
        if let Some(result) = result {
            for builtin in [
                Builtin::PrimString,
                Builtin::PrimNumber,
                Builtin::PrimBoolean,
                Builtin::PrimUndefined,
                Builtin::PrimNull,
            ] {
                self.add_builtin(result, builtin);
            }
        }
    }

    pub(super) fn implicit(&mut self, value: Node) {
        if self.first_implicit(value) {
            self.watch_conversions(value);
        }
    }

    pub(super) fn convert(&mut self, object: ObjectId) {
        let value = self.singleton(object);
        let callee = self.node();
        if matches!(self.objects.get(object), Obj::Builtin(_)) {
            self.builtin_conversions.insert(callee);
        }
        self.load_named(value, "toString", callee);
        self.load_named(value, "valueOf", callee);
        self.call(Call {
            callee,
            this: Some(value),
            args: Vec::new(),
            spread: None,
            result: None,
            construct: false,
            loose: false,
            firm: false,
            span: Span::default(),
        });
    }

    pub(super) fn iterate(&mut self, value: Node) -> Node {
        let (elements, fresh) = self.derived(3, vec![value], Span::default());
        if !fresh {
            return elements;
        }
        self.load(value, ELEMENTS, elements);
        let iterator_fn = self.node();
        self.load_named(value, "@@iterator", iterator_fn);
        let iterator = self.node();
        self.call(Call {
            callee: iterator_fn,
            this: Some(value),
            args: Vec::new(),
            spread: None,
            result: Some(iterator),
            construct: false,
            loose: false,
            firm: false,
            span: Span::default(),
        });
        let next = self.node();
        self.load_named(iterator, "next", next);
        let step = self.node();
        self.call(Call {
            callee: next,
            this: Some(iterator),
            args: Vec::new(),
            spread: None,
            result: Some(step),
            construct: false,
            loose: false,
            firm: false,
            span: Span::default(),
        });
        self.load_named(step, "value", elements);
        let closer = self.node();
        self.load_named(iterator, "return", closer);
        self.load_named(iterator, "throw", closer);
        self.call(Call {
            callee: closer,
            this: Some(iterator),
            args: Vec::new(),
            spread: None,
            result: None,
            construct: false,
            loose: false,
            firm: false,
            span: Span::default(),
        });
        elements
    }

    fn simple_call(
        &mut self,
        callee: Node,
        this: Option<Node>,
        args: Vec<Node>,
        span: Span,
    ) -> Node {
        let site = self.simple_site(span, args.len());
        self.copy(callee, site.callee);
        if let Some(this) = this {
            self.copy(this, site.this);
        }
        for (arg, param) in args.iter().zip(&site.args) {
            self.copy(*arg, *param);
        }
        site.result
    }

    fn root_call(&mut self, builtin: Builtin, call: &Call) {
        let first = call.args.first().copied().or(call.spread);
        match builtin {
            Builtin::ObjectCtor => {
                let (_, node) = self.fresh(call.span, SiteKind::Fresh, &[Builtin::ObjectProto]);
                if let Some(result) = call.result {
                    self.copy(node, result);
                    if let Some(first) = first {
                        self.copy(first, result);
                    }
                }
            }
            Builtin::ArrayCtor => {
                let elements = self.fresh_array(call.span, call.result);
                for arg in call.args.iter().chain(&call.spread) {
                    self.copy(*arg, elements);
                }
            }
            Builtin::StringCtor | Builtin::NumberCtor | Builtin::BooleanCtor => self.pure(call),
            Builtin::SymbolCtor => {
                if let Some(first) = first {
                    self.implicit(first);
                }
                if let Some(result) = call.result {
                    self.add_builtin(result, Builtin::PrimSymbol);
                }
            }
            Builtin::FunctionCtor => {
                self.generic(call);
                let global = self.builtin_node(Builtin::Global);
                self.escape(global);
                self.returns(Some(global), call.result);
            }
            Builtin::Other => self.generic(call),
            Builtin::CollectionCtor => {
                let (object, node) =
                    self.fresh(call.span, SiteKind::Collection, &[Builtin::CollectionProto]);
                if let Some(first) = first {
                    let items = self.iterate(first);
                    let entries = self.field_node(object, Field::ENTRIES);
                    self.copy(items, entries);
                    self.load(items, ELEMENTS, entries);
                }
                if let Some(result) = call.result {
                    self.copy(node, result);
                }
            }
            Builtin::Machinery => {
                if let Some(result) = call.result {
                    self.add_builtin(result, Builtin::Machinery);
                    self.add_builtin(result, Builtin::PrimString);
                }
            }
            _ => {}
        }
    }

    fn pure(&mut self, call: &Call) {
        let values: Vec<Node> = call
            .this
            .iter()
            .chain(&call.args)
            .chain(&call.spread)
            .copied()
            .collect();
        for value in values {
            self.implicit(value);
        }
        self.prims(call.result);
        if call.result.is_some() {
            let elements = self.fresh_array(call.span, call.result);
            self.add_builtin(elements, Builtin::PrimString);
        }
    }

    fn patch(&mut self, index: u32, method: Method, target: Option<Node>, key: PatchKey) {
        let call = &self.calls[index as usize];
        let needed = match method {
            Method::DefineProperty => 3,
            Method::SetPrototypeOf | Method::DefineGetter | Method::DefineSetter => 2,
            Method::DefineProperties | Method::Assign => 2,
            _ => 1,
        };
        if call.spread.is_none() && call.args.len() < needed {
            return;
        }
        let Some(target) = target else {
            return;
        };
        let key = if call.spread.is_some() {
            PatchKey::Whole
        } else {
            key
        };
        let span = self.calls[index as usize].span;
        self.record_patch(index, method, Patch { span, target, key });
    }

    fn arg_key(&mut self, arg: Option<Node>) -> (Key, PatchKey) {
        match arg {
            Some(node) => {
                self.implicit(node);
                (Key::Dynamic(node), PatchKey::Node(node))
            }
            None => (Key::Static(self.name("undefined")), PatchKey::Absent),
        }
    }

    fn method(&mut self, index: u32, method: Method, call: &Call) {
        let arg = |position: usize| call.args.get(position).copied().or(call.spread);
        let this = call.this;
        let result = call.result;
        match method {
            Method::DefineProperty => {
                let (key, patch_key) = self.arg_key(arg(1));
                self.patch(index, method, arg(0), patch_key);
                if let (Some(target), Some(descriptor)) = (arg(0), arg(2)) {
                    self.define_one(target, key, descriptor, call.span);
                }
                self.returns(arg(0), result);
            }
            Method::DefineProperties => {
                let props: Vec<Node> = arg(1).into_iter().collect();
                self.patch(index, method, arg(0), PatchKey::Fields(props));
                if let (Some(target), Some(props)) = (arg(0), arg(1)) {
                    self.define_fields(props, target);
                }
                self.returns(arg(0), result);
            }
            Method::Assign => {
                let sources: Vec<Node> = call.args.iter().skip(1).copied().collect();
                let mut key_sources = sources.clone();
                key_sources.extend(call.spread);
                self.patch(index, method, arg(0), PatchKey::Fields(key_sources));
                if let Some(target) = arg(0) {
                    for source in sources {
                        self.assign_fields(source, target);
                    }
                    if let Some(spread) = call.spread {
                        let world = self.world;
                        let fields = self.loaded(spread, Field::OWN);
                        self.store(target, UNKNOWN, fields);
                        self.store(target, UNKNOWN, spread);
                        self.store(target, UNKNOWN, world);
                    }
                }
                self.returns(arg(0), result);
            }
            Method::SetPrototypeOf => {
                self.patch(index, method, arg(0), PatchKey::Whole);
                if let (Some(target), Some(proto)) = (arg(0), arg(1)) {
                    self.proto_set(target, proto);
                }
                self.returns(arg(0), result);
            }
            Method::ProtoSetter => {
                self.patch(index, method, this, PatchKey::Whole);
                if let (Some(target), Some(proto)) = (this, arg(0)) {
                    self.proto_set(target, proto);
                }
            }
            Method::ReflectSet => {
                let (key, patch_key) = self.arg_key(arg(1));
                let targets = match (arg(0), arg(3)) {
                    (Some(target), Some(receiver)) => {
                        let (both, _) = self.derived(6, vec![target, receiver], call.span);
                        self.copy(target, both);
                        self.copy(receiver, both);
                        self.add_receiver(target, receiver);
                        Some(both)
                    }
                    (target, _) => target,
                };
                self.patch(index, method, targets, patch_key);
                if let (Some(target), Some(value)) = (targets, arg(2)) {
                    self.store(target, key, value);
                }
                self.prims(result);
            }
            Method::ReflectDeleteProperty => {
                let (_, patch_key) = self.arg_key(arg(1));
                self.patch(index, method, arg(0), patch_key);
                self.prims(result);
            }
            Method::DefineGetter | Method::DefineSetter => {
                let (key, patch_key) = self.arg_key(arg(0));
                self.patch(index, method, this, patch_key);
                if let (Some(target), Some(function)) = (this, arg(1)) {
                    if method == Method::DefineGetter {
                        let (value, fresh) = self.derived(2, vec![target, function], call.span);
                        if fresh {
                            self.getter_call(target, function, value, call.span);
                        }
                        self.store(target, key, value);
                    } else {
                        self.setter_call(target, key, function, call.span);
                    }
                    self.store(target, ACCESSORS, function);
                }
            }
            _ => self.query_method(method, call),
        }
    }

    fn proto_accessors(&mut self, to: Node) {
        self.add_builtin(to, Builtin::Method(Method::ProtoGetter));
        self.add_builtin(to, Builtin::Method(Method::ProtoSetter));
    }

    fn returns(&mut self, value: Option<Node>, result: Option<Node>) {
        if let (Some(value), Some(result)) = (value, result) {
            self.copy(value, result);
        }
    }

    fn query_method(&mut self, method: Method, call: &Call) {
        let arg = |position: usize| call.args.get(position).copied().or(call.spread);
        let this = call.this;
        let result = call.result;
        let span = call.span;
        match method {
            Method::Create => {
                let (object, node) = self.fresh(span, SiteKind::Fresh, &[]);
                let proto = self.proto_node(object);
                if let Some(first) = arg(0) {
                    self.copy(first, proto);
                }
                if let Some(props) = arg(1) {
                    self.define_fields(props, node);
                }
                self.returns(Some(node), result);
            }
            Method::GetPrototypeOf => {
                if let (Some(target), Some(result)) = (arg(0), result) {
                    self.load_named(target, "__proto__", result);
                }
            }
            Method::GetOwnPropertyDescriptor | Method::GetOwnPropertyDescriptors => {
                let (_, descriptor) =
                    self.fresh(span, SiteKind::Descriptor, &[Builtin::ObjectProto]);
                let value = self.node();
                if let Some(target) = arg(0) {
                    let key = if method == Method::GetOwnPropertyDescriptor {
                        self.arg_key(arg(1)).0
                    } else {
                        UNKNOWN
                    };
                    self.load(target, key, value);
                    self.load(target, ACCESSORS, value);
                    self.proto_accessors(value);
                }
                for name in ["value", "get", "set"] {
                    self.store_named(descriptor, name, value);
                }
                if method == Method::GetOwnPropertyDescriptor {
                    self.returns(Some(descriptor), result);
                } else {
                    let (holder, node) = self.fresh(span, SiteKind::Fresh, &[Builtin::ObjectProto]);
                    let fields = self.field_node(holder, Field::UNKNOWN);
                    self.copy(descriptor, fields);
                    self.returns(Some(node), result);
                }
            }
            Method::Keys => {
                let elements = self.fresh_array(span, result);
                if let Some(target) = arg(0) {
                    self.keys_of(target, elements);
                }
            }
            Method::Values => {
                let elements = self.fresh_array(span, result);
                if let Some(target) = arg(0) {
                    self.load(target, OWN, elements);
                }
            }
            Method::Entries => {
                let elements = self.fresh_array(span, result);
                let (pair, pair_node) = self.fresh(span, SiteKind::Rest, &[Builtin::ArrayProto]);
                self.copy(pair_node, elements);
                let pair_fields = self.field_node(pair, Field::INDEX);
                if let Some(target) = arg(0) {
                    self.load(target, OWN, pair_fields);
                    self.keys_of(target, pair_fields);
                }
            }
            Method::FromEntries => {
                let (object, node) = self.fresh(span, SiteKind::Fresh, &[Builtin::ObjectProto]);
                let fields = self.field_node(object, Field::UNKNOWN);
                if let Some(source) = arg(0) {
                    let entries = self.iterate(source);
                    self.load(entries, ELEMENTS, fields);
                }
                self.returns(Some(node), result);
            }
            Method::ReturnsFirst => self.returns(arg(0), result),
            Method::ReflectGet => {
                if let (Some(target), Some(receiver)) = (arg(0), arg(2)) {
                    self.add_receiver(target, receiver);
                }
                if let (Some(target), Some(result)) = (arg(0), result) {
                    let key = self.arg_key(arg(1)).0;
                    self.load(target, key, result);
                }
            }
            Method::ReflectApply | Method::ReflectConstruct => {
                let (callee, this, list) = if method == Method::ReflectApply {
                    (arg(0), arg(1), arg(2))
                } else {
                    (arg(0), None, arg(1))
                };
                if let Some(callee) = callee {
                    let spread = list.map(|list| self.loaded(list, Field::ELEMENTS));
                    self.live_call(Call {
                        callee,
                        this,
                        args: Vec::new(),
                        spread,
                        result,
                        construct: method == Method::ReflectConstruct,
                        loose: true,
                        firm: false,
                        span,
                    });
                }
                if let (Method::ReflectConstruct, Some(new_target)) = (method, arg(2)) {
                    self.escape(new_target);
                    if let Some(result) = result {
                        let prototype = self.node();
                        self.load_named(new_target, "prototype", prototype);
                        self.proto_set(result, prototype);
                    }
                }
            }
            Method::LookupAccessor => {
                if let Some(key) = arg(0) {
                    self.implicit(key);
                }
                if let (Some(target), Some(result)) = (this, result) {
                    self.load(target, ACCESSORS, result);
                    self.proto_accessors(result);
                }
            }
            Method::ProtoGetter => {
                if let (Some(target), Some(result)) = (this, result) {
                    self.load_named(target, "__proto__", result);
                }
            }
            _ => self.function_method(method, call),
        }
    }

    fn function_method(&mut self, method: Method, call: &Call) {
        let arg = |position: usize| call.args.get(position).copied().or(call.spread);
        let this = call.this;
        let result = call.result;
        let span = call.span;
        match method {
            Method::FnCall => {
                if let Some(target) = this {
                    self.live_call(Call {
                        callee: target,
                        this: arg(0).or(call.spread),
                        args: call.args.iter().skip(1).copied().collect(),
                        spread: call.spread,
                        result,
                        construct: false,
                        loose: !call.firm,
                        firm: false,
                        span,
                    });
                }
            }
            Method::FnApply => {
                if let Some(target) = this {
                    let spread = match (arg(1), call.spread) {
                        (Some(list), _) => Some(self.loaded(list, Field::ELEMENTS)),
                        (None, spread) => spread,
                    };
                    self.live_call(Call {
                        callee: target,
                        this: arg(0).or(call.spread),
                        args: Vec::new(),
                        spread,
                        result,
                        construct: false,
                        loose: !call.firm,
                        firm: false,
                        span,
                    });
                }
            }
            Method::FnBind => {
                let Some(target) = this else {
                    return;
                };
                if call.spread.is_some() {
                    self.generic(call);
                    return;
                }
                let (bound, node) = self.fresh(span, SiteKind::Bound, &[Builtin::FunctionProto]);
                let args: Vec<Node> = call.args.iter().skip(1).copied().collect();
                let loose = arg(0).is_none() || !call.firm;
                self.bind_into(bound, target, arg(0), args, loose);
                self.returns(Some(node), result);
            }
            Method::ArrayFrom | Method::ArrayOf => {
                let number = self.builtin_node(Builtin::PrimNumber);
                let fields = self.fresh_array(span, result);
                if method == Method::ArrayOf {
                    for value in call.args.iter().chain(&call.spread) {
                        self.copy(*value, fields);
                    }
                    return;
                }
                let Some(source) = arg(0) else {
                    return;
                };
                let items = self.iterate(source);
                self.load(source, ELEMENTS, items);
                match arg(1) {
                    Some(mapper) => {
                        let mapped = self.simple_call(mapper, arg(2), vec![items, number], span);
                        self.copy(mapped, fields);
                    }
                    None => self.copy(items, fields),
                }
            }
            Method::FnToString => {
                if let Some(result) = result {
                    self.add_builtin(result, Builtin::PrimString);
                }
            }
            Method::Pure => self.pure(call),
            Method::ReturnsThis => self.returns(this, result),
            Method::StringPattern => {
                self.pattern_hook(call);
                self.pure(call);
            }
            Method::StringReplace => {
                self.pattern_hook(call);
                if let Some(pattern) = arg(0) {
                    self.implicit(pattern);
                }
                if let Some(replacer) = arg(1) {
                    self.implicit(replacer);
                    let text = self.builtin_node(Builtin::PrimString);
                    self.simple_call(replacer, None, vec![text, text, text], span);
                }
                if let Some(result) = result {
                    self.add_builtin(result, Builtin::PrimString);
                }
            }
            Method::JsonParse => {
                if call.args.len() > 1 || call.spread.is_some() {
                    self.generic(call);
                    return;
                }
                if let Some(text) = arg(0) {
                    self.implicit(text);
                }
                let (object, node) = self.fresh(
                    span,
                    SiteKind::Fresh,
                    &[Builtin::ObjectProto, Builtin::ArrayProto],
                );
                let fields = self.field_node(object, Field::UNKNOWN);
                self.copy(node, fields);
                self.prims(Some(fields));
                self.returns(Some(node), result);
                self.prims(result);
            }
            _ => self.array_method(method, call),
        }
    }

    fn pattern_hook(&mut self, call: &Call) {
        let Some(pattern) = call.args.first().copied().or(call.spread) else {
            return;
        };
        let hook = self.node();
        for name in ["@@replace", "@@split", "@@match", "@@matchAll", "@@search"] {
            self.load_named(pattern, name, hook);
        }
        let mut args: Vec<Node> = call.this.into_iter().collect();
        args.extend(call.args.iter().skip(1).copied());
        args.extend(call.spread);
        self.simple_call(hook, Some(pattern), args, call.span);
    }

    fn array_method(&mut self, method: Method, call: &Call) {
        let arg = |position: usize| call.args.get(position).copied().or(call.spread);
        let Some(this) = call.this else {
            return;
        };
        let result = call.result;
        let span = call.span;
        let elements = self.loaded(this, Field::ELEMENTS);
        let collection = matches!(
            method,
            Method::CollectionSet
                | Method::CollectionAdd
                | Method::CollectionGet
                | Method::CollectionEach
                | Method::CollectionIter
                | Method::IterNext
        );
        let entries = if collection {
            self.loaded(this, Field::ENTRIES)
        } else {
            self.empty
        };
        let number = self.builtin_node(Builtin::PrimNumber);
        let coerced = match method {
            Method::ArrayTake | Method::ArraySlice | Method::ArraySelf | Method::ArrayJoin => {
                Some(0..call.args.len())
            }
            Method::ArraySplice => Some(0..call.args.len().min(2)),
            Method::ArrayWith => Some(0..call.args.len().min(2)),
            Method::ArrayFill => Some(call.args.len().min(1)..call.args.len()),
            _ => None,
        };
        if let Some(range) = coerced {
            let values: Vec<Node> = call.args[range]
                .iter()
                .chain(&call.spread)
                .copied()
                .collect();
            for value in values {
                self.implicit(value);
            }
        }
        match method {
            Method::ArrayPush => {
                for value in call.args.iter().chain(&call.spread) {
                    self.store(this, INDEX, *value);
                }
                self.prims(result);
            }
            Method::ArrayTake => self.returns(Some(elements), result),
            Method::ArraySplice => {
                let fields = self.fresh_array(span, result);
                self.copy(elements, fields);
                for value in call.args.iter().skip(2).chain(&call.spread) {
                    self.store(this, INDEX, *value);
                }
            }
            Method::ArrayCopy => {
                let fields = self.fresh_array(span, result);
                self.copy(elements, fields);
                self.copy(this, fields);
                for value in call.args.iter().chain(&call.spread) {
                    self.copy(*value, fields);
                    self.load(*value, ELEMENTS, fields);
                }
            }
            Method::ArraySlice => {
                let fields = self.fresh_array(span, result);
                self.copy(elements, fields);
                self.load(elements, ELEMENTS, fields);
            }
            Method::ArrayWith => {
                let fields = self.fresh_array(span, result);
                self.copy(elements, fields);
                for value in call.args.iter().skip(1).chain(&call.spread) {
                    self.copy(*value, fields);
                }
            }
            Method::ArraySelf => self.returns(Some(this), result),
            Method::ArrayFill => {
                if let Some(value) = arg(0) {
                    self.store(this, INDEX, value);
                }
                self.returns(Some(this), result);
            }
            Method::ArraySort | Method::ArrayResort => {
                self.implicit(elements);
                if let Some(compare) = arg(0) {
                    self.simple_call(compare, None, vec![elements, elements], span);
                }
                if method == Method::ArraySort {
                    self.returns(Some(this), result);
                } else {
                    let fields = self.fresh_array(span, result);
                    self.copy(elements, fields);
                }
            }
            Method::ArrayEach(each) => self.each(each, call, elements),
            Method::ArrayReduce => {
                let (acc, _) = self.derived(1, vec![this], span);
                if let Some(initial) = arg(1) {
                    self.copy(initial, acc);
                }
                self.copy(elements, acc);
                if let Some(reducer) = arg(0) {
                    let value =
                        self.simple_call(reducer, None, vec![acc, elements, number, this], span);
                    self.copy(value, acc);
                }
                self.returns(Some(acc), result);
            }
            Method::ArrayJoin => {
                self.implicit(elements);
                if let Some(result) = result {
                    self.add_builtin(result, Builtin::PrimString);
                }
            }
            Method::ArrayIter | Method::CollectionIter => {
                let (iterator, node) =
                    self.fresh(span, SiteKind::Iterator, &[Builtin::IteratorProto]);
                let fields = self.field_node(iterator, Field::ENTRIES);
                let source = if method == Method::ArrayIter {
                    elements
                } else {
                    entries
                };
                self.copy(source, fields);
                let (pair, pair_node) = self.fresh(span, SiteKind::Rest, &[Builtin::ArrayProto]);
                let pair_fields = self.field_node(pair, Field::INDEX);
                self.copy(source, pair_fields);
                self.add_builtin(pair_fields, Builtin::PrimNumber);
                self.copy(pair_node, fields);
                self.returns(Some(node), result);
            }
            Method::CollectionSet | Method::CollectionAdd => {
                for value in call.args.iter().take(2).chain(&call.spread) {
                    self.store(this, ENTRIES, *value);
                }
                self.returns(Some(this), result);
            }
            Method::CollectionGet => self.returns(Some(entries), result),
            Method::CollectionEach => {
                if let Some(visitor) = arg(0) {
                    self.simple_call(visitor, arg(1), vec![entries, entries, this], span);
                }
            }
            Method::IterNext => {
                let (_, step) = self.fresh(span, SiteKind::Fresh, &[Builtin::ObjectProto]);
                self.store_named(step, "value", entries);
                self.returns(Some(step), result);
            }
            _ => {}
        }
    }

    fn each(&mut self, each: Each, call: &Call, elements: Node) {
        let Some(this) = call.this else {
            return;
        };
        let number = self.builtin_node(Builtin::PrimNumber);
        let returned = match call.args.first().or(call.spread.as_ref()) {
            Some(visitor) => {
                let this_arg = call.args.get(1).copied().or(call.spread);
                self.simple_call(*visitor, this_arg, vec![elements, number, this], call.span)
            }
            None => self.empty,
        };
        let result = call.result;
        match each {
            Each::Visit => self.prims(result),
            Each::Find => self.returns(Some(elements), result),
            Each::Map | Each::Filter | Each::FlatMap => {
                let fields = self.fresh_array(call.span, result);
                match each {
                    Each::Filter => self.copy(elements, fields),
                    Each::FlatMap => {
                        self.copy(returned, fields);
                        self.load(returned, ELEMENTS, fields);
                    }
                    _ => self.copy(returned, fields),
                }
            }
        }
    }
}
