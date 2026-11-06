pub mod agnostic;
mod global_bindings;
mod normalize;
mod sequenced;
mod variables;

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::FxHashMap;
use untwist_core::ast_util::matcher::is_literal_argument;
use untwist_core::ast_util::prop::identifier_name;

use crate::proof::{
    record_definer_patch, record_identifier_patch, record_member_patch, PatchSite, RotatorSite,
};
use crate::stamp::{decoder_index, may_name_machinery, string_array_unit};
use crate::units::MachineryDepth;

pub use normalize::normalize_calls;
pub use untwist_eval::decode::Call;
pub(crate) use variables::constant_initialisers;
#[cfg(all(debug_assertions, feature = "test-hooks"))]
pub(crate) use variables::{fused_and_separate_initialisers, fused_and_separate_variables};

fn decoder_call<'k, 'a>(expr: &'k Expression<'a>) -> Option<(usize, &'k CallExpression<'a>)> {
    let call = match expr {
        Expression::CallExpression(call) => call,
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => call,
            _ => return None,
        },
        _ => return None,
    };
    let index = decoder_index(identifier_name(&call.callee)?)?;
    Some((index, call))
}

fn is_decode_argument(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void => {
            matches!(unary.argument, Expression::NumericLiteral(_))
        }
        other => is_literal_argument(other),
    }
}

fn is_splittable_branch(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ConditionalExpression(conditional) => {
            is_splittable_branch(&conditional.consequent)
                && is_splittable_branch(&conditional.alternate)
        }
        other => is_literal_argument(other),
    }
}

struct Collector<'p> {
    units: MachineryDepth,
    found: Vec<Call<'p>>,
    escapes: Vec<(usize, Span)>,
    decoder_values: Vec<(usize, Span)>,
    dynamic_calls: Vec<(usize, Span)>,
    patches: Vec<PatchSite<'p>>,
}

impl<'p> Collector<'p> {
    fn note_reference(&mut self, it: &IdentifierReference<'_>) {
        let name = it.name.as_str();
        if !may_name_machinery(name) || self.units.exempts(name) {
            return;
        }
        if let Some(unit) = string_array_unit(name) {
            self.escapes.push((unit, it.span));
        } else if let Some(index) = decoder_index(name) {
            self.decoder_values.push((index, it.span));
        }
    }

    fn walk_inside_machinery(&mut self, it: &Expression<'p>) {
        let foreign = decoder_call(it).filter(|(_, call)| {
            identifier_name(&call.callee).is_some_and(|name| !self.units.exempts(name))
        });
        let Some((index, call)) = foreign else {
            walk_js::walk_expression(self, it);
            return;
        };
        self.dynamic_calls.push((index, call.span));
        for argument in &call.arguments {
            self.visit_argument(argument);
        }
    }

    fn note_decoder_call(&mut self, it: &Expression<'p>, index: usize, call: &CallExpression<'p>) {
        let literal = !call.arguments.is_empty()
            && call
                .arguments
                .iter()
                .all(|a| a.as_expression().is_some_and(is_decode_argument));
        if !literal {
            self.dynamic_calls.push((index, call.span));
            for argument in &call.arguments {
                self.visit_argument(argument);
            }
            return;
        }
        let expression: &'p Expression<'p> = self.alloc(it);
        if let Some((_, call)) = decoder_call(expression) {
            self.found.push(Call {
                decoder: index,
                span: expression.span(),
                arguments: call
                    .arguments
                    .iter()
                    .filter_map(Argument::as_expression)
                    .collect(),
            });
        }
    }
}

impl<'p> VisitJs<'p> for Collector<'p> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        let entered = self.units.enter(it);
        walk_js::walk_statement(self, it);
        self.units.leave(entered);
    }

    fn visit_expression(&mut self, it: &Expression<'p>) {
        if let Expression::Identifier(identifier) = it {
            self.note_reference(identifier);
            return;
        }
        if self.units.inside() {
            self.walk_inside_machinery(it);
            return;
        }
        if let Some((index, call)) = decoder_call(it) {
            self.note_decoder_call(it, index, call);
            return;
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'p>) {
        self.note_reference(it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'p>) {
        match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.patches.extend(record_identifier_patch(id));
            }
            other => {
                if let Some(site) = other.as_member_expression().and_then(record_member_patch) {
                    self.patches.push(site);
                }
            }
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'p>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(site) = it
                .argument
                .without_parentheses()
                .as_member_expression()
                .and_then(record_member_patch)
            {
                self.patches.push(site);
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'p>) {
        self.patches.extend(record_definer_patch(it));
        walk_js::walk_call_expression(self, it);
    }
}

pub struct Collected<'p> {
    pub calls: Vec<Call<'p>>,
    pub escapes: Vec<(usize, Span)>,
    pub decoder_values: Vec<(usize, Span)>,
    pub dynamic_calls: Vec<(usize, Span)>,
    pub patches: Vec<PatchSite<'p>>,
    pub rotator_sites: Vec<(usize, RotatorSite)>,
}

pub fn collect_base<'p>(
    program: &'p Program<'p>,
    decoder_units: FxHashMap<usize, usize>,
) -> Collected<'p> {
    let mut collector = Collector {
        units: MachineryDepth::owned_by(decoder_units),
        found: Vec::new(),
        escapes: Vec::new(),
        decoder_values: Vec::new(),
        dynamic_calls: Vec::new(),
        patches: Vec::new(),
    };
    collector.visit_program(program);
    let mut patches = std::mem::take(&mut collector.patches);
    patches.shrink_to_fit();
    Collected {
        rotator_sites: collector.units.take_rotator_sites(),
        calls: collector.found,
        escapes: collector.escapes,
        decoder_values: collector.decoder_values,
        dynamic_calls: collector.dynamic_calls,
        patches,
    }
}

pub fn collect_calls<'p>(program: &'p Program<'p>) -> Vec<Call<'p>> {
    collect_base(program, FxHashMap::default()).calls
}
