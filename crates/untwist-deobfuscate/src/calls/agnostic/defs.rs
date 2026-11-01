use oxc_ast::ast::{Expression, Function};
use oxc_syntax::symbol::SymbolId;

pub(super) const CALLABLE: u16 = 1;
pub(super) const CONSTRUCTS_FRESH: u16 = 1 << 1;
pub(super) const OPAQUE: u16 = 1 << 2;
pub(super) const ESCAPES: u16 = 1 << 3;
pub(super) const FIELDS_WRITTEN: u16 = 1 << 4;
pub(super) const METHOD_CALLED: u16 = 1 << 5;
pub(super) const MUTATED: u16 = 1 << 6;
pub(super) const ESCAPING_REF: u16 = 1 << 7;
const PRESENT: u16 = 1 << 8;

type Value<'a> = Option<&'a Expression<'a>>;

#[derive(Clone, Copy, Default)]
pub(super) struct Def {
    func: u32,
    pub(super) refs: u32,
    pub(super) callee_refs: u32,
    start: u32,
    inits: u32,
    flags: u16,
}

impl Def {
    pub(super) fn set(&mut self, flag: u16) {
        self.flags |= flag;
    }

    pub(super) fn set_to(&mut self, flag: u16, on: bool) {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    fn has(self, flag: u16) -> bool {
        self.flags & flag != 0
    }
}

#[derive(Clone, Copy)]
pub(super) struct Defs<'b, 'a> {
    pub(super) inits: &'b [Value<'a>],
    pub(super) writes: &'b [Value<'a>],
    pub(super) func: Option<&'a Function<'a>>,
    pub(super) refs: u32,
    pub(super) callee_refs: u32,
    pub(super) callable: bool,
    pub(super) constructs_fresh: bool,
    pub(super) opaque: bool,
    pub(super) escapes: bool,
    pub(super) fields_written: bool,
    pub(super) method_called: bool,
    pub(super) mutated: bool,
    pub(super) escaping_ref: bool,
}

pub(super) struct DefTable<'a> {
    defs: Vec<Def>,
    funcs: Vec<&'a Function<'a>>,
    log: Vec<(u32, bool, Value<'a>)>,
    values: Vec<Value<'a>>,
}

impl<'a> DefTable<'a> {
    pub(super) fn new(symbols: usize) -> Self {
        Self {
            defs: vec![Def::default(); symbols],
            funcs: Vec::new(),
            log: Vec::new(),
            values: Vec::new(),
        }
    }

    pub(super) fn entry(&mut self, symbol: SymbolId) -> &mut Def {
        let index = symbol.index();
        if index >= self.defs.len() {
            self.defs.resize(index + 1, Def::default());
        }
        let def = &mut self.defs[index];
        def.set(PRESENT);
        def
    }

    pub(super) fn set_func(&mut self, symbol: SymbolId, func: &'a Function<'a>) {
        let slot = self.entry(symbol).func;
        if let Some(stored) = slot
            .checked_sub(1)
            .and_then(|at| self.funcs.get_mut(at as usize))
        {
            *stored = func;
            return;
        }
        self.funcs.push(func);
        let ordinal = u32::try_from(self.funcs.len()).unwrap_or(0);
        self.entry(symbol).func = ordinal;
    }

    pub(super) fn func(&self, symbol: SymbolId) -> Option<&'a Function<'a>> {
        self.defs
            .get(symbol.index())
            .and_then(|def| self.func_of(*def))
    }

    fn func_of(&self, def: Def) -> Option<&'a Function<'a>> {
        let at = def.func.checked_sub(1)?;
        self.funcs.get(at as usize).copied()
    }

    pub(super) fn push_init(&mut self, symbol: SymbolId, value: Value<'a>) {
        self.entry(symbol);
        self.log.push((index_u32(symbol), false, value));
    }

    pub(super) fn push_write(&mut self, symbol: SymbolId, value: Value<'a>) {
        self.entry(symbol);
        self.log.push((index_u32(symbol), true, value));
    }

    pub(super) fn finish(&mut self) {
        let mut log = std::mem::take(&mut self.log);
        log.sort_by_key(|(symbol, write, _)| (*symbol, *write));
        let mut cursor = 0usize;
        for (index, def) in self.defs.iter_mut().enumerate() {
            def.start = u32::try_from(cursor).unwrap_or(u32::MAX);
            let owned = log[cursor..]
                .iter()
                .take_while(|(symbol, _, _)| *symbol as usize == index)
                .count();
            let inits = log[cursor..cursor + owned]
                .iter()
                .take_while(|(_, write, _)| !write)
                .count();
            def.inits = u32::try_from(inits).unwrap_or(u32::MAX);
            cursor += owned;
        }
        self.values = log.iter().map(|(_, _, value)| *value).collect();
    }

    pub(super) fn get(&self, symbol: SymbolId) -> Option<Defs<'_, 'a>> {
        self.view(symbol.index())
    }

    pub(super) fn present(&self) -> impl Iterator<Item = (SymbolId, Defs<'_, 'a>)> + '_ {
        (0..self.defs.len())
            .filter_map(|index| Some((SymbolId::from_usize(index), self.view(index)?)))
    }

    pub(super) fn each_present_mut(&mut self, mut each: impl FnMut(SymbolId, &mut Def)) {
        for (index, def) in self.defs.iter_mut().enumerate() {
            if def.has(PRESENT) {
                each(SymbolId::from_usize(index), def);
            }
        }
    }

    fn view(&self, index: usize) -> Option<Defs<'_, 'a>> {
        let def = *self.defs.get(index)?;
        if !def.has(PRESENT) {
            return None;
        }
        let start = def.start as usize;
        let mid = start + def.inits as usize;
        let end = self
            .defs
            .get(index + 1)
            .map_or(self.values.len(), |next| next.start as usize);
        Some(Defs {
            inits: self.values.get(start..mid).unwrap_or_default(),
            writes: self.values.get(mid..end).unwrap_or_default(),
            func: self.func_of(def),
            refs: def.refs,
            callee_refs: def.callee_refs,
            callable: def.has(CALLABLE),
            constructs_fresh: def.has(CONSTRUCTS_FRESH),
            opaque: def.has(OPAQUE),
            escapes: def.has(ESCAPES),
            fields_written: def.has(FIELDS_WRITTEN),
            method_called: def.has(METHOD_CALLED),
            mutated: def.has(MUTATED),
            escaping_ref: def.has(ESCAPING_REF),
        })
    }
}

fn index_u32(symbol: SymbolId) -> u32 {
    u32::try_from(symbol.index()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use untwist_core::parse::{parse_with, SourceType};

    fn symbol(index: usize) -> SymbolId {
        SymbolId::from_usize(index)
    }

    #[test]
    fn each_symbol_sees_its_own_inits_then_writes_in_push_order() {
        let alloc = Allocator::default();
        let parsed = parse_with(&alloc, "1; 2;", SourceType::cjs());
        let values: Vec<&Expression<'_>> = parsed
            .program
            .body
            .iter()
            .filter_map(|statement| match statement {
                Statement::ExpressionStatement(it) => Some(&it.expression),
                _ => None,
            })
            .collect();
        let mut table = DefTable::new(4);
        table.push_write(symbol(2), Some(values[1]));
        table.push_init(symbol(0), Some(values[0]));
        table.push_init(symbol(2), None);
        table.push_init(symbol(2), Some(values[0]));
        table.entry(symbol(3)).refs += 1;
        table.finish();
        let first = table.get(symbol(0)).unwrap();
        assert_eq!(first.inits.len(), 1);
        assert!(first.writes.is_empty());
        let third = table.get(symbol(2)).unwrap();
        assert_eq!(third.inits.len(), 2);
        assert!(third.inits[0].is_none());
        assert!(third.writes[0].is_some_and(|value| std::ptr::eq(value, values[1])));
        assert!(table.get(symbol(1)).is_none());
        let fourth = table.get(symbol(3)).unwrap();
        assert!(fourth.inits.is_empty() && fourth.writes.is_empty());
        assert_eq!(fourth.refs, 1);
        assert_eq!(table.present().count(), 3);
    }

    #[test]
    fn flags_are_kept_per_symbol() {
        let mut table = DefTable::new(2);
        table.entry(symbol(1)).set(CALLABLE | OPAQUE);
        table.entry(symbol(1)).set_to(OPAQUE, false);
        table.finish();
        let defs = table.get(symbol(1)).unwrap();
        assert!(defs.callable && !defs.opaque && !defs.mutated);
        assert!(table.func(symbol(1)).is_none());
        assert!(table.get(symbol(0)).is_none());
    }
}
