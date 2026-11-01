use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;

use super::identify::resolved;
use super::machine::{Loc, Machine, Part, Slot, Target, Term, TryRegion};
use super::machine_check::Reads;
use super::machine_read::{self, Machinery, Parsed, Read};

pub struct Sent<'s> {
    pub scoping: &'s Scoping,
    pub state: SymbolId,
}

impl Sent<'_> {
    fn is_state(&self, expression: &Expression<'_>) -> bool {
        resolved(expression, self.scoping) == Some(self.state)
    }

    fn member<'k, 'a>(&self, expression: &'k Expression<'a>, name: &str) -> bool {
        matches!(expression.without_parentheses(), Expression::StaticMemberExpression(member)
            if member.property.name.as_str() == name && self.is_state(&member.object))
    }
}

impl Reads for Sent<'_> {
    fn is_read(&self, expression: &Expression<'_>) -> bool {
        matches!(expression, Expression::CallExpression(call)
            if call.arguments.is_empty() && !call.optional && self.member(&call.callee, "sent"))
    }
}

pub struct Tslib<'s> {
    pub sent: Sent<'s>,
    pub home: Option<ScopeId>,
    pub values: &'s dyn Fn(&Expression<'_>) -> bool,
}

fn numeric(expression: &Expression<'_>) -> Option<u32> {
    match expression.without_parentheses() {
        Expression::NumericLiteral(literal)
            if literal.value.fract() == 0.0
                && (0.0..=f64::from(u16::MAX)).contains(&literal.value) =>
        {
            Some(literal.value as u32)
        }
        _ => None,
    }
}

fn element<'k, 'a>(array: &'k ArrayExpression<'a>, index: usize) -> Option<&'k Expression<'a>> {
    array.elements.get(index)?.as_expression()
}

fn op_array<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k ArrayExpression<'a>> {
    let Statement::ReturnStatement(it) = statement else {
        return None;
    };
    match it.argument.as_ref()?.without_parentheses() {
        Expression::ArrayExpression(array)
            if array.elements.len() <= 2
                && array.elements.iter().all(|element| element.is_expression()) =>
        {
            Some(array)
        }
        _ => None,
    }
}

impl Tslib<'_> {
    fn op_term(&self, array: &ArrayExpression<'_>, loc: Loc, label: u32) -> Option<Term> {
        let op = numeric(element(array, 0)?)?;
        let value = (array.elements.len() == 2).then_some(Slot {
            at: loc,
            part: Part::Element,
        });
        Some(match op {
            2 => Term::Return(value),
            3 => Term::Goto(Target::Label(numeric(element(array, 1)?)?)),
            4 => Term::Yield {
                value,
                delegate: false,
                resume: label + 1,
            },
            5 => {
                let Expression::CallExpression(call) = element(array, 1)?.without_parentheses()
                else {
                    return None;
                };
                let delegated = call.arguments.len() == 1
                    && !call.optional
                    && call.arguments.first()?.is_expression()
                    && (self.values)(&call.callee);
                delegated.then_some(Term::Yield {
                    value: Some(Slot {
                        at: loc,
                        part: Part::ElementArgument,
                    }),
                    delegate: true,
                    resume: label + 1,
                })?
            }
            7 if array.elements.len() == 1 => Term::EndFinally,
            _ => return None,
        })
    }

    fn label_set(&self, statement: &Statement<'_>) -> Option<u32> {
        let Statement::ExpressionStatement(it) = statement else {
            return None;
        };
        let Expression::AssignmentExpression(assignment) = it.expression.without_parentheses()
        else {
            return None;
        };
        let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
            return None;
        };
        let set = assignment.operator == AssignmentOperator::Assign
            && member.property.name.as_str() == "label"
            && self.sent.is_state(&member.object);
        set.then(|| numeric(&assignment.right)).flatten()
    }

    fn trys_push(&self, statement: &Statement<'_>, label: u32) -> Option<TryRegion> {
        let Statement::ExpressionStatement(it) = statement else {
            return None;
        };
        let Expression::CallExpression(call) = it.expression.without_parentheses() else {
            return None;
        };
        let Expression::StaticMemberExpression(push) = call.callee.without_parentheses() else {
            return None;
        };
        if push.property.name.as_str() != "push" || !self.sent.member(&push.object, "trys") {
            return None;
        }
        let [Argument::ArrayExpression(locs)] = call.arguments.as_slice() else {
            return None;
        };
        let slot = |index: usize| match locs.elements.get(index) {
            Some(ArrayExpressionElement::Elision(_)) | None => Some(None),
            Some(element) => element.as_expression().and_then(numeric).map(Some),
        };
        let region = TryRegion {
            start: slot(0)??,
            catch: slot(1)?,
            finally: slot(2)?,
            end: slot(3)??,
        };
        (locs.elements.len() == 4 && region.start == label).then_some(region)
    }
}

impl<'a> Machinery<'a> for Tslib<'_> {
    fn reads(&self) -> &dyn Reads {
        &self.sent
    }

    fn scoping(&self) -> &Scoping {
        self.sent.scoping
    }

    fn home(&self) -> Option<ScopeId> {
        self.home
    }

    fn falls_through(&self) -> bool {
        false
    }

    fn needs_received(&self) -> bool {
        true
    }

    fn statement<'k>(
        &self,
        statement: &'k Statement<'a>,
        _next: Option<&'k Statement<'a>>,
        loc: Loc,
        _next_loc: Option<Loc>,
        label: u32,
    ) -> Option<Parsed> {
        let parsed = |read, uses| Parsed {
            read,
            consumed: 1,
            uses,
        };
        if let Some(array) = op_array(statement) {
            return Some(parsed(Read::Term(self.op_term(array, loc, label)?), 0));
        }
        if let Some(target) = self.label_set(statement) {
            return Some(parsed(Read::LabelSet(target), 1));
        }
        if let Some(region) = self.trys_push(statement, label) {
            return Some(parsed(Read::Try(region), 1));
        }
        Some(parsed(Read::Other, 0))
    }
}

pub fn cases<'k, 'a>(
    statements: &'k [Statement<'a>],
    sent: &Sent<'_>,
) -> Option<(Vec<&'k [Statement<'a>]>, usize)> {
    match statements {
        [Statement::SwitchStatement(switch)] if sent.member(&switch.discriminant, "label") => {
            let mut cases = Vec::with_capacity(switch.cases.len());
            for (index, case) in switch.cases.iter().enumerate() {
                let label = case.test.as_ref().and_then(numeric)?;
                if label as usize != index {
                    return None;
                }
                cases.push(case.consequent.as_slice());
            }
            Some((cases, 1))
        }
        _ => Some((vec![statements], 0)),
    }
}

pub struct Loaded<'k, 'a> {
    pub machine: Machine,
    pub cases: Vec<&'k [Statement<'a>]>,
    pub labels: Vec<&'a str>,
}

pub fn read<'k, 'a>(statements: &'k [Statement<'a>], tslib: &Tslib<'_>) -> Option<Loaded<'k, 'a>> {
    let (cases, uses) = cases(statements, &tslib.sent)?;
    let out = machine_read::read(&cases, tslib, uses)?;
    let total = tslib
        .sent
        .scoping
        .get_resolved_references(tslib.sent.state)
        .count();
    (out.uses == total).then_some(Loaded {
        machine: out.machine,
        cases,
        labels: out.labels,
    })
}

pub struct SentReads {
    pub references: Vec<ReferenceId>,
}

impl SentReads {
    pub fn of(scoping: &Scoping, state: SymbolId) -> Self {
        Self {
            references: scoping.get_resolved_reference_ids(state).to_vec(),
        }
    }
}

impl Reads for SentReads {
    fn is_read(&self, expression: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expression else {
            return false;
        };
        if !call.arguments.is_empty() || call.optional {
            return false;
        }
        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(object) = member.object.without_parentheses() else {
            return false;
        };
        member.property.name.as_str() == "sent"
            && object
                .reference_id
                .get()
                .is_some_and(|reference| self.references.contains(&reference))
    }
}
