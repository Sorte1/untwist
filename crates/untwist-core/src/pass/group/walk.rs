use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_traverse::{traverse_mut, Traverse, TraverseCtx};

use super::super::interest::Interest;
use super::super::Cx;
use super::scope::{Edit, Steer};
use super::{above, park, unpark, Dispatched, Mode, PassGroup};

macro_rules! listed {
    ($self:ident, $record:ident, $hook:ident, $bit:ident, $node:ident, $ctx:ident) => {{
        let PassGroup {
            passes,
            names,
            counts,
            hooks,
            opts,
            notes,
            scratch,
            scope,
            ..
        } = $self;
        let listed = &hooks[Interest::$bit.slot()];
        if listed.is_empty() {
            return;
        }
        for &index in listed {
            let (Some(pass), Some(count), Some(name)) = (
                passes.get_mut(index),
                counts.get_mut(index),
                names.get(index),
            ) else {
                continue;
            };
            let mut cx = Cx {
                tcx: $ctx,
                changes: count,
                opts,
                notes,
                pass_name: name,
                scratch,
            };
            pass.$hook($node, &mut cx);
        }
        if $record && scratch.marks.touched {
            scope.touched(&mut scratch.marks, $node.edited(), $ctx, counts, opts);
        }
    }};
}

macro_rules! forward {
    ($($name:ident / $hook:ident($node_ty:ty) => $bit:ident;)*) => {
        $(
            #[inline]
            fn $name<const RECORD: bool>(
                &mut self,
                node: &mut $node_ty,
                ctx: &mut TraverseCtx<'a, ()>,
            ) {
                listed!(self, RECORD, $hook, $bit, node, ctx)
            }
        )*
    };
}

macro_rules! by_variant {
    ($($name:ident / $hook:ident($node_ty:ty) => $probe:ident, $bit:ident;)*) => {
        $(
            #[inline]
            fn $name<const RECORD: bool>(
                &mut self,
                node: &mut $node_ty,
                ctx: &mut TraverseCtx<'a, ()>,
            ) {
                match self.mode {
                    Mode::ByVariant => {
                        let mut kind = node.kind_index();
                        let Some(&first) = self.tables.$hook.get(kind) else {
                            return;
                        };
                        if RECORD && first == 0 {
                            return;
                        }
                        let mut bits = first;
                        let Self {
                            passes,
                            names,
                            counts,
                            tables,
                            opts,
                            notes,
                            scratch,
                            scope,
                            ..
                        } = self;
                        while bits != 0 {
                            let index = bits.trailing_zeros() as usize;
                            bits &= bits - 1;
                            let (Some(pass), Some(count), Some(name)) =
                                (passes.get_mut(index), counts.get_mut(index), names.get(index))
                            else {
                                continue;
                            };
                            let mut cx = Cx {
                                tcx: ctx,
                                changes: count,
                                opts,
                                notes,
                                pass_name: name,
                                scratch,
                            };
                            pass.$hook(node, &mut cx);
                            let now = node.kind_index();
                            if now != kind {
                                kind = now;
                                bits = tables.$hook.get(kind).copied().unwrap_or(0) & above(index);
                            }
                        }
                        if RECORD && scratch.marks.touched {
                            scope.touched(&mut scratch.marks, node.edited(), ctx, counts, opts);
                        }
                    }
                    Mode::List => listed!(self, RECORD, $hook, $bit, node, ctx),
                    #[cfg(feature = "test-hooks")]
                    Mode::Probe => self.$probe(node, ctx),
                }
            }
        )*
    };
}

impl<'a> PassGroup<'a> {
    by_variant! {
        dispatch_enter_statement / enter_statement(Statement<'a>) => probe_enter_statement, ENTER_STATEMENT;
        dispatch_exit_statement / exit_statement(Statement<'a>) => probe_exit_statement, EXIT_STATEMENT;
        dispatch_enter_expression / enter_expression(Expression<'a>) => probe_enter_expression, ENTER_EXPRESSION;
        dispatch_exit_expression / exit_expression(Expression<'a>) => probe_exit_expression, EXIT_EXPRESSION;
    }

    forward! {
        dispatch_enter_statements / enter_statements(AVec<'a, Statement<'a>>) => ENTER_STATEMENTS;
        dispatch_exit_statements / exit_statements(AVec<'a, Statement<'a>>) => EXIT_STATEMENTS;
        dispatch_exit_function / exit_function(Function<'a>) => EXIT_FUNCTION;
        dispatch_exit_arrow_function_expression / exit_arrow_function_expression(ArrowFunctionExpression<'a>) => EXIT_ARROW_FUNCTION_EXPRESSION;
        dispatch_exit_variable_declaration / exit_variable_declaration(VariableDeclaration<'a>) => EXIT_VARIABLE_DECLARATION;
        dispatch_exit_object_property / exit_object_property(ObjectProperty<'a>) => EXIT_OBJECT_PROPERTY;
        dispatch_exit_method_definition / exit_method_definition(MethodDefinition<'a>) => EXIT_METHOD_DEFINITION;
        dispatch_exit_property_definition / exit_property_definition(PropertyDefinition<'a>) => EXIT_PROPERTY_DEFINITION;
        dispatch_exit_template_literal / exit_template_literal(TemplateLiteral<'a>) => EXIT_TEMPLATE_LITERAL;
        dispatch_exit_assignment_target / exit_assignment_target(AssignmentTarget<'a>) => EXIT_ASSIGNMENT_TARGET;
    }

    pub(in super::super) fn traverse(
        &mut self,
        alloc: &'a oxc_allocator::Allocator,
        program: &mut Program<'a>,
        scoping: Scoping,
    ) -> Scoping {
        let steering = self.scope.steers_next() && self.walks;
        self.scope.set_steering(steering);
        if steering {
            return traverse_mut(&mut Steering(self), alloc, program, scoping, ());
        }
        traverse_mut(self, alloc, program, scoping, ())
    }

    fn enter_group_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.run_program_hook(
            Interest::ENTER_PROGRAM,
            |pass, node, cx| pass.enter_program(node, cx),
            node,
            ctx,
        );
        self.apply_retirement();
        self.begin_scope(ctx.scoping().scopes_len());
        if !self.walks {
            self.parked = park(node, ctx);
        }
    }

    fn exit_group_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        unpark(node, &mut self.parked);
        let before = self.total_changes();
        self.run_program_hook(
            Interest::EXIT_PROGRAM,
            |pass, node, cx| pass.exit_program(node, cx),
            node,
            ctx,
        );
        if self.total_changes() != before {
            self.scratch.marks.unplace();
        }
        #[cfg(feature = "test-hooks")]
        self.record_work();
        let settles = self.settles();
        let changes = self.total_changes();
        self.scope.finish(&mut self.scratch.marks, settles, changes);
    }

    #[inline(never)]
    fn steer_list(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) -> bool {
        if !self.scope.steered() {
            return false;
        }
        if self.opts.walk_whole.get() {
            self.scope.stop_parking();
            return false;
        }
        match self
            .scope
            .park(&mut self.scratch.marks, node, ctx, &mut self.notes)
        {
            Steer::Parked => true,
            Steer::Walk => false,
            Steer::Shelve => {
                let before = self.total_changes();
                self.dispatch_enter_statements::<true>(node, ctx);
                if self.total_changes() == before {
                    self.scope.shelve(node, ctx);
                }
                true
            }
        }
    }

    #[inline(never)]
    fn enter_steered_statement(
        &mut self,
        node: &mut Statement<'a>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) -> bool {
        let depth = ctx.ancestors_depth();
        if self.scope.shelving() {
            let walk_whole = self.opts.walk_whole.get();
            let marks = &mut self.scratch.marks;
            if self
                .scope
                .visit_shelved(marks, node, depth, walk_whole, &mut self.notes, ctx)
            {
                return true;
            }
        }
        if self.scope.steered() {
            self.scope.enter_statement(node, depth);
        }
        false
    }

    #[cfg(feature = "test-hooks")]
    fn record_work(&self) {
        super::super::work::record(super::super::work::Traversal {
            changes: self.total_changes(),
            walked: self.walks,
            steered: self.scope.was_steered(),
            lists_parked: self.scope.lists_parked,
            statements_shelved: self.scope.statements_shelved,
            confirming: self.scope.confirming(),
        });
    }
}

fn group<'s, 'a>(group: &'s mut PassGroup<'a>) -> &'s mut PassGroup<'a> {
    group
}

fn steered<'s, 'a>(steering: &'s mut Steering<'_, 'a>) -> &'s mut PassGroup<'a> {
    steering.0
}

macro_rules! node_hooks {
    ($record:literal, $group:ident) => {
        #[inline]
        fn enter_expression(&mut self, node: &mut Expression<'a>, ctx: &mut TraverseCtx<'a, ()>) {
            $group(self).dispatch_enter_expression::<$record>(node, ctx)
        }

        #[inline]
        fn exit_expression(&mut self, node: &mut Expression<'a>, ctx: &mut TraverseCtx<'a, ()>) {
            $group(self).dispatch_exit_expression::<$record>(node, ctx)
        }

        #[inline]
        fn exit_function(&mut self, node: &mut Function<'a>, ctx: &mut TraverseCtx<'a, ()>) {
            $group(self).dispatch_exit_function::<$record>(node, ctx)
        }

        #[inline]
        fn exit_arrow_function_expression(
            &mut self,
            node: &mut ArrowFunctionExpression<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_arrow_function_expression::<$record>(node, ctx)
        }

        #[inline]
        fn exit_variable_declaration(
            &mut self,
            node: &mut VariableDeclaration<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_variable_declaration::<$record>(node, ctx)
        }

        #[inline]
        fn exit_object_property(
            &mut self,
            node: &mut ObjectProperty<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_object_property::<$record>(node, ctx)
        }

        #[inline]
        fn exit_method_definition(
            &mut self,
            node: &mut MethodDefinition<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_method_definition::<$record>(node, ctx)
        }

        #[inline]
        fn exit_property_definition(
            &mut self,
            node: &mut PropertyDefinition<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_property_definition::<$record>(node, ctx)
        }

        #[inline]
        fn exit_template_literal(
            &mut self,
            node: &mut TemplateLiteral<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_template_literal::<$record>(node, ctx)
        }

        #[inline]
        fn exit_assignment_target(
            &mut self,
            node: &mut AssignmentTarget<'a>,
            ctx: &mut TraverseCtx<'a, ()>,
        ) {
            $group(self).dispatch_exit_assignment_target::<$record>(node, ctx)
        }

        fn enter_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
            $group(self).enter_group_program(node, ctx)
        }

        fn exit_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
            $group(self).exit_group_program(node, ctx)
        }
    };
}

impl<'a> Traverse<'a, ()> for PassGroup<'a> {
    node_hooks!(false, group);

    #[inline]
    fn enter_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.dispatch_enter_statements::<false>(node, ctx)
    }

    #[inline]
    fn exit_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.dispatch_exit_statements::<false>(node, ctx)
    }

    #[inline]
    fn enter_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.dispatch_enter_statement::<false>(node, ctx)
    }

    #[inline]
    fn exit_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        self.dispatch_exit_statement::<false>(node, ctx)
    }
}

struct Steering<'g, 'a>(&'g mut PassGroup<'a>);

impl<'a> Traverse<'a, ()> for Steering<'_, 'a> {
    node_hooks!(true, steered);

    #[inline]
    fn enter_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        if self.0.scope.engaged && self.0.steer_list(node, ctx) {
            return;
        }
        self.0.dispatch_enter_statements::<true>(node, ctx);
    }

    #[inline]
    fn exit_statements(
        &mut self,
        node: &mut AVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, ()>,
    ) {
        let depth = ctx.ancestors_depth();
        if self.0.scope.engaged && self.0.scope.leave_list_before(node, depth) {
            return;
        }
        self.0.dispatch_exit_statements::<true>(node, ctx);
        if self.0.scope.engaged {
            self.0.scope.leave_list(depth);
        }
    }

    #[inline]
    fn enter_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        if self.0.scope.engaged && self.0.enter_steered_statement(node, ctx) {
            return;
        }
        self.0.dispatch_enter_statement::<true>(node, ctx);
    }

    #[inline]
    fn exit_statement(&mut self, node: &mut Statement<'a>, ctx: &mut TraverseCtx<'a, ()>) {
        if self.0.scope.engaged && self.0.scope.skip_exit() {
            return;
        }
        self.0.dispatch_exit_statement::<true>(node, ctx);
        if self.0.scope.engaged {
            self.0.scope.leave_statement(ctx.ancestors_depth());
        }
    }
}
