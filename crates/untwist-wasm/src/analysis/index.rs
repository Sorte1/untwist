use rustc_hash::FxHashMap;
use serde::Serialize;

pub const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Plain,
    Property,
    Import,
    JsxTag,
}

#[derive(Clone, Copy, Debug)]
pub struct Occurrence {
    pub start: u32,
    pub end: u32,
    pub symbol: u32,
    pub scope: u32,
    pub name: u32,
    pub shape: Shape,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Var,
    Let,
    Const,
    Function,
    Class,
    Import,
    Other,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Var => "var",
            Kind::Let => "let",
            Kind::Const => "const",
            Kind::Function => "function",
            Kind::Class => "class",
            Kind::Import => "import",
            Kind::Other => "binding",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub module: Box<str>,
    pub name: Box<str>,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: u32,
    pub decl: [u32; 2],
    pub scope: u32,
    pub kind: Kind,
    pub exported: bool,
    pub link: Option<Link>,
}

#[derive(Clone, Copy, Debug)]
pub struct Scope {
    pub parent: u32,
    pub start: u32,
    pub end: u32,
    pub last_descendant: u32,
    pub hazard: bool,
    pub var_scope: bool,
    pub strict: bool,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub start: u32,
    pub end: u32,
    pub object: u32,
    pub name: Box<str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Problem {
    pub message: String,
    pub severity: &'static str,
    pub from: u32,
    pub to: u32,
}

#[derive(Default)]
pub struct Names {
    pub list: Vec<Box<str>>,
    ids: FxHashMap<Box<str>, u32>,
}

impl Names {
    pub fn intern(&mut self, name: &str) -> u32 {
        if let Some(id) = self.ids.get(name) {
            return *id;
        }
        let id = u32::try_from(self.list.len()).unwrap_or(NONE);
        self.list.push(name.into());
        self.ids.insert(name.into(), id);
        id
    }

    pub fn find(&self, name: &str) -> Option<u32> {
        self.ids.get(name).copied()
    }

    pub fn get(&self, id: u32) -> &str {
        self.list.get(id as usize).map_or("", |name| name)
    }
}

pub struct Index {
    pub occurrences: Vec<Occurrence>,
    pub by_symbol_start: Vec<u32>,
    pub by_symbol: Vec<u32>,
    pub symbols: Vec<Symbol>,
    pub scopes: Vec<Scope>,
    pub scopes_by_start: Vec<u32>,
    pub bindings: Vec<(u32, u32, u32)>,
    pub members: Vec<Member>,
    pub exports: Vec<(Box<str>, [u32; 2])>,
    pub names: Names,
    pub problems: Vec<Problem>,
}

impl Index {
    pub fn occurrence_at(&self, pos: u32) -> Option<&Occurrence> {
        let after = self.occurrences.partition_point(|occ| occ.start <= pos);
        let occ = self.occurrences.get(after.checked_sub(1)?)?;
        (pos <= occ.end).then_some(occ)
    }

    pub fn member_at(&self, pos: u32) -> Option<&Member> {
        let after = self.members.partition_point(|member| member.start <= pos);
        let member = self.members.get(after.checked_sub(1)?)?;
        (pos <= member.end).then_some(member)
    }

    pub fn symbol(&self, id: u32) -> Option<&Symbol> {
        self.symbols.get(id as usize)
    }

    pub fn scope(&self, id: u32) -> Option<&Scope> {
        self.scopes.get(id as usize)
    }

    pub fn occurrences_of(&self, symbol: u32) -> impl Iterator<Item = &Occurrence> + '_ {
        let from = self
            .by_symbol_start
            .get(symbol as usize)
            .copied()
            .unwrap_or(0) as usize;
        let to = self
            .by_symbol_start
            .get(symbol as usize + 1)
            .copied()
            .unwrap_or(0) as usize;
        self.by_symbol
            .get(from..to)
            .unwrap_or_default()
            .iter()
            .filter_map(|index| self.occurrences.get(*index as usize))
    }

    pub fn is_within(&self, scope: u32, ancestor: u32) -> bool {
        self.scope(ancestor)
            .is_some_and(|outer| ancestor <= scope && scope <= outer.last_descendant)
    }

    pub fn binding(&self, scope: u32, name: u32) -> Option<u32> {
        let at = self
            .bindings
            .partition_point(|binding| (binding.0, binding.1) < (scope, name));
        self.bindings
            .get(at)
            .filter(|binding| binding.0 == scope && binding.1 == name)
            .map(|binding| binding.2)
    }

    pub fn bindings_of(&self, scope: u32) -> &[(u32, u32, u32)] {
        let from = self.bindings.partition_point(|binding| binding.0 < scope);
        let to = self.bindings.partition_point(|binding| binding.0 <= scope);
        self.bindings.get(from..to).unwrap_or_default()
    }

    pub fn scope_at(&self, pos: u32) -> u32 {
        let after = self
            .scopes_by_start
            .partition_point(|id| self.scope(*id).is_some_and(|scope| scope.start <= pos));
        let mut current = after
            .checked_sub(1)
            .and_then(|index| self.scopes_by_start.get(index))
            .copied()
            .unwrap_or(0);
        while let Some(scope) = self.scope(current) {
            if (scope.start <= pos && pos <= scope.end) || scope.parent == NONE {
                return current;
            }
            current = scope.parent;
        }
        0
    }

    pub fn ancestors(&self, from: u32) -> impl Iterator<Item = u32> + '_ {
        std::iter::successors(Some(from), |id| {
            self.scope(*id)
                .map(|scope| scope.parent)
                .filter(|parent| *parent != NONE)
        })
    }
}
