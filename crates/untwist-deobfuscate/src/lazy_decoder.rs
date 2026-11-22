use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;

use crate::alias_forms::literal_headed_identifier;
use crate::bindings::{reference_symbol, resolved_symbol};
use untwist_core::machinery_name::string_array_unit;

pub(crate) struct LazyShape {
    pub(crate) self_write: Span,
    pub(crate) stateless: bool,
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
    let stateless = forwards_parameters(outer, arguments, scoping)
        .and_then(|params| array_prelude(prelude, &params))
        .is_some_and(|arrays| inner_is_stateless(outer, inner, symbol, &arrays, scoping));
    Some(LazyShape {
        self_write: target.span,
        stateless,
    })
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

fn array_prelude(prelude: &[Statement<'_>], params: &[SymbolId]) -> Option<FxHashSet<SymbolId>> {
    let mut arrays = FxHashSet::default();
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
            arrays.insert(symbol);
        }
    }
    Some(arrays)
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
}

pub(crate) fn callee_uses(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
) -> CalleeUses {
    let mut copies = CopyScan {
        scoping,
        found: Vec::new(),
        members: Vec::new(),
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
    CalleeUses {
        callee_only,
        member_callees,
    }
}

struct CopyScan<'s> {
    scoping: &'s Scoping,
    found: Vec<(SymbolId, SymbolId)>,
    members: Vec<(Span, SymbolId)>,
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
            self.copies
                .insert(right.span, resolved_symbol(target, self.scoping));
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
