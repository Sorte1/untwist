use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::js_text::js_text;

use super::bootstrap::Resolve;

pub(super) type Objects<'f, 'a> = &'f dyn Fn(SymbolId) -> Option<&'a ObjectExpression<'a>>;

const MAX_TEXT: usize = 16;

#[derive(Clone, Copy)]
pub(super) struct Facts<'f, 'a> {
    pub(super) scoping: &'f Scoping,
    pub(super) resolve: Resolve<'f>,
    pub(super) objects: Objects<'f, 'a>,
}

fn param_symbol(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match (&param.pattern, &param.initializer) {
        (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
        _ => None,
    }
}

fn returned<'a>(value: &'a Expression<'a>) -> Option<&'a Expression<'a>> {
    match value.without_parentheses() {
        Expression::FunctionExpression(func) => {
            if func.generator || func.r#async || func.params.rest.is_some() {
                return None;
            }
            let body = func.body.as_ref()?;
            match (body.directives.as_slice(), body.statements.as_slice()) {
                ([], [Statement::ReturnStatement(it)]) => it.argument.as_ref(),
                _ => None,
            }
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            if arrow.params.rest.is_some() {
                return None;
            }
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => match body.statements.as_slice() {
                    [Statement::ReturnStatement(it)] if body.directives.is_empty() => {
                        it.argument.as_ref()
                    }
                    _ => None,
                },
                other => other.as_expression(),
            }
        }
        _ => None,
    }
}

fn params<'a>(value: &'a Expression<'a>) -> Option<&'a FormalParameters<'a>> {
    match value.without_parentheses() {
        Expression::FunctionExpression(func) => Some(&func.params),
        Expression::ArrowFunctionExpression(arrow) => Some(&arrow.params),
        _ => None,
    }
}

pub(super) fn property<'a>(
    object: &'a ObjectExpression<'a>,
    name: &str,
) -> Option<&'a Expression<'a>> {
    let mut found = None;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property.kind != PropertyKind::Init || property.method {
            return None;
        }
        let key = match &property.key {
            PropertyKey::StaticIdentifier(id) => id.name.to_string(),
            PropertyKey::StringLiteral(text) => js_text(text.value).into_owned(),
            _ => return None,
        };
        if key == "__proto__" {
            return None;
        }
        if key == name {
            if found.is_some() {
                return None;
            }
            found = Some(&property.value);
        }
    }
    found
}

impl<'a> Facts<'_, 'a> {
    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    pub(super) fn holder(&self, expr: &Expression<'_>) -> Option<&'a ObjectExpression<'a>> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        let symbol = self.symbol(id)?;
        if self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id() {
            return None;
        }
        (self.objects)(symbol)
    }

    pub(super) fn member_name(
        &self,
        member: &MemberExpression<'_>,
        depth: usize,
    ) -> Option<String> {
        match member {
            MemberExpression::StaticMemberExpression(it) => Some(it.property.name.to_string()),
            MemberExpression::ComputedMemberExpression(it) => self.text(&it.expression, depth + 1),
            MemberExpression::PrivateFieldExpression(_) => None,
        }
    }

    pub(super) fn field(
        &self,
        member: &MemberExpression<'_>,
        depth: usize,
    ) -> Option<&'a Expression<'a>> {
        if depth > MAX_TEXT || member.optional() {
            return None;
        }
        let object = self.holder(member.object())?;
        let name = self.member_name(member, depth)?;
        property(object, &name)
    }

    pub(super) fn text(&self, expr: &Expression<'_>, depth: usize) -> Option<String> {
        if depth > MAX_TEXT {
            return None;
        }
        match expr.without_parentheses() {
            Expression::StringLiteral(literal) => Some(js_text(literal.value).into_owned()),
            Expression::CallExpression(call) => self.resolve?.get(&call.span)?.as_string(),
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                let left = self.text(&binary.left, depth + 1)?;
                let right = self.text(&binary.right, depth + 1)?;
                Some(left + &right)
            }
            other => {
                let member = other.as_member_expression()?;
                let value = self.field(member, depth + 1)?;
                self.text(value, depth + 1)
            }
        }
    }

    pub(super) fn function(&self, call: &CallExpression<'_>) -> Option<&'a Expression<'a>> {
        if call.optional {
            return None;
        }
        let member = call.callee.without_parentheses().as_member_expression()?;
        let value = self.field(member, 0)?;
        params(value).map(|_| value)
    }

    pub(super) fn compare(&self, call: &CallExpression<'_>) -> Option<BinaryOperator> {
        let value = self.function(call)?;
        let params = params(value)?;
        let [left, right] = params.items.as_slice() else {
            return None;
        };
        let (left, right) = (param_symbol(left)?, param_symbol(right)?);
        let Expression::BinaryExpression(binary) = returned(value)?.without_parentheses() else {
            return None;
        };
        let names = |expr: &Expression<'_>, param: SymbolId| match expr.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id) == Some(param),
            _ => false,
        };
        let shaped = call.arguments.len() == 2
            && !call.arguments.iter().any(Argument::is_spread)
            && names(&binary.left, left)
            && names(&binary.right, right);
        (shaped && (binary.operator.is_equality() || binary.operator.is_compare()))
            .then_some(binary.operator)
    }

    pub(super) fn forwards(&self, call: &CallExpression<'_>) -> bool {
        let Some(value) = self.function(call) else {
            return false;
        };
        let Some(params) = params(value) else {
            return false;
        };
        let symbols: Option<Vec<SymbolId>> = params.items.iter().map(param_symbol).collect();
        let Some(symbols) = symbols else {
            return false;
        };
        if symbols.is_empty() || symbols.len() != call.arguments.len() {
            return false;
        }
        let Some(Expression::CallExpression(inner)) =
            returned(value).map(Expression::without_parentheses)
        else {
            return false;
        };
        let names = |expr: &Expression<'_>, param: SymbolId| match expr.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id) == Some(param),
            _ => false,
        };
        !inner.optional
            && inner.arguments.len() + 1 == symbols.len()
            && names(&inner.callee, symbols[0])
            && inner
                .arguments
                .iter()
                .zip(&symbols[1..])
                .all(|(argument, param)| argument.as_expression().is_some_and(|e| names(e, *param)))
    }
}
