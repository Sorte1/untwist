use oxc_ast::ast::Expression;
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0100_0000_01b3;
const DECLARATIONS: &[&str] = &["var", "let", "const"];

pub struct Print {
    pub hash: u64,
    pub nodes: usize,
}

enum Mark {
    Symbol(SymbolId),
    Free,
    Label,
}

pub struct Marks {
    items: Vec<(Span, Mark)>,
    shorthands: Vec<u32>,
}

impl Marks {
    pub fn of(nodes: &AstNodes<'_>, scoping: &Scoping) -> Self {
        let mut shorthands: Vec<u32> = nodes
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::ObjectProperty(property) if property.shorthand => {
                    Some(property.key.span().start)
                }
                AstKind::BindingProperty(property) if property.shorthand => {
                    Some(property.key.span().start)
                }
                AstKind::AssignmentTargetPropertyIdentifier(property) => {
                    Some(property.binding.span.start)
                }
                _ => None,
            })
            .collect();
        shorthands.sort_unstable();
        let mut items: Vec<(Span, Mark)> = nodes
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::IdentifierReference(ident) => Some((
                    ident.span,
                    ident
                        .reference_id
                        .get()
                        .and_then(|reference| scoping.get_reference(reference).symbol_id())
                        .map_or(Mark::Free, Mark::Symbol),
                )),
                AstKind::BindingIdentifier(ident) => {
                    Some((ident.span, Mark::Symbol(ident.symbol_id.get()?)))
                }
                AstKind::LabelIdentifier(label) => Some((label.span, Mark::Label)),
                _ => None,
            })
            .collect();
        items.sort_by_key(|(span, _)| span.start);
        Marks { items, shorthands }
    }
}

struct Hasher {
    hash: u64,
    len: usize,
    space: bool,
}

impl Hasher {
    fn byte(&mut self, byte: u8) {
        self.hash ^= u64::from(byte);
        self.hash = self.hash.wrapping_mul(PRIME);
        self.len += 1;
    }

    fn text(&mut self, text: &str) {
        let mut words = text.split(|c: char| c.is_ascii_whitespace()).peekable();
        while let Some(word) = words.next() {
            if !word.is_empty() {
                let declares = words.peek().is_some()
                    && DECLARATIONS.iter().any(|keyword| {
                        word.strip_suffix(keyword).is_some_and(|head| {
                            head.bytes().all(|b| matches!(b, b'(' | b'{' | b';'))
                        })
                    });
                let word = match declares {
                    true => word.trim_end_matches(|c: char| c.is_ascii_alphabetic()),
                    false => word,
                };
                for byte in word.bytes() {
                    self.byte(byte);
                }
                if declares {
                    self.byte(b'd');
                }
                self.space = false;
            }
            if words.peek().is_some() && !self.space {
                self.byte(b' ');
                self.space = true;
            }
        }
    }

    fn token(&mut self, tag: u8, index: usize) {
        self.byte(0);
        self.byte(tag);
        for byte in (index as u32).to_le_bytes() {
            self.byte(byte);
        }
        self.space = false;
    }
}

fn shape(
    source: &str,
    marks: &Marks,
    scoping: &Scoping,
    span: Span,
    root: ScopeId,
    name: Option<Span>,
) -> Print {
    let mut hasher = Hasher {
        hash: OFFSET,
        len: 0,
        space: false,
    };
    let mut locals: FxHashMap<SymbolId, usize> = FxHashMap::default();
    let mut outers: FxHashMap<SymbolId, usize> = FxHashMap::default();
    let mut cursor = span.start;
    let first = marks.items.partition_point(|(at, _)| at.start < span.start);
    for (at, symbol) in &marks.items[first..] {
        if at.start >= span.end {
            break;
        }
        hasher.text(&source[cursor as usize..at.start as usize]);
        cursor = at.end;
        if Some(*at) == name {
            continue;
        }
        if marks.shorthands.binary_search(&at.start).is_ok() {
            hasher.text(at.source_text(source));
            hasher.text(": ");
        }
        match symbol {
            Mark::Free => hasher.text(at.source_text(source)),
            Mark::Label => hasher.token(b'L', 0),
            Mark::Symbol(symbol) => {
                let inside = scoping
                    .scope_ancestors(scoping.symbol_scope_id(*symbol))
                    .any(|scope| scope == root);
                let (tag, table) = match inside {
                    true => (b'l', &mut locals),
                    false => (b'o', &mut outers),
                };
                let next = table.len();
                let index = *table.entry(*symbol).or_insert(next);
                hasher.token(tag, index);
            }
        }
    }
    hasher.text(&source[cursor as usize..span.end as usize]);
    Print {
        hash: hasher.hash,
        nodes: hasher.len,
    }
}

pub fn function(
    source: &str,
    marks: &Marks,
    function: &oxc_ast::ast::Function<'_>,
    scoping: &Scoping,
) -> Option<Print> {
    let root = function.scope_id.get()?;
    let name = function.id.as_ref().map(|id| id.span);
    Some(shape(source, marks, scoping, function.span, root, name))
}

pub fn arrow(
    source: &str,
    marks: &Marks,
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    scoping: &Scoping,
) -> Option<Print> {
    let root = arrow.scope_id.get()?;
    Some(shape(source, marks, scoping, arrow.span, root, None))
}

pub fn bound_functions(
    source: &str,
    marks: &Marks,
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
) -> Vec<(SymbolId, Print)> {
    let mut out = Vec::new();
    for symbol in scoping.symbol_ids() {
        if scoping
            .scope_ancestors(scoping.symbol_scope_id(symbol))
            .count()
            > 2
        {
            continue;
        }
        let print = match nodes.kind(scoping.symbol_declaration(symbol)) {
            AstKind::Function(function) => self::function(source, marks, function, scoping),
            AstKind::VariableDeclarator(declarator) => match declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses)
            {
                Some(Expression::FunctionExpression(function)) => {
                    self::function(source, marks, function, scoping)
                }
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    self::arrow(source, marks, arrow, scoping)
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(print) = print {
            out.push((symbol, print));
        }
    }
    out
}
