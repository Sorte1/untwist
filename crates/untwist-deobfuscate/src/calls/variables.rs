use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::ast_util::search::{expressions_at, statements_at};
use untwist_core::semantic::is_constant_binding;

use super::sequenced::block_function_rebinds;
use crate::call_checks::evaluates_every_operand;
use crate::init_order::{capacity_hint, with_slack};

#[derive(Clone, Copy, Hash)]
pub(super) struct Reference {
    pub(super) span: Span,
}

#[derive(Clone, Copy, Hash)]
pub(super) struct Assignment {
    pub(super) statement: Span,
    pub(super) expression: Span,
    pub(super) value: Span,
}

#[derive(Clone, Default)]
struct Found {
    declaration: Option<Span>,
    init: Option<Span>,
    initialised_in: Vec<Span>,
    rebound: bool,
    conditional: bool,
    assignments: Vec<Assignment>,
    references: Vec<Reference>,
}

pub(super) struct VariableFacts<'v> {
    pub(super) statement: Option<Span>,
    pub(super) declaration: Option<Span>,
    pub(super) init: Option<Span>,
    pub(super) initialised_in: &'v [Span],
    pub(super) rebound: bool,
    pub(super) conditional: bool,
    pub(super) assignments: &'v [Assignment],
    pub(super) references: &'v [Reference],
    pub(super) members: &'v FxHashMap<Span, usize>,
}

pub(super) struct Variables {
    members: FxHashMap<Span, usize>,
    found: Vec<Found>,
    sole: Vec<bool>,
    positions: Vec<Option<usize>>,
}

impl Variables {
    fn slot_facts(&self, slot: usize) -> Option<VariableFacts<'_>> {
        let found = self.found.get(slot)?;
        let sole = self.sole.get(slot).copied().unwrap_or(false);
        Some(VariableFacts {
            statement: found.declaration,
            declaration: found.declaration.filter(|_| sole),
            init: found.init,
            initialised_in: &found.initialised_in,
            rebound: found.rebound,
            conditional: found.conditional,
            assignments: &found.assignments,
            references: &found.references,
            members: &self.members,
        })
    }

    pub(super) fn facts(&self, index: usize) -> Option<VariableFacts<'_>> {
        self.slot_facts(self.positions.get(index).copied().flatten()?)
    }
}

struct Targets {
    slots: FxHashMap<SymbolId, usize>,
    unique: Vec<SymbolId>,
    positions: Vec<Option<usize>>,
}

impl Targets {
    fn of(symbols: &[SymbolId], keep: impl Fn(SymbolId) -> bool) -> Self {
        let mut slots: FxHashMap<SymbolId, usize> = FxHashMap::default();
        let mut unique = Vec::with_capacity(symbols.len());
        let positions = symbols
            .iter()
            .map(|symbol| {
                if !keep(*symbol) {
                    return None;
                }
                let next = unique.len();
                let slot = *slots.entry(*symbol).or_insert(next);
                if slot == next {
                    unique.push(*symbol);
                }
                Some(slot)
            })
            .collect();
        Self {
            slots,
            unique,
            positions,
        }
    }
}

struct VariableScan<'s> {
    uses: bool,
    scoping: &'s Scoping,
    targets: &'s FxHashMap<SymbolId, usize>,
    statement: Span,
    lists: usize,
    written: FxHashSet<Span>,
    members: FxHashMap<Span, usize>,
    found: Vec<Found>,
    loop_head: bool,
    names: Vec<(&'s str, usize)>,
    symbols: Vec<SymbolId>,
    plain: FxHashSet<Span>,
    top_list: bool,
    top_functions: FxHashSet<Span>,
    conditional: bool,
}

fn runs_its_expressions_once(statement: &Statement<'_>) -> bool {
    matches!(
        statement,
        Statement::ExpressionStatement(_)
            | Statement::VariableDeclaration(_)
            | Statement::ReturnStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::IfStatement(_)
    )
}

impl VariableScan<'_> {
    fn target(&self, reference: Option<ReferenceId>) -> Option<usize> {
        let symbol = self.scoping.get_reference(reference?).symbol_id()?;
        self.targets.get(&symbol).copied()
    }

    fn found_at(&mut self, slot: Option<usize>) -> Option<&mut Found> {
        self.found.get_mut(slot?)
    }
}

impl<'a> VisitJs<'a> for VariableScan<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.top_list = true;
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.top_list = true;
        walk_js::walk_function_body(self, it);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        if self.uses {
            self.plain
                .extend(it.items.iter().filter_map(|param| match &param.pattern {
                    BindingPattern::BindingIdentifier(id) => Some(id.span),
                    _ => None,
                }));
        }
        walk_js::walk_formal_parameters(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.uses && it.is_declaration() && !self.top_functions.contains(&it.span) {
            let name = it.id.as_ref().map(|id| id.name.as_str());
            let scoping = self.scoping;
            let slots: Vec<usize> = self
                .names
                .iter()
                .filter(|(target, _)| Some(*target) == name)
                .filter(|(_, slot)| {
                    self.symbols
                        .get(*slot)
                        .is_some_and(|symbol| block_function_rebinds(scoping, it, *symbol))
                })
                .map(|(_, slot)| *slot)
                .collect();
            for slot in slots {
                if let Some(found) = self.found.get_mut(slot) {
                    found.rebound = true;
                }
            }
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if !self.uses || self.plain.contains(&it.span) {
            return;
        }
        let slot = it
            .symbol_id
            .get()
            .and_then(|symbol| self.targets.get(&symbol).copied());
        if let Some(found) = self.found_at(slot) {
            found.rebound = true;
        }
    }

    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        if std::mem::take(&mut self.top_list) && self.uses {
            self.top_functions
                .extend(it.iter().filter_map(|statement| match statement {
                    Statement::FunctionDeclaration(function) => Some(function.span),
                    _ => None,
                }));
        }
        if self.uses {
            self.lists += 1;
            let list = self.lists;
            for statement in it {
                self.members.insert(statement.span(), list);
            }
        }
        walk_js::walk_statements(self, it);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        let outer = (self.statement, self.conditional);
        self.statement = it.span();
        self.conditional = !runs_its_expressions_once(it);
        walk_js::walk_statement(self, it);
        (self.statement, self.conditional) = outer;
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let outer = self.conditional;
        self.conditional = outer || !evaluates_every_operand(it);
        walk_js::walk_expression(self, it);
        self.conditional = outer;
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        let outer = std::mem::replace(&mut self.conditional, true);
        walk_js::walk_assignment_pattern(self, it);
        self.conditional = outer;
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        let outer = std::mem::replace(&mut self.conditional, true);
        walk_js::walk_assignment_target_with_default(self, it);
        self.conditional = outer;
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let slot = binding_symbol(&it.id).and_then(|symbol| self.targets.get(&symbol).copied());
        let statement = self.statement;
        let loop_head = self.loop_head;
        if let (BindingPattern::BindingIdentifier(id), false) = (&it.id, loop_head) {
            if self.uses {
                self.plain.insert(id.span);
            }
        }
        if let Some(found) = self.found_at(slot) {
            found.declaration = Some(statement);
            found.init = it.init.as_ref().map(GetSpan::span);
            if it.init.is_some() || loop_head {
                found.initialised_in.push(statement);
            }
        }
        self.loop_head = false;
        walk_js::walk_variable_declarator(self, it);
        self.loop_head = loop_head;
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.loop_head = true;
        self.visit_for_statement_left(&it.left);
        self.loop_head = false;
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.loop_head = true;
        self.visit_for_statement_left(&it.left);
        self.loop_head = false;
        self.visit_expression(&it.right);
        self.visit_statement(&it.body);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if self.uses && it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
                let slot = self.target(target.reference_id.get());
                let statement = self.statement;
                let conditional = self.conditional;
                if let Some(found) = self.found_at(slot) {
                    found.conditional |= conditional;
                    found.assignments.push(Assignment {
                        statement,
                        expression: it.span,
                        value: it.right.span(),
                    });
                }
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(target) = it {
            if self.uses {
                self.written.insert(target.span);
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.uses {
            let slot = self.target(it.reference_id.get());
            if let Some(found) = self.found_at(slot) {
                found.references.push(Reference { span: it.span });
            }
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

fn scan(program: &Program<'_>, scoping: &Scoping, targets: Targets, uses: bool) -> Variables {
    let capacity = if uses {
        with_slack(capacity_hint().slots)
    } else {
        0
    };
    let mut scan = VariableScan {
        uses,
        scoping,
        targets: &targets.slots,
        statement: Span::empty(0),
        lists: 0,
        written: FxHashSet::default(),
        members: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
        found: vec![Found::default(); targets.unique.len()],
        loop_head: false,
        names: targets
            .unique
            .iter()
            .enumerate()
            .map(|(slot, symbol)| (scoping.symbol_name(*symbol), slot))
            .collect(),
        symbols: targets.unique.clone(),
        plain: FxHashSet::default(),
        top_list: false,
        top_functions: FxHashSet::default(),
        conditional: false,
    };
    scan.visit_program(program);
    let VariableScan {
        written,
        members,
        mut found,
        ..
    } = scan;
    for entry in &mut found {
        entry
            .references
            .retain(|reference| !written.contains(&reference.span));
    }
    let sole = sole_declarations(program, &targets.unique, &found);
    Variables {
        members,
        found,
        sole,
        positions: targets.positions,
    }
}

fn is_sole_declaration(statement: Option<&Statement<'_>>, symbol: SymbolId) -> bool {
    let Some(Statement::VariableDeclaration(declaration)) = statement else {
        return false;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return false;
    };
    binding_symbol(&declarator.id) == Some(symbol)
}

fn sole_declarations(program: &Program<'_>, symbols: &[SymbolId], found: &[Found]) -> Vec<bool> {
    let spans: Vec<Span> = found.iter().filter_map(|entry| entry.declaration).collect();
    let mut statements = statements_at(program, &spans).into_iter();
    found
        .iter()
        .zip(symbols)
        .map(|(entry, symbol)| {
            entry.declaration.is_some() && is_sole_declaration(statements.next().flatten(), *symbol)
        })
        .collect()
}

pub(super) fn variable_facts(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &[SymbolId],
) -> Variables {
    scan(program, scoping, Targets::of(symbols, |_| true), true)
}

pub(crate) fn constant_initialisers<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    symbols: &[SymbolId],
) -> Vec<Option<&'p Expression<'p>>> {
    let targets = Targets::of(symbols, |symbol| is_constant_binding(scoping, symbol));
    if targets.unique.is_empty() {
        return vec![None; symbols.len()];
    }
    let variables = scan(program, scoping, targets, false);
    let inits: Vec<Option<Span>> = (0..variables.found.len())
        .map(|slot| {
            let facts = variables.slot_facts(slot)?;
            facts.declaration.and(facts.init)
        })
        .collect();
    let spans: Vec<Span> = inits.iter().flatten().copied().collect();
    let mut found = expressions_at(program, &spans).into_iter();
    let by_slot: Vec<Option<&'p Expression<'p>>> = inits
        .iter()
        .map(|init| init.and_then(|_| found.next().flatten()))
        .collect();
    variables
        .positions
        .iter()
        .map(|slot| slot.and_then(|slot| by_slot.get(slot).copied().flatten()))
        .collect()
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
fn facts_digest(variables: &Variables, index: usize) -> u64 {
    use crate::fused::check::{digest_of, sorted};
    let Some(facts) = variables.facts(index) else {
        return 0;
    };
    let members = sorted(
        facts
            .members
            .iter()
            .map(|(span, list)| ((span.start, span.end), *list)),
    );
    digest_of(&(
        (facts.statement, facts.declaration, facts.init),
        (facts.assignments, facts.references),
        members,
    ))
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) fn fused_and_separate_variables(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &[SymbolId],
) -> (u64, u64) {
    use crate::fused::check::digest_of;
    let batched = variable_facts(program, scoping, symbols);
    let fused: Vec<u64> = (0..symbols.len())
        .map(|index| facts_digest(&batched, index))
        .collect();
    let separate: Vec<u64> = symbols
        .iter()
        .map(|symbol| facts_digest(&variable_facts(program, scoping, &[*symbol]), 0))
        .collect();
    (digest_of(&fused), digest_of(&separate))
}

#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) fn fused_and_separate_initialisers(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &[SymbolId],
) -> (u64, u64) {
    use crate::fused::check::digest_of;
    let site = |init: Option<&Expression<'_>>| {
        init.map(|init| {
            (
                init.span(),
                untwist_core::pass::ExpressionKind::of(init).index(),
            )
        })
    };
    let fused: Vec<Option<(Span, usize)>> = constant_initialisers(program, scoping, symbols)
        .into_iter()
        .map(site)
        .collect();
    let separate: Vec<Option<(Span, usize)>> = symbols
        .iter()
        .map(|symbol| {
            let found = constant_initialisers(program, scoping, &[*symbol]);
            site(found.first().copied().flatten())
        })
        .collect();
    (digest_of(&fused), digest_of(&separate))
}
