use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_span::{GetSpan, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::facts::{FnRef, SlotKind};
use crate::infer::Env;
use crate::view::Annotations;
use crate::Mode;

const RANK_OPEN: u8 = 0;
const RANK_PARAM: u8 = 1;
const RANK_CLOSE: u8 = 2;
const RANK_RETURN: u8 = 3;

pub struct Edit {
    pub at: u32,
    pub rank: u8,
    pub tie: u32,
    pub text: String,
}

pub fn apply(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| (edit.at, edit.rank, edit.tie));
    let extra: usize = edits.iter().map(|edit| edit.text.len()).sum();
    let mut out = String::with_capacity(source.len() + extra);
    let mut from = 0usize;
    for edit in edits {
        let at = (edit.at as usize).min(source.len());
        if at < from {
            continue;
        }
        out.push_str(&source[from..at]);
        out.push_str(&edit.text);
        from = at;
    }
    out.push_str(&source[from..]);
    out
}

fn bare_arrow(source: &str, func: FnRef<'_>) -> Option<Span> {
    let FnRef::Arrow(arrow) = func else {
        return None;
    };
    let span = arrow.params.span;
    let opens = source.as_bytes().get(span.start as usize) == Some(&b'(');
    (!opens).then_some(span)
}

pub fn edits(env: &Env<'_, '_>, notes: &Annotations, mode: Mode, source: &str) -> Vec<Edit> {
    match mode {
        Mode::Ts => ts_edits(env, notes, source),
        Mode::Jsdoc => jsdoc_edits(env, notes),
    }
}

fn ts_edits(env: &Env<'_, '_>, notes: &Annotations, source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();
    let mut wrapped: FxHashSet<usize> = FxHashSet::default();
    let mut wrap = |edits: &mut Vec<Edit>, index: usize| {
        let func = env.facts.functions[index].func;
        if let Some(span) = bare_arrow(source, func) {
            if wrapped.insert(index) {
                edits.push(Edit {
                    at: span.start,
                    rank: RANK_OPEN,
                    tie: 0,
                    text: "(".to_string(),
                });
                edits.push(Edit {
                    at: span.end,
                    rank: RANK_CLOSE,
                    tie: 0,
                    text: ")".to_string(),
                });
            }
        }
    };
    for (symbol, ty) in &notes.symbols {
        let Some(slot) = env.facts.slots.get(symbol) else {
            continue;
        };
        let rank = match slot.kind {
            SlotKind::Binding { .. } => RANK_PARAM,
            SlotKind::Param { function, .. } => {
                wrap(&mut edits, function);
                RANK_PARAM
            }
        };
        edits.push(Edit {
            at: slot.at.end,
            rank,
            tie: 0,
            text: format!(": {}", ty.ts()),
        });
    }
    for (index, ty) in &notes.returns {
        wrap(&mut edits, *index);
        let func = env.facts.functions[*index].func;
        edits.push(Edit {
            at: func.params().span.end,
            rank: RANK_RETURN,
            tie: 0,
            text: format!(": {}", ty.ts()),
        });
    }
    edits
}

fn jsdoc_anchor(env: &Env<'_, '_>, index: usize) -> u32 {
    let fact = &env.facts.functions[index];
    match env.cx.nodes.parent_kind(fact.node) {
        AstKind::MethodDefinition(method) => method.span.start,
        AstKind::ObjectProperty(property) if property.method => property.span.start,
        _ => fact.func.span().start,
    }
}

fn jsdoc_edits(env: &Env<'_, '_>, notes: &Annotations) -> Vec<Edit> {
    let mut edits = Vec::new();
    let mut tags: FxHashMap<usize, Vec<(usize, String)>> = FxHashMap::default();
    for (symbol, ty) in &notes.symbols {
        let Some(slot) = env.facts.slots.get(symbol) else {
            continue;
        };
        match slot.kind {
            SlotKind::Binding { .. } => {
                let Some(declarator) = slot.declarator else {
                    continue;
                };
                let declaration = env.cx.nodes.parent_id(declarator.node_id.get());
                edits.push(Edit {
                    at: env.cx.nodes.kind(declaration).span().start,
                    rank: RANK_OPEN,
                    tie: 0,
                    text: format!("/** @type {{{}}} */ ", ty.ts()),
                });
            }
            SlotKind::Param { function, position } => {
                let name = env.cx.scoping.symbol_name(*symbol);
                tags.entry(function)
                    .or_default()
                    .push((position, format!("@param {{{}}} {name}", ty.ts())));
            }
        }
    }
    for (index, ty) in &notes.returns {
        tags.entry(*index)
            .or_default()
            .push((usize::MAX, format!("@returns {{{}}}", ty.ts())));
    }
    for (index, mut lines) in tags {
        lines.sort();
        let body: Vec<&str> = lines.iter().map(|(_, line)| line.as_str()).collect();
        edits.push(Edit {
            at: jsdoc_anchor(env, index),
            rank: RANK_OPEN,
            tie: 0,
            text: format!("/** {} */ ", body.join(" ")),
        });
    }
    edits
}

pub fn has_jsx(program: &Program<'_>) -> bool {
    use oxc_ast_visit::Visit;
    struct Scan(bool);
    impl<'a> Visit<'a> for Scan {
        fn visit_jsx_element(&mut self, _it: &JSXElement<'a>) {
            self.0 = true;
        }
        fn visit_jsx_fragment(&mut self, _it: &JSXFragment<'a>) {
            self.0 = true;
        }
    }
    let mut scan = Scan(false);
    scan.visit_program(program);
    scan.0
}
