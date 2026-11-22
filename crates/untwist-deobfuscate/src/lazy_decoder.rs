use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;

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
    let (redefinition, prelude) = redefinition(&body.statements)?;
    let (first, second) = redefinition;
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
    let (callee, arguments) = call_parts(second.without_parentheses())?;
    if reference_symbol(callee, scoping) != Some(symbol) {
        return None;
    }
    let stateless = forwards_parameters(outer, arguments, scoping)
        .and_then(|params| array_prelude(prelude, &params))
        .is_some_and(|arrays| inner_is_stateless(outer, inner, symbol, &arrays, scoping));
    Some(LazyShape {
        self_write: target.span,
        stateless,
    })
}

type Redefinition<'k, 'a> = (&'k Expression<'a>, &'k Expression<'a>);

fn redefinition<'k, 'a>(
    statements: &'k [Statement<'a>],
) -> Option<(Redefinition<'k, 'a>, &'k [Statement<'a>])> {
    let (Statement::ReturnStatement(ret), rest) = statements.split_last()? else {
        return None;
    };
    let returned = ret.argument.as_ref()?.without_parentheses();
    if let Expression::SequenceExpression(sequence) = returned {
        let [first, second] = sequence.expressions.as_slice() else {
            return None;
        };
        return Some(((first, second), rest));
    }
    let (Statement::ExpressionStatement(assignment), prelude) = rest.split_last()? else {
        return None;
    };
    Some(((&assignment.expression, returned), prelude))
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
    let mut scan = CalleeScan {
        scoping,
        candidates,
        callees: FxHashSet::default(),
        member_callees: FxHashSet::default(),
        observed: FxHashSet::default(),
    };
    scan.visit_program(program);
    let callee_only = candidates
        .iter()
        .copied()
        .filter(|symbol| !scan.observed.contains(symbol))
        .collect();
    CalleeUses {
        callee_only,
        member_callees: scan.member_callees,
    }
}

struct CalleeScan<'s> {
    scoping: &'s Scoping,
    candidates: &'s FxHashSet<SymbolId>,
    callees: FxHashSet<Span>,
    member_callees: FxHashSet<Span>,
    observed: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for CalleeScan<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !it.optional {
            match &it.callee {
                Expression::Identifier(id) => {
                    self.callees.insert(id.span);
                }
                callee if callee.is_member_expression() => {
                    self.member_callees.insert(callee.span());
                }
                _ => {}
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        let Some(symbol) = reference.symbol_id() else {
            return;
        };
        if !self.candidates.contains(&symbol) {
            return;
        }
        if reference.is_read() && !self.callees.contains(&it.span) {
            self.observed.insert(symbol);
        }
    }
}
