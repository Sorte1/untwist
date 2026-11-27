use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;

pub(in crate::js_confuser) struct Ctx<'s> {
    pub(in crate::js_confuser) scoping: &'s Scoping,
}

impl Ctx<'_> {
    pub(in crate::js_confuser) fn symbol(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self
                .scoping
                .get_reference(id.reference_id.get()?)
                .symbol_id(),
            _ => None,
        }
    }

    pub(in crate::js_confuser) fn is(&self, expr: &Expression<'_>, symbol: SymbolId) -> bool {
        self.symbol(expr) == Some(symbol)
    }

    pub(in crate::js_confuser) fn target_symbol(
        &self,
        target: &AssignmentTarget<'_>,
    ) -> Option<(SymbolId, Span)> {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = target else {
            return None;
        };
        let symbol = self
            .scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()?;
        Some((symbol, id.span))
    }
}

pub(in crate::js_confuser) fn string(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::StringLiteral(lit) => Some(lit.value.as_str()?.to_string()),
        _ => None,
    }
}

pub(in crate::js_confuser) fn property_key(
    key: &PropertyKey<'_>,
    computed: bool,
) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) if !computed => Some(id.name.to_string()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.as_str()?.to_string()),
        _ => None,
    }
}

pub(in crate::js_confuser) enum Key<'k, 'a> {
    Str(String),
    Expr(&'k Expression<'a>),
}

pub(in crate::js_confuser) fn member<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k Expression<'a>, Key<'k, 'a>)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(m) if !m.optional => {
            Some((&m.object, Key::Str(m.property.name.to_string())))
        }
        Expression::ComputedMemberExpression(m) if !m.optional => match string(&m.expression) {
            Some(key) => Some((&m.object, Key::Str(key))),
            None => Some((&m.object, Key::Expr(&m.expression))),
        },
        _ => None,
    }
}
