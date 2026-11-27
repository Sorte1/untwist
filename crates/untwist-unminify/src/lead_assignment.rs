use oxc_allocator::{CloneIn, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use std::cell::OnceCell;

use rustc_hash::FxHashMap;
use untwist_core::pass::{Cx, Interest, Locality, Pass};
use untwist_core::semantic::{has_module_syntax, scope_hazard};

#[derive(Default)]
pub struct LeadAssignment {
    hazard: bool,
    module: bool,
    functions: OnceCell<FxHashMap<String, usize>>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(LeadAssignment::default())
}

fn symbol_of(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn plain_variable(symbol: SymbolId, scoping: &Scoping) -> bool {
    let flags = scoping.symbol_flags(symbol);
    let variable = flags.intersects(
        SymbolFlags::FunctionScopedVariable
            | SymbolFlags::BlockScopedVariable
            | SymbolFlags::CatchVariable,
    );
    let other = flags.intersects(
        SymbolFlags::ConstVariable
            | SymbolFlags::Function
            | SymbolFlags::Class
            | SymbolFlags::Import,
    );
    variable && !other
}

fn is_lead(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    let Expression::AssignmentExpression(assign) = expr else {
        return false;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return false;
    };
    let literal = matches!(
        assign.right.without_parentheses(),
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
    );
    assign.operator == AssignmentOperator::Assign
        && !literal
        && symbol_of(id, scoping).is_some_and(|symbol| plain_variable(symbol, scoping))
}

fn function_names(scoping: &Scoping) -> FxHashMap<String, usize> {
    let mut names: FxHashMap<String, usize> = FxHashMap::default();
    for symbol in scoping.symbol_ids() {
        if scoping.symbol_flags(symbol).is_function() {
            *names
                .entry(scoping.symbol_name(symbol).to_string())
                .or_default() += 1;
        }
    }
    names
}

fn keeps_direct_lead(expr: &Expression<'_>, bare: bool) -> bool {
    match expr {
        Expression::AssignmentExpression(assign) => {
            matches!(assign.left, AssignmentTarget::AssignmentTargetIdentifier(_))
        }
        Expression::UnaryExpression(_)
        | Expression::ParenthesizedExpression(_)
        | Expression::AwaitExpression(_) => bare,
        _ => false,
    }
}

struct Spine<'s> {
    scoping: &'s Scoping,
    local_root: bool,
    functions: &'s OnceCell<FxHashMap<String, usize>>,
}

impl Spine<'_> {
    fn stable_callee(&self, callee: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = callee else {
            return false;
        };
        let Some(symbol) = symbol_of(id, self.scoping) else {
            return false;
        };
        let scoping = self.scoping;
        let flags = scoping.symbol_flags(symbol);
        let kind = flags.intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function)
            && !flags.intersects(SymbolFlags::BlockScopedVariable | SymbolFlags::Class);
        let global = !self.local_root && scoping.symbol_scope_id(symbol) == scoping.root_scope_id();
        let unwritten = scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .all(|reference| !scoping.get_reference(*reference).is_write());
        if !(kind && !global && unwritten) {
            return false;
        }
        let declared_once = self
            .functions
            .get_or_init(|| function_names(scoping))
            .get(scoping.symbol_name(symbol))
            .is_none_or(|count| *count <= usize::from(flags.is_function()));
        let alone = scoping.symbol_redeclarations(symbol).is_empty() && declared_once;
        alone
    }

    fn child<'k, 'a>(&self, expr: &'k mut Expression<'a>) -> Option<&'k mut Expression<'a>> {
        match expr {
            Expression::ParenthesizedExpression(inner) => Some(&mut inner.expression),
            Expression::StaticMemberExpression(member) => Some(&mut member.object),
            Expression::ComputedMemberExpression(member) => Some(&mut member.object),
            Expression::CallExpression(call) if !call.optional => {
                let leads = call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_some_and(|first| is_lead(first, self.scoping));
                if leads && self.stable_callee(&call.callee) {
                    return call.arguments.first_mut()?.as_expression_mut();
                }
                Some(&mut call.callee)
            }
            Expression::BinaryExpression(binary) => Some(&mut binary.left),
            Expression::LogicalExpression(logical) => Some(&mut logical.left),
            Expression::ConditionalExpression(conditional) => Some(&mut conditional.test),
            Expression::UnaryExpression(unary)
                if unary.operator != UnaryOperator::Delete
                    || unary.argument.is_member_expression() =>
            {
                Some(&mut unary.argument)
            }
            Expression::AwaitExpression(wait) => Some(&mut wait.argument),
            Expression::AssignmentExpression(assign) => {
                let plain = assign.operator == AssignmentOperator::Assign;
                let AssignmentExpression { left, right, .. } = &mut **assign;
                match left {
                    AssignmentTarget::AssignmentTargetIdentifier(_) if plain => Some(right),
                    AssignmentTarget::StaticMemberExpression(member) => Some(&mut member.object),
                    AssignmentTarget::ComputedMemberExpression(member) => Some(&mut member.object),
                    _ => None,
                }
            }
            Expression::UpdateExpression(update) => match &mut update.argument {
                SimpleAssignmentTarget::StaticMemberExpression(member) => Some(&mut member.object),
                SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                    Some(&mut member.object)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn lead<'k, 'a>(
        &self,
        expr: &'k mut Expression<'a>,
        bare: bool,
    ) -> Option<&'k mut Expression<'a>> {
        let keeps = keeps_direct_lead(expr, bare);
        let child = self.child(expr)?;
        if !keeps && is_lead(child, self.scoping) {
            return Some(child);
        }
        self.lead(child, false)
    }
}

fn take_lead<'a>(
    root: &mut Expression<'a>,
    bare: bool,
    spine: &Spine<'_>,
    cx: &Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let slot = spine.lead(root, bare)?;
    let Expression::AssignmentExpression(assign) = &*slot else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return None;
    };
    let read = Expression::Identifier(id.clone_in(cx.allocator()));
    Some(std::mem::replace(slot, read))
}

fn take_leads<'a>(
    root: &mut Expression<'a>,
    bare: bool,
    spine: &Spine<'_>,
    cx: &Cx<'a, '_>,
) -> Vec<Expression<'a>> {
    let Some(mut lead) = take_lead(root, bare, spine, cx) else {
        return Vec::new();
    };
    let mut leads = take_leads(&mut lead, false, spine, cx);
    leads.push(lead);
    leads
}

fn root_of<'k, 'a>(stmt: &'k mut Statement<'a>) -> Option<(&'k mut Expression<'a>, bool)> {
    match stmt {
        Statement::ExpressionStatement(node) => Some((&mut node.expression, true)),
        Statement::ReturnStatement(node) => node.argument.as_mut().map(|root| (root, true)),
        Statement::ThrowStatement(node) => Some((&mut node.argument, false)),
        Statement::IfStatement(node) => Some((&mut node.test, false)),
        Statement::SwitchStatement(node) => Some((&mut node.discriminant, false)),
        Statement::VariableDeclaration(node) => match node.declarations.as_mut_slice() {
            [declarator] => declarator.init.as_mut().map(|root| (root, false)),
            _ => None,
        },
        _ => None,
    }
}

impl<'a> Pass<'a> for LeadAssignment {
    fn name(&self) -> &'static str {
        "lead-assignment"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::ENTER_STATEMENTS
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.hazard = scope_hazard(cx.scoping()).is_some();
        self.module = has_module_syntax(program);
        self.functions = OnceCell::new();
    }

    fn enter_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.hazard {
            return;
        }
        let spine = Spine {
            scoping: cx.scoping(),
            local_root: self.module,
            functions: &self.functions,
        };
        let mut leads: Vec<(usize, Expression<'a>)> = Vec::new();
        for (at, stmt) in stmts.iter_mut().enumerate() {
            if let Some((root, bare)) = root_of(stmt) {
                let taken = take_leads(root, bare, &spine, &*cx);
                leads.extend(taken.into_iter().map(|lead| (at, lead)));
            }
        }
        if leads.is_empty() {
            return;
        }
        let count = leads.len();
        let old = std::mem::replace(stmts, AVec::with_capacity_in(stmts.len() + count, &*cx));
        let mut leads = leads.into_iter().peekable();
        for (at, stmt) in old.into_iter().enumerate() {
            while let Some((_, lead)) = leads.next_if(|(index, _)| *index == at) {
                let span = lead.span();
                stmts.push(Statement::new_expression_statement(span, lead, &*cx));
            }
            stmts.push(stmt);
        }
        cx.changed_by(count);
    }
}
