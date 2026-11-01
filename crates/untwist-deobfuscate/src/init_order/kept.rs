use std::cell::RefCell;
use std::ops::Deref;

use oxc_ast::ast::Program;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::readonly::{
    constant_bindings, readonly_uses, scan_symbol_uses, ObjectUses, UsesScan,
};
use untwist_core::pass::{edits, Cx};

use super::{try_scan_init_order, try_scan_init_order_with, InitOrder};
use crate::fused::{Member, Walk};
use crate::notes;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    program: usize,
    edits: u64,
    changes: u32,
    scopes: usize,
    symbols: usize,
    references: usize,
}

impl Mark {
    pub(crate) fn of(program: &Program<'_>, cx: &Cx<'_, '_>) -> Self {
        let scoping = cx.scoping();
        Self {
            program: std::ptr::from_ref(program) as usize,
            edits: edits(),
            changes: cx
                .opts
                .changes_by_others_before_program_hook
                .get()
                .saturating_add(*cx.changes),
            scopes: scoping.scopes_len(),
            symbols: scoping.symbols_len(),
            references: scoping.references_len(),
        }
    }
}

struct Kept {
    mark: Mark,
    order: InitOrder,
    overflowed: bool,
}

thread_local! {
    static KEPT: RefCell<Option<Kept>> = const { RefCell::new(None) };
}

pub fn clear_kept_order() {
    KEPT.with(|kept| kept.borrow_mut().take());
}

pub fn pass_done(shares: bool, changed: bool) {
    if shares && changed {
        clear_kept_order();
    }
}

fn take_matching(mark: Mark) -> Option<(InitOrder, bool)> {
    KEPT.with(|kept| {
        let mut kept = kept.borrow_mut();
        if kept.as_ref().is_some_and(|kept| kept.mark == mark) {
            return kept.take().map(|kept| (kept.order, kept.overflowed));
        }
        *kept = None;
        None
    })
}

pub(crate) struct LentOrder {
    order: InitOrder,
    overflowed: bool,
    mark: Option<Mark>,
}

impl Deref for LentOrder {
    type Target = InitOrder;

    fn deref(&self) -> &InitOrder {
        &self.order
    }
}

impl LentOrder {
    pub(crate) fn give_back(self, program: &Program<'_>, cx: &Cx<'_, '_>) {
        self.give_back_if(program, cx, InitOrder::lean_for_reuse);
    }

    pub(crate) fn give_back_any(self, program: &Program<'_>, cx: &Cx<'_, '_>) {
        self.give_back_if(program, cx, InitOrder::reusable);
    }

    fn give_back_if(
        self,
        program: &Program<'_>,
        cx: &Cx<'_, '_>,
        keepable: fn(&mut InitOrder) -> bool,
    ) {
        let Some(mark) = self.mark.filter(|mark| *mark == Mark::of(program, cx)) else {
            return;
        };
        let mut order = self.order;
        if !keepable(&mut order) {
            return;
        }
        let kept = Kept {
            mark,
            order,
            overflowed: self.overflowed,
        };
        KEPT.with(|slot| *slot.borrow_mut() = Some(kept));
    }
}

pub(crate) struct Found<M> {
    pub(crate) order: LentOrder,
    pub(crate) member: M,
}

fn lent(scanned: Option<InitOrder>, mark: Option<Mark>) -> LentOrder {
    LentOrder {
        overflowed: scanned.is_none(),
        order: scanned.unwrap_or_default(),
        mark,
    }
}

pub(crate) fn lend_order_with<'a, M: Member<'a>>(
    mark: Option<Mark>,
    program: &Program<'a>,
    scoping: &Scoping,
    member: M,
) -> Found<M> {
    if let Some((order, overflowed)) = mark.and_then(take_matching) {
        let mut walk = Walk(member);
        walk.visit_program(program);
        return Found {
            order: LentOrder {
                order,
                overflowed,
                mark,
            },
            member: walk.0,
        };
    }
    let (scanned, member) = try_scan_init_order_with(program, scoping, member);
    Found {
        order: lent(scanned, mark),
        member,
    }
}

pub(crate) fn note_overflow(order: &LentOrder, cx: &mut Cx<'_, '_>) {
    if order.overflowed {
        cx.note(notes::INIT_ORDER_OVERFLOW, None);
    }
}

pub(crate) fn sharing_mark(shares: bool, program: &Program<'_>, cx: &Cx<'_, '_>) -> Option<Mark> {
    shares.then(|| Mark::of(program, cx))
}

pub(crate) fn lend_order(shares: bool, program: &Program<'_>, cx: &mut Cx<'_, '_>) -> LentOrder {
    let mark = sharing_mark(shares, program, cx);
    let order = match mark.and_then(take_matching) {
        Some((order, overflowed)) => LentOrder {
            order,
            overflowed,
            mark,
        },
        None => lent(try_scan_init_order(program, cx.scoping()), mark),
    };
    note_overflow(&order, cx);
    order
}

pub(crate) fn lend_order_and_readonly_uses(
    shares: bool,
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
    symbols: &FxHashSet<SymbolId>,
) -> (LentOrder, FxHashMap<SymbolId, ObjectUses>) {
    let constant = constant_bindings(cx.scoping(), symbols);
    if constant.is_empty() {
        return (lend_order(shares, program, cx), FxHashMap::default());
    }
    let mark = sharing_mark(shares, program, cx);
    let (order, uses) = match mark.and_then(take_matching) {
        Some((order, overflowed)) => {
            let uses = scan_symbol_uses(program, cx.scoping(), &constant);
            let order = LentOrder {
                order,
                overflowed,
                mark,
            };
            (order, uses)
        }
        None => {
            let scoping = cx.scoping();
            let member = UsesScan::new(scoping, &constant);
            let (scanned, member) = try_scan_init_order_with(program, scoping, member);
            (lent(scanned, mark), member.finish())
        }
    };
    note_overflow(&order, cx);
    (order, readonly_uses(uses))
}

#[cfg(test)]
mod tests;
