use oxc_ast::ast::*;
use oxc_ast_visit::walk_js;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use untwist_core::ast_util::prop::number_key;

use super::generate::{Gen, MemberSite, SiteKey};
use super::object::{Builtin, SiteKind};
use super::solve::{Call, Field, Key, Node};
use crate::proof::writes::{record_definer_patch, record_identifier_patch, record_member_patch};

fn primitive_syntax(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_)
    )
}

pub(super) struct Member {
    base: Option<Node>,
    key: Key,
    site: SiteKey,
}

impl Gen<'_> {
    pub(super) fn expr(&mut self, expr: &Expression<'_>) -> Option<Node> {
        let value = self.value(expr);
        if let (Some(dump), Some(node)) = (self.dump.as_mut(), value) {
            dump.push((expr.span(), node));
        }
        value
    }

    fn value(&mut self, expr: &Expression<'_>) -> Option<Node> {
        match expr {
            Expression::BooleanLiteral(_) => Some(self.prim(Builtin::PrimBoolean)),
            Expression::NullLiteral(_) => Some(self.prim(Builtin::PrimNull)),
            Expression::NumericLiteral(_) | Expression::BigIntLiteral(_) => {
                Some(self.prim(Builtin::PrimNumber))
            }
            Expression::StringLiteral(text) => Some(match text.value.as_str() {
                Some(text) => self.string(text),
                None => self.prim(Builtin::PrimString),
            }),
            Expression::TemplateLiteral(template) => Some(self.template(template)),
            Expression::RegExpLiteral(regex) => {
                let object = self
                    .g
                    .new_site(regex.span, SiteKind::Regex, &[Builtin::Other]);
                Some(self.g.singleton(object))
            }
            Expression::Identifier(id) => self.identifier(id),
            Expression::ThisExpression(_) => Some(self.frame().this),
            Expression::Super(_) => None,
            Expression::ArrayExpression(array) => Some(self.array(array)),
            Expression::ObjectExpression(object) => Some(self.object(object)),
            Expression::FunctionExpression(function) => Some(self.function(function, false)),
            Expression::ArrowFunctionExpression(arrow) => Some(self.arrow(arrow)),
            Expression::ClassExpression(class) => Some(self.class(class)),
            Expression::AssignmentExpression(assignment) => self.assignment(assignment),
            Expression::AwaitExpression(it) => {
                let value = self.expr(&it.argument);
                self.escape(value);
                let world = self.world();
                self.union(value, Some(world))
            }
            Expression::YieldExpression(it) => {
                let value = it.argument.as_ref().and_then(|value| self.expr(value));
                self.escape(value);
                Some(self.world())
            }
            Expression::BinaryExpression(binary) => Some(self.binary(binary)),
            Expression::LogicalExpression(logical) => {
                let left = self.expr(&logical.left);
                let right = self.expr(&logical.right);
                self.union(left, right)
            }
            Expression::ConditionalExpression(it) => {
                self.expr(&it.test);
                let consequent = self.expr(&it.consequent);
                let alternate = self.expr(&it.alternate);
                self.union(consequent, alternate)
            }
            Expression::CallExpression(call) => self.call(call),
            Expression::NewExpression(new) => self.new_call(new),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => self.call(call),
                ChainElement::TSNonNullExpression(it) => self.expr(&it.expression),
                other => other
                    .as_member_expression()
                    .and_then(|member| self.member_read(member)),
            },
            Expression::ImportExpression(it) => {
                let source = self.expr(&it.source);
                self.escape(source);
                if let Some(options) = &it.options {
                    let options = self.expr(options);
                    self.escape(options);
                }
                Some(self.world())
            }
            Expression::ParenthesizedExpression(it) => self.expr(&it.expression),
            Expression::SequenceExpression(it) => {
                let mut last = None;
                for expr in &it.expressions {
                    last = self.expr(expr);
                }
                last
            }
            Expression::TaggedTemplateExpression(it) => self.tagged(it),
            Expression::UnaryExpression(unary) => self.unary(unary),
            Expression::UpdateExpression(update) => Some(self.update(update)),
            Expression::PrivateInExpression(it) => {
                self.expr(&it.right);
                Some(self.prim(Builtin::PrimBoolean))
            }
            Expression::TSAsExpression(it) => self.expr(&it.expression),
            Expression::TSSatisfiesExpression(it) => self.expr(&it.expression),
            Expression::TSTypeAssertion(it) => self.expr(&it.expression),
            Expression::TSNonNullExpression(it) => self.expr(&it.expression),
            Expression::TSInstantiationExpression(it) => self.expr(&it.expression),
            other => match other.as_member_expression() {
                Some(member) => self.member_read(member),
                None => {
                    walk_js::walk_expression(self, other);
                    Some(self.world())
                }
            },
        }
    }

    fn identifier(&mut self, id: &IdentifierReference<'_>) -> Option<Node> {
        if let Some(symbol) = self.symbol_of(id) {
            return Some(self.binding(symbol));
        }
        match id.name.as_str() {
            "undefined" => Some(self.prim(Builtin::PrimUndefined)),
            "arguments" => match self.arguments_node() {
                Some(node) => Some(node),
                None => Some(self.global_read("arguments")),
            },
            name => Some(self.global_read(name)),
        }
    }

    fn template(&mut self, template: &TemplateLiteral<'_>) -> Node {
        for expr in &template.expressions {
            let value = self.expr(expr);
            if !primitive_syntax(expr) {
                if let Some(value) = value {
                    self.g.implicit(value);
                }
            }
        }
        let cooked = (template.expressions.is_empty())
            .then(|| template.quasis.first())
            .flatten()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str().map(str::to_string));
        match cooked {
            Some(text) => self.string(&text),
            None => self.prim(Builtin::PrimString),
        }
    }

    fn array(&mut self, array: &ArrayExpression<'_>) -> Node {
        let object = self
            .g
            .new_site(array.span, SiteKind::Array, &[Builtin::ArrayProto]);
        let fields = self.g.field_node(object, Field::UNKNOWN);
        for element in &array.elements {
            match element {
                ArrayExpressionElement::SpreadElement(spread) => {
                    if let Some(source) = self.expr(&spread.argument) {
                        let items = self.g.iterate(source);
                        self.g.copy(items, fields);
                    }
                }
                ArrayExpressionElement::Elision(_) => {}
                other => {
                    if let Some(expr) = other.as_expression() {
                        let value = self.expr(expr);
                        self.flow(value, fields);
                    }
                }
            }
        }
        self.g.singleton(object)
    }

    fn object(&mut self, object: &ObjectExpression<'_>) -> Node {
        let site = self
            .g
            .new_site(object.span, SiteKind::Object, &[Builtin::ObjectProto]);
        let node = self.g.singleton(site);
        let home = self.g.proto_node(site);
        for property in &object.properties {
            match property {
                ObjectPropertyKind::SpreadProperty(spread) => {
                    if let Some(source) = self.expr(&spread.argument) {
                        self.g.assign_fields(source, node);
                    }
                }
                ObjectPropertyKind::ObjectProperty(property) => {
                    let key = self.property_key(&property.key, property.computed);
                    let value = match &property.value {
                        Expression::FunctionExpression(function)
                            if property.method || property.kind != PropertyKind::Init =>
                        {
                            Some(self.function_with(function, true, Some(home)))
                        }
                        value => self.expr(value),
                    };
                    if let Some(value) = value {
                        self.define_member(node, key, value, property.kind);
                    }
                }
            }
        }
        node
    }

    pub(super) fn property_key(&mut self, key: &PropertyKey<'_>, computed: bool) -> Key {
        match key {
            PropertyKey::StaticIdentifier(id) => Key::Static(self.g.name(id.name.as_str())),
            PropertyKey::PrivateIdentifier(id) => {
                Key::Static(self.g.name(&format!("#{}", id.name.as_str())))
            }
            other => match other.as_expression() {
                Some(expr) if computed => self.key_of(expr).0,
                Some(Expression::StringLiteral(text)) => match text.value.as_str() {
                    Some(name) => Key::Static(self.g.name(name)),
                    None => Key::Static(Field::UNKNOWN),
                },
                Some(Expression::NumericLiteral(number)) => {
                    Key::Static(self.g.name(&number_key(number.value)))
                }
                Some(expr) => self.key_of(expr).0,
                None => Key::Static(Field::UNKNOWN),
            },
        }
    }

    fn key_of(&mut self, expr: &Expression<'_>) -> (Key, SiteKey) {
        match expr.without_parentheses() {
            Expression::StringLiteral(text) => {
                if let Some(name) = text.value.as_str() {
                    return (
                        Key::Static(self.g.name(name)),
                        SiteKey::Name(name.to_string()),
                    );
                }
            }
            Expression::NumericLiteral(number) => {
                let name = number_key(number.value);
                return (Key::Static(self.g.name(&name)), SiteKey::Name(name));
            }
            Expression::CallExpression(call) => {
                if let Some(name) = self.decoded.get(&call.span).cloned() {
                    self.call_args(&call.arguments);
                    return (Key::Static(self.g.name(&name)), SiteKey::Name(name));
                }
            }
            _ => {}
        }
        let value = self.expr(expr);
        match value {
            Some(node) => {
                if !primitive_syntax(expr) {
                    self.g.implicit(node);
                }
                (Key::Dynamic(node), SiteKey::Node(node))
            }
            None => (Key::Static(Field::UNKNOWN), SiteKey::Unknown),
        }
    }

    fn member_parts(&mut self, member: &MemberExpression<'_>) -> Member {
        let base = match member.object() {
            Expression::Super(_) => self.frame().home,
            object => self.expr(object),
        };
        let (key, site) = match member {
            MemberExpression::StaticMemberExpression(it) => {
                let name = it.property.name.as_str();
                (
                    Key::Static(self.g.name(name)),
                    SiteKey::Name(name.to_string()),
                )
            }
            MemberExpression::ComputedMemberExpression(it) => self.key_of(&it.expression),
            MemberExpression::PrivateFieldExpression(it) => {
                let name = format!("#{}", it.field.name.as_str());
                (Key::Static(self.g.name(&name)), SiteKey::Name(name))
            }
        };
        Member { base, key, site }
    }

    fn member_read(&mut self, member: &MemberExpression<'_>) -> Option<Node> {
        let parts = self.member_parts(member);
        let base = parts.base?;
        let node = self.g.node();
        self.g.load(base, parts.key, node);
        Some(node)
    }

    fn record_member(&mut self, member: &MemberExpression<'_>, parts: &Member) {
        if record_member_patch(member).is_some() {
            return;
        }
        let Some(base) = parts.base else {
            return;
        };
        let key = match &parts.site {
            SiteKey::Name(name) => SiteKey::Name(name.clone()),
            SiteKey::Node(node) => SiteKey::Node(*node),
            SiteKey::Unknown => SiteKey::Unknown,
        };
        self.members.push(MemberSite {
            span: member.span(),
            base,
            key,
        });
    }

    fn write_member(&mut self, member: &MemberExpression<'_>, value: Option<Node>) -> Member {
        let mut parts = self.member_parts(member);
        if matches!(member.object(), Expression::Super(_)) {
            parts.base = Some(self.frame().this);
        }
        self.record_member(member, &parts);
        if let (Some(base), Some(value)) = (parts.base, value) {
            self.g.store(base, parts.key, value);
        }
        parts
    }

    fn write_identifier(&mut self, id: &IdentifierReference<'_>, value: Option<Node>) {
        if let Some(symbol) = self.symbol_of(id) {
            let node = self.binding(symbol);
            self.flow(value, node);
            return;
        }
        let name = id.name.as_str();
        let global = self.global;
        if record_identifier_patch(id).is_none() {
            self.members.push(MemberSite {
                span: id.span,
                base: global,
                key: SiteKey::Name(name.to_string()),
            });
        }
        if let Some(value) = value {
            self.g.store_named(global, name, value);
        }
    }

    pub(super) fn assign_target(&mut self, target: &AssignmentTarget<'_>, value: Option<Node>) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => self.write_identifier(id, value),
            AssignmentTarget::ArrayAssignmentTarget(array) => {
                let items = value.map(|value| self.g.iterate(value));
                for element in array.elements.iter().flatten() {
                    self.assign_maybe_default(element, items);
                }
                if let Some(rest) = &array.rest {
                    let node = self.rest_array(rest.span, items);
                    self.assign_target(&rest.target, Some(node));
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(object) => {
                for property in &object.properties {
                    match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(it) => {
                            let read = self.read_named(value, it.binding.name.as_str());
                            let default = it.init.as_ref().and_then(|init| self.expr(init));
                            let read = self.union(read, default);
                            self.write_identifier(&it.binding, read);
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(it) => {
                            let key = self.property_key(&it.name, it.computed);
                            let read = self.read_key(value, key);
                            self.assign_maybe_default(&it.binding, read);
                        }
                    }
                }
                if let Some(rest) = &object.rest {
                    let node = self.rest_object(rest.span, value);
                    self.assign_target(&rest.target, Some(node));
                }
            }
            other => {
                if let Some(member) = other.as_member_expression() {
                    self.write_member(member, value);
                } else if let Some(simple) = other.as_simple_assignment_target() {
                    if let Some(inner) = simple.get_expression() {
                        let target = self.expr(inner);
                        self.escape(target);
                    }
                }
            }
        }
    }

    fn assign_maybe_default(
        &mut self,
        target: &AssignmentTargetMaybeDefault<'_>,
        value: Option<Node>,
    ) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(it) => {
                let default = self.expr(&it.init);
                let value = self.union(value, default);
                self.assign_target(&it.binding, value);
            }
            other => {
                if let Some(target) = other.as_assignment_target() {
                    self.assign_target(target, value);
                }
            }
        }
    }

    pub(super) fn bind_pattern(&mut self, pattern: &BindingPattern<'_>, value: Option<Node>) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                if let Some(symbol) = id.symbol_id.get() {
                    let node = self.binding(symbol);
                    self.flow(value, node);
                }
            }
            BindingPattern::AssignmentPattern(it) => {
                let default = self.expr(&it.right);
                let value = self.union(value, default);
                self.bind_pattern(&it.left, value);
            }
            BindingPattern::ArrayPattern(array) => {
                let items = value.map(|value| self.g.iterate(value));
                for element in array.elements.iter().flatten() {
                    self.bind_pattern(element, items);
                }
                if let Some(rest) = &array.rest {
                    let node = self.rest_array(rest.span, items);
                    self.bind_pattern(&rest.argument, Some(node));
                }
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    let key = self.property_key(&property.key, property.computed);
                    let read = self.read_key(value, key);
                    self.bind_pattern(&property.value, read);
                }
                if let Some(rest) = &object.rest {
                    let node = self.rest_object(rest.span, value);
                    self.bind_pattern(&rest.argument, Some(node));
                }
            }
        }
    }

    fn read_named(&mut self, value: Option<Node>, name: &str) -> Option<Node> {
        let field = self.g.name(name);
        self.read_key(value, Key::Static(field))
    }

    fn read_key(&mut self, value: Option<Node>, key: Key) -> Option<Node> {
        let value = value?;
        let node = self.g.node();
        self.g.load(value, key, node);
        Some(node)
    }

    fn rest_array(&mut self, span: Span, items: Option<Node>) -> Node {
        let object = self
            .g
            .new_site(span, SiteKind::Rest, &[Builtin::ArrayProto]);
        let fields = self.g.field_node(object, Field::UNKNOWN);
        self.flow(items, fields);
        self.g.singleton(object)
    }

    fn rest_object(&mut self, span: Span, value: Option<Node>) -> Node {
        let object = self
            .g
            .new_site(span, SiteKind::Fresh, &[Builtin::ObjectProto]);
        let node = self.g.singleton(object);
        if let Some(value) = value {
            self.g.assign_fields(value, node);
        }
        node
    }

    fn assignment(&mut self, it: &AssignmentExpression<'_>) -> Option<Node> {
        let value = self.expr(&it.right);
        match it.operator {
            AssignmentOperator::Assign => {
                self.assign_target(&it.left, value);
                value
            }
            operator => {
                let logical = matches!(
                    operator,
                    AssignmentOperator::LogicalOr
                        | AssignmentOperator::LogicalAnd
                        | AssignmentOperator::LogicalNullish
                );
                let old = self.read_target(&it.left);
                let written = if logical {
                    value
                } else {
                    for operand in [old, value].into_iter().flatten() {
                        self.g.implicit(operand);
                    }
                    Some(self.arithmetic(operator == AssignmentOperator::Addition, old, value))
                };
                self.assign_target(&it.left, written);
                if logical {
                    self.union(old, value)
                } else {
                    written
                }
            }
        }
    }

    fn read_target(&mut self, target: &AssignmentTarget<'_>) -> Option<Node> {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => self.identifier(id),
            other => other.as_member_expression().and_then(|member| {
                let parts = self.member_parts(member);
                let base = parts.base?;
                let node = self.g.node();
                self.g.load(base, parts.key, node);
                Some(node)
            }),
        }
    }

    fn arithmetic(&mut self, addition: bool, left: Option<Node>, right: Option<Node>) -> Node {
        if !addition {
            return self.prim(Builtin::PrimNumber);
        }
        let text = self.prim(Builtin::PrimString);
        let left = left.unwrap_or(text);
        let right = right.unwrap_or(text);
        let result = self.g.node();
        self.g.concat(left, right, result);
        result
    }

    fn binary(&mut self, binary: &BinaryExpression<'_>) -> Node {
        let left = self.expr(&binary.left);
        let right = self.expr(&binary.right);
        let operator = binary.operator;
        let converts = !matches!(
            operator,
            BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::Instanceof
        );
        if converts {
            if !primitive_syntax(&binary.left) {
                if let Some(left) = left {
                    self.g.implicit(left);
                }
            }
            if operator != BinaryOperator::In && !primitive_syntax(&binary.right) {
                if let Some(right) = right {
                    self.g.implicit(right);
                }
            }
        }
        match operator {
            BinaryOperator::Addition => self.arithmetic(true, left, right),
            BinaryOperator::Instanceof => {
                if let Some(right) = right {
                    let callee = self.g.node();
                    self.g.load_named(right, "@@hasInstance", callee);
                    self.g.call(Call {
                        callee,
                        this: Some(right),
                        args: left.into_iter().collect(),
                        spread: None,
                        result: None,
                        construct: false,
                        loose: false,
                        firm: false,
                        span: binary.span,
                    });
                }
                self.prim(Builtin::PrimBoolean)
            }
            operator
                if operator.is_equality()
                    || operator.is_compare()
                    || operator == BinaryOperator::In =>
            {
                self.prim(Builtin::PrimBoolean)
            }
            _ => self.prim(Builtin::PrimNumber),
        }
    }

    fn unary(&mut self, unary: &UnaryExpression<'_>) -> Option<Node> {
        match unary.operator {
            UnaryOperator::Delete => {
                match unary.argument.without_parentheses().as_member_expression() {
                    Some(member) => {
                        let parts = self.member_parts(member);
                        self.record_member(member, &parts);
                    }
                    None => {
                        self.expr(&unary.argument);
                    }
                }
                Some(self.prim(Builtin::PrimBoolean))
            }
            UnaryOperator::Typeof => {
                self.expr(&unary.argument);
                Some(self.prim(Builtin::PrimString))
            }
            UnaryOperator::Void => {
                self.expr(&unary.argument);
                Some(self.prim(Builtin::PrimUndefined))
            }
            UnaryOperator::LogicalNot => {
                self.expr(&unary.argument);
                Some(self.prim(Builtin::PrimBoolean))
            }
            _ => {
                let value = self.expr(&unary.argument);
                if !primitive_syntax(&unary.argument) {
                    if let Some(value) = value {
                        self.g.implicit(value);
                    }
                }
                Some(self.prim(Builtin::PrimNumber))
            }
        }
    }

    fn update(&mut self, update: &UpdateExpression<'_>) -> Node {
        let number = self.prim(Builtin::PrimNumber);
        match &update.argument {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                if let Some(old) = self.identifier(id) {
                    self.g.implicit(old);
                }
                self.write_identifier(id, Some(number));
            }
            other => {
                if let Some(member) = other.as_member_expression() {
                    let parts = self.write_member(member, Some(number));
                    if let Some(base) = parts.base {
                        let old = self.g.node();
                        self.g.load(base, parts.key, old);
                        self.g.implicit(old);
                    }
                } else if let Some(inner) = other.get_expression() {
                    self.expr(inner);
                }
            }
        }
        number
    }

    pub(super) fn call_args(&mut self, arguments: &[Argument<'_>]) -> (Vec<Node>, Option<Node>) {
        let mut args = Vec::new();
        let mut spread: Option<Node> = None;
        for argument in arguments {
            let value = match argument {
                Argument::SpreadElement(element) => {
                    let source = self.expr(&element.argument);
                    let items = source.map(|source| self.g.iterate(source));
                    let node = spread.unwrap_or_else(|| self.g.node());
                    self.flow(items, node);
                    spread = Some(node);
                    continue;
                }
                other => other.as_expression().and_then(|expr| self.expr(expr)),
            };
            match spread {
                Some(node) => self.flow(value, node),
                None => {
                    let empty = self.g.empty;
                    args.push(value.unwrap_or(empty));
                }
            }
        }
        (args, spread)
    }

    fn callee(&mut self, callee: &Expression<'_>) -> (Option<Node>, Option<Node>) {
        let callee = callee.without_parentheses();
        if let Expression::Super(_) = callee {
            return (self.frame().heritage, Some(self.frame().this));
        }
        if let Some(member) = callee.as_member_expression() {
            let parts = self.member_parts(member);
            let this = if matches!(member.object(), Expression::Super(_)) {
                Some(self.frame().this)
            } else {
                parts.base
            };
            let Some(base) = parts.base else {
                return (None, this);
            };
            let node = self.g.node();
            self.g.load(base, parts.key, node);
            return (Some(node), this);
        }
        if let Expression::ChainExpression(chain) = callee {
            if let Some(member) = chain.expression.as_member_expression() {
                let parts = self.member_parts(member);
                let Some(base) = parts.base else {
                    return (None, None);
                };
                let node = self.g.node();
                self.g.load(base, parts.key, node);
                return (Some(node), Some(base));
            }
        }
        (self.expr(callee), None)
    }

    fn call(&mut self, call: &CallExpression<'_>) -> Option<Node> {
        if let Some(text) = self.decoded.get(&call.span).cloned() {
            self.call_args(&call.arguments);
            return Some(self.string(&text));
        }
        if let Expression::Identifier(id) = call.callee.without_parentheses() {
            let direct = id.name == "eval"
                && self.symbol_of(id).is_none()
                && matches!(call.callee, Expression::Identifier(_));
            if direct {
                self.fallback.get_or_insert("a direct eval");
            }
        }
        if record_definer_patch(call).is_some() {
            self.g.skip_definers.insert(call.span);
        }
        let (callee, this) = self.callee(&call.callee);
        let firm = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|first| self.never_nullish(first));
        let (args, spread) = self.call_args(&call.arguments);
        let callee = callee?;
        let result = self.g.node();
        self.g.call(Call {
            callee,
            this,
            args,
            spread,
            result: Some(result),
            construct: false,
            loose: false,
            firm,
            span: call.span,
        });
        if matches!(call.callee.without_parentheses(), Expression::Super(_)) {
            if let Some(supers) = self.frame().supers {
                self.g.copy(result, supers);
            }
        }
        Some(result)
    }

    fn never_nullish(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::ThisExpression(_) => !self.strict(),
            Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::NewExpression(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::BigIntLiteral(_) => true,
            _ => false,
        }
    }

    fn new_call(&mut self, new: &NewExpression<'_>) -> Option<Node> {
        let callee = self.expr(&new.callee);
        let (args, spread) = self.call_args(&new.arguments);
        let callee = callee?;
        let result = self.g.node();
        self.g.call(Call {
            callee,
            this: None,
            args,
            spread,
            result: Some(result),
            construct: true,
            loose: false,
            firm: false,
            span: new.span,
        });
        Some(result)
    }

    fn tagged(&mut self, it: &TaggedTemplateExpression<'_>) -> Option<Node> {
        let (callee, this) = self.callee(&it.tag);
        let strings = self
            .g
            .new_site(it.quasi.span, SiteKind::Array, &[Builtin::ArrayProto]);
        let fields = self.g.field_node(strings, Field::UNKNOWN);
        let text = self.prim(Builtin::PrimString);
        self.g.copy(text, fields);
        let raw = self.g.singleton(strings);
        self.g.store_named(raw, "raw", raw);
        let mut args = vec![raw];
        for expr in &it.quasi.expressions {
            let value = self.expr(expr);
            let empty = self.g.empty;
            args.push(value.unwrap_or(empty));
        }
        let callee = callee?;
        let result = self.g.node();
        self.g.call(Call {
            callee,
            this,
            args,
            spread: None,
            result: Some(result),
            construct: false,
            loose: false,
            firm: false,
            span: it.span,
        });
        Some(result)
    }
}
