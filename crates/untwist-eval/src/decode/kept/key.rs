use oxc_ast::ast::{ArrowFunctionExpression, Expression, Function};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;
use sha2::{Digest, Sha256};
use untwist_core::ast_util::js_text::js_text;

use super::super::{Call, DecoderProgram};
use super::{arguments_keyed, MachineryDigest, Start, UnitKey};
use crate::rotation::{hash_decoders, hash_function, hash_rotator, hash_span, hash_text};
use crate::source::compact_expression;

struct FunctionSpans<'h> {
    hasher: &'h mut Sha256,
}

impl<'a> VisitJs<'a> for FunctionSpans<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        hash_span(self.hasher, it.span);
        walk_js::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        hash_span(self.hasher, it.span);
        walk_js::walk_arrow_function_expression(self, it);
    }
}

fn hash_machinery(hasher: &mut Sha256, program: &DecoderProgram<'_>) {
    hasher.update((program.source_text.len() as u64).to_le_bytes());
    hasher.update([u8::from(program.strict)]);
    hash_function(
        hasher,
        program.source_text,
        program.string_array.name,
        program.string_array.function,
    );
    hash_decoders(hasher, program);
    hash_rotator(hasher, program);
    let mut spans = FunctionSpans { hasher };
    for function in program.functions() {
        spans.visit_function(function, ScopeFlags::empty());
    }
    for expression in program.expressions() {
        spans.visit_expression(expression);
    }
}

fn hash_argument(hasher: &mut Sha256, argument: &Expression<'_>) {
    match argument {
        Expression::NumericLiteral(number) => {
            hasher.update([0u8]);
            hasher.update(number.value.to_bits().to_le_bytes());
        }
        Expression::StringLiteral(text) => {
            hasher.update([1u8, u8::from(text.value.has_lone_surrogate())]);
            hash_text(hasher, &js_text(text.value));
        }
        other => {
            hasher.update([2u8]);
            hash_text(hasher, &compact_expression(other));
            FunctionSpans { hasher }.visit_expression(other);
        }
    }
}

fn hash_calls(hasher: &mut Sha256, calls: &[Call<'_>]) {
    hasher.update((calls.len() as u64).to_le_bytes());
    if !arguments_keyed() {
        return;
    }
    for call in calls {
        hasher.update((call.decoder as u64).to_le_bytes());
        hasher.update((call.arguments.len() as u64).to_le_bytes());
        for argument in &call.arguments {
            hash_argument(hasher, argument);
        }
    }
}

fn hash_start(hasher: &mut Sha256, start: &Start) {
    hasher.update(start.steps_used.to_le_bytes());
    hasher.update(start.steps_left.to_le_bytes());
    let Some((over, limit, live)) = start.heap else {
        hasher.update([0u8]);
        return;
    };
    hasher.update([1u8, u8::from(over)]);
    hasher.update(limit.to_le_bytes());
    hasher.update(live.to_le_bytes());
}

fn hash_escaped(hasher: &mut Sha256, escaped_decoders: &FxHashSet<usize>) {
    let mut escaped: Vec<usize> = escaped_decoders.iter().copied().collect();
    escaped.sort_unstable();
    hasher.update((escaped.len() as u64).to_le_bytes());
    for index in escaped {
        hasher.update((index as u64).to_le_bytes());
    }
}

pub(super) fn machinery_digest(program: &DecoderProgram<'_>) -> MachineryDigest {
    let mut hasher = Sha256::new();
    hash_machinery(&mut hasher, program);
    hasher.finalize().into()
}

pub(super) fn unit_key(
    start: &Start,
    machinery: &MachineryDigest,
    calls: &[Call<'_>],
    escaped_decoders: &FxHashSet<usize>,
) -> UnitKey {
    let mut hasher = Sha256::new();
    hasher.update(machinery);
    hash_calls(&mut hasher, calls);
    hash_start(&mut hasher, start);
    hash_escaped(&mut hasher, escaped_decoders);
    hasher.update([
        super::super::recipe_mode_tag(),
        u8::from(crate::native_body::enabled()),
    ]);
    hasher.finalize().into()
}
