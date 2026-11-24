use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;

use crate::alias_forms::{alias_source, literal_headed_identifier, single_property_source};
use crate::bindings::{reference_symbol, resolved_symbol};
use untwist_core::machinery_name::string_array_unit;

pub(crate) struct LazyShape {
    pub(crate) self_write: Span,
    pub(crate) stateless: bool,
    pub(crate) arrays: Vec<SymbolId>,
}

pub(crate) fn lazy_shape(
    outer: &Function<'_>,
    symbol: SymbolId,
    scoping: &Scoping,
) -> Option<LazyShape> {
    let body = outer.body.as_ref()?;
    let (redefinition, prelude) = redefinition(&body.statements, symbol, scoping)?;
    let (first, arguments) = redefinition;
    let Expression::AssignmentExpression(assignment) = first.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign
        || resolved_symbol(target, scoping) != Some(symbol)
    {
        return None;
    }
    let Expression::FunctionExpression(inner) = assignment.right.without_parentheses() else {
        return None;
    };
    if !plain_function(outer) || !plain_function(inner) || !same_parameters(outer, inner) {
        return None;
    }
    if names_itself(inner, scoping) {
        return None;
    }
    let prelude = forwards_parameters(outer, arguments, scoping)
        .and_then(|params| array_prelude(prelude, &params, scoping));
    let (stateless, arrays) = match prelude {
        Some(prelude) => (
            inner_is_stateless(outer, inner, symbol, &prelude.locals, scoping),
            prelude.sources,
        ),
        None => (false, Vec::new()),
    };
    Some(LazyShape {
        self_write: target.span,
        stateless,
        arrays,
    })
}

fn plain_function(function: &Function<'_>) -> bool {
    !function.r#async && !function.generator
}

fn plain_parameters<'p>(params: &'p FormalParameters<'_>) -> Option<Vec<&'p str>> {
    if params.rest.is_some() {
        return None;
    }
    let mut names = Vec::with_capacity(params.items.len());
    for param in &params.items {
        let BindingPattern::BindingIdentifier(id) = &param.pattern else {
            return None;
        };
        if param.initializer.is_some() || names.contains(&id.name.as_str()) {
            return None;
        }
        names.push(id.name.as_str());
    }
    Some(names)
}

fn same_parameters(outer: &Function<'_>, inner: &Function<'_>) -> bool {
    match (
        plain_parameters(&outer.params),
        plain_parameters(&inner.params),
    ) {
        (Some(outer), Some(inner)) => outer.len() == inner.len(),
        _ => false,
    }
}

fn names_itself(inner: &Function<'_>, scoping: &Scoping) -> bool {
    inner
        .id
        .as_ref()
        .and_then(|id| id.symbol_id.get())
        .is_some_and(|symbol| !scoping.get_resolved_reference_ids(symbol).is_empty())
}

type Redefinition<'k, 'a> = (&'k Expression<'a>, &'k [Argument<'a>]);

fn redefinition<'k, 'a>(
    statements: &'k [Statement<'a>],
    symbol: SymbolId,
    scoping: &Scoping,
) -> Option<(Redefinition<'k, 'a>, &'k [Statement<'a>])> {
    let (Statement::ReturnStatement(ret), rest) = statements.split_last()? else {
        return None;
    };
    let returned = ret.argument.as_ref()?.without_parentheses();
    let calls_decoder = |expr: &'k Expression<'a>| {
        let (callee, arguments) = call_parts(expr.without_parentheses())?;
        (reference_symbol(callee, scoping) == Some(symbol)).then_some(arguments.as_slice())
    };
    if let Some((callee, arguments)) = call_parts(returned) {
        if let Expression::AssignmentExpression(_) = callee.without_parentheses() {
            return Some(((callee, arguments.as_slice()), rest));
        }
    }
    if let Expression::SequenceExpression(sequence) = returned {
        let [first, second] = sequence.expressions.as_slice() else {
            return None;
        };
        return Some(((first, calls_decoder(second)?), rest));
    }
    let (Statement::ExpressionStatement(assignment), prelude) = rest.split_last()? else {
        return None;
    };
    Some(((&assignment.expression, calls_decoder(returned)?), prelude))
}

fn forwards_parameters(
    outer: &Function<'_>,
    arguments: &[Argument<'_>],
    scoping: &Scoping,
) -> Option<Vec<SymbolId>> {
    if outer.params.rest.is_some() || outer.params.items.len() != arguments.len() {
        return None;
    }
    let mut params = Vec::with_capacity(arguments.len());
    for (param, argument) in outer.params.items.iter().zip(arguments) {
        let BindingPattern::BindingIdentifier(id) = &param.pattern else {
            return None;
        };
        let param_symbol = id.symbol_id.get()?;
        let Argument::Identifier(passed) = argument else {
            return None;
        };
        if param.initializer.is_some() || resolved_symbol(passed, scoping) != Some(param_symbol) {
            return None;
        }
        params.push(param_symbol);
    }
    Some(params)
}

struct ArrayPrelude {
    locals: FxHashSet<SymbolId>,
    sources: Vec<SymbolId>,
}

fn array_prelude(
    prelude: &[Statement<'_>],
    params: &[SymbolId],
    scoping: &Scoping,
) -> Option<ArrayPrelude> {
    let mut arrays = FxHashSet::default();
    let mut sources = Vec::new();
    for statement in prelude {
        let Statement::VariableDeclaration(declaration) = statement else {
            return None;
        };
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let symbol = id.symbol_id.get()?;
            let (callee, arguments) = call_parts(declarator.init.as_ref()?)?;
            let is_array = identifier_name(callee)
                .and_then(string_array_unit)
                .is_some();
            if !is_array || !arguments.is_empty() || params.contains(&symbol) {
                return None;
            }
            sources.push(reference_symbol(callee, scoping)?);
            arrays.insert(symbol);
        }
    }
    Some(ArrayPrelude {
        locals: arrays,
        sources,
    })
}

fn inner_is_stateless(
    outer: &Function<'_>,
    inner: &Function<'_>,
    decoder: SymbolId,
    arrays: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    let Some(outer_scope) = outer.scope_id.get() else {
        return false;
    };
    let mut scan = InnerScan {
        scoping,
        outer_scope,
        decoder,
        arrays,
        indexed: FxHashSet::default(),
        stateful: false,
    };
    scan.visit_function(inner, ScopeFlags::Function);
    !scan.stateful
}

struct InnerScan<'s> {
    scoping: &'s Scoping,
    outer_scope: ScopeId,
    decoder: SymbolId,
    arrays: &'s FxHashSet<SymbolId>,
    indexed: FxHashSet<Span>,
    stateful: bool,
}

impl InnerScan<'_> {
    fn is_array(&self, expr: &Expression<'_>) -> bool {
        reference_symbol(expr.without_parentheses(), self.scoping)
            .is_some_and(|symbol| self.arrays.contains(&symbol))
    }

    fn member_object_is_array(&self, member: &MemberExpression<'_>) -> bool {
        self.is_array(member.object())
    }

    fn note_object(&mut self, object: &Expression<'_>) {
        if let Expression::Identifier(id) = object {
            self.indexed.insert(id.span);
        }
    }

    fn target_is_array(&self, target: &SimpleAssignmentTarget<'_>) -> bool {
        target
            .as_member_expression()
            .is_some_and(|member| self.member_object_is_array(member))
    }
}

impl<'a> VisitJs<'a> for InnerScan<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            self.stateful = true;
            return;
        };
        let reference = self.scoping.get_reference(reference);
        let Some(symbol) = reference.symbol_id() else {
            self.stateful |= it.name == "arguments";
            return;
        };
        if symbol == self.decoder {
            self.stateful = true;
            return;
        }
        if self.scoping.symbol_scope_id(symbol) != self.outer_scope {
            return;
        }
        let indexed = self.indexed.contains(&it.span);
        if !self.arrays.contains(&symbol) || reference.is_write() || !indexed {
            self.stateful = true;
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.note_object(&it.object);
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.note_object(&it.object);
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(target) = it.left.as_simple_assignment_target() {
            self.stateful |= self.target_is_array(target);
        } else if !matches!(it.left, AssignmentTarget::AssignmentTargetIdentifier(_)) {
            self.stateful = true;
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        self.stateful |= self.target_is_array(&it.argument);
        walk_js::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            self.stateful |= it
                .argument
                .without_parentheses()
                .as_member_expression()
                .is_some_and(|member| self.member_object_is_array(member));
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.stateful |= it
            .callee
            .without_parentheses()
            .as_member_expression()
            .is_some_and(|member| self.member_object_is_array(member));
        walk_js::walk_call_expression(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.stateful |= !matches!(it.left, ForStatementLeft::VariableDeclaration(_));
        walk_js::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.stateful |= !matches!(it.left, ForStatementLeft::VariableDeclaration(_));
        walk_js::walk_for_of_statement(self, it);
    }
}

pub(crate) struct CalleeUses {
    pub(crate) callee_only: FxHashSet<SymbolId>,
    pub(crate) member_callees: FxHashSet<Span>,
    pub(crate) copied_to: FxHashMap<SymbolId, Vec<SymbolId>>,
}

impl CalleeUses {
    pub(crate) fn reach(&self, symbol: SymbolId) -> FxHashSet<SymbolId> {
        let mut reached = FxHashSet::default();
        let mut pending = vec![symbol];
        while let Some(next) = pending.pop() {
            if reached.insert(next) {
                pending.extend(self.copied_to.get(&next).into_iter().flatten());
            }
        }
        reached
    }
}

pub(crate) fn callee_uses(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
    discarded: &FxHashSet<ReferenceId>,
) -> CalleeUses {
    let mut copies = CopyScan {
        scoping,
        discarded,
        found: Vec::new(),
        members: Vec::new(),
        member_objects: Vec::new(),
    };
    copies.visit_program(program);
    let mut reached = candidates.clone();
    reached.extend(copies.members.iter().map(|(_, to)| *to));
    let mut grew = true;
    while grew {
        grew = false;
        for (from, to) in &copies.found {
            if reached.contains(from) && reached.insert(*to) {
                grew = true;
            }
        }
    }
    let mut scan = CalleeScan {
        scoping,
        discarded,
        candidates: &reached,
        callees: FxHashSet::default(),
        member_callees: FxHashSet::default(),
        copies: FxHashMap::default(),
        observed: FxHashSet::default(),
        copied: Vec::new(),
    };
    scan.visit_program(program);
    let mut observed = scan.observed;
    let mut changed = true;
    while changed {
        changed = false;
        for (from, to) in &scan.copied {
            if !observed.contains(from) && (observed.contains(to) || !reached.contains(to)) {
                observed.insert(*from);
                changed = true;
            }
        }
    }
    let callee_only = candidates
        .iter()
        .copied()
        .filter(|symbol| !observed.contains(symbol))
        .collect();
    let mut member_callees = scan.member_callees;
    member_callees.extend(
        copies
            .members
            .iter()
            .filter(|(_, to)| !observed.contains(to))
            .map(|(member, _)| *member),
    );
    let mut copied_to: FxHashMap<SymbolId, Vec<SymbolId>> = FxHashMap::default();
    for (from, to) in copies.found.iter().chain(&copies.member_objects) {
        copied_to.entry(*from).or_default().push(*to);
    }
    CalleeUses {
        callee_only,
        member_callees,
        copied_to,
    }
}

struct CopyScan<'s> {
    scoping: &'s Scoping,
    discarded: &'s FxHashSet<ReferenceId>,
    found: Vec<(SymbolId, SymbolId)>,
    members: Vec<(Span, SymbolId)>,
    member_objects: Vec<(SymbolId, SymbolId)>,
}

fn dropped(right: &IdentifierReference<'_>, discarded: &FxHashSet<ReferenceId>) -> bool {
    right
        .reference_id
        .get()
        .is_some_and(|reference| discarded.contains(&reference))
}

impl<'a> VisitJs<'a> for CopyScan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            match (plain_identifier(init), init, id.symbol_id.get()) {
                (Some(init), _, Some(to)) => {
                    if let Some(from) = resolved_symbol(init, self.scoping) {
                        self.found.push((from, to));
                    }
                }
                (None, Expression::StaticMemberExpression(member), Some(to)) => {
                    self.members.push((member.span, to));
                    if let Some(object) = reference_symbol(&member.object, self.scoping) {
                        self.member_objects.push((object, to));
                    }
                }
                _ => {}
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let (
            AssignmentOperator::Assign,
            AssignmentTarget::AssignmentTargetIdentifier(target),
            Some(right),
        ) = (it.operator, &it.left, plain_identifier(&it.right))
        {
            if !dropped(right, self.discarded) {
                walk_js::walk_assignment_expression(self, it);
                return;
            }
            let from = resolved_symbol(right, self.scoping);
            if let (Some(from), Some(to)) = (from, resolved_symbol(target, self.scoping)) {
                self.found.push((from, to));
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }
}

struct CalleeScan<'s> {
    scoping: &'s Scoping,
    discarded: &'s FxHashSet<ReferenceId>,
    candidates: &'s FxHashSet<SymbolId>,
    callees: FxHashSet<Span>,
    member_callees: FxHashSet<Span>,
    copies: FxHashMap<Span, Option<SymbolId>>,
    observed: FxHashSet<SymbolId>,
    copied: Vec<(SymbolId, SymbolId)>,
}

impl<'a> VisitJs<'a> for CalleeScan<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !it.optional {
            match plain_identifier(&it.callee) {
                Some(id) => {
                    self.callees.insert(id.span);
                }
                None if it.callee.is_member_expression() => {
                    self.member_callees.insert(it.callee.span());
                }
                None => {}
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Some(init) = plain_identifier(init) {
                self.copies.insert(init.span, id.symbol_id.get());
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let (
            AssignmentOperator::Assign,
            AssignmentTarget::AssignmentTargetIdentifier(target),
            Some(right),
        ) = (it.operator, &it.left, plain_identifier(&it.right))
        {
            if dropped(right, self.discarded) {
                self.copies
                    .insert(right.span, resolved_symbol(target, self.scoping));
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        let Some(symbol) = reference.symbol_id() else {
            return;
        };
        if !self.candidates.contains(&symbol) || !reference.is_read() {
            return;
        }
        if self.callees.contains(&it.span) {
            return;
        }
        match self.copies.get(&it.span) {
            Some(Some(to)) if *to != symbol => self.copied.push((symbol, *to)),
            _ => {
                self.observed.insert(symbol);
            }
        }
    }
}

fn plain_identifier<'e, 'a>(expr: &'e Expression<'a>) -> Option<&'e IdentifierReference<'a>> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id),
        _ => literal_headed_identifier(expr),
    }
}

pub(crate) fn written_outside(scoping: &Scoping, symbol: SymbolId, home: ScopeId) -> bool {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .map(|reference| scoping.get_reference(*reference))
        .any(|reference| reference.is_write() && !within(scoping, reference.scope_id(), home))
}

fn within(scoping: &Scoping, scope: ScopeId, home: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| ancestor == home)
}

pub(crate) struct ValueReadFacts {
    pub(crate) found: FxHashSet<SymbolId>,
    pub(crate) copies: Vec<(Option<SymbolId>, SymbolId)>,
    pub(crate) parameters: FxHashSet<SymbolId>,
    pub(crate) exported: FxHashSet<SymbolId>,
    pub(crate) discarded: FxHashSet<ReferenceId>,
}

pub(crate) fn read_as_values(
    program: &Program<'_>,
    scoping: &Scoping,
    homes: &FxHashMap<SymbolId, ScopeId>,
) -> ValueReadFacts {
    let mut scan = ValueReads {
        scoping,
        homes,
        passed: FxHashSet::default(),
        copied_into: FxHashMap::default(),
        copying: None,
        facts: ValueReadFacts {
            found: FxHashSet::default(),
            copies: Vec::new(),
            parameters: FxHashSet::default(),
            exported: FxHashSet::default(),
            discarded: FxHashSet::default(),
        },
    };
    scan.visit_program(program);
    scan.facts
}

struct ValueReads<'s> {
    scoping: &'s Scoping,
    homes: &'s FxHashMap<SymbolId, ScopeId>,
    passed: FxHashSet<ReferenceId>,
    copied_into: FxHashMap<ReferenceId, Option<SymbolId>>,
    copying: Option<Option<SymbolId>>,
    facts: ValueReadFacts,
}

impl ValueReads<'_> {
    fn pass(&mut self, id: &IdentifierReference<'_>) {
        if let Some(reference) = id.reference_id.get() {
            self.passed.insert(reference);
        }
    }

    fn discard(&mut self, expr: &Expression<'_>) {
        match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => {
                for expr in &sequence.expressions {
                    self.discard(expr);
                }
            }
            Expression::AssignmentExpression(it) => self.discard_assignment(it),
            _ => {}
        }
    }

    fn discard_assignment(&mut self, it: &AssignmentExpression<'_>) {
        let (
            AssignmentOperator::Assign,
            AssignmentTarget::AssignmentTargetIdentifier(target),
            Some(right),
        ) = (it.operator, &it.left, plain_identifier(&it.right))
        else {
            return;
        };
        if let Some(reference) = right.reference_id.get() {
            self.facts.discarded.insert(reference);
            self.copied_into
                .insert(reference, resolved_symbol(target, self.scoping));
        }
    }

    fn export(&mut self, id: Option<&BindingIdentifier<'_>>) {
        self.facts
            .exported
            .extend(id.and_then(|id| id.symbol_id.get()));
    }

    fn outside_home(&self, reference: ReferenceId) -> Option<SymbolId> {
        let reference = self.scoping.get_reference(reference);
        let symbol = reference.symbol_id()?;
        let home = self.homes.get(&symbol)?;
        let outside = !within(self.scoping, reference.scope_id(), *home);
        (outside && reference.is_read()).then_some(symbol)
    }
}

impl<'a> VisitJs<'a> for ValueReads<'_> {
    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if let BindingPattern::BindingIdentifier(id) = &it.pattern {
            self.facts.parameters.extend(id.symbol_id.get());
        }
        walk_js::walk_formal_parameter(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let (false, Some(id)) = (it.optional, plain_identifier(&it.callee)) {
            self.pass(id);
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let copy = match (&it.id, &it.init) {
            (BindingPattern::BindingIdentifier(id), Some(init))
                if alias_source(init, self.scoping).is_some()
                    || single_property_source(init, self.scoping).is_some() =>
            {
                Some(id.symbol_id.get())
            }
            _ => None,
        };
        let outer = self.copying;
        self.copying = outer.or(copy);
        walk_js::walk_variable_declarator(self, it);
        self.copying = outer;
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.discard(&it.expression);
        walk_js::walk_expression_statement(self, it);
    }

    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
        if let Some((_, heads)) = it.expressions.split_last() {
            for head in heads {
                self.discard(head);
            }
        }
        walk_js::walk_sequence_expression(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        if let Some(init) = it.init.as_ref().and_then(ForStatementInit::as_expression) {
            self.discard(init);
        }
        if let Some(update) = &it.update {
            self.discard(update);
        }
        walk_js::walk_for_statement(self, it);
    }

    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        match &it.declaration {
            Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    for id in declarator.id.get_binding_identifiers() {
                        self.export(Some(id));
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => self.export(function.id.as_ref()),
            Declaration::ClassDeclaration(class) => self.export(class.id.as_ref()),
            _ => {}
        }
        walk_js::walk_export_declaration(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                self.export(function.id.as_ref())
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => self.export(class.id.as_ref()),
            _ => {}
        }
        walk_js::walk_export_default_declaration(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(id) = it.reference_id.get() else {
            return;
        };
        if self.passed.contains(&id) {
            return;
        }
        let Some(symbol) = self.outside_home(id) else {
            return;
        };
        match self.copying.or_else(|| self.copied_into.get(&id).copied()) {
            Some(to) => self.facts.copies.push((to, symbol)),
            None => {
                self.facts.found.insert(symbol);
            }
        }
    }
}
