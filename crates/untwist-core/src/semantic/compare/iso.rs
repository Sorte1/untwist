use std::fmt::Debug;

use oxc_semantic::Scoping;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

use super::Side;

trait Id: Copy + Eq + Debug {
    fn at(self) -> usize;
}

impl Id for ScopeId {
    fn at(self) -> usize {
        self.index()
    }
}

impl Id for SymbolId {
    fn at(self) -> usize {
        self.index()
    }
}

impl Id for ReferenceId {
    fn at(self) -> usize {
        self.index()
    }
}

struct Maps {
    scopes: Vec<Option<ScopeId>>,
    symbols: Vec<Option<SymbolId>>,
    references: Vec<Option<ReferenceId>>,
}

impl Maps {
    fn scope(&self, held: ScopeId) -> Option<ScopeId> {
        self.scopes.get(held.index()).copied().flatten()
    }

    fn symbol(&self, held: SymbolId) -> Option<SymbolId> {
        self.symbols.get(held.index()).copied().flatten()
    }

    fn reference(&self, held: ReferenceId) -> Option<ReferenceId> {
        self.references.get(held.index()).copied().flatten()
    }
}

struct Cover {
    what: &'static str,
    held_len: usize,
    fresh_len: usize,
    repeats: bool,
    onto: bool,
}

fn injection<I: Id>(
    cover: &Cover,
    held: &[Option<I>],
    fresh: &[Option<I>],
) -> Result<Vec<Option<I>>, String> {
    let what = cover.what;
    if held.len() != fresh.len() {
        return Err(format!("the {what} cells differ in number"));
    }
    let mut map: Vec<Option<I>> = vec![None; cover.held_len];
    let mut back: Vec<Option<I>> = vec![None; cover.fresh_len];
    for (index, (h, f)) in held.iter().zip(fresh).enumerate() {
        let (Some(h), Some(f)) = (*h, *f) else {
            return Err(format!("{what} cell {index} is empty"));
        };
        let (Some(slot), Some(rev)) = (map.get(h.at()).copied(), back.get(f.at()).copied()) else {
            return Err(format!("{what} cell {index} names {h:?}, past the table"));
        };
        match (slot, rev) {
            (None, None) => {
                map[h.at()] = Some(f);
                back[f.at()] = Some(h);
            }
            (Some(slot), Some(rev)) if cover.repeats && slot == f && rev == h => {}
            _ => return Err(format!("{what} {h:?} sits in two cells")),
        }
    }
    if cover.onto && back.iter().any(Option::is_none) {
        return Err(format!("a fresh {what} has no held counterpart"));
    }
    Ok(map)
}

fn maps(held: &Side<'_>, fresh: &Side<'_>) -> Result<Maps, String> {
    let (h, f) = (held.scoping, fresh.scoping);
    let cover = |what, held_len, fresh_len, repeats, onto| Cover {
        what,
        held_len,
        fresh_len,
        repeats,
        onto,
    };
    let scopes = cover("scope", h.scopes_len(), f.scopes_len(), false, true);
    let symbols = cover("binding", h.symbols_len(), f.symbols_len(), true, true);
    let references = cover(
        "reference",
        h.references_len(),
        f.references_len(),
        false,
        false,
    );
    Ok(Maps {
        scopes: injection(&scopes, &held.cells.scopes, &fresh.cells.scopes)?,
        symbols: injection(&symbols, &held.cells.bindings, &fresh.cells.bindings)?,
        references: injection(&references, &held.cells.references, &fresh.cells.references)?,
    })
}

pub(super) fn compare(held: &Side<'_>, fresh: &Side<'_>, canonical: bool) -> Option<String> {
    let maps = match maps(held, fresh) {
        Ok(maps) => maps,
        Err(difference) => return Some(difference),
    };
    let numbered = canonical.then(|| numbered_differently(&maps)).flatten();
    numbered
        .or_else(|| symbols_differ(held, fresh, &maps))
        .or_else(|| scopes_differ(held, fresh, &maps))
        .or_else(|| dead_differ(held.scoping, &maps))
        .or_else(|| references_differ(held, fresh, &maps))
        .or_else(|| lists_differ(held.scoping, fresh.scoping, &maps))
}

fn identity<I: Id>(map: &[Option<I>]) -> bool {
    map.iter()
        .enumerate()
        .all(|(index, mapped)| mapped.is_some_and(|id| id.at() == index))
}

fn numbered_differently(maps: &Maps) -> Option<String> {
    if !identity(&maps.scopes) {
        return Some("scope cells against a fresh build's, numbered differently".to_string());
    }
    (!identity(&maps.symbols))
        .then(|| "binding cells against a fresh build's, numbered differently".to_string())
}

type Redeclared = Vec<(u32, u32, SymbolFlags)>;

fn redeclarations(scoping: &Scoping, id: SymbolId) -> Redeclared {
    scoping
        .symbol_redeclarations(id)
        .iter()
        .map(|r| (r.span.start, r.span.end, r.flags))
        .collect()
}

fn symbols_differ(held: &Side<'_>, fresh: &Side<'_>, maps: &Maps) -> Option<String> {
    let (h, f) = (held.scoping, fresh.scoping);
    maps.symbols.iter().enumerate().find_map(|(index, mapped)| {
        let fid = (*mapped)?;
        let hid = SymbolId::new(index);
        let same = h.symbol_name(hid) == f.symbol_name(fid)
            && h.symbol_flags(hid) == f.symbol_flags(fid)
            && maps.scope(h.symbol_scope_id(hid)) == Some(f.symbol_scope_id(fid))
            && h.symbol_span(hid) == f.symbol_span(fid)
            && redeclarations(h, hid) == redeclarations(f, fid);
        (!same).then(|| format!("symbol {hid:?} ({}) against {fid:?}", f.symbol_name(fid)))
    })
}

fn bindings_of(
    scoping: &Scoping,
    id: ScopeId,
    map: impl Fn(SymbolId) -> Option<SymbolId>,
) -> Vec<(String, Option<SymbolId>)> {
    let mut bindings: Vec<(String, Option<SymbolId>)> = scoping
        .get_bindings(id)
        .iter()
        .map(|(name, symbol)| (name.as_str().to_string(), map(*symbol)))
        .collect();
    bindings.sort();
    bindings
}

fn scopes_differ(held: &Side<'_>, fresh: &Side<'_>, maps: &Maps) -> Option<String> {
    let (h, f) = (held.scoping, fresh.scoping);
    maps.scopes.iter().enumerate().find_map(|(index, mapped)| {
        let fid = (*mapped)?;
        let hid = ScopeId::new(index);
        let parent = h.scope_parent_id(hid).map(|parent| maps.scope(parent));
        let same = parent == f.scope_parent_id(fid).map(Some)
            && h.scope_flags(hid) == f.scope_flags(fid)
            && bindings_of(h, hid, |symbol| maps.symbol(symbol)) == bindings_of(f, fid, Some);
        (!same).then(|| format!("scope {hid:?} against {fid:?}"))
    })
}

fn dead_differ(h: &Scoping, maps: &Maps) -> Option<String> {
    let scope = maps.scopes.iter().enumerate().find_map(|(index, mapped)| {
        let id = ScopeId::new(index);
        let inert = h.scope_flags(id) == ScopeFlags::empty() && h.get_bindings(id).is_empty();
        (mapped.is_none() && !inert).then(|| format!("dead scope {id:?} is still live"))
    });
    scope.or_else(|| {
        maps.symbols.iter().enumerate().find_map(|(index, mapped)| {
            let id = SymbolId::new(index);
            let inert = h.symbol_name(id).is_empty()
                && h.symbol_flags(id) == SymbolFlags::empty()
                && h.symbol_redeclarations(id).is_empty()
                && h.get_resolved_reference_ids(id).is_empty();
            (mapped.is_none() && !inert).then(|| format!("dead symbol {id:?} is still live"))
        })
    })
}

fn references_differ(held: &Side<'_>, fresh: &Side<'_>, maps: &Maps) -> Option<String> {
    let pairs = held.cells.references.iter().zip(&fresh.cells.references);
    let pairs = pairs.filter_map(|(h, f)| Some(((*h)?, (*f)?)));
    pairs.into_iter().find_map(|(h_id, f_id)| {
        let h = held.scoping.get_reference(h_id);
        let f = fresh.scoping.get_reference(f_id);
        let symbol = h.symbol_id().map(|symbol| maps.symbol(symbol));
        let same = symbol == f.symbol_id().map(Some)
            && h.flags() == f.flags()
            && maps.scope(h.scope_id()) == Some(f.scope_id());
        (!same).then(|| format!("reference {h_id:?} ({h:?} against {f:?})"))
    })
}

fn mapped(ids: &[ReferenceId], maps: &Maps) -> Option<Vec<ReferenceId>> {
    ids.iter().map(|id| maps.reference(*id)).collect()
}

fn lists_differ(h: &Scoping, f: &Scoping, maps: &Maps) -> Option<String> {
    let symbol = maps.symbols.iter().enumerate().find_map(|(index, fid)| {
        let fid = (*fid)?;
        let hid = SymbolId::new(index);
        let held = mapped(h.get_resolved_reference_ids(hid), maps);
        (held.as_deref() != Some(f.get_resolved_reference_ids(fid))).then(|| {
            format!(
                "the resolved references of symbol {hid:?} ({}): {held:?} against {:?}",
                f.symbol_name(fid),
                f.get_resolved_reference_ids(fid)
            )
        })
    });
    symbol.or_else(|| unresolved_differ(h, f, maps))
}

fn unresolved_differ(h: &Scoping, f: &Scoping, maps: &Maps) -> Option<String> {
    let held = h.root_unresolved_references();
    let fresh = f.root_unresolved_references();
    if held.len() != fresh.len() {
        return Some(format!(
            "{} unresolved names against a fresh build's {}",
            held.len(),
            fresh.len()
        ));
    }
    fresh.iter().find_map(|(name, ids)| {
        let same = held
            .get(name.as_str())
            .and_then(|held_ids| mapped(held_ids, maps))
            .is_some_and(|held_ids| held_ids.as_slice() == ids.as_slice());
        (!same).then(|| format!("the unresolved references to {}", name.as_str()))
    })
}
