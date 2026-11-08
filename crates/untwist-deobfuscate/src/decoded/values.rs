use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::decode::Decoded as DecodedValue;
use untwist_eval::object::PROTOTYPE_LINKS;
use untwist_eval::value::JsString;

use super::{all_root_names_set, prescan_for, root_keys, ScanCache, UnitRun};
use crate::bridge::{DecodePlan, DecodeRequest};
use crate::proof::{
    pending_member_keys, resolve_pending_sites, PatchKey, PendingSite, PreScan, ProtoWrite,
};

fn record_decoded_values<'a>(
    request: &DecodeRequest<'a>,
    values: &[Option<DecodedValue>],
    queried: &FxHashSet<Span>,
    map: &mut FxHashMap<Span, JsString>,
) {
    for (call, value) in request.calls.iter().zip(values.iter()) {
        if !queried.contains(&call.span) {
            continue;
        }
        if let Some(DecodedValue::Str(text)) = value {
            map.insert(call.span, text.clone());
        }
    }
}

fn build_value_map<'a>(
    runs: &[UnitRun<'a, '_>],
    queried: &FxHashSet<Span>,
) -> FxHashMap<Span, JsString> {
    let mut value_map = FxHashMap::default();
    if queried.is_empty() {
        return value_map;
    }
    for run in runs.iter().filter(|run| run.request.reliance.is_none()) {
        if let Ok(values) = &run.result {
            record_decoded_values(run.request, values, queried, &mut value_map);
        }
    }
    value_map
}

fn write_key_pending_spans(plan: &DecodePlan<'_>, spans: &mut FxHashSet<Span>) {
    for write in plan
        .requests
        .iter()
        .flat_map(|request| &request.guard.proto_writes)
    {
        if let PatchKey::Pending(span) = write.key {
            spans.insert(span);
        }
    }
}

fn pending_site_spans(scan: &PreScan, spans: &mut FxHashSet<Span>) {
    root_keys::written_spans(scan, spans);
    for site in &scan.pending {
        match site {
            PendingSite::Sink { text: key, .. }
            | PendingSite::GlobalKey { key, .. }
            | PendingSite::RootKey { key, .. } => key.pending_spans(spans),
            PendingSite::ProxyKey { key_span, .. } => {
                spans.insert(*key_span);
            }
        }
    }
}

fn queried_spans(
    plan: &DecodePlan<'_>,
    program: &Program<'_>,
    scoping: &Scoping,
    any_builtin_roots_touched: bool,
    cache: &mut ScanCache,
) -> FxHashSet<Span> {
    let mut spans: FxHashSet<Span> = pending_member_keys(program).into_iter().collect();
    write_key_pending_spans(plan, &mut spans);
    if any_builtin_roots_touched {
        let (_, scan) = prescan_for(cache, program, scoping, plan);
        pending_site_spans(scan, &mut spans);
    }
    spans
}

pub(super) fn value_map_for<'a>(
    runs: &[UnitRun<'a, '_>],
    plan: &DecodePlan<'_>,
    program: &Program<'_>,
    scoping: &Scoping,
    cache: &mut ScanCache,
) -> (bool, FxHashMap<Span, JsString>) {
    let any_builtin_roots_touched = runs.iter().any(|run| {
        run.result.is_ok()
            && !(run.evaluator.builtin_roots.is_empty() && run.evaluator.proto_reads.is_empty())
    });
    let queried = queried_spans(plan, program, scoping, any_builtin_roots_touched, cache);
    (any_builtin_roots_touched, build_value_map(runs, &queried))
}

pub(super) fn judge_resolved_roots<'p>(
    cache: &mut ScanCache,
    (program, scoping): (&'p Program<'p>, &Scoping),
    (plan, late): (&DecodePlan<'p>, Option<&[ProtoWrite<'p>]>),
    runs: &[UnitRun<'p, '_>],
    value_map: &FxHashMap<Span, JsString>,
) -> (FxHashSet<&'static str>, Option<Span>, bool) {
    let (ctx, scan) = prescan_for(cache, program, scoping, plan);
    let resolve = |span: Span| value_map.get(&span).and_then(JsString::as_string);
    let mut resolved = resolve_pending_sites(scan, all_root_names_set(), ctx, &resolve);
    root_keys::refuse_written_roots(&mut resolved.names, scan, runs, &resolve, scoping);
    let code_text = resolved.names.contains(PROTOTYPE_LINKS);
    let relies_on_links = runs
        .iter()
        .any(|run| run.evaluator.builtin_roots.contains(&PROTOTYPE_LINKS));
    if resolved.object_operand.is_none() && !relies_on_links {
        return (resolved.names, resolved.span, code_text);
    }
    if !intrinsics_proven((program, scoping), (plan, late), runs, value_map) {
        resolved.names.insert("Object");
        resolved.names.insert(PROTOTYPE_LINKS);
        if let Some(span) = resolved.object_operand {
            resolved.span.get_or_insert(span);
        }
    }
    (resolved.names, resolved.span, code_text)
}

const INTRINSIC_KEYS: [&str; 3] = ["__proto__", "@@hasInstance", "Object"];

fn reaches_intrinsics(write: &ProtoWrite<'_>, known: &dyn Fn(Span) -> Option<String>) -> bool {
    match write.key {
        PatchKey::Named(name) => INTRINSIC_KEYS.contains(&name),
        PatchKey::Pending(span) => {
            known(span).is_none_or(|name| INTRINSIC_KEYS.contains(&name.as_str()))
        }
        PatchKey::Dynamic | PatchKey::Whole => true,
    }
}

pub(super) fn decoded_strings<'r>(runs: &'r [UnitRun<'_, '_>]) -> FxHashMap<Span, &'r JsString> {
    let mut strings: FxHashMap<Span, &JsString> = FxHashMap::default();
    for run in runs {
        if let Ok(values) = &run.result {
            for (call, value) in run.request.calls.iter().zip(values.iter()) {
                if let Some(DecodedValue::Str(text)) = value {
                    strings.insert(call.span, text);
                }
            }
        }
    }
    strings
}

fn intrinsics_proven<'p>(
    (program, scoping): (&'p Program<'p>, &Scoping),
    (plan, late): (&DecodePlan<'p>, Option<&[ProtoWrite<'p>]>),
    runs: &[UnitRun<'p, '_>],
    value_map: &FxHashMap<Span, JsString>,
) -> bool {
    if !plan.intrinsic_object {
        return false;
    }
    if let Some(writes) = late {
        let known = |span: Span| value_map.get(&span).and_then(JsString::as_string);
        return !writes.iter().any(|write| reaches_intrinsics(write, &known));
    }
    let strings = decoded_strings(runs);
    let found = crate::calls::agnostic::guard_writes(
        program,
        scoping,
        (Some(&strings), &plan.machinery_spans()),
        None,
    );
    let known = |span: Span| {
        found
            .extra
            .iter()
            .find(|(marker, _)| *marker == span)
            .map(|(_, text)| text.clone())
    };
    !found
        .writes
        .iter()
        .any(|write| reaches_intrinsics(write, &known))
}
