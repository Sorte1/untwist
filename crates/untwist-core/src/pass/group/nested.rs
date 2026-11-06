use oxc_allocator::{Dummy, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_traverse::{traverse_mut, Ancestor, Traverse, TraverseCtx};

use super::scope::Edit;
use super::{Dispatched, PassGroup};

const NESTING_MAX: u8 = 8;

#[cfg(feature = "test-hooks")]
pub mod mutants {
    use std::sync::atomic::{AtomicBool, Ordering};

    static OFF: AtomicBool = AtomicBool::new(false);
    static ROOT_LISTS: AtomicBool = AtomicBool::new(false);

    #[doc(hidden)]
    pub fn cascade_off_for_tests(on: bool) {
        OFF.store(on, Ordering::SeqCst);
    }

    #[doc(hidden)]
    pub fn cascade_root_lists_for_tests(on: bool) {
        ROOT_LISTS.store(on, Ordering::SeqCst);
    }

    pub(super) fn off() -> bool {
        OFF.load(Ordering::SeqCst)
    }

    pub(super) fn root_lists() -> bool {
        ROOT_LISTS.load(Ordering::SeqCst)
    }
}

fn cascading(group: &PassGroup<'_>) -> bool {
    #[cfg(feature = "test-hooks")]
    if mutants::off() {
        return false;
    }
    group.nesting < NESTING_MAX
}

pub(super) struct Nested<'g, 'a>(pub(super) &'g mut PassGroup<'a>);

fn nested<'s, 'a>(nested: &'s mut Nested<'_, 'a>) -> &'s mut PassGroup<'a> {
    nested.0
}

impl<'a> PassGroup<'a> {
    #[inline(never)]
    pub(super) fn exit_statement_walking_replacements<const RECORD: bool>(
        &mut self,
        node: &mut Statement<'a>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        let kind = node.kind_index();
        self.dispatch_exit_statement::<RECORD>(node, ctx);
        if !cascading(self) || node.kind_index() == kind {
            return;
        }
        self.walk_replacement(node, ctx);
        if RECORD && self.scratch.marks.touched {
            let edited = node.edited();
            self.scope.touched(
                &mut self.scratch.marks,
                edited,
                ctx,
                &self.counts,
                &self.opts,
            );
        }
    }

    fn walk_replacement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        let alloc = ctx.ast.allocator();
        let scope_id = ctx.current_scope_id();
        let spare = self.spare_scoping.take().unwrap_or_default();
        let scoping = std::mem::replace(ctx.scoping_mut(), spare);
        let mut body = self.bodies.pop().unwrap_or_else(|| {
            let mut fresh = AVec::with_capacity_in(1, &ctx.ast);
            fresh.push(Statement::new_empty_statement(SPAN, &ctx.ast));
            fresh
        });
        if let Some(slot) = body.first_mut() {
            std::mem::swap(node, slot);
        }
        let mut program = Program::dummy(alloc);
        program.body = body;
        program.scope_id.set(Some(scope_id));
        self.nesting += 1;
        let scoping = traverse_mut(&mut Nested(self), alloc, &mut program, scoping, ());
        self.nesting -= 1;
        self.spare_scoping = Some(std::mem::replace(ctx.scoping_mut(), scoping));
        if let Some(walked) = program.body.first_mut() {
            std::mem::swap(node, walked);
        }
        self.bodies.push(program.body);
    }
}

impl<'a> Traverse<'a, ()> for Nested<'_, 'a> {
    super::walk::node_hooks!(false, nested);

    fn enter_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        if !at_root_list(ctx) {
            self.0.dispatch_enter_statements::<false>(node, ctx)
        }
    }

    fn exit_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        if !at_root_list(ctx) {
            self.0.dispatch_exit_statements::<false>(node, ctx)
        }
    }

    fn enter_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.0.dispatch_enter_statement::<false>(node, ctx)
    }

    fn exit_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.0
            .exit_statement_walking_replacements::<false>(node, ctx)
    }
}

pub(super) struct Cascading<'g, 'a>(pub(super) &'g mut PassGroup<'a>);

fn cascaded<'s, 'a>(cascading: &'s mut Cascading<'_, 'a>) -> &'s mut PassGroup<'a> {
    cascading.0
}

impl<'a> Traverse<'a, ()> for Cascading<'_, 'a> {
    super::walk::node_hooks!(false, cascaded);
    super::walk::program_hooks!(cascaded);

    fn enter_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.0.dispatch_enter_statements::<false>(node, ctx)
    }

    fn exit_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.0.dispatch_exit_statements::<false>(node, ctx)
    }

    fn enter_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.0.dispatch_enter_statement::<false>(node, ctx)
    }

    fn exit_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.0
            .exit_statement_walking_replacements::<false>(node, ctx)
    }
}

fn at_root_list(ctx: &TraverseCtx<'_, ()>) -> bool {
    #[cfg(feature = "test-hooks")]
    if mutants::root_lists() {
        return false;
    }
    matches!(ctx.parent(), Ancestor::ProgramBody(_)) || ctx.ancestors_depth() <= 1
}
