use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use std::borrow::Cow;

use super::bootstrap::Resolve;
use super::{AKey, ASite, Agnostic};
use crate::control_flow_object::single_declarator;
use crate::proof::ProtoRoot;

type Field<'a> = (Cow<'a, str>, &'a Expression<'a>);

pub(super) struct Assembled<'a> {
    pub(super) fields: Vec<Field<'a>>,
    writes: Vec<Span>,
}

pub(super) type AssembledTables<'a> = FxHashMap<SymbolId, Assembled<'a>>;

pub(super) fn assembled_tables<'a>(
    program: &'a Program<'a>,
    scoping: &Scoping,
    resolve: Resolve<'_>,
) -> AssembledTables<'a> {
    let mut scan = Scan {
        scoping,
        resolve,
        params: Vec::new(),
        found: FxHashMap::default(),
    };
    scan.visit_program(program);
    scan.found
}

pub(super) fn literal_fields<'a>(object: &'a ObjectExpression<'a>) -> Option<Vec<Field<'a>>> {
    object
        .properties
        .iter()
        .map(|property| {
            let ObjectPropertyKind::ObjectProperty(p) = property else {
                return None;
            };
            if p.kind != PropertyKind::Init || p.computed {
                return None;
            }
            let name = match &p.key {
                PropertyKey::StaticIdentifier(name) => name.name.as_str(),
                PropertyKey::StringLiteral(text) => text.value.as_str(),
                _ => return None,
            };
            (name != "__proto__").then_some((Cow::Borrowed(name), &p.value))
        })
        .collect()
}

pub(super) fn broken_tables(
    out: &Agnostic<'_>,
    tables: &AssembledTables<'_>,
    machinery: &[Span],
) -> Vec<SymbolId> {
    let unjudged = out.saturated || out.code_text.is_some();
    tables
        .iter()
        .filter(|(_, table)| unjudged || table.may_be_shadowed(&out.sites, machinery))
        .map(|(alias, _)| *alias)
        .collect()
}

impl Assembled<'_> {
    fn may_be_shadowed(&self, sites: &[ASite<'_>], machinery: &[Span]) -> bool {
        let inside = |span: Span| {
            machinery
                .iter()
                .any(|outer| outer.start <= span.start && span.end <= outer.end)
        };
        sites
            .iter()
            .filter(|site| matches!(site.root, ProtoRoot::Object | ProtoRoot::Either))
            .filter(|site| !inside(site.span) && !self.writes.contains(&site.span))
            .any(|site| {
                self.fields
                    .iter()
                    .any(|(name, _)| key_may_be(&site.key, name))
            })
    }
}

fn key_may_be(key: &AKey<'_>, name: &str) -> bool {
    match key {
        AKey::Named(written) => written == name,
        AKey::Index => !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()),
        AKey::Pending(_) | AKey::Unknown => true,
    }
}

const OBJECT_PROTOTYPE_KEYS: [&str; 12] = [
    "constructor",
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "__proto__",
];

pub(super) fn drop_tables(tables: &mut AssembledTables<'_>, aliases: &[SymbolId]) {
    for alias in aliases {
        tables.remove(alias);
    }
}

struct Scan<'s, 'k, 'a> {
    scoping: &'s Scoping,
    resolve: Resolve<'k>,
    params: Vec<SymbolId>,
    found: AssembledTables<'a>,
}

impl<'a> Scan<'_, '_, 'a> {
    fn run_at(
        &self,
        statements: &'a [Statement<'a>],
        index: usize,
    ) -> Option<(SymbolId, Assembled<'a>)> {
        let (table, object) = object_declaration(statements.get(index)?)?;
        if self.params.contains(&table) {
            return None;
        }
        let mut fields = literal_fields(object)?;
        let mut writes = Vec::new();
        while let Some((span, field)) = statements
            .get(index + 1 + writes.len())
            .and_then(|statement| self.key_assignment(statement, table))
        {
            fields.push(field);
            writes.push(span);
        }
        if writes.is_empty() {
            return None;
        }
        let alias = self.alias_of(statements.get(index + 1 + writes.len())?, table)?;
        let references = self.scoping.get_resolved_reference_ids(table).len();
        if references != writes.len() + 1 || self.scoping.symbol_is_mutated(table) {
            return None;
        }
        Some((alias, Assembled { fields, writes }))
    }

    fn key_assignment(
        &self,
        statement: &'a Statement<'a>,
        table: SymbolId,
    ) -> Option<(Span, Field<'a>)> {
        let Statement::ExpressionStatement(statement) = statement else {
            return None;
        };
        let Expression::AssignmentExpression(assignment) = &statement.expression else {
            return None;
        };
        if assignment.operator != AssignmentOperator::Assign {
            return None;
        }
        let member = assignment.left.as_member_expression()?;
        if self.symbol_of(member.object())? != table {
            return None;
        }
        let key = match member {
            MemberExpression::StaticMemberExpression(m) => Cow::Borrowed(m.property.name.as_str()),
            MemberExpression::ComputedMemberExpression(m) => self.key_text(&m.expression)?,
            MemberExpression::PrivateFieldExpression(_) => return None,
        };
        (!OBJECT_PROTOTYPE_KEYS.contains(&key.as_ref()))
            .then_some((member.span(), (key, &assignment.right)))
    }

    fn key_text(&self, expr: &'a Expression<'a>) -> Option<Cow<'a, str>> {
        match expr.without_parentheses() {
            Expression::StringLiteral(text) => Some(Cow::Borrowed(text.value.as_str())),
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => {
                Some(Cow::Borrowed(t.quasis.first()?.value.cooked?.as_str()))
            }
            Expression::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                let left = self.key_text(&b.left)?;
                let right = self.key_text(&b.right)?;
                Some(Cow::Owned(format!("{left}{right}")))
            }
            Expression::CallExpression(call) => {
                self.resolve?.get(&call.span)?.as_string().map(Cow::Owned)
            }
            _ => None,
        }
    }

    fn alias_of(&self, statement: &Statement<'a>, table: SymbolId) -> Option<SymbolId> {
        let (id, init) = single_declarator(statement)?;
        (self.symbol_of(init?)? == table).then_some(id.symbol_id.get()?)
    }

    fn symbol_of(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        self.scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()
    }
}

fn object_declaration<'a>(
    statement: &'a Statement<'a>,
) -> Option<(SymbolId, &'a ObjectExpression<'a>)> {
    let (id, init) = single_declarator(statement)?;
    let Expression::ObjectExpression(object) = init? else {
        return None;
    };
    Some((id.symbol_id.get()?, object))
}

impl<'a> VisitJs<'a> for Scan<'_, '_, 'a> {
    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        for param in &it.items {
            if let BindingPattern::BindingIdentifier(id) = &param.pattern {
                self.params.extend(id.symbol_id.get());
            }
        }
        walk_js::walk_formal_parameters(self, it);
    }

    fn visit_statements(&mut self, it: &ArenaVec<'a, Statement<'a>>) {
        let statements: &'a ArenaVec<'a, Statement<'a>> = self.alloc(it);
        for index in 0..statements.len() {
            if let Some((alias, table)) = self.run_at(statements, index) {
                self.found.insert(alias, table);
            }
        }
        walk_js::walk_statements(self, it);
    }
}
