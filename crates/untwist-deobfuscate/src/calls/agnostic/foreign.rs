use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

use super::defs::Defs;
use super::sites::Sites;

const LOCAL_DEPTH: u8 = 8;

const GLOBAL_OBJECTS: [&str; 7] = [
    "globalThis",
    "window",
    "self",
    "top",
    "parent",
    "frames",
    "global",
];

pub(super) fn sourced(program: &Program<'_>, scoping: &Scoping) -> bool {
    if !untwist_core::semantic::other_inputs_given() {
        return false;
    }
    if untwist_core::semantic::every_global_foreign() {
        return true;
    }
    let mut scan = Sources {
        scoping,
        found: false,
    };
    scan.visit_program(program);
    scan.found
}

struct Sources<'s> {
    scoping: &'s Scoping,
    found: bool,
}

impl Sources<'_> {
    fn global_object(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if GLOBAL_OBJECTS.contains(&id.name.as_str()) && self.unresolved(id))
    }

    fn unresolved(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_none()
    }
}

fn foreign_name(name: &str) -> bool {
    untwist_core::semantic::is_foreign_global(name)
}

impl<'a> VisitJs<'a> for Sources<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if !foreign_name(it.name.as_str()) {
            return;
        }
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.found |= symbol.is_none_or(|symbol| {
            self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
        });
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.found |= foreign_name(it.property.name.as_str());
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let literal = matches!(
            it.expression.without_parentheses(),
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        );
        self.found |= !literal && self.global_object(&it.object);
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.found |= it.value.as_str().is_some_and(foreign_name);
    }

    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        self.found |= it
            .value
            .cooked
            .and_then(|cooked| cooked.as_str())
            .is_some_and(foreign_name);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= foreign_name(it.name.as_str())
            && it.symbol_id.get().is_some_and(|symbol| {
                self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            });
    }
}

fn returns_of<'k, 'a>(statements: &'k [Statement<'a>], out: &mut Vec<Option<&'k Expression<'a>>>) {
    for statement in statements {
        returns_in(statement, out);
    }
}

fn returns_in<'k, 'a>(statement: &'k Statement<'a>, out: &mut Vec<Option<&'k Expression<'a>>>) {
    match statement {
        Statement::ReturnStatement(it) => out.push(it.argument.as_ref()),
        Statement::BlockStatement(it) => returns_of(&it.body, out),
        Statement::IfStatement(it) => {
            returns_in(&it.consequent, out);
            if let Some(alternate) = &it.alternate {
                returns_in(alternate, out);
            }
        }
        Statement::ForStatement(it) => returns_in(&it.body, out),
        Statement::ForInStatement(it) => returns_in(&it.body, out),
        Statement::ForOfStatement(it) => returns_in(&it.body, out),
        Statement::WhileStatement(it) => returns_in(&it.body, out),
        Statement::DoWhileStatement(it) => returns_in(&it.body, out),
        Statement::LabeledStatement(it) => returns_in(&it.body, out),
        Statement::WithStatement(it) => returns_in(&it.body, out),
        Statement::TryStatement(it) => {
            returns_of(&it.block.body, out);
            if let Some(handler) = &it.handler {
                returns_of(&handler.body.body, out);
            }
            if let Some(finalizer) = &it.finalizer {
                returns_of(&finalizer.body, out);
            }
        }
        Statement::SwitchStatement(it) => {
            for case in &it.cases {
                returns_of(&case.consequent, out);
            }
        }
        _ => {}
    }
}

fn function_returns<'k, 'a>(func: &'k Function<'a>) -> Option<Vec<Option<&'k Expression<'a>>>> {
    if func.r#async || func.generator {
        return None;
    }
    let mut out = Vec::new();
    returns_of(&func.body.as_ref()?.statements, &mut out);
    Some(out)
}

fn arrow_returns<'k, 'a>(
    arrow: &'k ArrowFunctionExpression<'a>,
) -> Option<Vec<Option<&'k Expression<'a>>>> {
    if arrow.r#async {
        return None;
    }
    match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => {
            let mut out = Vec::new();
            returns_of(&body.statements, &mut out);
            Some(out)
        }
        body => body.as_expression().map(|expr| vec![Some(expr)]),
    }
}

enum Callee<'k, 'a> {
    User(Vec<Option<&'k Expression<'a>>>),
    Native,
    Unknown,
}

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn foreign_callee(&self, callee: &Expression<'a>) -> bool {
        self.foreign_sourced && !self.value_local(callee, LOCAL_DEPTH)
    }

    pub(super) fn foreign_arguments(
        &self,
        callee: &Expression<'a>,
        arguments: &[Argument<'a>],
    ) -> bool {
        self.foreign_sourced
            && !self.user_callee(callee)
            && !self.console_method(callee)
            && !arguments
                .iter()
                .all(|argument| self.argument_local(argument, LOCAL_DEPTH))
    }

    fn console_method(&self, callee: &Expression<'a>) -> bool {
        let Expression::StaticMemberExpression(member) = callee.without_parentheses() else {
            return false;
        };
        matches!(member.object.without_parentheses(), Expression::Identifier(id)
            if id.name == "console" && self.native_root(id))
    }

    fn user_callee(&self, callee: &Expression<'a>) -> bool {
        matches!(self.callee_kind(callee, LOCAL_DEPTH), Callee::User(_))
    }

    fn argument_local(&self, argument: &Argument<'a>, depth: u8) -> bool {
        match argument {
            Argument::SpreadElement(spread) => self.value_local(&spread.argument, depth),
            other => other
                .as_expression()
                .is_some_and(|expr| self.value_local(expr, depth)),
        }
    }

    fn value_local(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return false;
        };
        match expr.without_parentheses() {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_)
            | Expression::PrivateInExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(id) => self.identifier_local(id, next),
            Expression::ArrayExpression(array) => {
                array.elements.iter().all(|element| match element {
                    ArrayExpressionElement::SpreadElement(spread) => {
                        self.value_local(&spread.argument, next)
                    }
                    ArrayExpressionElement::Elision(_) => true,
                    other => other
                        .as_expression()
                        .is_some_and(|expr| self.value_local(expr, next)),
                })
            }
            Expression::ObjectExpression(object) => {
                object.properties.iter().all(|property| match property {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        p.kind == PropertyKind::Init && self.value_local(&p.value, next)
                    }
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        self.value_local(&spread.argument, next)
                    }
                })
            }
            Expression::LogicalExpression(logical) => {
                self.value_local(&logical.left, next) && self.value_local(&logical.right, next)
            }
            Expression::ConditionalExpression(conditional) => {
                self.value_local(&conditional.consequent, next)
                    && self.value_local(&conditional.alternate, next)
            }
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .is_some_and(|last| self.value_local(last, next)),
            Expression::AssignmentExpression(assign) => match assign.operator {
                AssignmentOperator::Assign => self.value_local(&assign.right, next),
                operator => operator.is_arithmetic() || operator.is_bitwise(),
            },
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => expr
                .without_parentheses()
                .as_member_expression()
                .is_some_and(|member| self.member_local(member, next)),
            Expression::CallExpression(call) => {
                self.call_local(&call.callee, &call.arguments, next)
            }
            Expression::NewExpression(new) => self.call_local(&new.callee, &new.arguments, next),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.call_local(&call.callee, &call.arguments, next)
                }
                ChainElement::TSNonNullExpression(_) => false,
                other => other
                    .as_member_expression()
                    .is_some_and(|member| self.member_local(member, next)),
            },
            _ => false,
        }
    }

    fn local_defs(&self, id: &IdentifierReference<'a>) -> Option<Defs<'_, 'a>> {
        let symbol = self.fresh.symbol(id)?;
        if self.foreign_root(symbol, id.name.as_str()) {
            return None;
        }
        self.fresh
            .bindings
            .defs
            .get(symbol)
            .filter(|defs| !defs.opaque)
    }

    fn foreign_root(&self, symbol: SymbolId, name: &str) -> bool {
        let scoping = self.fresh.scoping;
        scoping.symbol_scope_id(symbol) == scoping.root_scope_id() && foreign_name(name)
    }

    fn native_root(&self, id: &IdentifierReference<'a>) -> bool {
        self.fresh.symbol(id).is_none()
            && id.name != "arguments"
            && !GLOBAL_OBJECTS.contains(&id.name.as_str())
            && !foreign_name(id.name.as_str())
    }

    fn identifier_local(&self, id: &IdentifierReference<'a>, depth: u8) -> bool {
        if self.fresh.symbol(id).is_none() {
            return self.native_root(id);
        }
        let Some(defs) = self.local_defs(id) else {
            return false;
        };
        defs.inits
            .iter()
            .all(|init| init.is_none_or(|init| self.value_local(init, depth)))
            && defs
                .writes
                .iter()
                .all(|write| write.is_some_and(|write| self.value_local(write, depth)))
    }

    fn member_local(&self, member: &MemberExpression<'a>, depth: u8) -> bool {
        let object = member.object().without_parentheses();
        if let Expression::Identifier(id) = object {
            if GLOBAL_OBJECTS.contains(&id.name.as_str()) && self.fresh.symbol(id).is_none() {
                return member
                    .static_property_name()
                    .and_then(|name| name.as_str())
                    .is_some_and(|name| !GLOBAL_OBJECTS.contains(&name) && !foreign_name(name));
            }
        }
        self.contents_local(object, depth)
    }

    fn contents_local(&self, object: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return false;
        };
        match object.without_parentheses() {
            Expression::Identifier(id) if self.fresh.symbol(id).is_none() => self.native_root(id),
            Expression::Identifier(id) => self.literal_holder(id, next),
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                self.native_chain_root(object)
            }
            Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::BinaryExpression(_)
            | Expression::UnaryExpression(_) => self.value_local(object, next),
            _ => false,
        }
    }

    fn native_chain_root(&self, expr: &Expression<'a>) -> bool {
        let mut current = expr.without_parentheses();
        loop {
            let Some(member) = current.as_member_expression() else {
                return matches!(current, Expression::Identifier(id) if self.native_root(id));
            };
            let named = member
                .static_property_name()
                .and_then(|name| name.as_str())
                .is_some_and(|name| !GLOBAL_OBJECTS.contains(&name) && !foreign_name(name));
            if !named {
                return false;
            }
            current = member.object().without_parentheses();
        }
    }

    fn literal_holder(&self, id: &IdentifierReference<'a>, depth: u8) -> bool {
        let Some(defs) = self.local_defs(id) else {
            return false;
        };
        if defs.callable
            || defs.func.is_some()
            || defs.fields_written
            || defs.escapes
            || defs.method_called
            || defs.prototype_set
        {
            return false;
        }
        let plain = |value: &Expression<'a>| {
            matches!(
                value.without_parentheses(),
                Expression::ArrayExpression(_)
                    | Expression::ObjectExpression(_)
                    | Expression::StringLiteral(_)
                    | Expression::TemplateLiteral(_)
                    | Expression::NumericLiteral(_)
                    | Expression::BooleanLiteral(_)
                    | Expression::NullLiteral(_)
            ) && self.value_local(value, depth)
        };
        defs.inits.iter().all(|init| init.is_none_or(plain))
            && defs.writes.iter().all(|write| write.is_some_and(plain))
    }

    fn callee_kind<'k>(&'k self, callee: &'k Expression<'a>, depth: u8) -> Callee<'k, 'a> {
        match callee.without_parentheses() {
            Expression::FunctionExpression(func) => {
                function_returns(func).map_or(Callee::Unknown, Callee::User)
            }
            Expression::ArrowFunctionExpression(arrow) => {
                arrow_returns(arrow).map_or(Callee::Unknown, Callee::User)
            }
            Expression::Identifier(id) if self.fresh.symbol(id).is_none() => {
                if self.native_root(id) {
                    Callee::Native
                } else {
                    Callee::Unknown
                }
            }
            Expression::Identifier(id) => self.user_function(id, depth),
            other => match other.as_member_expression() {
                Some(member) if self.native_method(member, depth) => Callee::Native,
                _ => Callee::Unknown,
            },
        }
    }

    fn user_function<'k>(&'k self, id: &IdentifierReference<'a>, _depth: u8) -> Callee<'k, 'a> {
        self.user_returns(id).map_or(Callee::Unknown, Callee::User)
    }

    fn user_returns(
        &self,
        id: &IdentifierReference<'a>,
    ) -> Option<Vec<Option<&'a Expression<'a>>>> {
        let defs = self.local_defs(id)?;
        let mut returns = Vec::new();
        let mut functions = 0usize;
        if let Some(func) = defs.func {
            returns.extend(function_returns(func)?);
            functions += 1;
        }
        let assigned = defs.inits.iter().flatten().copied().chain(
            defs.writes
                .iter()
                .map(|write| write.ok_or(()))
                .collect::<Result<Vec<_>, ()>>()
                .ok()?,
        );
        for value in assigned {
            let found = match value.without_parentheses() {
                Expression::FunctionExpression(func) => function_returns(func),
                Expression::ArrowFunctionExpression(arrow) => arrow_returns(arrow),
                _ => None,
            };
            returns.extend(found?);
            functions += 1;
        }
        (functions > 0).then_some(returns)
    }

    fn native_method(&self, member: &MemberExpression<'a>, depth: u8) -> bool {
        let object = member.object().without_parentheses();
        if let Expression::Identifier(id) = object {
            if self.fresh.symbol(id).is_none() {
                return self.native_root(id) && self.member_local(member, depth);
            }
            return self.literal_holder(id, depth) && self.holds_no_function(id);
        }
        match object {
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                self.native_chain_root(object) && self.member_local(member, depth)
            }
            Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BinaryExpression(_) => true,
            Expression::ArrayExpression(_) | Expression::ObjectExpression(_) => {
                self.value_local(object, depth) && !holds_function(object)
            }
            _ => false,
        }
    }

    fn holds_no_function(&self, id: &IdentifierReference<'a>) -> bool {
        self.local_defs(id).is_some_and(|defs| {
            defs.inits
                .iter()
                .chain(defs.writes.iter())
                .flatten()
                .all(|value| !holds_function(value))
        })
    }

    fn call_local(&self, callee: &Expression<'a>, arguments: &[Argument<'a>], depth: u8) -> bool {
        if !arguments
            .iter()
            .all(|argument| self.argument_local(argument, depth))
        {
            return false;
        }
        match self.callee_kind(callee, depth) {
            Callee::Native => true,
            Callee::User(returns) => returns
                .iter()
                .all(|value| value.is_none_or(|value| self.value_local(value, depth))),
            Callee::Unknown => false,
        }
    }
}

fn holds_function(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::ClassExpression(_) => true,
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| match element {
            ArrayExpressionElement::SpreadElement(_) => true,
            ArrayExpressionElement::Elision(_) => false,
            other => other.as_expression().is_none_or(holds_function),
        }),
        Expression::ObjectExpression(object) => {
            object.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(p) => {
                    p.kind != PropertyKind::Init || p.method || holds_function(&p.value)
                }
                ObjectPropertyKind::SpreadProperty(_) => true,
            })
        }
        _ => false,
    }
}
