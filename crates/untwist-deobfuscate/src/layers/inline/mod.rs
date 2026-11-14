mod names;
mod payload;
mod regex;
mod sites;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_ast_visit::Visit;
use oxc_semantic::SemanticBuilder;
use oxc_span::Span;
use rustc_hash::{FxHashMap, FxHashSet};

use self::names::Watch;
use self::payload::Facts;
use self::sites::{Context, Finder, Kind, List, Site};
use super::decode::Rebound;
use super::model::{DecodeCx, LayerForm, Refusal, Refused};
use untwist_core::parse::{has_error, parse_with, SourceType};
use untwist_core::semantic::builtins::{scan, Reliance};
use untwist_core::semantic::repair_blocked_hoists;

pub use super::INLINE_LAYER_PASS;

const RESERVED_DECLARATIONS: [&str; 2] = ["arguments", "eval"];

const FOLD_ROOTS: [&str; 22] = [
    "eval",
    "String",
    "atob",
    "btoa",
    "unescape",
    "escape",
    "decodeURIComponent",
    "decodeURI",
    "encodeURIComponent",
    "encodeURI",
    "parseInt",
    "parseFloat",
    "Number",
    "Math",
    "JSON",
    "Array",
    "Object",
    "Boolean",
    "RegExp",
    "Function",
    "undefined",
    "Infinity",
];

static RELIED_ON: Reliance = Reliance {
    roots: &FOLD_ROOTS,
    members: &[],
};

pub struct InlineCx<'c> {
    pub decode: DecodeCx,
    pub admits: &'c dyn Fn(&str) -> Result<(), Refusal>,
}

#[derive(Debug)]
pub struct Inlined {
    pub text: String,
    pub form: LayerForm,
    pub sites: usize,
    pub steps: u64,
    pub heap: u64,
}

#[derive(Debug)]
pub enum Plan {
    Nothing,
    Peel(Inlined),
    Kept(Refused),
}

enum Shape {
    Bare(String),
    Block(String),
    StrictCall(String, bool),
    Parenthesized(String, bool),
}

struct Pending {
    span: Span,
    form: LayerForm,
    shape: Shape,
    watched: Vec<String>,
    function: Option<Span>,
    site: usize,
    runs_eval: bool,
}

pub fn plan(program: &Program<'_>, cx: &InlineCx<'_>) -> Plan {
    if program.source_type.is_module() || !mentions_an_eval_call(program.source_text) {
        return Plan::Nothing;
    }
    let mut finder = Finder::new(program, cx.decode);
    finder.visit_program(program);
    if finder.sites.is_empty() {
        return Plan::Nothing;
    }
    let (steps, heap) = (finder.steps, finder.heap);
    let mut scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    repair_blocked_hoists(program, &mut scoping);
    let rebound = Rebound::of(&scoping);
    let mut refused: Vec<(Span, Refused)> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    for (index, site) in finder.sites.iter().enumerate() {
        let form = form_of(site.kind);
        match judge(site, &rebound, cx) {
            Ok(shape) => pending.push(Pending {
                site: index,
                ..shape
            }),
            Err(reason) => refused.push((site.span, Refused { form, reason })),
        }
    }
    settle_names(program, &mut pending, &mut refused);
    settle_shadowing(&finder.sites, &mut pending, &mut refused);
    if pending.is_empty() {
        refused.sort_by_key(|(span, _)| span.start);
        return match refused.first() {
            Some((_, first)) => Plan::Kept(*first),
            None => Plan::Nothing,
        };
    }
    pending.sort_by_key(|site| site.span.start);
    let text = splice(program.source_text, &pending);
    if let Err(reason) = builtins_hold(&text, cx.decode.unsafe_transforms) {
        let form = pending[0].form;
        return Plan::Kept(Refused { form, reason });
    }
    let form = match pending
        .iter()
        .any(|site| site.form == LayerForm::InlineStmt)
    {
        true => LayerForm::InlineStmt,
        false => LayerForm::InlineExpr,
    };
    Plan::Peel(Inlined {
        text,
        form,
        sites: pending.len(),
        steps,
        heap,
    })
}

fn mentions_an_eval_call(source: &str) -> bool {
    source.match_indices("eval").any(|(at, word)| {
        source[at + word.len()..]
            .trim_start()
            .starts_with(['(', ')'])
    })
}

fn form_of(kind: Kind) -> LayerForm {
    match kind {
        Kind::Statement(_) => LayerForm::InlineStmt,
        Kind::Expression | Kind::Indirect => LayerForm::InlineExpr,
    }
}

fn builtins_hold(text: &str, unsafe_transforms: bool) -> Result<(), Refusal> {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, text, SourceType::script());
    if has_error(&parsed.diagnostics) {
        return Err(Refusal::Unparsable);
    }
    let program = &parsed.program;
    let built = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(program);
    if has_error(&built.diagnostics) {
        return Err(Refusal::Unparsable);
    }
    let mut scoping = built.semantic.into_scoping();
    repair_blocked_hoists(program, &mut scoping);
    let verdict = scan(program, &scoping, false, false, &RELIED_ON).verdict;
    match verdict.allows_safe(unsafe_transforms) {
        true => Ok(()),
        false => Err(Refusal::Shape),
    }
}

fn judge(site: &Site<'_>, rebound: &Rebound<'_>, cx: &InlineCx<'_>) -> Result<Pending, Refusal> {
    let text = site.text.as_ref().map_err(|reason| *reason)?;
    let mut roots = vec!["eval"];
    roots.extend(site.names.iter().copied());
    if !rebound.holds(&roots) || !roots.iter().all(|name| FOLD_ROOTS.contains(name)) {
        return Err(Refusal::Shape);
    }
    if site.over_cap {
        return Err(Refusal::TooLarge);
    }
    let context = site.context;
    if context.class_member || context.in_with {
        return Err(Refusal::Shape);
    }
    (cx.admits)(text)?;
    let facts = payload::read(text, context.strict)?;
    if facts.own_strict && !context.strict {
        return strict_call(site, facts);
    }
    let runs_eval = facts.calls_eval && !context.strict;
    if facts.reserved_words && context.async_or_generator {
        return Err(Refusal::Shape);
    }
    if facts.labels && context.labelled {
        return Err(Refusal::Shape);
    }
    let pending = match site.kind {
        Kind::Statement(list) => statement(site.span, list, context, facts),
        Kind::Expression => expression(site.span, facts),
        Kind::Indirect => Err(Refusal::Shape),
    }?;
    Ok(Pending {
        runs_eval,
        ..pending
    })
}

fn strict_call(site: &Site<'_>, facts: Facts) -> Result<Pending, Refusal> {
    let Kind::Statement(list) = site.kind else {
        return Err(Refusal::Shape);
    };
    if site.context.async_or_generator {
        return Err(Refusal::Shape);
    }
    Ok(Pending {
        span: site.span,
        form: LayerForm::InlineStmt,
        shape: Shape::StrictCall(facts.statements, list.is_some()),
        watched: Vec::new(),
        function: None,
        site: 0,
        runs_eval: false,
    })
}

fn expression(span: Span, facts: Facts) -> Result<Pending, Refusal> {
    let (text, bare) = facts.expression.ok_or(Refusal::Shape)?;
    Ok(Pending {
        span,
        form: LayerForm::InlineExpr,
        shape: Shape::Parenthesized(text, bare),
        watched: Vec::new(),
        function: None,
        site: 0,
        runs_eval: false,
    })
}

fn statement(
    span: Span,
    list: Option<List>,
    context: Context,
    facts: Facts,
) -> Result<Pending, Refusal> {
    let pending = |shape, watched, function| Pending {
        span,
        form: LayerForm::InlineStmt,
        shape,
        watched,
        function,
        site: 0,
        runs_eval: false,
    };
    let in_body = list == Some(List::FunctionBody) && context.function.is_some();
    if facts.var_names.is_empty() {
        if facts.lexical_names.is_empty() && list.is_some() {
            return Ok(pending(Shape::Bare(facts.statements), Vec::new(), None));
        }
        if !in_body {
            return Ok(pending(Shape::Block(facts.statements), Vec::new(), None));
        }
    }
    if !in_body || !context.simple_params {
        return Err(Refusal::Shape);
    }
    if !context.strict && (facts.deletes_a_name || facts.calls_eval) {
        return Err(Refusal::Shape);
    }
    let mut watched = facts.var_names;
    watched.extend(facts.lexical_names);
    if watched
        .iter()
        .any(|name| RESERVED_DECLARATIONS.contains(&name.as_str()))
    {
        return Err(Refusal::Shape);
    }
    Ok(pending(
        Shape::Bare(facts.statements),
        watched,
        context.function,
    ))
}

fn settle_names(
    program: &Program<'_>,
    pending: &mut Vec<Pending>,
    refused: &mut Vec<(Span, Refused)>,
) {
    let mut watches: Vec<Watch<'_>> = pending
        .iter()
        .filter_map(|site| {
            let function = site.function?;
            Some(Watch {
                function,
                site: site.span,
                names: &site.watched,
                clashed: false,
            })
        })
        .collect();
    names::clashes(program, &mut watches);
    let clashed: FxHashSet<Span> = watches
        .iter()
        .filter(|watch| watch.clashed)
        .map(|watch| watch.site)
        .collect();
    refuse_where(pending, refused, |site| clashed.contains(&site.span));
}

fn refuse_where(
    pending: &mut Vec<Pending>,
    refused: &mut Vec<(Span, Refused)>,
    refuse: impl Fn(&Pending) -> bool,
) -> bool {
    let before = pending.len();
    pending.retain(|site| {
        let keep = !refuse(site);
        if !keep {
            let refusal = Refused {
                form: site.form,
                reason: Refusal::Shape,
            };
            refused.push((site.span, refusal));
        }
        keep
    });
    pending.len() != before
}

fn settle_shadowing(
    sites: &[Site<'_>],
    pending: &mut Vec<Pending>,
    refused: &mut Vec<(Span, Refused)>,
) {
    loop {
        let peeled: FxHashSet<usize> = pending.iter().map(|site| site.site).collect();
        let mut declaring: FxHashMap<Option<Span>, Vec<usize>> = FxHashMap::default();
        for (index, site) in sites.iter().enumerate() {
            let kept = !peeled.contains(&index);
            let scope = match site.shadows {
                Some(scope) if kept => Some(scope),
                _ => None,
            };
            if let Some(scope) = scope {
                declaring.entry(scope).or_default().push(index);
            }
        }
        for site in pending.iter().filter(|site| site.runs_eval) {
            let scope = sites[site.site].context.function;
            declaring.entry(scope).or_default().push(site.site);
        }
        if declaring.is_empty() {
            return;
        }
        let shadowed = |site: &Pending| {
            let chain = &sites[site.site].chain;
            std::iter::once(None)
                .chain(chain.iter().copied().map(Some))
                .filter_map(|scope| declaring.get(&scope))
                .any(|indices| indices.iter().any(|index| *index != site.site))
        };
        if !refuse_where(pending, refused, shadowed) {
            return;
        }
    }
}

fn splice(source: &str, pending: &[Pending]) -> String {
    let mut out = String::with_capacity(source.len());
    let mut at = 0_usize;
    for site in pending {
        let (start, end) = (site.span.start as usize, site.span.end as usize);
        out.push_str(source.get(at..start).unwrap_or_default());
        match &site.shape {
            Shape::Bare(text) => {
                out.push_str(";\n");
                out.push_str(text);
                out.push_str("\n;");
            }
            Shape::Block(text) => {
                out.push_str("{\n");
                out.push_str(text);
                out.push_str("\n}");
            }
            Shape::StrictCall(text, listed) => {
                out.push_str(if *listed {
                    ";\n(() => {\n"
                } else {
                    "{\n(() => {\n"
                });
                out.push_str(text);
                out.push_str(if *listed { "\n})();" } else { "\n})();\n}" });
            }
            Shape::Parenthesized(text, bare) => {
                out.push_str(if *bare { "(" } else { "(0, " });
                out.push_str(text);
                out.push_str("\n)");
            }
        }
        at = end;
    }
    out.push_str(source.get(at..).unwrap_or_default());
    out
}
