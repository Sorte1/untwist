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
    framework: &'static str,
    sites: RefCell<Vec<Site>>,
    refs: Vec<(Span, SymbolId)>,
    shorthands: Vec<u32>,
    pub forbidden: RefCell<FxHashSet<SymbolId>>,
    renames: RefCell<FxHashMap<SymbolId, String>>,
    pub seen: RefCell<FxHashSet<SymbolId>>,
    verbatim: Vec<Span>,
}

pub type Reason = String;

pub const LINE: char = '\u{FDD0}';

pub fn restore_lines(text: &str) -> String {
    text.replace(LINE, "\n")
}

impl<'s> Rewriter<'s> {
    pub fn new(
        source: &'s str,
        framework: &'static str,
        mut sites: Vec<Site>,
        mut refs: Vec<(Span, SymbolId)>,
        mut shorthands: Vec<u32>,
    ) -> Self {
        shorthands.sort_unstable();
        sites.sort_by_key(|site| (site.span.start, std::cmp::Reverse(site.span.end)));
        refs.sort_by_key(|(span, _)| span.start);
        Rewriter {
            source,
            framework,
            sites: RefCell::new(sites),
            refs,
            shorthands,
            forbidden: RefCell::new(FxHashSet::default()),
            renames: RefCell::new(FxHashMap::default()),
            seen: RefCell::new(FxHashSet::default()),
            verbatim: Vec::new(),
        }
    }

    pub fn keep_lines(&mut self, mut spans: Vec<Span>) {
        if self.source.contains(LINE) {
            return;
        }
        spans.sort_by_key(|span| span.start);
        self.verbatim = spans;
    }

    fn push(&self, out: &mut String, start: u32, end: u32) {
        let text = &self.source[start as usize..end as usize];
        if self.verbatim.is_empty() || !text.contains('\n') {
            out.push_str(text);
            return;
        }
        for (at, c) in text.char_indices() {
            let offset = start + at as u32;
            let first = self.verbatim.partition_point(|span| span.end <= offset);
            let kept = c == '\n'
                && self
                    .verbatim
                    .get(first)
                    .is_some_and(|span| span.start <= offset);
            out.push(if kept { LINE } else { c });
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
                    "`{}` is compiler machinery that has no {} spelling here",
                    span.source_text(self.source),
                    self.framework
                ));
            }
            if let Some(name) = renames.get(symbol) {
                self.push(out, cursor, span.start);
                if self.shorthands.binary_search(&span.start).is_ok() {
                    out.push_str(span.source_text(self.source));
                    out.push_str(": ");
                }
                out.push_str(name);
                cursor = span.end;
            }
        }
        self.push(out, cursor, end);
        Ok(())
    }

    pub fn render(&self, span: Span) -> Result<String, Reason> {
        let mut out = String::new();
        let mut cursor = span.start;
        let sites = self.sites.borrow();
        let first = sites.partition_point(|site| site.span.start < span.start);
        for site in &sites[first..] {
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

    pub fn add_site(&self, site: Site) {
        let mut sites = self.sites.borrow_mut();
        let at = sites.partition_point(|other| {
            (other.span.start, std::cmp::Reverse(other.span.end))
                < (site.span.start, std::cmp::Reverse(site.span.end))
        });
        sites.insert(at, site);
    }

    pub fn has_site(&self, span: Span) -> bool {
        self.sites.borrow().iter().any(|site| site.span == span)
    }

    pub fn text(&self, span: Span) -> &'s str {
        &self.source[span.start as usize..span.end as usize]
    }
}
