use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast::builder::GetAstBuilder;
use oxc_span::Span;
use oxc_str::Ident;
use untwist_core::ast_util::rename::Renames;
use untwist_core::pass::{Cx, NoteKind, RepeatedSites};

use crate::build;
use crate::notes;
use crate::uid::component_name;

pub(crate) trait Convert<'a> {
    type Plan;

    fn plan(&mut self, expr: &Expression<'a>, cx: &mut Cx<'a, '_>) -> Option<Self::Plan>;

    fn rewrite(&mut self, expr: &mut Expression<'a>, plan: Self::Plan, cx: &mut Cx<'a, '_>)
        -> bool;
}

pub(crate) fn convert<'a, C: Convert<'a>>(
    pass: &mut C,
    expr: &mut Expression<'a>,
    cx: &mut Cx<'a, '_>,
) {
    let Some(plan) = pass.plan(expr, cx) else {
        return;
    };
    if pass.rewrite(expr, plan, cx) {
        cx.changed();
    }
}

pub(crate) fn take_call<'a>(
    expr: &mut Expression<'a>,
    b: &impl GetAstBuilder<'a>,
) -> Option<(Span, AVec<'a, Argument<'a>>)> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    let span = call.span;
    let arguments = std::mem::replace(&mut call.arguments, AVec::new_in(b.builder()));
    Some((span, arguments))
}

pub(crate) fn plan_rename<'a>(
    conservative: bool,
    type_expr: &Expression<'a>,
    unbound: &mut RepeatedSites,
    exhausted: &mut RepeatedSites,
    cx: &mut Cx<'a, '_>,
) -> Option<Option<String>> {
    let Some(id) = build::lowercase_identifier(type_expr) else {
        return Some(None);
    };
    let symbol = id
        .reference_id
        .get()
        .and_then(|reference| cx.scoping().get_reference(reference).symbol_id());
    let Some(symbol) = symbol else {
        unbound.record(id.span, id.name.as_str());
        return None;
    };
    if let Some(name) = cx.planned_name(symbol) {
        return Some(Some(name.to_string()));
    }
    let Some(name) = component_name(conservative, cx) else {
        exhausted.record(id.span, id.name.as_str());
        return None;
    };
    cx.plan_rename(symbol, name.clone());
    Some(Some(name))
}

pub(crate) fn emit_notes(sites: &mut [(&mut RepeatedSites, &str, NoteKind)], cx: &mut Cx<'_, '_>) {
    for (site, message, kind) in sites.iter_mut() {
        let message: &str = message;
        std::mem::take(&mut **site).emit(
            *kind,
            |detail| notes::spanned(message, detail),
            |total, breakdown| notes::summary(message, total, breakdown),
            cx,
        );
    }
}

pub(crate) fn apply_renames<'a>(program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
    let renames = cx.take_renames();
    if renames.is_empty() {
        return;
    }
    let mut planned = Renames::new();
    for (symbol, name) in renames {
        planned.insert(symbol, Ident::from_str_in(&name, &*cx));
    }
    let applied = planned.apply(program, cx.scoping());
    cx.changed_by(applied);
}
