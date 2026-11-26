use std::fmt::Write;

use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0100_0000_01b3;

pub struct Hasher(u64);

impl Hasher {
    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(PRIME);
        }
    }

    fn token(&mut self, text: &str) {
        self.bytes(text.as_bytes());
        self.bytes(&[0]);
    }
}

impl Write for Hasher {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.bytes(text.as_bytes());
        Ok(())
    }
}

struct Shape<'s> {
    scoping: &'s Scoping,
    root: ScopeId,
    name: Option<Span>,
    hash: Hasher,
    locals: FxHashMap<SymbolId, usize>,
    outers: FxHashMap<SymbolId, usize>,
    nodes: usize,
}

impl Shape<'_> {
    fn symbol(&mut self, symbol: SymbolId) {
        let inside = self
            .scoping
            .scope_ancestors(self.scoping.symbol_scope_id(symbol))
            .any(|scope| scope == self.root);
        let (tag, table) = match inside {
            true => ("l", &mut self.locals),
            false => ("o", &mut self.outers),
        };
        let next = table.len();
        let index = *table.entry(symbol).or_insert(next);
        let _ = write!(self.hash, "{tag}{index}");
        self.hash.bytes(&[0]);
    }

    fn details(&mut self, kind: AstKind<'_>) {
        match kind {
            AstKind::IdentifierName(name) => self.hash.token(&name.name),
            AstKind::PrivateIdentifier(name) => self.hash.token(&name.name),
            AstKind::LabelIdentifier(_) => self.hash.token("label"),
            AstKind::IdentifierReference(ident) => {
                match ident
                    .reference_id
                    .get()
                    .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                {
                    Some(symbol) => self.symbol(symbol),
                    None => self.hash.token(&ident.name),
                }
            }
            AstKind::BindingIdentifier(ident) => match ident.symbol_id.get() {
                Some(symbol) => self.symbol(symbol),
                None => self.hash.token("?"),
            },
            AstKind::AssignmentTargetPropertyIdentifier(property) => {
                self.hash.token(&property.binding.name)
            }
            AstKind::StringLiteral(literal) => match literal.value.as_str() {
                Some(text) => self.hash.token(text),
                None => self.hash.token("\u{fffd}"),
            },
            AstKind::NumericLiteral(literal) => {
                let _ = write!(self.hash, "{}", literal.value);
                self.hash.bytes(&[0]);
            }
            AstKind::BigIntLiteral(literal) => self.hash.token(&literal.value),
            AstKind::BooleanLiteral(literal) => {
                self.hash.token(if literal.value { "t" } else { "f" })
            }
            AstKind::RegExpLiteral(literal) => {
                self.hash.token(&literal.regex.pattern.text);
                let _ = write!(self.hash, "{}", literal.regex.flags.bits());
            }
            AstKind::TemplateElement(element) => {
                match element.value.cooked.and_then(|cooked| cooked.as_str()) {
                    Some(cooked) => self.hash.token(cooked),
                    None => self.hash.token(&element.value.raw),
                }
            }
            AstKind::BinaryExpression(expr) => self.hash.token(expr.operator.as_str()),
            AstKind::LogicalExpression(expr) => self.hash.token(expr.operator.as_str()),
            AstKind::AssignmentExpression(expr) => self.hash.token(expr.operator.as_str()),
            AstKind::UnaryExpression(expr) => self.hash.token(expr.operator.as_str()),
            AstKind::UpdateExpression(expr) => {
                self.hash.token(expr.operator.as_str());
                self.hash.token(if expr.prefix { "p" } else { "s" });
            }
            AstKind::CallExpression(call) => self.hash.token(if call.optional { "?" } else { "" }),
            AstKind::StaticMemberExpression(member) => {
                self.hash.token(if member.optional { "?" } else { "" })
            }
            AstKind::ComputedMemberExpression(member) => {
                self.hash.token(if member.optional { "?" } else { "" })
            }
            AstKind::Function(function) => {
                self.hash
                    .token(match (function.r#async, function.generator) {
                        (false, false) => "",
                        (true, false) => "a",
                        (false, true) => "g",
                        (true, true) => "ag",
                    })
            }
            AstKind::ArrowFunctionExpression(arrow) => {
                self.hash.token(if arrow.r#async { "a" } else { "" })
            }
            AstKind::ForOfStatement(stmt) => self.hash.token(if stmt.r#await { "a" } else { "" }),
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for Shape<'_> {
    fn visit_binding_identifier(&mut self, ident: &oxc_ast::ast::BindingIdentifier<'a>) {
        if Some(ident.span) != self.name {
            oxc_ast_visit::walk::walk_binding_identifier(self, ident);
        }
    }

    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.nodes += 1;
        let _ = write!(self.hash, "({:?}", kind.ty());
        self.hash.bytes(&[0]);
        self.details(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.hash.bytes(b")");
    }
}

pub struct Print {
    pub hash: u64,
    pub nodes: usize,
}

pub fn function(function: &oxc_ast::ast::Function<'_>, scoping: &Scoping) -> Option<Print> {
    let root = function.scope_id.get()?;
    let mut shape = Shape {
        scoping,
        root,
        name: function.id.as_ref().map(|id| id.span),
        hash: Hasher(OFFSET),
        locals: FxHashMap::default(),
        outers: FxHashMap::default(),
        nodes: 0,
    };
    shape.visit_function(function, oxc_syntax::scope::ScopeFlags::Function);
    Some(Print {
        hash: shape.hash.0,
        nodes: shape.nodes,
    })
}

pub fn arrow(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    scoping: &Scoping,
) -> Option<Print> {
    let root = arrow.scope_id.get()?;
    let mut shape = Shape {
        scoping,
        root,
        name: None,
        hash: Hasher(OFFSET),
        locals: FxHashMap::default(),
        outers: FxHashMap::default(),
        nodes: 0,
    };
    shape.visit_arrow_function_expression(arrow);
    Some(Print {
        hash: shape.hash.0,
        nodes: shape.nodes,
    })
}
