use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::rollup::scan::Facts;

const PROTO: &str = "__proto__";

pub struct PlainWrite {
    pub item: u32,
    pub owner: u32,
    pub symbol: SymbolId,
    pub at: u32,
    pub named: bool,
}

pub struct Writes {
    pub plain: Vec<PlainWrite>,
    pub fixed: Vec<(u32, u32)>,
}

struct Collect<'s> {
    scoping: &'s Scoping,
    facts: &'s Facts,
    item: u32,
    plain: Vec<PlainWrite>,
    fixed: Vec<(u32, u32)>,
    unsettable: FxHashSet<SymbolId>,
}

impl Collect<'_> {
    fn foreign(&self, reference: &IdentifierReference<'_>) -> Option<(SymbolId, u32)> {
        let id = reference.reference_id.get()?;
        let symbol = self.scoping.get_reference(id).symbol_id()?;
        let owner = *self.facts.owner.get(symbol.index())?;
        let declared = (owner as usize) < self.facts.declares.len();
        (declared && owner != self.item).then_some((symbol, owner))
    }
}

fn anonymous(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::FunctionExpression(function) => function.id.is_none(),
        Expression::ClassExpression(class) => class.id.is_none(),
        Expression::ArrowFunctionExpression(_) => true,
        _ => false,
    }
}

impl<'a> Visit<'a> for Collect<'_> {
    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
                if let Some((symbol, owner)) = self.foreign(target) {
                    self.plain.push(PlainWrite {
                        item: self.item,
                        owner,
                        symbol,
                        at: it.span.start,
                        named: anonymous(&it.right),
                    });
                    self.visit_expression(&it.right);
                    return;
                }
            }
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let write = it
            .reference_id
            .get()
            .is_some_and(|id| self.scoping.get_reference(id).is_write());
        if !write {
            return;
        }
        if let Some((symbol, owner)) = self.foreign(it) {
            self.fixed.push((self.item, owner));
            self.unsettable.insert(symbol);
        }
    }
}

pub fn writes(scoping: &Scoping, items: &[Statement<'_>], facts: &Facts) -> Writes {
    let mut collect = Collect {
        scoping,
        facts,
        item: 0,
        plain: Vec::new(),
        fixed: Vec::new(),
        unsettable: FxHashSet::default(),
    };
    for (index, item) in items.iter().enumerate() {
        collect.item = index as u32;
        collect.visit_statement(item);
    }
    let Collect {
        plain,
        mut fixed,
        unsettable,
        ..
    } = collect;
    let (plain, kept): (Vec<PlainWrite>, Vec<PlainWrite>) = plain.into_iter().partition(|write| {
        !unsettable.contains(&write.symbol) && scoping.symbol_name(write.symbol) != PROTO
    });
    fixed.extend(kept.iter().map(|write| (write.item, write.owner)));
    Writes { plain, fixed }
}
