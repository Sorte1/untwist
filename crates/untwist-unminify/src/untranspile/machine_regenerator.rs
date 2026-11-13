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

    fn prev_set(&self, statement: &Statement<'_>) -> Option<u32> {
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
            && member.property.name.as_str() == "p"
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
    pub regions: Vec<TryRegion>,
}

impl Regenerator<'_> {
    fn location(&self, statement: &Statement<'_>, label: u32) -> Option<Read> {
        let set = self.context.prev_set(statement)?;
        if set != label {
            return None;
        }
        if let Some(region) = self.regions.iter().find(|region| region.start == label) {
            return Some(Read::Try(*region));
        }
        self.regions
            .iter()
            .any(|region| region.catch == Some(label) || region.finally == Some(label))
            .then_some(Read::Skip)
    }

    fn finally_end(&self, statement: &Statement<'_>, label: u32) -> Option<Term> {
        let Statement::ReturnStatement(it) = statement else {
            return None;
        };
        let Expression::CallExpression(call) = it.argument.as_ref()?.without_parentheses() else {
            return None;
        };
        if call.optional || !self.context.member(&call.callee, "f") {
            return None;
        }
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        let finally = numeric(argument.as_expression()?)?;
        let inside = self.regions.iter().any(|region| {
            region.finally == Some(finally) && finally <= label && label < region.end
        });
        inside.then_some(Term::EndFinally)
    }
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
        label: u32,
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
        if self.context.prev_set(statement).is_some() {
            return Some(parsed(self.location(statement, label)?, 1, 1));
        }
        if let Some(term) = self.finally_end(statement, label) {
            return Some(parsed(Read::Term(term), 1, 1));
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

fn discriminant_uses(discriminant: &Expression<'_>, context: &Context<'_>) -> Option<usize> {
    if context.member(discriminant, "n") {
        return Some(1);
    }
    let Expression::AssignmentExpression(assignment) = discriminant.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(member) = &assignment.left else {
        return None;
    };
    let tracked = assignment.operator == AssignmentOperator::Assign
        && member.property.name.as_str() == "p"
        && context.is_state(&member.object)
        && context.member(&assignment.right, "n");
    tracked.then_some(2)
}

pub fn cases<'k, 'a>(
    statements: &'k [Statement<'a>],
    context: &Context<'_>,
) -> Option<(Vec<&'k [Statement<'a>]>, usize)> {
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
    let mut cases = Vec::with_capacity(switch.cases.len());
    for (index, case) in switch.cases.iter().enumerate() {
        let label = case.test.as_ref().and_then(numeric)?;
        if label as usize != index {
            return None;
        }
        cases.push(case.consequent.as_slice());
    }
    Some((cases, uses))
}

pub struct Loaded<'a> {
    pub machine: Machine,
    pub labels: Vec<&'a str>,
}

pub fn read<'k, 'a>(
    statements: &'k [Statement<'a>],
    regenerator: &Regenerator<'_>,
) -> Option<Loaded<'a>> {
    let (cases, uses) = cases(statements, &regenerator.context)?;
    if uses == 1 && !regenerator.regions.is_empty() {
        return None;
    }
    let last_returns = cases
        .last()
        .and_then(|case| case.last())
        .is_some_and(|statement| regenerator.context.finished(statement).is_some());
    if !last_returns {
        return None;
    }
    let out = machine_read::read(&cases, regenerator, uses)?;
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

fn targets_in(statements: &[Statement<'_>], context: &Context<'_>, out: &mut Vec<u32>) {
    for statement in statements {
        if let Some(target) = context.next_set(statement) {
            out.push(target);
        }
        match statement {
            Statement::BlockStatement(block) => targets_in(&block.body, context, out),
            Statement::IfStatement(it) => {
                targets_in(std::slice::from_ref(&it.consequent), context, out);
                if let Some(alternate) = &it.alternate {
                    targets_in(std::slice::from_ref(alternate), context, out);
                }
            }
            _ => {}
        }
    }
}

fn targets(
    cases: &[&[Statement<'_>]],
    (from, to): (u32, u32),
    context: &Context<'_>,
) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for case in cases.get(from as usize..to as usize)? {
        targets_in(case, context, &mut out);
    }
    Some(out)
}

fn after_catch(
    cases: &[&[Statement<'_>]],
    (start, catch): (u32, u32),
    context: &Context<'_>,
) -> Option<u32> {
    let mut leaving: Vec<u32> = targets(cases, (start, catch), context)?
        .into_iter()
        .filter(|target| *target >= catch)
        .collect();
    leaving.sort_unstable();
    leaving.dedup();
    let [end] = leaving.as_slice() else {
        return None;
    };
    let end = *end;
    let closed = end > catch
        && targets(cases, (catch, end), context)?
            .iter()
            .all(|target| (catch..=end).contains(target));
    closed.then_some(end)
}

fn location(element: Option<&ArrayExpressionElement<'_>>) -> Option<Option<u32>> {
    match element {
        Some(ArrayExpressionElement::Elision(_)) | None => Some(None),
        Some(element) => element.as_expression().and_then(numeric).map(Some),
    }
}

pub fn regions(
    list: Option<&Expression<'_>>,
    statements: &[Statement<'_>],
    context: &Context<'_>,
) -> Option<Vec<TryRegion>> {
    let Some(list) = list else {
        return Some(Vec::new());
    };
    let Expression::ArrayExpression(list) = list.without_parentheses() else {
        return None;
    };
    let (cases, _) = cases(statements, context)?;
    let mut out = Vec::with_capacity(list.elements.len());
    for entry in &list.elements {
        let Some(Expression::ArrayExpression(locs)) = entry.as_expression() else {
            return None;
        };
        if locs.elements.len() > 4 {
            return None;
        }
        let start = location(locs.elements.first())??;
        let catch = location(locs.elements.get(1))?;
        let finally = location(locs.elements.get(2))?;
        let after = location(locs.elements.get(3))?;
        let end = match (catch, finally, after) {
            (_, Some(_), Some(end)) => end,
            (Some(catch), None, None) => after_catch(&cases, (start, catch), context)?,
            _ => return None,
        };
        out.push(TryRegion {
            start,
            catch,
            finally,
            end,
        });
    }
    Some(out)
}
