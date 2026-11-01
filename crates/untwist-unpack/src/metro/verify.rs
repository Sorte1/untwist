use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::build_scoping;

use super::layout::{Def, Layout, FACTORY_PARAMS};
use super::{CANONICAL, DEPENDENCY_MAP_NAME, RUNTIME_NAMES};
use crate::closed::{Reach, Region};
use crate::runtime::first_unclosed_helper;
use crate::scan::{is_direct_eval, Frame, Policy, Scan, Step};

const HOST_MODULE_NAMES: [&str; 5] = ["module", "exports", "require", "__filename", "__dirname"];
const LOADER_GLOBALS: [&str; 3] = ["__r", "__c", "__registerSegment"];
const REQUIRE: usize = 1;
const MODULE: usize = 4;
const DEPENDENCY_MAP: usize = 6;
const REQUIRE_MEMBERS: [&str; 2] = ["importDefault", "importAll"];

fn declarator_symbols(statements: &[Statement<'_>]) -> FxHashSet<SymbolId> {
    let mut symbols = FxHashSet::default();
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                symbols.extend(id.symbol_id.get());
            }
        }
    }
    symbols
}

fn chain_root<'k, 'a>(mut expression: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
    loop {
        expression = match expression.without_parentheses() {
            Expression::Identifier(id) => return Some(id),
            Expression::StaticMemberExpression(member) => &member.object,
            Expression::ComputedMemberExpression(member) => &member.object,
            Expression::PrivateFieldExpression(member) => &member.object,
            _ => return None,
        };
    }
}

fn target_root<'k, 'a>(
    target: &'k SimpleAssignmentTarget<'a>,
) -> Option<&'k IdentifierReference<'a>> {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => Some(id),
        other => chain_root(other.as_member_expression()?.object()),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Member {
    Exports,
    ImportHelper,
    Other,
}

impl Member {
    fn of(name: &str) -> Self {
        match name {
            "exports" => Self::Exports,
            name if REQUIRE_MEMBERS.contains(&name) => Self::ImportHelper,
            _ => Self::Other,
        }
    }
}

#[derive(Default)]
struct Contexts {
    watched: Vec<String>,
    calls: FxHashSet<Span>,
    method_objects: FxHashSet<Span>,
    members: FxHashMap<Span, Member>,
    indices: FxHashMap<Span, f64>,
    typeofs: FxHashSet<Span>,
    pure: FxHashSet<Span>,
    targets: FxHashSet<Span>,
}

impl Contexts {
    fn watching(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            watched: names.into_iter().collect(),
            ..Self::default()
        }
    }

    fn watched_id(&self, id: &IdentifierReference<'_>) -> Option<Span> {
        self.watched
            .iter()
            .any(|name| name == id.name.as_str())
            .then_some(id.span)
    }

    fn watched_span(&self, expression: &Expression<'_>) -> Option<Span> {
        match expression.without_parentheses() {
            Expression::Identifier(id) => self.watched_id(id),
            _ => None,
        }
    }

    fn call(&mut self, it: &CallExpression<'_>) {
        match it.callee.without_parentheses() {
            Expression::Identifier(id) => {
                self.calls.extend(self.watched_id(id));
            }
            Expression::StaticMemberExpression(member) => {
                self.method_objects
                    .extend(self.watched_span(&member.object));
            }
            Expression::ComputedMemberExpression(member) => {
                self.method_objects
                    .extend(self.watched_span(&member.object));
            }
            _ => {}
        }
    }

    fn static_member(&mut self, it: &StaticMemberExpression<'_>) {
        if let Some(span) = self.watched_span(&it.object) {
            if !it.optional {
                self.members
                    .insert(span, Member::of(it.property.name.as_str()));
            }
            self.pure.insert(span);
        }
    }

    fn computed_member(&mut self, it: &ComputedMemberExpression<'_>) {
        let Some(span) = self.watched_span(&it.object) else {
            return;
        };
        self.pure.insert(span);
        if let Expression::NumericLiteral(index) = it.expression.without_parentheses() {
            if !it.optional && index.value.fract() == 0.0 && index.value >= 0.0 {
                self.indices.insert(span, index.value);
            }
        }
    }

    fn unary(&mut self, it: &UnaryExpression<'_>) {
        if let Some(span) = self.watched_span(&it.argument) {
            if it.operator == UnaryOperator::Typeof {
                self.typeofs.insert(span);
            }
            if it.operator != UnaryOperator::Delete {
                self.pure.insert(span);
            }
        }
        if it.operator == UnaryOperator::Delete {
            let root = chain_root(&it.argument).and_then(|id| self.watched_id(id));
            self.targets.extend(root);
        }
    }

    fn target(&mut self, it: &SimpleAssignmentTarget<'_>) {
        let root = target_root(it).and_then(|id| self.watched_id(id));
        self.targets.extend(root);
    }

    fn pure_operand(&mut self, id: &IdentifierReference<'_>) {
        let span = self.watched_id(id);
        self.pure.extend(span);
    }
}

struct PreludeUse<'s> {
    vars: &'s FxHashSet<SymbolId>,
    inner_refs: Vec<(SymbolId, Span)>,
    host_module: bool,
}

impl PreludeUse<'_> {
    fn reference(
        &mut self,
        symbol: Option<SymbolId>,
        deferred: bool,
        it: &IdentifierReference<'_>,
    ) {
        match symbol {
            Some(symbol) if deferred && self.vars.contains(&symbol) => {
                self.inner_refs.push((symbol, it.span));
            }
            Some(_) => {}
            None => self.host_module |= HOST_MODULE_NAMES.contains(&it.name.as_str()),
        }
    }
}

struct FactoryUse<'s, 'p, 'k, 'a> {
    names: &'p [&'p str],
    scope: Option<ScopeId>,
    region: Region<'k, 'a>,
    prefix_define: &'p str,
    prelude: &'s FxHashSet<SymbolId>,
    param_refs: Vec<(usize, Span)>,
    referenced: FxHashSet<SymbolId>,
    refused: bool,
}

impl FactoryUse<'_, '_, '_, '_> {
    fn foreign_name(&self, name: &str) -> bool {
        let canonical = CANONICAL
            .iter()
            .zip(self.names)
            .any(|(canonical, param)| *canonical == name && *param != name);
        let loader = LOADER_GLOBALS.contains(&name) || name == self.prefix_define;
        canonical || loader || matches!(name, "__filename" | "__dirname")
    }

    fn reference(&mut self, scoping: &Scoping, frame: Frame, it: &IdentifierReference<'_>) {
        match self.region.reach((scoping, self.prelude), frame, it) {
            Reach::Local => {}
            Reach::Param(index) => self.param_refs.push((index, it.span)),
            Reach::Helper(symbol) => {
                self.referenced.insert(symbol);
                self.refused |= self.foreign_name(&it.name);
            }
            Reach::Free => self.refused |= self.foreign_name(&it.name),
            Reach::Binding(_) | Reach::Unmapped(_) | Reach::Arguments => self.refused = true,
        }
    }

    fn binding(&mut self, scoping: &Scoping, it: &BindingIdentifier<'_>) {
        let name = it.name.as_str();
        let root = it
            .symbol_id
            .get()
            .is_some_and(|symbol| Some(scoping.symbol_scope_id(symbol)) == self.scope);
        self.refused |= matches!(name, "require" | "globalThis" | DEPENDENCY_MAP_NAME)
            || (root && self.names.contains(&name));
    }
}

enum Role<'s, 'p, 'k, 'a> {
    Prelude(PreludeUse<'s>),
    Factory(FactoryUse<'s, 'p, 'k, 'a>),
}

struct Rules<'s, 'p, 'k, 'a> {
    scoping: &'s Scoping,
    contexts: Contexts,
    role: Role<'s, 'p, 'k, 'a>,
}

impl Rules<'_, '_, '_, '_> {
    fn refuse_factory(&mut self, refused: bool) {
        if let Role::Factory(factory) = &mut self.role {
            factory.refused |= refused;
        }
    }
}

impl<'a> Policy<'a> for Rules<'_, '_, '_, 'a> {
    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        match &mut self.role {
            Role::Prelude(prelude) => {
                let symbol = it
                    .reference_id
                    .get()
                    .and_then(|id| self.scoping.get_reference(id).symbol_id());
                prelude.reference(symbol, frame.deferred(), it);
            }
            Role::Factory(factory) => factory.reference(self.scoping, frame, it),
        }
    }

    fn binding(&mut self, it: &BindingIdentifier<'a>) {
        if let Role::Factory(factory) = &mut self.role {
            factory.binding(self.scoping, it);
        }
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        self.contexts.call(it);
        let eval = is_direct_eval(self.scoping, &it.callee);
        self.refuse_factory(eval);
        Step::Walk
    }

    fn static_member(&mut self, it: &StaticMemberExpression<'a>) -> Step {
        self.contexts.static_member(it);
        Step::Walk
    }

    fn computed_member(&mut self, it: &ComputedMemberExpression<'a>) {
        self.contexts.computed_member(it);
    }

    fn unary(&mut self, it: &UnaryExpression<'a>) -> Step {
        self.contexts.unary(it);
        Step::Walk
    }

    fn target(&mut self, it: &SimpleAssignmentTarget<'a>) -> Step {
        self.contexts.target(it);
        Step::Walk
    }

    fn operand(&mut self, it: &IdentifierReference<'a>) {
        self.contexts.pure_operand(it);
    }

    fn this(&mut self, frame: Frame) {
        self.refuse_factory(!frame.receiver());
    }

    fn with(&mut self) {
        self.refuse_factory(true);
    }
}

struct PreludeFacts {
    vars: FxHashSet<SymbolId>,
    impure: FxHashSet<SymbolId>,
}

fn prelude_facts(scoping: &Scoping, statements: &[Statement<'_>]) -> Option<PreludeFacts> {
    let vars = declarator_symbols(statements);
    let mut rules = Rules {
        scoping,
        contexts: Contexts::watching(vars.iter().map(|s| scoping.symbol_name(*s).to_string())),
        role: Role::Prelude(PreludeUse {
            vars: &vars,
            inner_refs: Vec::new(),
            host_module: false,
        }),
    };
    let mut scan = Scan::new(&mut rules);
    for statement in statements {
        scan.visit_statement(statement);
    }
    let Role::Prelude(prelude) = rules.role else {
        return None;
    };
    if prelude.host_module {
        return None;
    }
    let contexts = &rules.contexts;
    let impure = prelude
        .inner_refs
        .iter()
        .filter(|(_, span)| !contexts.pure.contains(span) || contexts.targets.contains(span))
        .map(|(symbol, _)| *symbol)
        .collect();
    Some(PreludeFacts { vars, impure })
}

fn param_use_allowed(
    contexts: &Contexts,
    (index, span): (usize, Span),
    deps: Option<usize>,
) -> bool {
    let member = contexts.members.get(&span).copied();
    let method = contexts.method_objects.contains(&span);
    let target = contexts.targets.contains(&span);
    match index {
        REQUIRE => {
            !target && (contexts.calls.contains(&span) || member == Some(Member::ImportHelper))
        }
        MODULE => contexts.typeofs.contains(&span) || (!method && member == Some(Member::Exports)),
        DEPENDENCY_MAP => {
            let Some(len) = deps else {
                return false;
            };
            let indexed = contexts.indices.get(&span).is_some_and(|k| *k < len as f64);
            !target && !method && (indexed || member.is_some())
        }
        _ => true,
    }
}

fn factory_allowed(
    scoping: &Scoping,
    def: &Def<'_, '_>,
    (prefix_define, prelude): (&str, &FxHashSet<SymbolId>),
    referenced: &mut FxHashSet<SymbolId>,
) -> bool {
    let Some(factory) = def.factory() else {
        return false;
    };
    let names = def.params();
    let mut rules = Rules {
        scoping,
        contexts: Contexts::watching(
            [REQUIRE, MODULE, DEPENDENCY_MAP].map(|index| names[index].to_string()),
        ),
        role: Role::Factory(FactoryUse {
            names: &names,
            scope: def.function.scope_id.get(),
            region: Region::factory(factory, FACTORY_PARAMS),
            prefix_define,
            prelude,
            param_refs: Vec::new(),
            referenced: FxHashSet::default(),
            refused: false,
        }),
    };
    Scan::new(&mut rules).visit_function_body(factory.body);
    let Role::Factory(visit) = rules.role else {
        return false;
    };
    let deps = def.deps.map(|deps| deps.elements.len());
    let refs_allowed = visit
        .param_refs
        .iter()
        .all(|reference| param_use_allowed(&rules.contexts, *reference, deps));
    referenced.extend(visit.referenced);
    !visit.refused && refs_allowed
}

fn importable(scoping: &Scoping, facts: &PreludeFacts, symbol: SymbolId) -> bool {
    facts.vars.contains(&symbol)
        && !facts.impure.contains(&symbol)
        && scoping.symbol_redeclarations(symbol).is_empty()
        && scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|id| !scoping.get_reference(*id).is_write())
}

fn names_free(scoping: &Scoping) -> bool {
    let synthesized = RUNTIME_NAMES.iter().all(|name| {
        scoping.get_root_binding((*name).into()).is_none()
            && !scoping.root_unresolved_references().contains_key(*name)
    });
    let host = HOST_MODULE_NAMES
        .iter()
        .chain(&["globalThis"])
        .all(|name| scoping.get_root_binding((*name).into()).is_none());
    synthesized && host
}

pub fn verify(source: &Program<'_>, layout: &Layout<'_, '_>) -> bool {
    let prefix_define = layout.define;
    if source
        .directives
        .iter()
        .any(|d| d.directive.as_str() == "use strict")
    {
        return false;
    }
    let scoping = build_scoping(source).scoping;
    let prelude = &source.body[..layout.prelude];
    let indices: Vec<usize> = (0..layout.prelude).collect();
    if !names_free(&scoping) || first_unclosed_helper(&scoping, &source.body, &indices).is_some() {
        return false;
    }
    let Some(facts) = prelude_facts(&scoping, prelude) else {
        return false;
    };
    let mut referenced = FxHashSet::default();
    let context = (prefix_define, &facts.vars);
    let factories_allowed = layout
        .defs
        .iter()
        .all(|def| factory_allowed(&scoping, def, context, &mut referenced));
    factories_allowed && referenced.iter().all(|s| importable(&scoping, &facts, *s))
}
