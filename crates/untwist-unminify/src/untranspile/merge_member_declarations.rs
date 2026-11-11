use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::pattern::object_pattern;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::scope_hazard;

use crate::settled::{arguments_can_rebind, is_unwritten_here, script_root};

use super::shapes;

pub const PASS: &str = "merge-member-declarations";

const SHAPE: &str = "member-run";

const MAX_RUN: usize = 32;

#[derive(Default)]
pub struct MergeMemberDeclarations {
    hazard: bool,
    script_root: Option<ScopeId>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(MergeMemberDeclarations::default())
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Read {
    kind: VariableDeclarationKind,
    base: SymbolId,
    target: SymbolId,
}

fn member_parts<'k, 'a>(init: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
    let object = match init {
        Expression::StaticMemberExpression(member) if !member.optional => &member.object,
        Expression::ComputedMemberExpression(member)
            if !member.optional && matches!(member.expression, Expression::StringLiteral(_)) =>
        {
            &member.object
        }
        _ => return None,
    };
    match object {
        Expression::Identifier(base) => Some(base),
        _ => None,
    }
}

impl MergeMemberDeclarations {
    fn read_of(&self, statement: &Statement<'_>, scoping: &Scoping) -> Option<Read> {
        let Statement::VariableDeclaration(declaration) = statement else {
            return None;
        };
        let [declarator] = declaration.declarations.as_slice() else {
            return None;
        };
        if declaration.declare {
            return None;
        }
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        let base = member_parts(declarator.init.as_ref()?)?;
        let base = scoping
            .get_reference(base.reference_id.get()?)
            .symbol_id()?;
        let target = id.symbol_id.get()?;
        let mergeable = target != base && is_unwritten_here(scoping, base, self.script_root);
        mergeable.then_some(Read {
            kind: declaration.kind,
            base,
            target,
        })
    }

    fn repeats(&self, statements: &[Statement<'_>], target: SymbolId, scoping: &Scoping) -> bool {
        statements
            .iter()
            .filter_map(|statement| self.read_of(statement, scoping))
            .any(|read| read.target == target)
    }

    fn run_end(&self, statements: &[Statement<'_>], start: usize, scoping: &Scoping) -> usize {
        let Some(first) = statements.get(start).and_then(|s| self.read_of(s, scoping)) else {
            return start;
        };
        let mut end = start + 1;
        while let Some(next) = statements.get(end).and_then(|s| self.read_of(s, scoping)) {
            let joins = end - start < MAX_RUN
                && next.kind == first.kind
                && next.base == first.base
                && !self.repeats(&statements[start..end], next.target, scoping);
            if !joins {
                break;
            }
            end += 1;
        }
        if end - start >= 2 && arguments_can_rebind(scoping, first.base) {
            return start + 1;
        }
        end
    }
}

fn into_member(
    statement: Statement<'_>,
) -> Option<(
    Span,
    VariableDeclarationKind,
    BindingPattern<'_>,
    Expression<'_>,
)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let declaration = declaration.unbox();
    let declarator = declaration.declarations.into_iter().next()?;
    Some((
        declaration.span,
        declaration.kind,
        declarator.id,
        declarator.init?,
    ))
}

fn key_and_object<'a>(
    init: Expression<'a>,
    cx: &Cx<'a, '_>,
) -> Option<(PropertyKey<'a>, Expression<'a>)> {
    match init {
        Expression::StaticMemberExpression(member) => {
            let member = member.unbox();
            let key =
                PropertyKey::StaticIdentifier(oxc_allocator::Box::new_in(member.property, cx));
            Some((key, member.object))
        }
        Expression::ComputedMemberExpression(member) => {
            let member = member.unbox();
            let Expression::StringLiteral(literal) = member.expression else {
                return None;
            };
            Some((PropertyKey::StringLiteral(literal), member.object))
        }
        _ => None,
    }
}

fn merged<'a>(run: impl Iterator<Item = Statement<'a>>, cx: &Cx<'a, '_>) -> Option<Statement<'a>> {
    let mut members = Vec::new();
    let mut head = None;
    for statement in run {
        let (span, kind, target, init) = into_member(statement)?;
        let (key, object) = key_and_object(init, cx)?;
        if head.is_none() {
            head = Some((span, kind, object));
        }
        members.push((key, target));
    }
    let (span, kind, object) = head?;
    let pattern = object_pattern(members, cx);
    let declarator = VariableDeclarator::new(span, pattern, None, Some(object), false, cx);
    let declarations = AVec::from_iter_in([declarator], cx);
    Some(Statement::new_variable_declaration(
        span,
        kind,
        declarations,
        false,
        cx,
    ))
}

impl<'a> Pass<'a> for MergeMemberDeclarations {
    fn name(&self) -> &'static str {
        PASS
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_STATEMENTS
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.hazard = scope_hazard(cx.scoping()).is_some();
        self.script_root = script_root(program, cx.scoping());
    }

    fn exit_statements(&mut self, node: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.hazard || node.len() < 2 {
            return;
        }
        let mut runs = Vec::new();
        let mut at = 0;
        while at < node.len() {
            let end = self.run_end(node, at, cx.scoping());
            if end - at >= 2 {
                runs.push((at, end));
                at = end;
            } else {
                at += 1;
            }
        }
        if runs.is_empty() {
            return;
        }
        let old = std::mem::replace(node, AVec::with_capacity_in(node.len(), cx));
        let mut statements = old.into_iter().enumerate().peekable();
        let mut runs = runs.into_iter().peekable();
        while let Some((index, statement)) = statements.next() {
            let Some(&(start, end)) = runs.peek().filter(|(start, _)| *start == index) else {
                node.push(statement);
                continue;
            };
            runs.next();
            let rest = (start + 1..end).filter_map(|_| statements.next().map(|(_, s)| s));
            let run = std::iter::once(statement).chain(rest);
            if let Some(merged) = merged(run, cx) {
                node.push(merged);
                shapes::fire(PASS, SHAPE);
                cx.changed();
            }
        }
    }
}
