use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{Scoping, SymbolId};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::{FxHashMap, FxHashSet};

use super::written::{text_template, WrittenKey};
use crate::value_flow::{flow_rule_disabled, STORAGE_PROXY};

#[derive(Clone, Debug)]
pub(super) enum StorageMember {
    Text(WrittenKey),
    Concat,
    Other,
}

pub(super) type StorageObjects = FxHashMap<SymbolId, FxHashMap<String, StorageMember>>;

pub(super) struct Storage<'s> {
    pub(super) scoping: &'s Scoping,
    pub(super) objects: &'s StorageObjects,
}

fn literal_key<'e>(key: &PropertyKey<'e>) -> Option<&'e str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

fn simple_param(param: &FormalParameter<'_>) -> Option<SymbolId> {
    match (&param.pattern, &param.initializer) {
        (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
        _ => None,
    }
}

fn is_param(scoping: &Scoping, expr: &Expression<'_>, symbol: SymbolId) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
        == Some(symbol)
}

fn concat_forwarder(scoping: &Scoping, function: &Function<'_>) -> bool {
    if function.r#async || function.generator || function.params.rest.is_some() {
        return false;
    }
    let [left, right] = function.params.items.as_slice() else {
        return false;
    };
    let (Some(left), Some(right)) = (simple_param(left), simple_param(right)) else {
        return false;
    };
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return false;
    };
    let Some(Expression::BinaryExpression(binary)) =
        ret.argument.as_ref().map(Expression::without_parentheses)
    else {
        return false;
    };
    binary.operator == BinaryOperator::Addition
        && is_param(scoping, &binary.left, left)
        && is_param(scoping, &binary.right, right)
}

fn member_of(scoping: &Scoping, property: &ObjectProperty<'_>, trusted: bool) -> StorageMember {
    if property.method {
        return StorageMember::Other;
    }
    match property.value.without_parentheses() {
        Expression::FunctionExpression(function)
            if trusted || concat_forwarder(scoping, function) =>
        {
            StorageMember::Concat
        }
        value => text_template(value, None).map_or(StorageMember::Other, StorageMember::Text),
    }
}

fn members_of(
    scoping: &Scoping,
    object: &ObjectExpression<'_>,
    trusted: bool,
) -> Option<FxHashMap<String, StorageMember>> {
    let mut members = FxHashMap::default();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.kind != PropertyKind::Init || property.computed {
            return None;
        }
        let key = literal_key(&property.key)?;
        if key == "__proto__" {
            return None;
        }
        let member = member_of(scoping, property, trusted);
        if members.insert(key.to_string(), member).is_some() {
            return None;
        }
    }
    Some(members)
}

struct StorageScan<'s> {
    scoping: &'s Scoping,
    objects: StorageObjects,
    rejected: FxHashSet<SymbolId>,
    dynamic_scope: bool,
    trusted: bool,
}

impl StorageScan<'_> {
    fn symbol_of(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn reject(&mut self, expr: &Expression<'_>) {
        if let Some(symbol) = self.symbol_of(expr) {
            self.rejected.insert(symbol);
        }
    }

    fn reject_member_target(&mut self, target: &SimpleAssignmentTarget<'_>) {
        if let Some(member) = target.as_member_expression() {
            self.reject(member.object());
        }
    }
}

impl<'a> VisitJs<'a> for StorageScan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let (Some(symbol), Expression::ObjectExpression(object)) =
                (id.symbol_id.get(), init.without_parentheses())
            {
                let fresh = !self.objects.contains_key(&symbol);
                match members_of(self.scoping, object, self.trusted).filter(|_| fresh) {
                    Some(members) => {
                        self.objects.insert(symbol, members);
                    }
                    None => {
                        self.rejected.insert(symbol);
                    }
                }
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        self.reject_member_target(it);
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(member) = it.argument.without_parentheses().as_member_expression() {
                self.reject(member.object());
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.symbol_of(&it.object).is_none() {
            self.visit_expression(&it.object);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let keyed = matches!(
            it.expression.without_parentheses(),
            Expression::StringLiteral(_) | Expression::CallExpression(_)
        );
        if !keyed {
            self.reject(&it.object);
        }
        if self.symbol_of(&it.object).is_none() {
            self.visit_expression(&it.object);
        }
        self.visit_expression(&it.expression);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        {
            self.rejected.insert(symbol);
        }
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.dynamic_scope = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if matches!(it.callee.without_parentheses(), Expression::Identifier(id) if id.name == "eval")
        {
            self.dynamic_scope = true;
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub(super) fn scan_storage_objects(program: &Program<'_>, scoping: &Scoping) -> StorageObjects {
    let mut scan = StorageScan {
        scoping,
        objects: FxHashMap::default(),
        rejected: FxHashSet::default(),
        dynamic_scope: false,
        trusted: flow_rule_disabled(STORAGE_PROXY),
    };
    scan.visit_program(program);
    if scan.trusted {
        return scan.objects;
    }
    if scan.dynamic_scope {
        return FxHashMap::default();
    }
    let StorageScan {
        mut objects,
        rejected,
        ..
    } = scan;
    objects.retain(|symbol, _| !rejected.contains(symbol) && !scoping.symbol_is_mutated(*symbol));
    objects
}

impl Storage<'_> {
    fn members(&self, object: &Expression<'_>) -> Option<&FxHashMap<String, StorageMember>> {
        let Expression::Identifier(id) = object.without_parentheses() else {
            return None;
        };
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())?;
        self.objects.get(&symbol)
    }

    fn choose(
        member: &MemberExpression<'_>,
        members: &FxHashMap<String, StorageMember>,
        pick: impl Fn(&StorageMember) -> Option<WrittenKey>,
    ) -> Option<WrittenKey> {
        if let Some(name) = member.static_property_name() {
            return members.get(name).and_then(pick);
        }
        let MemberExpression::ComputedMemberExpression(computed) = member else {
            return None;
        };
        let Expression::CallExpression(key) = computed.expression.without_parentheses() else {
            return None;
        };
        let options = members
            .iter()
            .filter_map(|(name, value)| Some((name.clone(), pick(value)?)))
            .collect();
        Some(WrittenKey::Choice {
            key: key.span,
            options,
        })
    }

    pub(super) fn read_template(&self, expr: &Expression<'_>) -> Option<WrittenKey> {
        let member = expr.without_parentheses().as_member_expression()?;
        let members = self.members(member.object())?;
        Self::choose(member, members, |value| match value {
            StorageMember::Text(text) => Some(text.clone()),
            StorageMember::Concat | StorageMember::Other => None,
        })
    }

    pub(super) fn call_template(&self, call: &CallExpression<'_>) -> Option<WrittenKey> {
        let member = call.callee.without_parentheses().as_member_expression()?;
        let members = self.members(member.object())?;
        let [left, right] = call.arguments.as_slice() else {
            return None;
        };
        let left = text_template(left.as_expression()?, Some(self))?;
        let right = text_template(right.as_expression()?, Some(self))?;
        Self::choose(member, members, |value| match value {
            StorageMember::Concat => Some(WrittenKey::Concat(vec![left.clone(), right.clone()])),
            StorageMember::Text(_) | StorageMember::Other => None,
        })
    }

    pub(super) fn is_storage_call(&self, call: &CallExpression<'_>) -> bool {
        call.callee
            .without_parentheses()
            .as_member_expression()
            .is_some_and(|member| self.members(member.object()).is_some())
    }
}
