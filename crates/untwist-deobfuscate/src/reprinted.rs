use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};
use untwist_eval::decode::{DecoderProgram, FnRef};

use crate::array_rotator::find_rotator;
use crate::stamp::{find_stamped, StampedFn};

#[derive(Default)]
struct Order {
    parents: Vec<Span>,
    cursor: u32,
    scrambled: bool,
}

impl Order {
    fn enter(&mut self, span: Span) {
        let inside = self
            .parents
            .last()
            .is_none_or(|parent| parent.start <= span.start && span.end <= parent.end);
        if !inside || span.start < self.cursor || span.start > span.end {
            self.scrambled = true;
        }
        self.cursor = span.start;
        self.parents.push(span);
    }

    fn leave(&mut self, span: Span) {
        self.parents.pop();
        if self.cursor > span.end {
            self.scrambled = true;
        }
        self.cursor = span.end;
    }
}

impl<'a> Visit<'a> for Order {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.scrambled {
            return;
        }
        self.enter(it.span());
        walk::walk_expression(self, it);
        self.leave(it.span());
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.scrambled {
            return;
        }
        self.enter(it.span());
        walk::walk_statement(self, it);
        self.leave(it.span());
    }
}

pub(crate) fn is_scrambled(program: &DecoderProgram<'_>) -> bool {
    let mut order = Order::default();
    for function in program.functions() {
        order.parents.clear();
        order.cursor = 0;
        order.visit_function(function, ScopeFlags::empty());
    }
    for expression in program.expressions() {
        order.parents.clear();
        order.cursor = 0;
        order.visit_expression(expression);
    }
    order.scrambled
}

pub struct Reprinted {
    alloc: Allocator,
    text: String,
}

impl Reprinted {
    pub fn of(main: &Program<'_>) -> Self {
        Self {
            alloc: Allocator::default(),
            text: print(main, PrintStyle::Pretty),
        }
    }

    pub fn program<'b>(&'b self, original: &DecoderProgram<'_>) -> Option<DecoderProgram<'b>> {
        let parsed = parse_program(&self.alloc, &self.text);
        if !parsed.diagnostics.is_empty() {
            return None;
        }
        let program: &'b Program<'b> = self.alloc.alloc(parsed.program);
        let stamped = find_stamped(program);
        let found = |machinery: &FnRef<'_>, among: &[StampedFn<'b>]| {
            among
                .iter()
                .find(|stamped| stamped.name == machinery.name)
                .map(|stamped| FnRef {
                    name: stamped.name,
                    function: stamped.function,
                    strict: machinery.strict,
                })
        };
        let string_array = found(&original.string_array, &stamped.arrays)?;
        let decoders = original
            .decoders
            .iter()
            .map(|decoder| found(decoder, &stamped.decoders))
            .collect::<Option<Vec<_>>>()?;
        let rotator = match original.rotator {
            Some(_) => Some(find_rotator(program, string_array.name)?.1),
            None => None,
        };
        Some(DecoderProgram {
            source_text: &self.text,
            string_array,
            decoders,
            rotator,
            strict: original.strict,
        })
    }
}
