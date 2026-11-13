use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::FxHashSet;

use super::plan::{initialised, quiet_value};

const DOCUMENT: &str = "document";
const CURRENT_SCRIPT: &str = "currentScript";

pub struct Free<'s> {
    pub scoping: &'s Scoping,
    pub outer: &'s FxHashSet<String>,
}

impl Free<'_> {
    pub(super) fn global(&self, id: &IdentifierReference<'_>) -> bool {
        let free = id
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).symbol_id().is_none());
        free && !self.outer.contains(id.name.as_str())
    }

    fn probe(&self, expr: &Expression<'_>, script: &mut bool) -> bool {
        match expr.without_parentheses() {
            Expression::Identifier(id) => self.global(id),
            Expression::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::Typeof => {
                    matches!(&unary.argument, Expression::Identifier(id) if self.global(id))
                }
                UnaryOperator::LogicalNot | UnaryOperator::Void => {
                    self.probe(&unary.argument, script)
                }
                _ => quiet_value(expr, self),
            },
            Expression::BinaryExpression(binary) => {
                let equality = matches!(
                    binary.operator,
                    BinaryOperator::Equality
                        | BinaryOperator::Inequality
                        | BinaryOperator::StrictEquality
                        | BinaryOperator::StrictInequality
                );
                equality && self.probe(&binary.left, script) && self.probe(&binary.right, script)
            }
            Expression::LogicalExpression(logical) => {
                self.probe(&logical.left, script) && self.probe(&logical.right, script)
            }
            Expression::ConditionalExpression(conditional) => {
                self.probe(&conditional.test, script)
                    && self.probe(&conditional.consequent, script)
                    && self.probe(&conditional.alternate, script)
            }
            Expression::StaticMemberExpression(member) => {
                let current = member.property.name == CURRENT_SCRIPT
                    && matches!(&member.object, Expression::Identifier(id) if id.name == DOCUMENT && self.global(id));
                *script |= current;
                current
            }
            other => quiet_value(other, self),
        }
    }

    pub fn probe_var(&self, item: &Statement<'_>) -> Option<bool> {
        let Statement::VariableDeclaration(declaration) = item else {
            return None;
        };
        if declaration.kind != VariableDeclarationKind::Var {
            return None;
        }
        let mut script = false;
        for declarator in &declaration.declarations {
            declarator.id.get_binding_identifier()?;
            if let Some(init) = &declarator.init {
                self.probe(init, &mut script).then_some(())?;
            }
        }
        Some(script)
    }
}

pub struct Absorb<'f> {
    pub refs: &'f [Vec<u32>],
    pub eligible: Vec<bool>,
}

pub fn prefix_end(items: &[Statement<'_>], free: &Free<'_>) -> usize {
    items
        .iter()
        .position(|item| initialised(item, free).is_none() && free.probe_var(item).is_none())
        .unwrap_or(items.len())
}

impl Absorb<'_> {
    pub fn closure(&self, part_of: &[Option<usize>]) -> Vec<u32> {
        let n = self.refs.len();
        let mut held = vec![false; n];
        let mut stack: Vec<usize> = (0..n).filter(|&item| part_of[item].is_some()).collect();
        while let Some(at) = stack.pop() {
            for &target in &self.refs[at] {
                let target = target as usize;
                if part_of[target].is_none() && !held[target] && self.eligible[target] {
                    held[target] = true;
                    stack.push(target);
                }
            }
        }
        loop {
            let mut changed = false;
            for item in 0..n {
                if !held[item] {
                    continue;
                }
                let stranded = self.refs[item]
                    .iter()
                    .any(|&target| part_of[target as usize].is_none() && !held[target as usize]);
                if stranded {
                    held[item] = false;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        (0..n as u32).filter(|&item| held[item as usize]).collect()
    }
}
