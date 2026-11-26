use std::cell::RefCell;

use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Debug)]
pub enum Piece {
    Lit(String),
    Src(Span),
    Name(SymbolId, String),
}

#[derive(Debug)]
pub struct Site {
    pub span: Span,
    pub pieces: Vec<Piece>,
}

pub struct Rewriter<'s> {
    pub source: &'s str,
    sites: Vec<Site>,
    refs: Vec<(Span, SymbolId)>,
    shorthands: Vec<u32>,
    pub forbidden: RefCell<FxHashSet<SymbolId>>,
    renames: RefCell<FxHashMap<SymbolId, String>>,
    pub seen: RefCell<FxHashSet<SymbolId>>,
}

pub type Reason = String;

impl<'s> Rewriter<'s> {
    pub fn new(
        source: &'s str,
        mut sites: Vec<Site>,
        mut refs: Vec<(Span, SymbolId)>,
        mut shorthands: Vec<u32>,
    ) -> Self {
        shorthands.sort_unstable();
        sites.sort_by_key(|site| (site.span.start, std::cmp::Reverse(site.span.end)));
        refs.sort_by_key(|(span, _)| span.start);
        Rewriter {
            source,
            sites,
            refs,
            shorthands,
            forbidden: RefCell::new(FxHashSet::default()),
            renames: RefCell::new(FxHashMap::default()),
            seen: RefCell::new(FxHashSet::default()),
        }
    }

    pub fn forbid(&self, symbol: SymbolId) {
        self.forbidden.borrow_mut().insert(symbol);
    }

    pub fn forbid_some(&self, symbol: Option<SymbolId>) {
        if let Some(symbol) = symbol {
            self.forbid(symbol);
        }
    }

    pub fn rename(&self, symbol: SymbolId, name: String) {
        self.renames.borrow_mut().insert(symbol, name);
    }

    fn gap(&self, out: &mut String, start: u32, end: u32) -> Result<(), Reason> {
        if start >= end {
            return Ok(());
        }
        let first = self.refs.partition_point(|(span, _)| span.start < start);
        let forbidden = self.forbidden.borrow();
        let renames = self.renames.borrow();
        let mut seen = self.seen.borrow_mut();
        let mut cursor = start;
        for (span, symbol) in &self.refs[first..] {
            if span.start >= end {
                break;
            }
            seen.insert(*symbol);
            if forbidden.contains(symbol) {
                return Err(format!(
                    "`{}` is compiler machinery that has no Svelte spelling here",
                    span.source_text(self.source)
                ));
            }
            if let Some(name) = renames.get(symbol) {
                out.push_str(&self.source[cursor as usize..span.start as usize]);
                if self.shorthands.binary_search(&span.start).is_ok() {
                    out.push_str(span.source_text(self.source));
                    out.push_str(": ");
                }
                out.push_str(name);
                cursor = span.end;
            }
        }
        out.push_str(&self.source[cursor as usize..end as usize]);
        Ok(())
    }

    pub fn render(&self, span: Span) -> Result<String, Reason> {
        let mut out = String::new();
        let mut cursor = span.start;
        let first = self
            .sites
            .partition_point(|site| site.span.start < span.start);
        for site in &self.sites[first..] {
            if site.span.start >= span.end {
                break;
            }
            if site.span.start < cursor || site.span.end > span.end {
                continue;
            }
            self.gap(&mut out, cursor, site.span.start)?;
            for piece in &site.pieces {
                match piece {
                    Piece::Lit(text) => out.push_str(text),
                    Piece::Src(inner) => out.push_str(&self.render(*inner)?),
                    Piece::Name(symbol, name) => {
                        self.seen.borrow_mut().insert(*symbol);
                        match self.renames.borrow().get(symbol) {
                            Some(renamed) => out.push_str(renamed),
                            None => out.push_str(name),
                        }
                    }
                }
            }
            cursor = site.span.end;
        }
        self.gap(&mut out, cursor, span.end)?;
        Ok(out)
    }

    pub fn text(&self, span: Span) -> &'s str {
        &self.source[span.start as usize..span.end as usize]
    }
}
