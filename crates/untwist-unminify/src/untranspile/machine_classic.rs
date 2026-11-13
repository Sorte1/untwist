use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;

use super::identify::resolved;
use super::machine::{Loc, Machine, Part, Slot, Target, Term};
use super::machine_check::Reads;
use super::machine_read::{self, Machinery, Parsed, Read};
use super::machine_regenerator::switch_of;

const END: &str = "end";

pub struct Context<'s> {
    pub scoping: &'s Scoping,
    pub state: SymbolId,
}

impl Context<'_> {
    fn is_state(&self, expression: &Expression<'_>) -> bool {
        resolved(expression, self.scoping) == Some(self.state)
    }

    fn member(&self, expression: &Expression<'_>, name: &str) -> bool {
        matches!(expression.without_parentheses(), Expression::StaticMemberExpression(member)
            if !member.optional && member.property.name.as_str() == name && self.is_state(&member.object))
    }

    fn next_set(&self, statement: &Statement<'_>) -> Option<u32> {
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
            && member.property.name.as_str() == "next"
            && self.is_state(&member.object);
        set.then(|| numeric(&assignment.right)).flatten()
    }

    fn returned_call<'k, 'a>(
        &self,
        statement: &'k Statement<'a>,
        method: &str,
    ) -> Option<&'k CallExpression<'a>> {
        let Statement::ReturnStatement(it) = statement else {
            return None;
        };
        let Expression::CallExpression(call) = it.argument.as_ref()?.without_parentheses() else {
            return None;
        };
        (!call.optional && self.member(&call.callee, method)).then_some(call)
    }

    fn finished(&self, statement: &Statement<'_>) -> Option<bool> {
        if let Some(call) = self.returned_call(statement, "stop") {
            return call.arguments.is_empty().then_some(false);
        }
        let call = self.returned_call(statement, "abrupt")?;
        let Some(Argument::StringLiteral(kind)) = call.arguments.first() else {
            return None;
        };
        if kind.value.as_str() != Some("return") {
            return None;
        }
        match call.arguments.as_slice() {
            [_] => Some(false),
            [_, value] if value.is_expression() => Some(true),
            _ => None,
        }
    }
}

impl Reads for Context<'_> {
    fn is_read(&self, expression: &Expression<'_>) -> bool {
        self.member(expression, "sent")
    }
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

pub struct Classic<'s> {
    pub context: Context<'s>,
    pub home: Option<ScopeId>,
    pub labels: Vec<u32>,
}

impl Classic<'_> {
    fn index(&self, label: u32) -> Option<u32> {
        let at = self.labels.iter().position(|known| *known == label)?;
        u32::try_from(at).ok()
    }
}

impl<'a> Machinery<'a> for Classic<'_> {
    fn reads(&self) -> &dyn Reads {
        &self.context
    }

    fn scoping(&self) -> &Scoping {
        self.context.scoping
    }

    fn home(&self) -> Option<ScopeId> {
        self.home
    }

    fn falls_through(&self) -> bool {
        true
    }

    fn needs_received(&self) -> bool {
        false
    }

    fn statement<'k>(
        &self,
        statement: &'k Statement<'a>,
        next: Option<&'k Statement<'a>>,
        loc: Loc,
        next_loc: Option<Loc>,
        _label: u32,
    ) -> Option<Parsed> {
        let parsed = |read, consumed, uses| Parsed {
            read,
            consumed,
            uses,
        };
        if let Some(valued) = self.context.finished(statement) {
            let value = valued.then_some(Slot {
                at: loc,
                part: Part::ReturnedArgument(1),
            });
            return Some(parsed(Read::Term(Term::Return(value)), 1, 1));
        }
        let Some(target) = self.context.next_set(statement) else {
            return Some(parsed(Read::Other, 1, 0));
        };
        let target = self.index(target)?;
        match next? {
            Statement::BreakStatement(it) if it.label.is_none() => {
                Some(parsed(Read::Term(Term::Goto(Target::Label(target))), 2, 1))
            }
            Statement::ReturnStatement(it) => {
                let at = next_loc?;
                let value = it.argument.as_ref().map(|_| Slot {
                    at,
                    part: Part::Returned,
                });
                Some(parsed(
                    Read::Term(Term::Yield {
                        value,
                        delegate: false,
                        resume: target,
                    }),
                    2,
                    1,
                ))
            }
            _ => None,
        }
    }
}

fn discriminant_uses(discriminant: &Expression<'_>, context: &Context<'_>) -> Option<usize> {
    if context.member(discriminant, "next") {
        return Some(1);
    }
    let Expression::AssignmentExpression(assignment) = discriminant.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
        return None;
    };
    let tracked = assignment.operator == AssignmentOperator::Assign
        && member.property.name.as_str() == "prev"
        && context.is_state(&member.object)
        && context.member(&assignment.right, "next");
    tracked.then_some(2)
}

pub struct Cases<'k, 'a> {
    pub cases: Vec<&'k [Statement<'a>]>,
    pub labels: Vec<u32>,
    pub uses: usize,
}

pub fn cases<'k, 'a>(
    statements: &'k [Statement<'a>],
    context: &Context<'_>,
) -> Option<Cases<'k, 'a>> {
    let [Statement::WhileStatement(looped)] = statements else {
        return None;
    };
    let forever = numeric(&looped.test) == Some(1)
        || matches!(looped.test.without_parentheses(), Expression::BooleanLiteral(it) if it.value);
    if !forever {
        return None;
    }
    let switch = switch_of(looped)?;
    let uses = discriminant_uses(&switch.discriminant, context)?;
    let (last, numbered) = switch.cases.split_last()?;
    let ends = matches!(last.test.as_ref().map(Expression::without_parentheses),
        Some(Expression::StringLiteral(it)) if it.value.as_str() == Some(END));
    if !ends {
        return None;
    }
    let mut labels = Vec::with_capacity(switch.cases.len());
    let mut cases = Vec::with_capacity(switch.cases.len());
    for case in numbered {
        let label = case.test.as_ref().and_then(numeric)?;
        if labels.last().is_some_and(|previous| *previous >= label) {
            return None;
        }
        labels.push(label);
        cases.push(case.consequent.as_slice());
    }
    labels.push(u32::MAX);
    cases.push(last.consequent.as_slice());
    Some(Cases {
        cases,
        labels,
        uses,
    })
}

pub fn read<'a>(
    statements: &[Statement<'a>],
    classic: &Classic<'_>,
) -> Option<(Machine, Vec<&'a str>)> {
    let found = cases(statements, &classic.context)?;
    if found.labels != classic.labels {
        return None;
    }
    let ends = found
        .cases
        .last()
        .and_then(|case| case.last())
        .is_some_and(|statement| classic.context.finished(statement).is_some());
    if !ends {
        return None;
    }
    let out = machine_read::read(&found.cases, classic, found.uses)?;
    let total = classic
        .context
        .scoping
        .get_resolved_references(classic.context.state)
        .count();
    (out.uses == total).then_some((out.machine, out.labels))
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
        let Expression::StaticMemberExpression(member) = expression else {
            return false;
        };
        let Expression::Identifier(object) = member.object.without_parentheses() else {
            return false;
        };
        !member.optional
            && member.property.name.as_str() == "sent"
            && object
                .reference_id
                .get()
                .is_some_and(|reference| self.references.contains(&reference))
    }
}
