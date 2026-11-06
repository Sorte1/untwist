use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) fn bound_methods(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<u32> {
    let mut literals = Literals {
        scoping,
        found: FxHashMap::default(),
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
    found: FxHashMap<SymbolId, Literal>,
}

impl<'a> VisitJs<'a> for Literals<'_> {
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
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        let Some(Expression::ObjectExpression(object)) = declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
        else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        if self.scoping.symbol_is_mutated(symbol) {
            return;
        }
        let mut literal = Literal::default();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            let Some(function) = own_function(&property.value) else {
                continue;
            };
            let Some(key) = property.key.static_name() else {
                return;
            };
            if property.kind == PropertyKind::Init {
                literal.methods.insert(key.to_string());
            }
            literal.functions.push(function.span.start);
        }
        self.found.insert(symbol, literal);
    }
}

fn own_function<'k, 'a>(value: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    match value.without_parentheses() {
        Expression::FunctionExpression(function) if function.id.is_none() => Some(function),
        _ => None,
    }
}

struct Uses<'s, 'l> {
    scoping: &'s Scoping,
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
        let key = member.static_property_name();
        let leaks = match key {
            Some(key) => !called && self.literals[&symbol].methods.contains(key),
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
