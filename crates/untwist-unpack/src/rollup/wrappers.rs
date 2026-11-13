use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use super::scan::{Facts, ENTRY};

const REQUIRE: &str = "require";

pub struct Group {
    pub function: u32,
    pub members: Vec<u32>,
    pub holder: Option<u32>,
    pub stem: String,
}

fn symbol(scoping: &Scoping, expression: &Expression<'_>) -> Option<SymbolId> {
    let Expression::Identifier(id) = expression.without_parentheses() else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn holder(scoping: &Scoping, expression: &Expression<'_>) -> Option<(SymbolId, bool)> {
    match expression.without_parentheses() {
        Expression::StaticMemberExpression(member) if member.property.name == "exports" => {
            Some((symbol(scoping, &member.object)?, false))
        }
        other => Some((symbol(scoping, other)?, true)),
    }
}

fn returned<'k, 'a>(statement: &'k Statement<'a>) -> Option<&'k Expression<'a>> {
    match statement {
        Statement::ReturnStatement(statement) => statement.argument.as_ref(),
        Statement::BlockStatement(block) => match &block.body[..] {
            [only] => returned(only),
            _ => None,
        },
        _ => None,
    }
}

fn sets_one(scoping: &Scoping, statement: &Statement<'_>) -> Option<SymbolId> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &statement.expression else {
        return None;
    };
    let one = matches!(&assign.right, Expression::NumericLiteral(literal) if literal.value == 1.0);
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return None;
    };
    (one && assign.operator == AssignmentOperator::Assign)
        .then(|| scoping.get_reference(id.reference_id.get()?).symbol_id())
        .flatten()
}

fn guarded<'k, 'a>(
    statements: &'k [Statement<'a>],
    loose: bool,
) -> Option<(&'k IfStatement<'a>, &'k Statement<'a>)> {
    match statements {
        [Statement::IfStatement(guard), set, ..] if guard.alternate.is_none() => Some((guard, set)),
        [Statement::IfStatement(guard)] if loose => {
            let Some(Statement::BlockStatement(block)) = &guard.alternate else {
                return None;
            };
            Some((guard, block.body.first()?))
        }
        _ => None,
    }
}

fn shape(
    scoping: &Scoping,
    function: &Function<'_>,
    loose: bool,
) -> Option<(SymbolId, SymbolId, bool)> {
    if !function.params.items.is_empty() || function.params.rest.is_some() {
        return None;
    }
    let statements = &function.body.as_ref()?.statements;
    let (guard, set) = guarded(statements, loose)?;
    let flag = symbol(scoping, &guard.test)?;
    let (held, value) = holder(scoping, returned(&guard.consequent)?)?;
    (sets_one(scoping, set)? == flag && flag != held).then_some((flag, held, value))
}

fn stem(name: &str) -> Option<String> {
    let rest = name.strip_prefix(REQUIRE)?.trim_start_matches('_');
    let mut chars = rest.chars();
    let first = chars.next()?;
    let stem: String = first.to_lowercase().chain(chars).collect();
    stem.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
        .then_some(stem)
}

fn owner(facts: &Facts, symbol: SymbolId) -> Option<u32> {
    let owner = *facts.owner.get(symbol.index())?;
    (owner != ENTRY && (owner as usize) < facts.declares.len()).then_some(owner)
}

pub fn groups(scoping: &Scoping, items: &[Statement<'_>], facts: &Facts) -> Vec<Group> {
    groups_with(scoping, items, facts, false)
}

pub fn groups_with(
    scoping: &Scoping,
    items: &[Statement<'_>],
    facts: &Facts,
    loose: bool,
) -> Vec<Group> {
    let mut found = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Statement::FunctionDeclaration(function) = item else {
            continue;
        };
        let Some((flag, held, _)) = shape(scoping, function, loose) else {
            continue;
        };
        let Some(name) = function.id.as_ref().map(|id| id.name.as_str()) else {
            continue;
        };
        let Some(flag_item) = owner(facts, flag) else {
            continue;
        };
        let written: Vec<u32> = facts
            .writes
            .iter()
            .filter(|(writer, _)| *writer == index as u32)
            .map(|(_, owner)| *owner)
            .collect();
        if !written.contains(&flag_item) {
            continue;
        }
        let mut members = vec![index as u32];
        members.extend(written);
        members.sort_unstable();
        members.dedup();
        found.push(Group {
            function: index as u32,
            members,
            holder: owner(facts, held),
            stem: stem(name).unwrap_or_else(|| scoping.symbol_name(held).to_string()),
        });
    }
    found
}
