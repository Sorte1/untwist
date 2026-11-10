use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::build_scoping;

use crate::esbuild::mentions::{binds_any, names_anywhere};
use crate::matcher::argument_expression;
use crate::model::{Bundle, Module};
use crate::scan::is_free;
use crate::webpack::esm_proof::marks_exports;
use crate::webpack::esm_scan::literal_id;

const REQUIRE: &str = "require";
const KNOWN_MEMBERS: [&str; 11] = ["bind", "d", "e", "g", "hmd", "n", "nmd", "o", "p", "r", "t"];

fn address<T>(it: &T) -> usize {
    std::ptr::from_ref(it) as usize
}

fn named_require(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name == REQUIRE)
}

#[derive(Default)]
struct Survey {
    interop: bool,
    opaque: bool,
    depth: u32,
    callee: usize,
    bound_this: usize,
}

fn binds_runtime(callee: &Expression<'_>) -> bool {
    let Expression::StaticMemberExpression(bind) = callee else {
        return false;
    };
    let helper = match &bind.object {
        Expression::StaticMemberExpression(helper) => named_require(&helper.object),
        _ => false,
    };
    bind.property.name == "bind" && (named_require(&bind.object) || helper)
}

impl<'a> VisitJs<'a> for Survey {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &it.callee {
            self.callee = address(&**callee);
        }
        if let (true, Some(Argument::Identifier(this))) =
            (binds_runtime(&it.callee), it.arguments.first())
        {
            self.bound_this = address(&**this);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if !named_require(&it.object) {
            return walk_js::walk_static_member_expression(self, it);
        }
        let name = it.property.name.as_str();
        self.interop |= name == "n";
        self.opaque |= !KNOWN_MEMBERS.contains(&name);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let here = address(it);
        match it.name.as_str() {
            REQUIRE => self.opaque |= here != self.callee && here != self.bound_this,
            "eval" => self.opaque = true,
            "arguments" => self.opaque |= self.depth == 0,
            _ => {}
        }
    }

    fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
        self.opaque = true;
        walk_js::walk_with_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }
}

fn survey(module: &Module<'_>) -> Survey {
    let mut survey = Survey::default();
    survey.visit_program(module.program());
    survey
}

fn proven_harmony(module: &Module<'_>) -> bool {
    if module.is_esm() {
        return true;
    }
    let program = module.program();
    let first = program
        .body
        .iter()
        .find(|statement| !matches!(statement, Statement::FunctionDeclaration(_)));
    first.is_some_and(marks_exports)
        && !binds_any(program, [REQUIRE, "exports"].into_iter())
        && !names_anywhere(program, "module")
}

struct Source<'k> {
    symbol: SymbolId,
    name: &'k str,
    target: String,
    lexical: bool,
}

struct Getter<'k> {
    symbol: SymbolId,
    source: SymbolId,
    name: &'k str,
    target: String,
}

fn lexical(kind: VariableDeclarationKind) -> Option<bool> {
    match kind {
        VariableDeclarationKind::Var => Some(false),
        VariableDeclarationKind::Let | VariableDeclarationKind::Const => Some(true),
        _ => None,
    }
}

fn free_require(scoping: &Scoping, expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name == REQUIRE && is_free(Some(scoping), id))
}

fn source_of<'k>(
    scoping: &Scoping,
    declarator: &'k VariableDeclarator<'_>,
    lexical: bool,
) -> Option<Source<'k>> {
    let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
        return None;
    };
    let Some(Expression::CallExpression(call)) = &declarator.init else {
        return None;
    };
    if call.optional || call.arguments.len() != 1 || !free_require(scoping, &call.callee) {
        return None;
    }
    Some(Source {
        symbol: binding.symbol_id(),
        name: binding.name.as_str(),
        target: literal_id(call.arguments.first())?,
        lexical,
    })
}

fn getter_of(
    scoping: &Scoping,
    declarator: &VariableDeclarator<'_>,
) -> Option<(SymbolId, SymbolId)> {
    let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
        return None;
    };
    let Some(Expression::CallExpression(call)) = &declarator.init else {
        return None;
    };
    let Expression::StaticMemberExpression(helper) = &call.callee else {
        return None;
    };
    let shaped = !call.optional
        && !helper.optional
        && helper.property.name == "n"
        && call.arguments.len() == 1
        && free_require(scoping, &helper.object);
    let Some(Expression::Identifier(argument)) =
        call.arguments.first().and_then(argument_expression)
    else {
        return None;
    };
    let source = scoping
        .get_reference(argument.reference_id.get()?)
        .symbol_id()?;
    shaped.then_some((binding.symbol_id(), source))
}

fn stable(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_redeclarations(symbol).is_empty()
        && scoping
            .get_resolved_references(symbol)
            .all(|reference| !reference.is_write())
}

fn getters<'k>(program: &'k Program<'_>, scoping: &Scoping) -> Vec<Getter<'k>> {
    let mut found = Vec::new();
    let mut previous: Option<Source<'k>> = None;
    for statement in &program.body {
        let kind = match statement {
            Statement::VariableDeclaration(declaration) => {
                lexical(declaration.kind).map(|kind| (kind, &declaration.declarations))
            }
            _ => None,
        };
        let Some((kind, declarators)) = kind else {
            previous = None;
            continue;
        };
        for declarator in declarators {
            let paired = previous.take().and_then(|source| {
                let (symbol, argument) = getter_of(scoping, declarator)?;
                (argument == source.symbol && source.lexical == kind).then_some(Getter {
                    symbol,
                    source: source.symbol,
                    name: source.name,
                    target: source.target,
                })
            });
            found.extend(
                paired.filter(|it| stable(scoping, it.symbol) && stable(scoping, it.source)),
            );
            previous = source_of(scoping, declarator, kind);
        }
    }
    found
}

struct Uses<'s> {
    scoping: &'s Scoping,
    getters: FxHashMap<SymbolId, usize>,
    found: Vec<Vec<ReferenceId>>,
    refused: Vec<bool>,
}

fn getter_reference<'k, 'a>(
    expr: &'k Expression<'a>,
) -> Option<(&'k IdentifierReference<'a>, bool)> {
    let (object, member) = match expr {
        Expression::CallExpression(call) if !call.optional && call.arguments.is_empty() => {
            (&call.callee, false)
        }
        Expression::StaticMemberExpression(member)
            if !member.optional && member.property.name == "a" =>
        {
            (&member.object, true)
        }
        _ => return None,
    };
    match object {
        Expression::Identifier(id) => Some((id, member)),
        _ => None,
    }
}

impl Uses<'_> {
    fn usage(&self, expr: &Expression<'_>) -> Option<(usize, ReferenceId, bool)> {
        let (id, member) = getter_reference(expr)?;
        let reference = id.reference_id.get()?;
        let symbol = self.scoping.get_reference(reference).symbol_id()?;
        Some((*self.getters.get(&symbol)?, reference, member))
    }

    fn keeps_receiver(&mut self, expr: &Expression<'_>) {
        if let Some((index, _, true)) = self.usage(expr) {
            self.refused[index] = true;
        }
    }
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Some((index, reference, _)) = self.usage(it) {
            self.found[index].push(reference);
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.keeps_receiver(&it.callee);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.keeps_receiver(&it.tag);
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            self.keeps_receiver(&it.argument);
        }
        walk_js::walk_unary_expression(self, it);
    }
}

#[derive(Default)]
struct Plan {
    uses: FxHashMap<ReferenceId, String>,
    getters: FxHashSet<SymbolId>,
}

fn reads_source(scoping: &Scoping, getter: &Getter<'_>, uses: &[ReferenceId]) -> bool {
    uses.iter().all(|reference| {
        let scope = scoping.get_reference(*reference).scope_id();
        scoping.find_binding(scope, Ident::from(getter.name)) == Some(getter.source)
    })
}

fn plan(module: &Module<'_>, proven: &mut impl FnMut(&str) -> bool) -> Option<Plan> {
    let program = module.program();
    let scoping = build_scoping(program).scoping;
    let mut candidates = getters(program, &scoping);
    candidates.retain(|getter| proven(&getter.target));
    if candidates.is_empty() {
        return None;
    }
    let mut uses = Uses {
        scoping: &scoping,
        getters: candidates
            .iter()
            .enumerate()
            .map(|(index, getter)| (getter.symbol, index))
            .collect(),
        found: vec![Vec::new(); candidates.len()],
        refused: vec![false; candidates.len()],
    };
    uses.visit_program(program);
    let mut plan = Plan::default();
    for (index, getter) in candidates.iter().enumerate() {
        let found = &uses.found[index];
        let complete = found.len() == scoping.get_resolved_reference_ids(getter.symbol).len();
        if uses.refused[index] || !complete || !reads_source(&scoping, getter, found) {
            continue;
        }
        plan.getters.insert(getter.symbol);
        plan.uses.extend(
            found
                .iter()
                .map(|reference| (*reference, getter.name.to_string())),
        );
    }
    (!plan.getters.is_empty()).then_some(plan)
}

struct Apply<'a, 'p> {
    builder: AstBuilder<'a>,
    plan: &'p Plan,
    changes: usize,
}

impl<'a, 'p> Apply<'a, 'p> {
    fn planned(&self, expr: &Expression<'a>) -> Option<&'p str> {
        let (id, _) = getter_reference(expr)?;
        let name = self.plan.uses.get(&id.reference_id.get()?)?;
        Some(name.as_str())
    }

    fn default_of(&self, name: &str, span: Span) -> Expression<'a> {
        let object = Expression::new_identifier(
            span,
            Ident::from_str_in(name, &self.builder),
            &self.builder,
        );
        let property = IdentifierName::new(
            span,
            Ident::from_str_in("default", &self.builder),
            &self.builder,
        );
        Expression::new_static_member_expression(span, object, property, false, &self.builder)
    }

    fn detached(&mut self, expr: &mut Expression<'a>) {
        let Some(name) = self.planned(expr) else {
            return;
        };
        let span = expr.span();
        let zero =
            Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, &self.builder);
        let list = AVec::from_iter_in([zero, self.default_of(name, span)], &self.builder);
        *expr = Expression::new_sequence_expression(span, list, &self.builder);
        self.changes += 1;
    }

    fn is_getter(&self, declarator: &VariableDeclarator<'a>) -> bool {
        matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
            if id.symbol_id.get().is_some_and(|symbol| self.plan.getters.contains(&symbol)))
    }

    fn drop_getters(&self, program: &mut Program<'a>) {
        for statement in program.body.iter_mut() {
            if let Statement::VariableDeclaration(declaration) = statement {
                declaration
                    .declarations
                    .retain(|declarator| !self.is_getter(declarator));
            }
        }
        program.body.retain(|statement| {
            !matches!(statement, Statement::VariableDeclaration(declaration)
                if declaration.declarations.is_empty())
        });
    }
}

impl<'a> VisitMut<'a> for Apply<'a, '_> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_mut::walk_program(self, it);
        self.drop_getters(it);
    }

    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        self.detached(&mut it.callee);
        walk_mut::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &mut TaggedTemplateExpression<'a>) {
        self.detached(&mut it.tag);
        walk_mut::walk_tagged_template_expression(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(name) = self.planned(it) {
            *it = self.default_of(name, it.span());
            self.changes += 1;
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

fn plans(bundle: &Bundle<'_>) -> Vec<Option<Plan>> {
    let surveys: Vec<Survey> = bundle.modules.values().map(survey).collect();
    if surveys.iter().any(|survey| survey.opaque) {
        return Vec::new();
    }
    let mut cache: FxHashMap<String, bool> = FxHashMap::default();
    let mut proven = |id: &str| {
        if let Some(known) = cache.get(id) {
            return *known;
        }
        let known = bundle.modules.get(id).is_some_and(proven_harmony);
        cache.insert(id.to_string(), known);
        known
    };
    bundle
        .modules
        .values()
        .zip(&surveys)
        .map(|(module, survey)| survey.interop.then(|| plan(module, &mut proven)).flatten())
        .collect()
}

pub fn convert_interop<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> usize {
    let plans = plans(bundle);
    let mut changes = 0;
    for (module, plan) in bundle.modules.values_mut().zip(&plans) {
        let Some(plan) = plan else {
            continue;
        };
        let mut apply = Apply {
            builder: AstBuilder::new(alloc),
            plan,
            changes: 0,
        };
        apply.visit_program(module.program_mut());
        changes += apply.changes;
    }
    changes
}
