use oxc_span::Span;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) struct ObjectId(pub(super) u32);

impl ObjectId {
    pub(super) const WORLD: ObjectId = ObjectId(0);
    pub(super) const FUNCTIONS: ObjectId = ObjectId(1);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Builtin {
    Global,
    ObjectCtor,
    ObjectProto,
    FunctionCtor,
    FunctionProto,
    ArrayCtor,
    ArrayProto,
    StringCtor,
    StringProto,
    NumberCtor,
    NumberProto,
    BooleanCtor,
    BooleanProto,
    SymbolCtor,
    SymbolProto,
    Reflect,
    Console,
    MathNs,
    Json,
    CollectionCtor,
    CollectionProto,
    IteratorProto,
    Other,
    PrimString,
    PrimNumber,
    PrimBoolean,
    PrimSymbol,
    PrimUndefined,
    PrimNull,
    Machinery,
    Method(Method),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Each {
    Visit,
    Map,
    Filter,
    Find,
    FlatMap,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Method {
    DefineProperty,
    DefineProperties,
    Assign,
    SetPrototypeOf,
    Create,
    GetPrototypeOf,
    GetOwnPropertyDescriptor,
    GetOwnPropertyDescriptors,
    Keys,
    Values,
    Entries,
    FromEntries,
    ReturnsFirst,
    ReflectSet,
    ReflectGet,
    ReflectDeleteProperty,
    ReflectApply,
    ReflectConstruct,
    DefineGetter,
    DefineSetter,
    LookupAccessor,
    ProtoGetter,
    ProtoSetter,
    FnCall,
    FnApply,
    FnBind,
    FnToString,
    Pure,
    ReturnsThis,
    ArrayFrom,
    ArrayOf,
    ArrayPush,
    ArrayTake,
    ArraySplice,
    ArrayCopy,
    ArraySlice,
    ArrayWith,
    ArrayResort,
    ArraySelf,
    ArrayFill,
    ArraySort,
    ArrayEach(Each),
    ArrayReduce,
    ArrayJoin,
    ArrayIter,
    StringReplace,
    StringPattern,
    CollectionSet,
    CollectionAdd,
    CollectionGet,
    CollectionEach,
    CollectionIter,
    IterNext,
    JsonParse,
}

impl Builtin {
    pub(super) fn is_primitive(self) -> bool {
        matches!(
            self,
            Builtin::PrimString
                | Builtin::PrimNumber
                | Builtin::PrimBoolean
                | Builtin::PrimSymbol
                | Builtin::PrimUndefined
                | Builtin::PrimNull
        )
    }

    pub(super) fn is_method(self) -> bool {
        matches!(self, Builtin::Method(_))
    }

    pub(super) fn patchable(self) -> bool {
        !self.is_primitive() && self != Builtin::Machinery
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum SiteKind {
    Object,
    Array,
    Function,
    FunctionPrototype,
    Instance,
    Fresh,
    FreshArray,
    Arguments,
    Bound,
    Regex,
    Collection,
    Descriptor,
    Iterator,
    Rest,
}

#[derive(Clone, Debug)]
pub(super) enum Obj {
    World,
    Functions,
    Builtin(Builtin),
    Str(Box<str>),
    Site(Span, SiteKind),
}

#[derive(Default)]
pub(super) struct Objects {
    table: Vec<Obj>,
    builtins: FxHashMap<Builtin, ObjectId>,
    strings: FxHashMap<Box<str>, ObjectId>,
    sites: FxHashMap<(Span, SiteKind), ObjectId>,
}

impl Objects {
    pub(super) fn new() -> Self {
        let mut objects = Self::default();
        objects.table.push(Obj::World);
        objects.table.push(Obj::Functions);
        objects
    }

    fn push(&mut self, obj: Obj) -> ObjectId {
        let id = ObjectId(self.table.len() as u32);
        self.table.push(obj);
        id
    }

    pub(super) fn builtin(&mut self, builtin: Builtin) -> ObjectId {
        if let Some(id) = self.builtins.get(&builtin) {
            return *id;
        }
        let id = self.push(Obj::Builtin(builtin));
        self.builtins.insert(builtin, id);
        id
    }

    pub(super) fn string(&mut self, text: &str) -> ObjectId {
        if let Some(id) = self.strings.get(text) {
            return *id;
        }
        let id = self.push(Obj::Str(text.into()));
        self.strings.insert(text.into(), id);
        id
    }

    pub(super) fn site(&mut self, span: Span, kind: SiteKind) -> (ObjectId, bool) {
        if let Some(id) = self.sites.get(&(span, kind)) {
            return (*id, false);
        }
        let id = self.push(Obj::Site(span, kind));
        self.sites.insert((span, kind), id);
        (id, true)
    }

    pub(super) fn get(&self, id: ObjectId) -> &Obj {
        &self.table[id.0 as usize]
    }

    pub(super) fn as_builtin(&self, id: ObjectId) -> Option<Builtin> {
        match self.get(id) {
            Obj::Builtin(builtin) => Some(*builtin),
            _ => None,
        }
    }

    pub(super) fn key_text(&self, id: ObjectId) -> Option<&str> {
        match self.get(id) {
            Obj::Str(text) => Some(text),
            Obj::Builtin(Builtin::PrimUndefined) => Some("undefined"),
            Obj::Builtin(Builtin::PrimNull) => Some("null"),
            _ => None,
        }
    }

    pub(super) fn is_site(&self, id: ObjectId) -> bool {
        matches!(self.get(id), Obj::Site(..))
    }

    pub(super) fn describe(&self, id: ObjectId) -> String {
        match self.get(id) {
            Obj::World => "world".to_string(),
            Obj::Functions => "every program function".to_string(),
            Obj::Builtin(builtin) => format!("builtin {builtin:?}"),
            Obj::Str(text) => format!("string {text:?}"),
            Obj::Site(span, kind) => format!("{kind:?}@{}..{}", span.start, span.end),
        }
    }
}
