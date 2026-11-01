use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use oxc_allocator::{Address, GetAddress, GetAllocator};
use oxc_ast::ast::{Expression, Statement};
use oxc_span::{GetSpan, Span};
use oxc_traverse::TraverseCtx;

use super::super::interest::{ExpressionKind, Interest, StatementKind};
use super::super::{Cx, Pass};
use super::{Dispatched, PassGroup};

static PROBE: AtomicBool = AtomicBool::new(false);

static VIOLATIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[doc(hidden)]
pub fn probe_variants_for_tests(on: bool) {
    PROBE.store(on, Ordering::SeqCst);
}

#[doc(hidden)]
pub fn take_variant_violations_for_tests() -> Vec<String> {
    let mut violations = VIOLATIONS.lock().unwrap_or_else(PoisonError::into_inner);
    std::mem::take(&mut *violations)
}

pub(super) fn probing() -> bool {
    PROBE.load(Ordering::SeqCst)
}

fn record_violation(violation: String) {
    let mut violations = VIOLATIONS.lock().unwrap_or_else(PoisonError::into_inner);
    violations.push(violation);
}

trait Probed: Dispatched + GetAddress + GetSpan {
    fn kind_name(kind: usize) -> String;
}

impl Probed for Expression<'_> {
    fn kind_name(kind: usize) -> String {
        ExpressionKind::from_index(kind)
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_default()
    }
}

impl Probed for Statement<'_> {
    fn kind_name(kind: usize) -> String {
        StatementKind::from_index(kind)
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_default()
    }
}

#[derive(PartialEq, Eq)]
struct Snapshot {
    address: Address,
    kind: usize,
    span: Span,
    arena: usize,
    changes: u32,
    notes: usize,
    invented: usize,
    renamed: usize,
    deferred: bool,
    code_text: bool,
    symbols: usize,
    scopes: usize,
    references: usize,
}

fn snapshot<N: Probed>(node: &N, cx: &Cx<'_, '_>) -> Snapshot {
    let scoping = cx.scoping();
    Snapshot {
        address: node.address(),
        kind: node.kind_index(),
        span: node.span(),
        arena: cx.allocator().used_bytes(),
        changes: *cx.changes,
        notes: cx.notes.len(),
        invented: cx.invented.len(),
        renamed: cx.renamed.len(),
        deferred: cx.opts.deferred_pending.get(),
        code_text: cx.opts.code_text.is_marked(),
        symbols: scoping.symbols_len(),
        scopes: scoping.scopes_len(),
        references: scoping.references_len(),
    }
}

type NodeHook<'a, N> = fn(&mut (dyn Pass<'a> + 'a), &mut N, &mut Cx<'a, '_>);

fn probe_call<'a, N: Probed>(
    pass: &mut (dyn Pass<'a> + 'a),
    hook: &'static str,
    node: &mut N,
    cx: &mut Cx<'a, '_>,
    call: NodeHook<'a, N>,
) {
    let before = snapshot(node, cx);
    call(pass, node, cx);
    let after = snapshot(node, cx);
    if before != after {
        record_violation(format!(
            "{} acted in {hook} on {}, outside its declared variants",
            cx.pass_name,
            N::kind_name(before.kind)
        ));
    }
}

macro_rules! probes {
    ($($hook:ident($node_ty:ty) => $probe:ident, $bit:ident;)*) => {
        $(
            #[inline(never)]
            pub(super) fn $probe(&mut self, node: &mut $node_ty, ctx: &mut TraverseCtx<'a, ()>) {
                let Self {
                    passes,
                    names,
                    counts,
                    hooks,
                    opts,
                    notes,
                    invented,
                    renamed,
                    ..
                } = self;
                for &index in &hooks[Interest::$bit.slot()] {
                    let (Some(pass), Some(count), Some(name)) =
                        (passes.get_mut(index), counts.get_mut(index), names.get(index))
                    else {
                        continue;
                    };
                    let interest = pass.interest();
                    let mut cx = Cx {
                        tcx: ctx,
                        changes: count,
                        opts,
                        notes,
                        pass_name: name,
                        invented,
                        renamed,
                    };
                    if <$node_ty>::includes(&interest, node.kind_index()) {
                        pass.$hook(node, &mut cx);
                        continue;
                    }
                    let call: NodeHook<'a, $node_ty> = |pass, node, cx| pass.$hook(node, cx);
                    probe_call(pass.as_mut(), stringify!($hook), node, &mut cx, call);
                }
            }
        )*
    };
}

impl<'a> PassGroup<'a> {
    probes! {
        enter_expression(Expression<'a>) => probe_enter_expression, ENTER_EXPRESSION;
        exit_expression(Expression<'a>) => probe_exit_expression, EXIT_EXPRESSION;
        enter_statement(Statement<'a>) => probe_enter_statement, ENTER_STATEMENT;
        exit_statement(Statement<'a>) => probe_exit_statement, EXIT_STATEMENT;
    }
}
