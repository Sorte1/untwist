use std::cell::Cell;

use oxc_allocator::{Allocator, TakeIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::naming_slot::{named_as, text_observes_names};
use untwist_core::semantic::{build_scoping, eval_payload_names};

use crate::closed::HOST_NAMES;

const MAX_UID_ATTEMPTS: u32 = 1000;

pub fn generate_uid(base: &str, scoping: &Scoping, used: &FxHashSet<String>) -> String {
    for index in 1..=MAX_UID_ATTEMPTS {
        let candidate = if index == 1 {
            format!("_{base}")
        } else {
            format!("_{base}{index}")
        };
        let taken = used.contains(&candidate)
            || scoping
                .symbol_ids()
                .any(|symbol| scoping.symbol_name(symbol) == candidate);
        if !taken {
            return candidate;
        }
    }
    format!("_{base}_{}", used.len())
}

pub fn reads_free_name(program: &Program<'_>, scoping: &Scoping, name: &str) -> bool {
    scoping.root_unresolved_references().contains_key(name)
        || evaluated_names(program, scoping).contains(name)
}

fn evaluated_names(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<String> {
    if !scoping.root_unresolved_references().contains_key("eval") {
        return FxHashSet::default();
    }
    eval_payload_names(program).into_iter().collect()
}

thread_local! {
    static NAME_HAZARD: Cell<bool> = const { Cell::new(false) };
}

pub fn clear_name_hazard() {
    NAME_HAZARD.with(|cell| cell.set(false));
}

pub fn took_name_hazard() -> bool {
    NAME_HAZARD.with(|cell| cell.replace(false))
}

struct Apply<'a, 'r> {
    builder: AstBuilder<'a>,
    free: &'r FxHashMap<&'r str, Ident<'a>>,
    symbols: &'r FxHashMap<SymbolId, Ident<'a>>,
    scoping: &'r Scoping,
    text: &'a str,
    names_read: Option<bool>,
    changes: usize,
}

fn naming_operator(operator: AssignmentOperator) -> bool {
    matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalNullish
    )
}

impl<'a> Apply<'a, '_> {
    fn names_read(&mut self) -> bool {
        let text = self.text;
        *self
            .names_read
            .get_or_insert_with(|| text_observes_names(text))
    }

    fn renamed_binding(&self, id: &BindingIdentifier<'a>) -> Option<&'a str> {
        let symbol = id.symbol_id.get()?;
        self.symbols
            .contains_key(&symbol)
            .then_some(id.name.as_str())
    }

    fn renamed_reference(&self, id: &IdentifierReference<'a>) -> Option<&'a str> {
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        let renamed = match symbol {
            Some(symbol) => self.symbols.contains_key(&symbol),
            None => self.free.contains_key(id.name.as_str()),
        };
        renamed.then_some(id.name.as_str())
    }

    fn keep_name(&mut self, name: Option<&'a str>, value: &mut Expression<'a>) {
        let Some(name) = name else {
            return;
        };
        if value.is_anonymous_function_definition() && self.names_read() {
            let taken = value.take_in(&self.builder);
            *value = named_as(name, taken, &self.builder);
        }
    }

    fn keep_declared_name(&mut self, id: Option<&BindingIdentifier<'a>>) {
        let renamed = id.and_then(|id| self.renamed_binding(id)).is_some();
        if renamed && self.names_read() {
            NAME_HAZARD.with(|cell| cell.set(true));
        }
    }
}

impl<'a> VisitMut<'a> for Apply<'a, '_> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        match symbol {
            Some(symbol) => {
                if let Some(name) = self.symbols.get(&symbol) {
                    it.name = *name;
                    self.changes += 1;
                }
            }
            None => {
                if let Some(name) = self.free.get(it.name.as_str()) {
                    it.name = *name;
                    self.changes += 1;
                }
            }
        }
        walk_mut::walk_identifier_reference(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(name) = it.symbol_id.get().and_then(|id| self.symbols.get(&id)) {
            it.name = *name;
            self.changes += 1;
        }
        walk_mut::walk_binding_identifier(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &mut it.init) {
            let name = self.renamed_binding(id);
            self.keep_name(name, init);
        }
        walk_mut::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &mut AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if naming_operator(it.operator) {
                let name = self.renamed_reference(id);
                self.keep_name(name, &mut it.right);
            }
        }
        walk_mut::walk_assignment_expression(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &mut AssignmentPattern<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.left {
            let name = self.renamed_binding(id);
            self.keep_name(name, &mut it.right);
        }
        walk_mut::walk_assignment_pattern(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &mut FormalParameter<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) =
            (&it.pattern, &mut it.initializer)
        {
            let name = self.renamed_binding(id);
            self.keep_name(name, init);
        }
        walk_mut::walk_formal_parameter(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &mut AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.binding {
            let name = self.renamed_reference(id);
            self.keep_name(name, &mut it.init);
        }
        walk_mut::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property(&mut self, it: &mut AssignmentTargetProperty<'a>) {
        if let AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(shorthand) = it {
            if let Some(name) = self.renamed_reference(&shorthand.binding) {
                *it = self.spelled_out(shorthand, name);
            }
        }
        walk_mut::walk_assignment_target_property(self, it);
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if let Some(statement) = self.class_as_let(it) {
            *it = statement;
        }
        walk_mut::walk_statement(self, it);
    }

    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        self.keep_declared_name(it.id.as_ref());
        walk_mut::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &mut Class<'a>) {
        self.keep_declared_name(it.id.as_ref());
        walk_mut::walk_class(self, it);
    }
}

struct Mentions<'r, 'a, 'b> {
    apply: &'r Apply<'a, 'b>,
    name: &'r str,
    found: bool,
}

impl<'a> VisitJs<'a> for Mentions<'_, 'a, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == self.name || self.apply.renamed_reference(it).is_some();
    }
}

impl<'a> Apply<'a, '_> {
    fn class_as_let(&mut self, it: &mut Statement<'a>) -> Option<Statement<'a>> {
        let Statement::ClassDeclaration(class) = it else {
            return None;
        };
        let id = class.id.as_ref()?;
        let (name, symbol) = (self.renamed_binding(id)?, id.symbol_id.get()?);
        if !self.names_read() {
            return None;
        }
        let mut mentions = Mentions {
            apply: self,
            name,
            found: false,
        };
        mentions.visit_class(class);
        if mentions.found {
            return None;
        }
        let Statement::ClassDeclaration(mut class) = it.take_in(&self.builder) else {
            return None;
        };
        let span = class.span;
        let id_span = class.id.as_ref().map_or(span, |id| id.span);
        class.r#type = ClassType::ClassExpression;
        class.id = Some(BindingIdentifier::new(id_span, name, &self.builder));
        let declarator = VariableDeclarator::new(
            span,
            BindingPattern::new_binding_identifier_with_symbol_id(
                id_span,
                name,
                symbol,
                &self.builder,
            ),
            None,
            Some(Expression::ClassExpression(class)),
            false,
            &self.builder,
        );
        Some(Statement::new_variable_declaration(
            span,
            VariableDeclarationKind::Let,
            oxc_allocator::Vec::from_iter_in([declarator], &self.builder),
            false,
            &self.builder,
        ))
    }

    fn spelled_out(
        &mut self,
        shorthand: &mut AssignmentTargetPropertyIdentifier<'a>,
        name: &'a str,
    ) -> AssignmentTargetProperty<'a> {
        let span = shorthand.span;
        let binding = &shorthand.binding;
        let target = match binding.reference_id.get() {
            Some(reference) => {
                AssignmentTarget::new_assignment_target_identifier_with_reference_id(
                    binding.span,
                    binding.name,
                    reference,
                    &self.builder,
                )
            }
            None => AssignmentTarget::new_assignment_target_identifier(
                binding.span,
                binding.name,
                &self.builder,
            ),
        };
        let target = match shorthand.init.take() {
            Some(mut init) => {
                self.keep_name(Some(name), &mut init);
                AssignmentTargetMaybeDefault::new_assignment_target_with_default(
                    span,
                    target,
                    init,
                    &self.builder,
                )
            }
            None => AssignmentTargetMaybeDefault::from(target),
        };
        AssignmentTargetProperty::new_assignment_target_property_property(
            span,
            PropertyKey::new_static_identifier(span, name, &self.builder),
            target,
            false,
            &self.builder,
        )
    }
}

fn apply_renames<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    scoping: &Scoping,
    (free, symbols): (&FxHashMap<&str, Ident<'a>>, &FxHashMap<SymbolId, Ident<'a>>),
) -> usize {
    let mut apply = Apply {
        builder: AstBuilder::new(alloc),
        free,
        symbols,
        scoping,
        text: program.source_text,
        names_read: None,
        changes: 0,
    };
    apply.visit_program(program);
    apply.changes
}

fn collect_collisions<'a>(
    alloc: &'a Allocator,
    scoping: &Scoping,
    original: &str,
    renamed: &str,
    symbols: &mut FxHashMap<SymbolId, Ident<'a>>,
    used: &mut FxHashSet<String>,
) {
    if original == renamed {
        return;
    }
    let Some(reference_ids) = scoping.root_unresolved_references().get(original) else {
        return;
    };
    let mut ordered: Vec<_> = reference_ids.iter().copied().collect();
    ordered.sort_unstable();
    for reference_id in ordered {
        let scope_id = scoping.get_reference(reference_id).scope_id();
        let Some(symbol) = scoping.find_binding(scope_id, Ident::from(renamed)) else {
            continue;
        };
        if symbols.contains_key(&symbol) {
            continue;
        }
        let uid = generate_uid(renamed, scoping, used);
        used.insert(uid.clone());
        symbols.insert(symbol, Ident::from_str_in(&uid, &alloc));
    }
}

fn uncaptured_target(
    scoping: &Scoping,
    (from, to): (&str, &str),
    (params, evaluated): (&[&str], &FxHashSet<String>),
    used: &mut FxHashSet<String>,
) -> Option<String> {
    let free = scoping.root_unresolved_references().contains_key(to) || evaluated.contains(to);
    if from == to || !free {
        return None;
    }
    if params.contains(&to) {
        return None;
    }
    let uid = generate_uid(to, scoping, used);
    used.insert(uid.clone());
    Some(uid)
}

pub fn rename_parameters<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    original: &[&str],
    renamed: &[&str],
) -> usize {
    rename_parameters_as(alloc, program, (original, renamed), false).0
}

pub fn rename_factory_parameters<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (original, renamed): (&[&str], &[&str]),
    conservative: bool,
) -> Vec<(usize, String)> {
    rename_parameters_as(alloc, program, (original, renamed), !conservative).1
}

pub fn rename_parameters_uncaptured<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    original: &[&str],
    renamed: &[&str],
) -> usize {
    rename_parameters_as(alloc, program, (original, renamed), true).0
}

fn rename_parameters_as<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    (original, renamed): (&[&str], &[&str]),
    uncaptured: bool,
) -> (usize, Vec<(usize, String)>) {
    let scoping = build_scoping(program).scoping;
    let mut symbols: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    let evaluated = evaluated_names(program, &scoping);
    let mut used: FxHashSet<String> = evaluated.clone();
    let mut free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();

    let pairs = original.len().min(renamed.len());
    let mut fresh = Vec::new();
    for index in 0..pairs {
        let from = original[index];
        let uid = uncaptured
            .then(|| {
                let names = (original, &evaluated);
                uncaptured_target(&scoping, (from, renamed[index]), names, &mut used)
            })
            .flatten();
        let to = uid.as_deref().unwrap_or(renamed[index]);
        collect_collisions(alloc, &scoping, from, to, &mut symbols, &mut used);
        if from != to {
            free.insert(from, Ident::from_str_in(to, &alloc));
        }
        if let Some(uid) = uid {
            fresh.push((index, uid));
        }
    }
    if free.is_empty() && symbols.is_empty() {
        return (0, fresh);
    }

    let changes = apply_renames(alloc, program, &scoping, (&free, &symbols));
    (changes, fresh)
}

pub fn rename_binding<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    from: &str,
    to: &str,
) -> bool {
    if from == to {
        return false;
    }
    let scoping = build_scoping(program).scoping;
    let Some(target) = scoping.get_root_binding(Ident::from(from)) else {
        return false;
    };
    let mut symbols: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    let mut used: FxHashSet<String> = FxHashSet::default();
    if let Some(conflict) = scoping.get_root_binding(Ident::from(to)) {
        if conflict != target {
            let uid = generate_uid(to, &scoping, &used);
            used.insert(uid.clone());
            symbols.insert(conflict, Ident::from_str_in(&uid, &alloc));
        }
    }
    symbols.insert(target, Ident::from_str_in(to, &alloc));
    let free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();
    apply_renames(alloc, program, &scoping, (&free, &symbols)) > 0
}

struct WrapperNames<'n> {
    names: &'n [&'n str],
    found: bool,
}

impl<'a> VisitJs<'a> for WrapperNames<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found |= self.names.contains(&it.name.as_str());
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.visit_binding_identifier(id);
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(id) = &it.id {
            self.visit_binding_identifier(id);
        }
    }
}

fn fresh_module_name(base: &str, scoping: &Scoping, used: &FxHashSet<String>) -> String {
    let unresolved = scoping.root_unresolved_references();
    (1..=MAX_UID_ATTEMPTS)
        .map(|index| match index {
            1 => format!("_{base}"),
            _ => format!("_{base}{index}"),
        })
        .find(|candidate| {
            !used.contains(candidate)
                && !unresolved.contains_key(candidate.as_str())
                && scoping
                    .get_root_binding(Ident::from(candidate.as_str()))
                    .is_none()
                && !scoping
                    .symbol_ids()
                    .any(|symbol| scoping.symbol_name(symbol) == candidate)
        })
        .unwrap_or_else(|| format!("_{base}_{}", used.len()))
}

pub fn rename_wrapper_bindings<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    params: &[&str],
) -> usize {
    if program.source_type.is_module() {
        return 0;
    }
    let names: Vec<&str> = HOST_NAMES
        .iter()
        .copied()
        .filter(|name| !params.contains(name))
        .collect();
    let mut probe = WrapperNames {
        names: &names,
        found: false,
    };
    probe.visit_program(program);
    if !probe.found {
        return 0;
    }
    let scoping = build_scoping(program).scoping;
    let mut symbols: FxHashMap<SymbolId, Ident<'a>> = FxHashMap::default();
    let mut used: FxHashSet<String> = FxHashSet::default();
    for name in names {
        let Some(symbol) = scoping.get_root_binding(Ident::from(name)) else {
            continue;
        };
        let uid = fresh_module_name(name, &scoping, &used);
        symbols.insert(symbol, Ident::from_str_in(&uid, &alloc));
        used.insert(uid);
    }
    if symbols.is_empty() {
        return 0;
    }
    let free: FxHashMap<&str, Ident<'a>> = FxHashMap::default();
    apply_renames(alloc, program, &scoping, (&free, &symbols))
}
