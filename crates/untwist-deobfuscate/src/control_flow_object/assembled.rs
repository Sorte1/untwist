use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::member_parts;
use untwist_core::semantic::is_constant_binding;

const PROTO_KEY: &str = "__proto__";

pub(super) struct Assembled<'p> {
    base: SymbolId,
    object: &'p ObjectExpression<'p>,
    added: Vec<(&'p str, &'p Expression<'p>)>,
    exempt: FxHashSet<usize>,
}

fn address_of(id: &IdentifierReference<'_>) -> usize {
    std::ptr::from_ref(id) as usize
}

fn resolved(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| scoping.get_reference(reference).symbol_id())
}

fn sole_const_declarator<'p>(stmt: &'p Statement<'p>) -> Option<(SymbolId, &'p Expression<'p>)> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Const || declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = &declaration.declarations[0];
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    Some((id.symbol_id.get()?, declarator.init.as_ref()?))
}

fn plain_literal_keys(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().all(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        let key = match &property.key {
            PropertyKey::StaticIdentifier(id) if !property.computed => Some(id.name.as_str()),
            PropertyKey::StringLiteral(text) => text.value.as_str(),
            _ => None,
        };
        !property.method
            && property.kind == PropertyKind::Init
            && key.is_some_and(|key| key != PROTO_KEY)
    })
}

fn added_key<'p>(
    stmt: &'p Statement<'p>,
    base: SymbolId,
    scoping: &Scoping,
) -> Option<(&'p str, &'p Expression<'p>, usize)> {
    let Statement::ExpressionStatement(statement) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let (object, name) = match &assignment.left {
        AssignmentTarget::StaticMemberExpression(member) if !member.optional => {
            (&member.object, member.property.name.as_str())
        }
        AssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
            let Expression::StringLiteral(key) = &member.expression else {
                return None;
            };
            (&member.object, key.value.as_str()?)
        }
        _ => return None,
    };
    let Expression::Identifier(object) = object else {
        return None;
    };
    if name == PROTO_KEY || resolved(object, scoping) != Some(base) {
        return None;
    }
    Some((name, &assignment.right, address_of(object)))
}

pub(super) fn assembled_alias<'p>(
    statements: &'p [Statement<'p>],
    scoping: &Scoping,
) -> Option<(SymbolId, Assembled<'p>)> {
    let (first, rest) = statements.split_first()?;
    let (base, Expression::ObjectExpression(object)) = sole_const_declarator(first)? else {
        return None;
    };
    if !plain_literal_keys(object) {
        return None;
    }
    let mut added = Vec::new();
    let mut exempt = FxHashSet::default();
    for stmt in rest {
        if let Some((name, value, address)) = added_key(stmt, base, scoping) {
            added.push((name, value));
            exempt.insert(address);
            continue;
        }
        let (alias, Expression::Identifier(init)) = sole_const_declarator(stmt)? else {
            return None;
        };
        if resolved(init, scoping) != Some(base) {
            return None;
        }
        exempt.insert(address_of(init));
        let constant = is_constant_binding(scoping, base) && is_constant_binding(scoping, alias);
        return constant.then_some((
            alias,
            Assembled {
                base,
                object,
                added,
                exempt,
            },
        ));
    }
    None
}

impl<'p> Assembled<'p> {
    pub(super) fn plain(
        symbol: SymbolId,
        object: &'p ObjectExpression<'p>,
        scoping: &Scoping,
    ) -> Option<Self> {
        let plain = is_constant_binding(scoping, symbol) && plain_literal_keys(object);
        plain.then(|| Self {
            base: symbol,
            object,
            added: Vec::new(),
            exempt: FxHashSet::default(),
        })
    }

    pub(super) fn property(&self, name: &str) -> Option<&'p Expression<'p>> {
        if let Some((_, value)) = self.added.iter().rev().find(|(key, _)| *key == name) {
            return Some(value);
        }
        self.object.properties.iter().rev().find_map(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return None;
            };
            let key = match &property.key {
                PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
                PropertyKey::StringLiteral(text) => text.value.as_str(),
                _ => None,
            };
            (key == Some(name)).then_some(&property.value)
        })
    }
}

struct ThisFree {
    found: bool,
}

impl<'a> VisitJs<'a> for ThisFree {
    fn visit_this_expression(&mut self, _: &ThisExpression) {
        self.found = true;
    }
}

fn this_free_function(value: &Expression<'_>) -> bool {
    let Expression::FunctionExpression(function) = value.without_parentheses() else {
        return false;
    };
    if function.r#async || function.generator {
        return false;
    }
    let mut scan = ThisFree { found: false };
    scan.visit_function(function, oxc_syntax::scope::ScopeFlags::Function);
    !scan.found
}

struct AliasUses<'s, 'k, 'p> {
    scoping: &'s Scoping,
    alias: SymbolId,
    assembled: &'k Assembled<'p>,
    dead: &'k FxHashSet<usize>,
    depth: usize,
    reads: FxHashSet<usize>,
    risky: FxHashSet<usize>,
    live: bool,
}

impl AliasUses<'_, '_, '_> {
    fn ours(&self, id: &IdentifierReference<'_>) -> bool {
        resolved(id, self.scoping)
            .is_some_and(|symbol| symbol == self.alias || symbol == self.assembled.base)
    }

    fn receiver<'e, 'a>(&self, expr: &'e Expression<'a>) -> Option<&'e IdentifierReference<'a>> {
        let (object, _) = member_parts(expr)?;
        let Expression::Identifier(id) = object else {
            return None;
        };
        self.ours(id).then_some(&**id)
    }

    fn entering(&mut self, node: usize) -> bool {
        let dead = self.dead.contains(&node);
        if dead {
            self.depth += 1;
        }
        dead
    }
}

impl<'a> VisitJs<'a> for AliasUses<'_, '_, 'a> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        walk_js::walk_statement(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        if let Some(id) = self.receiver(it) {
            self.reads.insert(address_of(id));
        }
        match it {
            Expression::CallExpression(call) => {
                let callee = call.callee.without_parentheses();
                if let Some(id) = self.receiver(callee) {
                    let safe = member_parts(callee)
                        .and_then(|(_, name)| self.assembled.property(&name))
                        .is_some_and(this_free_function);
                    if !safe {
                        self.risky.insert(address_of(id));
                    }
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                if let Some(id) = self.receiver(tagged.tag.without_parentheses()) {
                    self.risky.insert(address_of(id));
                }
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                if let Some(id) = self.receiver(unary.argument.without_parentheses()) {
                    self.risky.insert(address_of(id));
                }
            }
            _ => {}
        }
        walk_js::walk_expression(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.depth > 0 || !self.ours(it) {
            return;
        }
        let address = address_of(it);
        if self.assembled.exempt.contains(&address) {
            return;
        }
        if self.risky.contains(&address) || !self.reads.contains(&address) {
            self.live = true;
        }
    }
}

pub(super) fn alias_mutations_only_in_dead(
    program: &Program<'_>,
    scoping: &Scoping,
    alias: SymbolId,
    assembled: &Assembled<'_>,
    dead: &FxHashSet<usize>,
) -> bool {
    let mut uses = AliasUses {
        scoping,
        alias,
        assembled,
        dead,
        depth: 0,
        reads: FxHashSet::default(),
        risky: FxHashSet::default(),
        live: false,
    };
    uses.visit_program(program);
    !uses.live
}
