use oxc_semantic::SymbolId;
use oxc_span::Span;
use rustc_hash::FxHashSet;

use untwist_eval::object::PROTOTYPE_LINKS;

use super::escape::{EscapeContext, EscapeScan};
use super::escape_scan::scan_decoded_text;
use super::written::WrittenKey;

#[derive(Clone, Debug)]
pub enum PendingSite {
    Sink {
        outer_span: Span,
        arg_span: Span,
        forcing: bool,
    },
    ProxyKey {
        call_span: Span,
        key_span: Span,
        object_symbol: Option<SymbolId>,
        strict_names: FxHashSet<&'static str>,
        strict_span: Option<Span>,
        forcing: bool,
    },
}

pub struct PreScan {
    pub names: FxHashSet<&'static str>,
    pub span: Option<Span>,
    pub pending: Vec<PendingSite>,
    pub object_operand: Option<Span>,
    pub written: Vec<WrittenKey>,
}

pub fn resolve_pending_sites(
    scan: &PreScan,
    globals: &FxHashSet<&'static str>,
    context: &EscapeContext,
    resolve: &dyn Fn(Span) -> Option<String>,
) -> EscapeScan {
    let mut names = scan.names.clone();
    let mut span = scan.span;
    for site in &scan.pending {
        match site {
            PendingSite::Sink {
                outer_span,
                arg_span,
                ..
            } => resolve_sink_site(
                &mut names,
                &mut span,
                globals,
                resolve,
                *outer_span,
                *arg_span,
            ),
            PendingSite::ProxyKey { .. } => {
                resolve_proxy_key_site(&mut names, &mut span, context, resolve, site)
            }
        }
    }
    EscapeScan {
        names,
        span,
        object_operand: scan.object_operand,
    }
}

fn resolve_proxy_key_site(
    names: &mut FxHashSet<&'static str>,
    span: &mut Option<Span>,
    context: &EscapeContext,
    resolve: &dyn Fn(Span) -> Option<String>,
    site: &PendingSite,
) {
    let PendingSite::ProxyKey {
        key_span,
        object_symbol,
        strict_names,
        strict_span,
        ..
    } = site
    else {
        return;
    };
    let exemptible = match resolve(*key_span) {
        Some(key) => object_symbol
            .and_then(|symbol_id| context.proxy_functions.get(&symbol_id))
            .is_some_and(|functions| functions.contains(&key)),
        None => false,
    };
    if !exemptible {
        if let Some(at) = strict_span {
            span.get_or_insert(*at);
        }
        names.extend(strict_names.iter().copied());
    }
}

fn flag_all_into(
    names: &mut FxHashSet<&'static str>,
    span: &mut Option<Span>,
    globals: &FxHashSet<&'static str>,
    at: Span,
) {
    span.get_or_insert(at);
    names.extend(globals.iter().copied());
    names.insert(PROTOTYPE_LINKS);
}

fn resolve_sink_site(
    names: &mut FxHashSet<&'static str>,
    span: &mut Option<Span>,
    globals: &FxHashSet<&'static str>,
    resolve: &dyn Fn(Span) -> Option<String>,
    outer_span: Span,
    arg_span: Span,
) {
    let Some(text) = resolve(arg_span) else {
        flag_all_into(names, span, globals, outer_span);
        return;
    };
    match scan_decoded_text(&text, globals, None, 1) {
        Some((found, matched)) => {
            if let Some(sub_span) = found {
                span.get_or_insert(sub_span);
            }
            names.extend(matched);
        }
        None => flag_all_into(names, span, globals, outer_span),
    }
}
