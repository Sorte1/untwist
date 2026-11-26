use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use super::shape::{member, string, Ctx, Key, Machine, AS_OBJECT, CLEAR, NON_CALL, PROPERTY};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Call,
    Clear,
    NonCall,
}

pub(super) struct Site {
    pub(super) span: Span,
    pub(super) key: String,
    pub(super) kind: Kind,
}

pub(super) struct Pair {
    pub(super) sequence: Span,
    pub(super) index: usize,
    pub(super) key: String,
}

#[derive(Default)]
pub(super) struct Sites {
    pub(super) standalone: Vec<Site>,
    pub(super) pairs: Vec<Pair>,
    pub(super) dispatcher_refs: Vec<Span>,
    pub(super) payload_refs: Vec<Span>,
}

struct Scan<'s, 'm> {
    cx: Ctx<'s>,
    machine: &'m Machine,
    sites: Sites,
}

fn strings(arguments: &[Argument<'_>]) -> Option<Vec<String>> {
    arguments
        .iter()
        .map(|a| string(a.as_expression()?))
        .collect()
}

impl Scan<'_, '_> {
    fn flag(&self, value: &str) -> Kind {
        match value {
            v if v == self.machine.keys[CLEAR] => Kind::Clear,
            v if v == self.machine.keys[NON_CALL] => Kind::NonCall,
            _ => Kind::Call,
        }
    }

    fn callee(&self, callee: &Expression<'_>) -> Option<Span> {
        let Expression::Identifier(id) = callee.without_parentheses() else {
            return None;
        };
        self.cx
            .is(callee, self.machine.dispatcher)
            .then_some(id.span)
    }

    fn wrapped(&self, expr: &Expression<'_>) -> Option<(String, Kind, Span)> {
        let (arguments, callee) = match expr.without_parentheses() {
            Expression::CallExpression(call) if !call.optional => (&call.arguments, &call.callee),
            Expression::NewExpression(new) => (&new.arguments, &new.callee),
            _ => return None,
        };
        let callee = self.callee(callee)?;
        let [key, flag, returns] = strings(arguments)?.try_into().ok()?;
        (returns == self.machine.keys[AS_OBJECT]).then(|| (key, self.flag(&flag), callee))
    }

    fn site(&self, expr: &Expression<'_>) -> Option<(String, Kind, Span)> {
        if let Some((object, Key::Str(property))) = member(expr) {
            if property != self.machine.keys[PROPERTY] {
                return None;
            }
            return self.wrapped(object);
        }
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if call.optional {
            return None;
        }
        let callee = self.callee(&call.callee)?;
        match strings(&call.arguments)?.as_slice() {
            [key] => Some((key.clone(), Kind::Call, callee)),
            [key, flag] => match self.flag(flag) {
                Kind::Call => None,
                kind => Some((key.clone(), kind, callee)),
            },
            _ => None,
        }
    }

    fn payload_store(&self, expr: &Expression<'_>) -> Option<Span> {
        let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
            return None;
        };
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
            return None;
        };
        let Expression::ArrayExpression(array) = assign.right.without_parentheses() else {
            return None;
        };
        let symbol: Option<SymbolId> = self
            .cx
            .scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id();
        let holes = array
            .elements
            .iter()
            .any(|e| matches!(e, ArrayExpressionElement::Elision(_)));
        (assign.operator == AssignmentOperator::Assign
            && symbol == Some(self.machine.payload)
            && !holes)
            .then_some(id.span)
    }

    fn pairs(&mut self, sequence: &SequenceExpression<'_>) {
        let expressions = &sequence.expressions;
        let mut index = 0;
        while index + 1 < expressions.len() {
            let store = self.payload_store(&expressions[index]);
            let call = self.site(&expressions[index + 1]);
            match (store, call) {
                (Some(store), Some((key, Kind::Call, callee))) => {
                    self.sites.payload_refs.push(store);
                    self.sites.dispatcher_refs.push(callee);
                    self.sites.pairs.push(Pair {
                        sequence: sequence.span,
                        index,
                        key,
                    });
                    index += 2;
                }
                _ => index += 1,
            }
        }
    }
}

impl<'a> VisitJs<'a> for Scan<'_, '_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::SequenceExpression(sequence) = it {
            self.pairs(sequence);
        }
        if !matches!(it, Expression::ParenthesizedExpression(_)) {
            if let Some((key, kind, callee)) = self.site(it) {
                if kind != Kind::Call {
                    self.sites.dispatcher_refs.push(callee);
                    self.sites.standalone.push(Site {
                        span: it.span(),
                        key,
                        kind,
                    });
                }
            }
        }
        walk_js::walk_expression(self, it);
    }
}

pub(super) fn scan(program: &Program<'_>, cx: Ctx<'_>, machine: &Machine) -> Sites {
    let mut scan = Scan {
        cx,
        machine,
        sites: Sites::default(),
    };
    scan.visit_program(program);
    scan.sites
}
