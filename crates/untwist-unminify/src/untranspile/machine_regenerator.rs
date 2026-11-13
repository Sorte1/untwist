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

const RETURN_OP: u32 = 2;

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
            && member.property.name.as_str() == "n"
            && self.is_state(&member.object);
        set.then(|| numeric(&assignment.right)).flatten()
    }

    fn finished(&self, statement: &Statement<'_>) -> Option<bool> {
        let Statement::ReturnStatement(it) = statement else {
            return None;
        };
        let Expression::CallExpression(call) = it.argument.as_ref()?.without_parentheses() else {
            return None;
        };
        if call.optional || !self.member(&call.callee, "a") {
            return None;
        }
        let op = numeric(call.arguments.first()?.as_expression()?)?;
        if op != RETURN_OP {
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
        self.member(expression, "v")
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

pub struct Regenerator<'s> {
    pub context: Context<'s>,
    pub home: Option<ScopeId>,
}

impl<'a> Machinery<'a> for Regenerator<'_> {
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

pub fn switch_of<'k, 'a>(looped: &'k WhileStatement<'a>) -> Option<&'k SwitchStatement<'a>> {
    match &looped.body {
        Statement::SwitchStatement(switch) => Some(switch),
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [Statement::SwitchStatement(switch)] => Some(switch),
            _ => None,
        },
        _ => None,
    }
}

pub fn cases<'k, 'a>(
    statements: &'k [Statement<'a>],
    context: &Context<'_>,
) -> Option<Vec<&'k [Statement<'a>]>> {
    let [Statement::WhileStatement(looped)] = statements else {
        return None;
    };
    let forever = numeric(&looped.test) == Some(1)
        || matches!(looped.test.without_parentheses(), Expression::BooleanLiteral(it) if it.value);
    if !forever {
        return None;
    }
    let switch = switch_of(looped)?;
    if !context.member(&switch.discriminant, "n") {
        return None;
    }
    let mut cases = Vec::with_capacity(switch.cases.len());
    for (index, case) in switch.cases.iter().enumerate() {
        let label = case.test.as_ref().and_then(numeric)?;
        if label as usize != index {
            return None;
        }
        cases.push(case.consequent.as_slice());
    }
    Some(cases)
}

pub struct Loaded<'a> {
    pub machine: Machine,
    pub labels: Vec<&'a str>,
}

pub fn read<'k, 'a>(
    statements: &'k [Statement<'a>],
    regenerator: &Regenerator<'_>,
) -> Option<Loaded<'a>> {
    let cases = cases(statements, &regenerator.context)?;
    let last_returns = cases
        .last()
        .and_then(|case| case.last())
        .is_some_and(|statement| regenerator.context.finished(statement).is_some());
    if !last_returns {
        return None;
    }
    let out = machine_read::read(&cases, regenerator, 1)?;
    let total = regenerator
        .context
        .scoping
        .get_resolved_references(regenerator.context.state)
        .count();
    (out.uses == total).then_some(Loaded {
        machine: out.machine,
        labels: out.labels,
    })
}

pub struct ValueReads {
    pub references: Vec<ReferenceId>,
}

impl ValueReads {
    pub fn of(scoping: &Scoping, state: SymbolId) -> Self {
        Self {
            references: scoping.get_resolved_reference_ids(state).to_vec(),
        }
    }
}

impl Reads for ValueReads {
    fn is_read(&self, expression: &Expression<'_>) -> bool {
        let Expression::StaticMemberExpression(member) = expression else {
            return false;
        };
        let Expression::Identifier(object) = member.object.without_parentheses() else {
            return false;
        };
        !member.optional
            && member.property.name.as_str() == "v"
            && object
                .reference_id
                .get()
                .is_some_and(|reference| self.references.contains(&reference))
    }
}
