use oxc_allocator::Vec as AVec;
use oxc_ast::ast::{Argument, Expression, Function};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::inline::{inline_function_call, returned_expression};
use untwist_core::pass::Cx;

use crate::call_checks::is_plain_function;
use crate::probe::{expression_contains, holds_own_scope};
use crate::wrapper_args::{body_uses, BodyUses};
use crate::wrapper_calls::{binding_is_stable, body_is_self_contained, WrapperShape};

pub(crate) const CHAIN_DEPTH_CAP: usize = 16;

pub(crate) struct Candidate<'p> {
    pub(crate) shape: WrapperShape,
    pub(crate) function: &'p Function<'p>,
}

fn forwards_soundly(candidate: &Candidate<'_>, scoping: &Scoping) -> bool {
    let function = candidate.function;
    returned_expression(function)
        .is_some_and(|returned| !expression_contains(returned, holds_own_scope))
        && is_plain_function(function)
        && binding_is_stable(scoping, candidate.shape.symbol)
        && body_is_self_contained(function, scoping, candidate.shape.forwards_to)
}

pub(crate) fn promote_forwarders(
    candidates: Vec<Candidate<'_>>,
    found: &mut Vec<WrapperShape>,
    scoping: &Scoping,
) {
    if found.is_empty() || candidates.is_empty() {
        return;
    }
    let mut waiting: FxHashMap<SymbolId, Vec<Candidate<'_>>> = FxHashMap::default();
    for candidate in candidates {
        if let Some(target) = candidate.shape.forwards_to {
            waiting.entry(target).or_default().push(candidate);
        }
    }
    let mut next = 0;
    while let Some(known) = found.get(next).map(|shape| shape.symbol) {
        next += 1;
        for candidate in waiting.remove(&known).unwrap_or_default() {
            if forwards_soundly(&candidate, scoping) {
                found.push(candidate.shape);
            }
        }
    }
}

pub(crate) fn drop_broken_chains(found: &mut Vec<WrapperShape>) {
    let mut whole: FxHashSet<SymbolId> = FxHashSet::default();
    found.retain(|shape| {
        let reaches = shape
            .forwards_to
            .is_none_or(|target| whole.contains(&target));
        if reaches {
            whole.insert(shape.symbol);
        }
        reaches
    });
}

struct Link<'p> {
    function: &'p Function<'p>,
    uses: BodyUses,
    forwards_to: Option<SymbolId>,
}

#[derive(Default)]
pub(crate) struct Chain<'p> {
    links: FxHashMap<SymbolId, Link<'p>>,
}

pub(crate) type Fits<'f> = &'f dyn Fn(&BodyUses, &AVec<'_, Argument<'_>>, usize) -> bool;

impl<'p> Chain<'p> {
    pub(crate) fn add(&mut self, shape: &WrapperShape, function: &'p Function<'p>) {
        let Some(uses) = body_uses(function) else {
            return;
        };
        self.links.insert(
            shape.symbol,
            Link {
                function,
                uses,
                forwards_to: shape.forwards_to,
            },
        );
    }

    pub(crate) fn inline_through<'a>(
        &self,
        wrapper: SymbolId,
        arguments: &AVec<'_, Argument<'_>>,
        fits: Fits<'_>,
        cx: &Cx<'a, '_>,
    ) -> Option<Expression<'a>> {
        let mut link = self.links.get(&wrapper)?;
        if !fits(&link.uses, arguments, 0) {
            return None;
        }
        let mut inlined = inline_function_call(link.function, arguments, cx)?;
        let mut depth = 0;
        while let Some(target) = link.forwards_to {
            depth += 1;
            link = self.links.get(&target)?;
            let Expression::CallExpression(call) = &inlined else {
                return None;
            };
            if depth > CHAIN_DEPTH_CAP || call.optional || !fits(&link.uses, &call.arguments, depth)
            {
                return None;
            }
            inlined = inline_function_call(link.function, &call.arguments, cx)?;
        }
        Some(inlined)
    }
}
