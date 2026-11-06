use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;

use super::bootstrap::Resolve;
use crate::value_flow::{flow_rule_disabled, BUILT_LITERAL};

pub(super) fn bound_methods(
    program: &Program<'_>,
    scoping: &Scoping,
    resolve: Resolve<'_>,
) -> FxHashSet<u32> {
    let mut literals = Literals {
        scoping,
        resolve,
        found: FxHashMap::default(),
        tables: FxHashSet::default(),
    };
    literals.visit_program(program);
    if literals.found.is_empty() {
        return FxHashSet::default();
    }
    let owners = literals
        .found
        .iter()
        .flat_map(|(symbol, literal)| literal.functions.iter().map(|start| (*start, *symbol)))
        .collect();
    let mut uses = Uses {
        scoping,
        resolve,
        literals: &literals.found,
        owners,
        own: Vec::new(),
        leaked: FxHashSet::default(),
    };
    uses.visit_program(program);
    let leaked = uses.leaked;
    literals
        .found
        .into_iter()
        .filter(|(symbol, _)| !leaked.contains(symbol))
        .flat_map(|(_, literal)| literal.functions)
        .collect()
}

#[derive(Default)]
struct Literal {
    functions: Vec<u32>,
    methods: FxHashSet<String>,
}

struct Literals<'s> {
    scoping: &'s Scoping,
    resolve: Resolve<'s>,
    found: FxHashMap<SymbolId, Literal>,
    tables: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Literals<'_> {
    fn visit_statements(&mut self, it: &ArenaVec<'a, Statement<'a>>) {
        for index in 0..it.len() {
            self.note_built(it, index);
        }
        walk_js::walk_statements(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind.is_lexical() {
            for declarator in &it.declarations {
                self.note(declarator);
            }
        }
        walk_js::walk_variable_declaration(self, it);
    }
}

impl Literals<'_> {
    fn note(&mut self, declarator: &VariableDeclarator<'_>) {
        let Some((symbol, object)) = object_declarator(declarator) else {
            return;
        };
        if self.tables.contains(&symbol) || self.scoping.symbol_is_mutated(symbol) {
            return;
        }
        if let Some(literal) = literal_of(object) {
            self.found.insert(symbol, literal);
        }
    }

    fn note_built(&mut self, statements: &[Statement<'_>], index: usize) -> Option<()> {
        let Statement::VariableDeclaration(declaration) = statements.get(index)? else {
            return None;
        };
        let [declarator] = declaration.declarations.as_slice() else {
            return None;
        };
        if !declaration.kind.is_lexical() {
            return None;
        }
        let (table, object) = object_declarator(declarator)?;
        let mut literal = literal_of(object)?;
        let mut writes = 0;
        let mut next = index + 1;
        while let Some(statement) = statements.get(next) {
            if !matches!(statement, Statement::FunctionDeclaration(_)) {
                let Some(assigned) = self.keyed_writes(statement, table) else {
                    break;
                };
                for (key, value) in assigned {
                    if let Some(function) = own_function(value) {
                        literal.methods.insert(key);
                        literal.functions.push(function.span.start);
                    }
                    writes += 1;
                }
            }
            next += 1;
        }
        let alias = self.alias_of(statements.get(next)?, table)?;
        let references = self.scoping.get_resolved_reference_ids(table).len();
        let unchecked = flow_rule_disabled(BUILT_LITERAL);
        let sole = references == writes + 1 && !self.scoping.symbol_is_mutated(table);
        if writes == 0 || !(unchecked || sole) || self.scoping.symbol_is_mutated(alias) {
            return None;
        }
        self.tables.insert(table);
        self.found.insert(alias, literal);
        Some(())
    }

    fn keyed_writes<'k, 'a>(
        &self,
        statement: &'k Statement<'a>,
        table: SymbolId,
    ) -> Option<Vec<(String, &'k Expression<'a>)>> {
        let Statement::ExpressionStatement(statement) = statement else {
            return None;
        };
        match &statement.expression {
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .iter()
                .map(|expression| self.keyed_write(expression, table))
                .collect(),
            expression => Some(vec![self.keyed_write(expression, table)?]),
        }
    }

    fn keyed_write<'k, 'a>(
        &self,
        expression: &'k Expression<'a>,
        table: SymbolId,
    ) -> Option<(String, &'k Expression<'a>)> {
        let Expression::AssignmentExpression(assignment) = expression else {
            return None;
        };
        let member = assignment.left.as_member_expression()?;
        if assignment.operator != AssignmentOperator::Assign
            || symbol_of(self.scoping, member.object())? != table
        {
            return None;
        }
        let key = member_key(member, self.resolve)?;
        let proto = key == "__proto__" && !flow_rule_disabled(BUILT_LITERAL);
        (!proto).then(|| (key.into_owned(), &assignment.right))
    }

    fn alias_of(&self, statement: &Statement<'_>, table: SymbolId) -> Option<SymbolId> {
        let Statement::VariableDeclaration(declaration) = statement else {
            return None;
        };
        let declarator = declaration.declarations.first()?;
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        let init = declarator.init.as_ref()?;
        (declaration.kind.is_lexical() && symbol_of(self.scoping, init)? == table)
            .then_some(id.symbol_id.get()?)
    }
}

fn object_declarator<'k, 'a>(
    declarator: &'k VariableDeclarator<'a>,
) -> Option<(SymbolId, &'k ObjectExpression<'a>)> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let Some(Expression::ObjectExpression(object)) = declarator
        .init
        .as_ref()
        .map(Expression::without_parentheses)
    else {
        return None;
    };
    Some((id.symbol_id.get()?, object))
}

fn literal_of(object: &ObjectExpression<'_>) -> Option<Literal> {
    let mut literal = Literal::default();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let Some(function) = own_function(&property.value) else {
            continue;
        };
        let key = property.key.static_name()?;
        if property.kind == PropertyKind::Init {
            literal.methods.insert(key.to_string());
        }
        literal.functions.push(function.span.start);
    }
    Some(literal)
}

fn symbol_of(scoping: &Scoping, expr: &Expression<'_>) -> Option<SymbolId> {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn member_key<'k>(member: &'k MemberExpression<'_>, resolve: Resolve<'_>) -> Option<Cow<'k, str>> {
    if let Some(key) = member.static_property_name() {
        return Some(Cow::Borrowed(key));
    }
    let MemberExpression::ComputedMemberExpression(computed) = member else {
        return None;
    };
    let Expression::CallExpression(call) = computed.expression.without_parentheses() else {
        return None;
    };
    resolve?.get(&call.span)?.as_string().map(Cow::Owned)
}

fn own_function<'k, 'a>(value: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    match value.without_parentheses() {
        Expression::FunctionExpression(function) if function.id.is_none() => Some(function),
        _ => None,
    }
}

struct Uses<'s, 'l> {
    scoping: &'s Scoping,
    resolve: Resolve<'s>,
    literals: &'l FxHashMap<SymbolId, Literal>,
    owners: FxHashMap<u32, SymbolId>,
    own: Vec<Option<SymbolId>>,
    leaked: FxHashSet<SymbolId>,
}

impl Uses<'_, '_> {
    fn literal_of(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        match expr.without_parentheses() {
            Expression::Identifier(id) => {
                let reference = id.reference_id.get()?;
                let symbol = self.scoping.get_reference(reference).symbol_id()?;
                self.literals.contains_key(&symbol).then_some(symbol)
            }
            Expression::ThisExpression(_) => self.own.last().copied().flatten(),
            _ => None,
        }
    }

    fn member_read(&mut self, member: &MemberExpression<'_>, called: bool) -> bool {
        let Some(symbol) = self.literal_of(member.object()) else {
            return false;
        };
        let key = member_key(member, self.resolve);
        let leaks = match key {
            Some(key) => !called && self.literals[&symbol].methods.contains(key.as_ref()),
            None => true,
        };
        if leaks {
            self.leaked.insert(symbol);
        }
        if let MemberExpression::ComputedMemberExpression(computed) = member {
            self.visit_expression(&computed.expression);
        }
        true
    }

    fn enter(&mut self, function: &Function<'_>) {
        let bound = self.owners.get(&function.span.start).copied();
        if let Some(symbol) = bound {
            if super::bind::reads_arguments(function) {
                self.leaked.insert(symbol);
            }
        }
        self.own.push(bound);
    }
}

impl<'a> VisitJs<'a> for Uses<'_, '_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(member) = it.callee.without_parentheses().as_member_expression() {
            if self.member_read(member, true) {
                for argument in &it.arguments {
                    self.visit_argument(argument);
                }
                return;
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if !self.member_read(it, false) {
            walk_js::walk_member_expression(self, it);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        if let Some(symbol) = self.scoping.get_reference(reference).symbol_id() {
            if self.literals.contains_key(&symbol) {
                self.leaked.insert(symbol);
            }
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        let read = it
            .as_member_expression()
            .is_some_and(|member| self.member_read(member, false));
        if !read {
            walk_js::walk_simple_assignment_target(self, it);
        }
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        if let Some(Some(symbol)) = self.own.last() {
            self.leaked.insert(*symbol);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.enter(it);
        walk_js::walk_function(self, it, flags);
        self.own.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.own.push(None);
        walk_js::walk_class(self, it);
        self.own.pop();
    }
}
