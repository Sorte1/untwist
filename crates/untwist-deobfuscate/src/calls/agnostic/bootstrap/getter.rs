use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;

use super::{binding_symbol, is_function, names, symbol_of};

fn function_parts<'a>(
    expr: &'a Expression<'a>,
) -> Option<(&'a FormalParameters<'a>, &'a FunctionBody<'a>)> {
    match expr.without_parentheses() {
        Expression::FunctionExpression(func) => Some((&func.params, func.body.as_ref()?)),
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => Some((&arrow.params, body)),
            _ => None,
        },
        _ => None,
    }
}

fn function_literal(argument: &Argument<'_>) -> bool {
    let Some(Expression::ObjectExpression(object)) = argument
        .as_expression()
        .map(Expression::without_parentheses)
    else {
        return false;
    };
    object.properties.iter().all(|property| {
        let ObjectPropertyKind::ObjectProperty(p) = property else {
            return false;
        };
        let named = match &p.key {
            PropertyKey::StaticIdentifier(id) => !p.computed && id.name != "__proto__",
            PropertyKey::StringLiteral(text) => text.value != "__proto__",
            PropertyKey::NumericLiteral(_) => true,
            _ => false,
        };
        named && p.kind == PropertyKind::Init && is_function(&p.value)
    })
}

fn for_in_pair<'a>(
    scoping: &Scoping,
    statement: &'a Statement<'a>,
    definition: SymbolId,
) -> Option<(SymbolId, &'a ForInStatement<'a>)> {
    let Statement::ForInStatement(each) = statement else {
        return None;
    };
    let ForStatementLeft::VariableDeclaration(decl) = &each.left else {
        return None;
    };
    let [declarator] = decl.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let key = id.symbol_id.get()?;
    let plain = declarator.init.is_none()
        && !scoping.symbol_is_mutated(key)
        && names(scoping, &each.right, definition);
    plain.then_some((key, &**each))
}

struct DefinitionUses<'s> {
    scoping: &'s Scoping,
    definition: SymbolId,
    key: SymbolId,
    seen: Vec<Span>,
    allowed: Vec<Span>,
    read: Option<Span>,
    guarded: bool,
}

impl DefinitionUses<'_> {
    fn is(&self, expr: &Expression<'_>, symbol: SymbolId) -> bool {
        names(self.scoping, expr, symbol)
    }
}

impl<'a> VisitJs<'a> for DefinitionUses<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let owns = matches!(it.callee.without_parentheses(), Expression::StaticMemberExpression(m) if m.property.name == "o");
        if let (true, [object, key]) = (owns, it.arguments.as_slice()) {
            let (Some(object), Some(key)) = (object.as_expression(), key.as_expression()) else {
                return walk_js::walk_call_expression(self, it);
            };
            if self.is(object, self.definition) && self.is(key, self.key) {
                self.allowed.push(object.span());
                self.guarded = true;
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let getter = !it.computed
            && matches!(&it.key, PropertyKey::StaticIdentifier(id) if id.name == "get");
        if let (true, Expression::ComputedMemberExpression(read)) =
            (getter, it.value.without_parentheses())
        {
            if self.is(&read.object, self.definition) && self.is(&read.expression, self.key) {
                self.allowed.push(read.object.span());
                self.read.get_or_insert(read.span);
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if symbol_of(self.scoping, it) == Some(self.definition) {
            self.seen.push(it.span);
        }
    }
}

pub(super) fn getter_read(
    scoping: &Scoping,
    definer: &Expression<'_>,
    calls: &[&CallExpression<'_>],
) -> Option<Span> {
    let (params, body) = function_parts(definer)?;
    if params.rest.is_some() || params.items.iter().any(|param| param.initializer.is_some()) {
        return None;
    }
    let definition = params.items.get(1).and_then(binding_symbol)?;
    if scoping.symbol_is_mutated(definition)
        || calls
            .iter()
            .any(|call| !call.arguments.get(1).is_some_and(function_literal))
    {
        return None;
    }
    let [statement] = body.statements.as_slice() else {
        return None;
    };
    let (key, each) = for_in_pair(scoping, statement, definition)?;
    let mut uses = DefinitionUses {
        scoping,
        definition,
        key,
        seen: Vec::new(),
        allowed: vec![each.right.span()],
        read: None,
        guarded: false,
    };
    uses.visit_statement(&each.body);
    uses.visit_expression(&each.right);
    let only_allowed = uses.seen.iter().all(|span| uses.allowed.contains(span));
    (only_allowed && uses.guarded)
        .then_some(uses.read)
        .flatten()
}
