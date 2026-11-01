use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;

use crate::scan::{is_direct_eval, Frame, Policy, Scan, Step};

pub const ENTRY: u32 = u32::MAX - 1;
const NONE: u32 = u32::MAX;
const MODULE_RESERVED: &str = "await";
const HOST_NAMES: [&str; 4] = ["module", "exports", "__filename", "__dirname"];

pub struct Facts {
    pub owner: Vec<u32>,
    pub first_decl: Vec<Option<SymbolId>>,
    pub declares: Vec<u32>,
    pub uses: Vec<(u32, SymbolId)>,
    pub writes: Vec<(u32, u32)>,
    pub spans: Vec<(u32, u32)>,
    pub pinned: Vec<bool>,
    pub requires: Vec<bool>,
}

pub struct Context {
    pub scope: ScopeId,
    pub arrow: bool,
}

fn item_of(items: &[Statement<'_>], span: Span) -> Option<u32> {
    let at = items.partition_point(|item| item.span().end <= span.start);
    let item = items.get(at)?;
    let inside = item.span().start <= span.start && span.end <= item.span().end;
    inside.then_some(at as u32)
}

fn owners(scoping: &Scoping, items: &[Statement<'_>], scope: ScopeId) -> Facts {
    let n = items.len();
    let mut facts = Facts {
        owner: vec![NONE; scoping.symbols_len()],
        first_decl: vec![None; n],
        declares: vec![0; n],
        uses: Vec::new(),
        writes: Vec::new(),
        spans: Vec::new(),
        pinned: vec![false; n],
        requires: vec![false; n],
    };
    for &symbol in scoping.get_bindings(scope).values() {
        let span = scoping.symbol_span(symbol);
        let Some(item) = item_of(items, span) else {
            facts.owner[symbol.index()] = ENTRY;
            continue;
        };
        facts.owner[symbol.index()] = item;
        facts.declares[item as usize] += 1;
        let first = &mut facts.first_decl[item as usize];
        if first.is_none_or(|known| scoping.symbol_span(known).start > span.start) {
            *first = Some(symbol);
        }
        for redeclaration in scoping.symbol_redeclarations(symbol) {
            match item_of(items, redeclaration.span) {
                Some(other) if other != item => {
                    facts.spans.push((item.min(other), item.max(other)))
                }
                Some(_) => {}
                None => facts.pinned[item as usize] = true,
            }
        }
    }
    facts
}

struct Rules<'s> {
    scoping: &'s Scoping,
    context: &'s Context,
    facts: Facts,
    seen: Vec<u32>,
    item: u32,
    refused: bool,
}

fn bare_specifier(arguments: &[Argument<'_>]) -> bool {
    let [Argument::StringLiteral(literal)] = arguments else {
        return false;
    };
    let value = literal.value.as_str();
    !value.is_empty() && !value.starts_with('.') && !value.starts_with('/')
}

impl Rules<'_> {
    fn unresolved(&self, it: &IdentifierReference<'_>) -> bool {
        it.reference_id
            .get()
            .is_some_and(|id| self.scoping.get_reference(id).symbol_id().is_none())
    }

    fn pin(&mut self) {
        self.facts.pinned[self.item as usize] = true;
    }

    fn receiver_bound(&mut self, frame: Frame) {
        if !frame.receiver() {
            if self.context.arrow {
                self.refused = true;
            }
            self.pin();
        }
    }

    fn free(&mut self, name: &str, frame: Frame) {
        if name == "arguments" {
            self.receiver_bound(frame);
        } else if name == "require" || HOST_NAMES.contains(&name) {
            self.refused = true;
        }
    }

    fn resolved(&mut self, symbol: SymbolId, write: bool) {
        let owner = self.facts.owner[symbol.index()];
        if owner == ENTRY {
            self.pin();
        } else if owner != NONE && owner != self.item {
            if self.seen[symbol.index()] != self.item {
                self.seen[symbol.index()] = self.item;
                self.facts.uses.push((self.item, symbol));
            }
            if write {
                self.facts.writes.push((self.item, owner));
            }
        }
    }
}

impl<'a> Policy<'a> for Rules<'_> {
    fn binding(&mut self, it: &BindingIdentifier<'a>) {
        self.refused |= it.name == MODULE_RESERVED;
    }

    fn label(&mut self, it: &LabelIdentifier<'a>) {
        self.refused |= it.name == MODULE_RESERVED;
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        self.refused |= it.name == MODULE_RESERVED;
        let Some(id) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(id);
        match reference.symbol_id() {
            Some(symbol) => self.resolved(symbol, reference.is_write()),
            None => self.free(it.name.as_str(), frame),
        }
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        self.refused |= is_direct_eval(self.scoping, &it.callee);
        if let Expression::Identifier(callee) = &it.callee {
            if callee.name == "require" && self.unresolved(callee) {
                if !bare_specifier(&it.arguments) || it.optional {
                    self.refused = true;
                }
                self.facts.requires[self.item as usize] = true;
                return Step::Skip;
            }
        }
        Step::Walk
    }

    fn this(&mut self, frame: Frame) {
        self.receiver_bound(frame);
    }

    fn import_meta(&mut self) {
        self.refused = true;
    }

    fn new_target(&mut self, frame: Frame) {
        self.receiver_bound(frame);
    }

    fn returns(&mut self, frame: Frame) {
        if frame.functions + frame.arrows == 0 {
            self.pin();
        }
    }
}

pub fn scan(scoping: &Scoping, items: &[Statement<'_>], context: &Context) -> Option<Facts> {
    let facts = owners(scoping, items, context.scope);
    let mut rules = Rules {
        scoping,
        context,
        seen: vec![NONE; facts.owner.len()],
        facts,
        item: 0,
        refused: false,
    };
    for (index, item) in items.iter().enumerate() {
        rules.item = index as u32;
        Scan::new(&mut rules).visit_statement(item);
        if rules.refused {
            return None;
        }
    }
    Some(rules.facts)
}
