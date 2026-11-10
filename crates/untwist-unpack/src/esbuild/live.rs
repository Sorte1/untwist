use oxc_allocator::{Allocator, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::esbuild::mentions::{fresh_name, mentioned};

const RUNTIME: &str = "runtime";

struct Reads<'s> {
    scoping: &'s Scoping,
    live: &'s FxHashSet<SymbolId>,
    moved: bool,
    fitting: FxHashSet<ReferenceId>,
    deleted: bool,
}

impl Reads<'_> {
    fn live(&self, id: &IdentifierReference<'_>) -> Option<ReferenceId> {
        let reference = id.reference_id.get()?;
        let symbol = self.scoping.get_reference(reference).symbol_id()?;
        self.live.contains(&symbol).then_some(reference)
    }
}

impl<'a> VisitJs<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.moved {
            self.fitting.extend(self.live(it));
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::Identifier(id) = it {
            self.fitting.extend(self.live(id));
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Expression::Identifier(id) = it.argument.without_parentheses() {
                self.deleted |= self.live(id).is_some();
            }
        }
        walk_js::walk_unary_expression(self, it);
    }
}

fn written(scoping: &Scoping, helpers: &FxHashSet<SymbolId>) -> FxHashSet<SymbolId> {
    helpers
        .iter()
        .copied()
        .filter(|&symbol| {
            scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
        })
        .collect()
}

pub fn read_as_members(
    scoping: &Scoping,
    list: &[Statement<'_>],
    moved: &[usize],
    helpers: &FxHashSet<SymbolId>,
) -> bool {
    let live = written(scoping, helpers);
    if live.is_empty() {
        return true;
    }
    let mut reads = Reads {
        scoping,
        live: &live,
        moved: false,
        fitting: FxHashSet::default(),
        deleted: false,
    };
    for (index, statement) in list.iter().enumerate() {
        reads.moved = moved.binary_search(&index).is_ok();
        reads.visit_statement(statement);
    }
    !reads.deleted
        && live.iter().all(|&symbol| {
            scoping
                .get_resolved_reference_ids(symbol)
                .iter()
                .all(|reference| reads.fitting.contains(reference))
        })
}

struct Live<'a, 'r> {
    builder: AstBuilder<'a>,
    scoping: Option<&'r Scoping>,
    names: &'r FxHashMap<String, usize>,
    binding: &'a str,
}

impl<'a> Live<'a, '_> {
    fn is_free(&self, id: &IdentifierReference<'_>) -> bool {
        let Some(scoping) = self.scoping else {
            return true;
        };
        id.reference_id
            .get()
            .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
    }

    fn reads(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Identifier(id)
            if self.names.contains_key(id.name.as_str()) && self.is_free(id))
    }

    fn member(&self, span: Span, name: &'a str) -> Expression<'a> {
        Expression::new_static_member_expression(
            span,
            Expression::new_identifier(span, self.binding, &self.builder),
            IdentifierName::new(span, name, &self.builder),
            false,
            &self.builder,
        )
    }

    fn unbound(&self, callee: &mut Expression<'a>) {
        if !self.reads(callee.without_parentheses()) {
            return;
        }
        let span = oxc_span::GetSpan::span(callee);
        let zero =
            Expression::new_numeric_literal(span, 0.0, None, NumberBase::Decimal, &self.builder);
        let mut list = AVec::with_capacity_in(2, &self.builder);
        list.push(zero);
        list.push(callee.take_in(&self.builder));
        *callee = Expression::new_sequence_expression(span, list, &self.builder);
    }
}

impl<'a> VisitMut<'a> for Live<'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if !self.reads(it) {
            walk_mut::walk_expression(self, it);
            return;
        }
        if let Expression::Identifier(id) = it {
            let (span, name) = (id.span, id.name.as_str());
            *it = self.member(span, name);
        }
    }

    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        self.unbound(&mut it.callee);
        walk_mut::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &mut TaggedTemplateExpression<'a>) {
        self.unbound(&mut it.tag);
        walk_mut::walk_tagged_template_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &mut ObjectProperty<'a>) {
        if it.shorthand && self.reads(&it.value) {
            it.shorthand = false;
        }
        walk_mut::walk_object_property(self, it);
    }
}

fn reads_any(
    program: &Program<'_>,
    names: &FxHashMap<String, usize>,
    scoping: Option<&Scoping>,
) -> bool {
    match scoping {
        Some(scoping) => {
            let free = scoping.root_unresolved_references();
            names.keys().any(|name| free.contains_key(name.as_str()))
        }
        None => !mentioned(names, program.body.iter()).is_empty(),
    }
}

pub fn read_through_runtime<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    names: &FxHashMap<String, usize>,
    scoping: Option<&Scoping>,
) -> Option<String> {
    if !reads_any(program, names, scoping) {
        return None;
    }
    let binding = fresh_name(program, RUNTIME);
    let mut live = Live {
        builder: AstBuilder::new(alloc),
        scoping,
        names,
        binding: alloc.alloc_str(&binding),
    };
    live.visit_program(program);
    Some(binding)
}
