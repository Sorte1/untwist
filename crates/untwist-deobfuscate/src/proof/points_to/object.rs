use oxc_span::Span;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) struct ObjectId(u32);

impl ObjectId {
    pub(super) const WORLD: ObjectId = ObjectId(0);

    pub(super) fn builtin(root: Builtin) -> ObjectId {
        ObjectId(1 + root as u32)
    }

    pub(super) fn is_world(self) -> bool {
        self == Self::WORLD
    }

    pub(super) fn as_builtin(self) -> Option<Builtin> {
        if self == Self::WORLD {
            return None;
        }
        Builtin::from_index(self.0 - 1)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Builtin {
    Global,
    ObjectCtor,
    ArrayCtor,
    FunctionCtor,
    Reflect,
    StringProto,
    NumberProto,
    BooleanProto,
    SymbolProto,
    FunctionProto,
    ObjectProto,
    ArrayProto,
    Console,
    Other,
}

impl Builtin {
    const ALL: [Builtin; 14] = [
        Builtin::Global,
        Builtin::ObjectCtor,
        Builtin::ArrayCtor,
        Builtin::FunctionCtor,
        Builtin::Reflect,
        Builtin::StringProto,
        Builtin::NumberProto,
        Builtin::BooleanProto,
        Builtin::SymbolProto,
        Builtin::FunctionProto,
        Builtin::ObjectProto,
        Builtin::ArrayProto,
        Builtin::Console,
        Builtin::Other,
    ];

    pub(super) fn from_index(index: u32) -> Option<Builtin> {
        Self::ALL.get(index as usize).copied()
    }

    pub(super) fn all() -> impl Iterator<Item = Builtin> {
        Self::ALL.into_iter()
    }
}

/// A program allocation site. Objects created at the same syntactic site collapse into one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Site(u32);

#[derive(Default)]
pub(super) struct Sites {
    spans: Vec<Span>,
}

impl Sites {
    pub(super) fn intern(&mut self, span: Span) -> Site {
        if let Some(index) = self.spans.iter().position(|existing| *existing == span) {
            return Site(index as u32);
        }
        let index = self.spans.len();
        self.spans.push(span);
        Site(index as u32)
    }

    pub(super) fn object(&self, site: Site) -> ObjectId {
        ObjectId(1 + Builtin::ALL.len() as u32 + site.0)
    }
}
