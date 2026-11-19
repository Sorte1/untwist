use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::member_parts;
use untwist_core::semantic::is_constant_binding;

use super::dead::OBJECT_PROTOTYPE_KEYS;

const PROTO_KEY: &str = "__proto__";

pub(super) struct Assembled<'p> {
    root: Option<&'p FunctionBody<'p>>,
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
            PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
            PropertyKey::StringLiteral(text) => text.value.as_str(),
            _ => None,
        };
        !property.method
            && !property.computed
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
                root: None,
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
    pub(super) fn reaches_its_receiver(&self) -> bool {
        let mut scan = ThisFree { found: false };
        scan.visit_object_expression(self.object);
        for (_, value) in &self.added {
            scan.visit_expression(value);
        }
        scan.found
    }

    pub(super) fn within(self, root: Option<&'p FunctionBody<'p>>) -> Self {
        Self { root, ..self }
    }

    pub(super) fn plain(
        symbol: SymbolId,
        object: &'p ObjectExpression<'p>,
        scoping: &Scoping,
    ) -> Option<Self> {
        let plain = is_constant_binding(scoping, symbol) && plain_literal_keys(object);
        plain.then(|| Self {
            root: None,
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

    fn visit_super(&mut self, _: &Super) {
        self.found = true;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if matches!(it.name.as_str(), "arguments" | "eval") {
            self.found = true;
        }
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
    objects: FxHashMap<SymbolId, &'k Assembled<'p>>,
    dead: &'k FxHashSet<usize>,
    builtins_unpatched: bool,
    depth: usize,
    reads: FxHashSet<usize>,
    risky: FxHashSet<usize>,
    live: bool,
}

impl<'k, 'p> AliasUses<'_, 'k, 'p> {
    fn ours(&self, id: &IdentifierReference<'_>) -> Option<&'k Assembled<'p>> {
        resolved(id, self.scoping).and_then(|symbol| self.objects.get(&symbol).copied())
    }

    fn receiver<'e, 'a>(
        &self,
        expr: &'e Expression<'a>,
    ) -> Option<(&'e IdentifierReference<'a>, &'k Assembled<'p>)> {
        let (object, _) = member_parts(expr)?;
        let Expression::Identifier(id) = object else {
            return None;
        };
        self.ours(id).map(|assembled| (&**id, assembled))
    }

    fn entering(&mut self, node: usize) -> bool {
        let dead = self.dead.contains(&node);
        if dead {
            self.depth += 1;
        }
        dead
    }
}

impl<'a> VisitJs<'a> for AliasUses<'_, '_, '_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        walk_js::walk_statement(self, it);
        if dead {
            self.depth -= 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let dead = self.entering(std::ptr::from_ref(it) as usize);
        if let Some((id, assembled)) = self.receiver(it) {
            let own = member_parts(it).is_some_and(|(_, name)| {
                assembled.property(&name).is_some()
                    || (self.builtins_unpatched && !OBJECT_PROTOTYPE_KEYS.contains(&name.as_ref()))
            });
            if own {
                self.reads.insert(address_of(id));
            } else {
                self.risky.insert(address_of(id));
            }
        }
        match it {
            Expression::CallExpression(call) => {
                let callee = call.callee.without_parentheses();
                if let Some((id, assembled)) = self.receiver(callee) {
                    let safe = member_parts(callee)
                        .and_then(|(_, name)| assembled.property(&name))
                        .is_some_and(this_free_function);
                    if !safe {
                        self.risky.insert(address_of(id));
                    }
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                if let Some((id, _)) = self.receiver(tagged.tag.without_parentheses()) {
                    self.risky.insert(address_of(id));
                }
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                if let Some((id, _)) = self.receiver(unary.argument.without_parentheses()) {
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
        if self.depth > 0 || self.live {
            return;
        }
        let Some(assembled) = self.ours(it) else {
            return;
        };
        let address = address_of(it);
        if assembled.exempt.contains(&address) {
            return;
        }
        if self.risky.contains(&address) || !self.reads.contains(&address) {
            self.live = true;
        }
    }
}

pub(super) fn aliases_mutated_only_in_dead<'k, 'p>(
    program: &Program<'_>,
    (scoping, builtins_unpatched): (&Scoping, bool),
    used: impl Iterator<Item = (SymbolId, &'k Assembled<'p>)>,
    dead: &FxHashSet<usize>,
) -> bool
where
    'p: 'k,
{
    let mut objects = FxHashMap::default();
    for (alias, assembled) in used {
        objects.insert(alias, assembled);
        objects.insert(assembled.base, assembled);
    }
    let mut roots: Vec<&FunctionBody<'_>> = Vec::new();
    let mut whole = false;
    for assembled in objects.values() {
        match assembled.root {
            Some(root) if !roots.iter().any(|seen| std::ptr::eq(*seen, root)) => roots.push(root),
            Some(_) => {}
            None => whole = true,
        }
    }
    let mut uses = AliasUses {
        scoping,
        objects,
        dead,
        builtins_unpatched,
        depth: 0,
        reads: FxHashSet::default(),
        risky: FxHashSet::default(),
        live: false,
    };
    if whole {
        uses.visit_program(program);
        return !uses.live;
    }
    for root in &roots {
        uses.visit_function_body(root);
    }
    !uses.live
}
