pub mod driver;
pub mod group;
pub mod interest;
pub mod repeated;
mod settled;
#[cfg(feature = "test-hooks")]
mod work;

use std::cell::Cell;
use std::rc::Rc;

use oxc_allocator::{Address, Allocator, GetAllocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::{AstBuilder, GetAstBuilder};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use oxc_traverse::TraverseCtx;

use crate::ast_util::global::is_global_name_at;
use crate::ast_util::rename::unique_name_avoiding;

pub use driver::{
    replayed_report, run_once, run_to_fixpoint, run_to_fixpoint_noting_last,
    scoping_after_traversal, IterationTiming, RunReport, StopReason, CYCLE_CHECK_START,
    ITERATION_CEILING,
};
pub use group::PassGroup;
use group::Scratch;
#[cfg(feature = "test-hooks")]
pub use group::{
    cascade_off_for_tests, cascade_root_lists_for_tests, probe_variants_for_tests,
    skip_innermost_list_for_tests, take_variant_violations_for_tests, treat_as_neutral_for_tests,
    walk_every_group_for_tests,
};
pub use interest::{
    ExpressionKind, ExpressionVariants, Interest, StatementKind, StatementVariants,
    EXPRESSION_KINDS, HOOK_COUNT, HOOK_NAMES, STATEMENT_KINDS,
};
pub use repeated::{RepeatedSites, SPANNED_NOTE_CAP};
pub use settled::{edits, replay_allowed};
#[cfg(feature = "test-hooks")]
pub use settled::{full_rounds_for_tests, stale_settled_for_tests, walk_every_list_for_tests};
#[cfg(feature = "test-hooks")]
pub use work::{take_work_for_tests, Traversal};

pub fn note_survives_retraction(note: &Note) -> bool {
    note.span.is_none() || note.kind == NoteKind::Progress
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Safety {
    Safe,
    Unsafe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locality {
    Program,
    Local,
    Neutral,
}

#[derive(Clone, Debug)]
pub struct PassOptions {
    pub unsafe_transforms: bool,
    pub extended: bool,
    pub max_iterations: Option<u32>,
    pub fuel: Option<u64>,
    #[doc(hidden)]
    pub heap_budget: Option<u64>,
    #[doc(hidden)]
    pub room: Option<InputRoom>,
    #[doc(hidden)]
    pub deferred_pending: Cell<bool>,
    #[doc(hidden)]
    pub defer_disabled: Cell<bool>,
    #[doc(hidden)]
    pub round_requested: Cell<bool>,
    #[doc(hidden)]
    pub walk_whole: Cell<bool>,
    #[doc(hidden)]
    pub binding_moved: Cell<bool>,
    #[doc(hidden)]
    pub changes_before_program_hook: Cell<u32>,
    #[doc(hidden)]
    pub changes_by_others_before_program_hook: Cell<u32>,
    #[doc(hidden)]
    pub code_text: CodeTextSeen,
    #[doc(hidden)]
    pub nested_root: bool,
    #[doc(hidden)]
    pub keep_script_globals: bool,
    #[doc(hidden)]
    pub scoping_patched: Cell<bool>,
    #[doc(hidden)]
    pub shares_init_order: bool,
    pub disabled_rules: Vec<&'static str>,
}

impl PassOptions {
    pub fn rule_disabled(&self, rule: &str) -> bool {
        self.disabled_rules.contains(&rule)
    }
}

#[derive(Clone, Debug)]
pub struct InputRoom {
    print: Rc<Cell<u64>>,
    nesting: u32,
}

impl InputRoom {
    pub fn new(print: u64, nesting: u32) -> Self {
        Self {
            print: Rc::new(Cell::new(print)),
            nesting,
        }
    }

    pub fn print(&self) -> u64 {
        self.print.get()
    }

    pub fn spend_print(&self, bytes: u64) -> bool {
        let left = self.print.get();
        if bytes > left {
            return false;
        }
        self.print.set(left - bytes);
        true
    }

    pub fn nesting(&self) -> u32 {
        self.nesting
    }
}

#[derive(Clone, Debug, Default)]
pub struct CodeTextSeen(Rc<Cell<bool>>);

impl CodeTextSeen {
    pub fn mark(&self) {
        self.0.set(true);
    }

    pub fn is_marked(&self) -> bool {
        self.0.get()
    }
}

impl Default for PassOptions {
    fn default() -> Self {
        Self {
            unsafe_transforms: false,
            extended: true,
            max_iterations: None,
            fuel: None,
            heap_budget: None,
            room: None,
            deferred_pending: Cell::new(false),
            defer_disabled: Cell::new(false),
            round_requested: Cell::new(false),
            walk_whole: Cell::new(false),
            binding_moved: Cell::new(false),
            changes_before_program_hook: Cell::new(0),
            changes_by_others_before_program_hook: Cell::new(0),
            code_text: CodeTextSeen::default(),
            nested_root: false,
            keep_script_globals: false,
            scoping_patched: Cell::new(false),
            shares_init_order: false,
            disabled_rules: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoteKind {
    Progress,
    Refusal,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Note {
    pub pass: &'static str,
    pub kind: NoteKind,
    pub message: String,
    pub span: Option<Span>,
}

pub struct Cx<'a, 'c> {
    pub tcx: &'c mut TraverseCtx<'a, ()>,
    pub changes: &'c mut u32,
    pub opts: &'c PassOptions,
    pub notes: &'c mut Vec<Note>,
    pub pass_name: &'static str,
    scratch: &'c mut Scratch,
}

impl<'a> Cx<'a, '_> {
    pub fn changed(&mut self) {
        *self.changes += 1;
        self.scratch.marks.changed();
    }

    pub fn changed_by(&mut self, count: usize) {
        *self.changes += count as u32;
        self.scratch.marks.changed();
    }

    pub fn defer(&mut self) {
        self.opts.deferred_pending.set(true);
    }

    pub fn request_round(&mut self) {
        self.opts.round_requested.set(true);
    }

    #[inline]
    pub fn pins(&self) -> bool {
        self.scratch.marks.pins()
    }

    pub fn pin(&mut self, node: Address) {
        self.scratch.marks.pin(node, self.tcx);
    }

    pub fn walk_whole_round(&self) {
        self.opts.walk_whole.set(true);
    }

    pub fn binding_moved(&self) {
        self.opts.binding_moved.set(true);
    }

    pub fn defer_disabled(&self) -> bool {
        self.opts.defer_disabled.get()
    }

    pub fn is_program_unchanged_this_traversal(&self) -> bool {
        self.opts.changes_before_program_hook.get() == 0
    }

    pub fn ast(&self) -> &AstBuilder<'a> {
        &self.tcx.ast
    }

    pub fn scoping(&self) -> &Scoping {
        self.tcx.scoping()
    }

    pub fn scoping_mut(&mut self) -> &mut Scoping {
        self.tcx.scoping_mut()
    }

    pub fn compact_patched_scoping(&mut self) {
        if self.opts.scoping_patched.replace(false) {
            let compact = self
                .tcx
                .scoping()
                .clone_in_with_semantic_ids_with_another_arena();
            *self.tcx.scoping_mut() = compact;
        }
    }

    pub fn current_scope_id(&self) -> ScopeId {
        self.tcx.current_scope_id()
    }

    pub fn is_global_name(&self, name: &str) -> bool {
        is_global_name_at(self.tcx.scoping(), self.current_scope_id(), name)
    }

    pub fn invent_name(&mut self, base: &str) -> Option<String> {
        let name = unique_name_avoiding(base, self.tcx.scoping(), &self.scratch.invented, self)?
            .as_str()
            .to_string();
        self.scratch.invented.insert(name.clone());
        Some(name)
    }

    pub fn planned_name(&self, symbol: SymbolId) -> Option<&str> {
        self.scratch.renamed.get(&symbol).map(String::as_str)
    }

    pub fn plan_rename(&mut self, symbol: SymbolId, name: String) {
        self.scratch.renamed.insert(symbol, name);
    }

    pub fn take_renames(&mut self) -> Vec<(SymbolId, String)> {
        self.scratch.renamed.drain().collect()
    }

    pub fn note(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.note_kind(NoteKind::Refusal, message, span);
    }

    pub fn progress(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.note_kind(NoteKind::Progress, message, span);
    }

    pub fn note_kind(&mut self, kind: NoteKind, message: impl Into<String>, span: Option<Span>) {
        let note = Note {
            pass: self.pass_name,
            kind,
            message: message.into(),
            span,
        };
        if !note_survives_retraction(&note) {
            self.scratch.marks.note(&note);
        }
        self.notes.push(note);
    }

    pub fn block(&mut self, span: Span, body: AVec<'a, Statement<'a>>) -> Statement<'a> {
        let scope_id = self
            .tcx
            .insert_scope_below_statements(&body, ScopeFlags::empty());
        Statement::new_block_statement_with_scope_id(span, body, scope_id, self)
    }

    pub fn block_of(&mut self, stmt: Statement<'a>) -> Statement<'a> {
        if matches!(stmt, Statement::BlockStatement(_)) {
            return stmt;
        }
        let span = stmt.span();
        let scope_id = self
            .tcx
            .insert_scope_below_statement(&stmt, ScopeFlags::empty());
        let body = AVec::from_iter_in([stmt], &*self);
        Statement::new_block_statement_with_scope_id(span, body, scope_id, self)
    }
}

impl<'a> GetAllocator<'a> for Cx<'a, '_> {
    fn allocator(&self) -> &'a Allocator {
        self.tcx.ast.allocator()
    }
}

impl<'a> GetAstBuilder<'a> for Cx<'a, '_> {
    type Builder = AstBuilder<'a>;
    fn builder(&self) -> &AstBuilder<'a> {
        &self.tcx.ast
    }
}

pub trait Pass<'a> {
    fn name(&self) -> &'static str;
    fn safety(&self) -> Safety {
        Safety::Safe
    }
    fn requires_extended(&self) -> bool {
        false
    }
    fn cascades(&self) -> bool {
        false
    }
    fn reset(&mut self) {}
    fn interest(&self) -> Interest {
        Interest::ALL
    }
    fn retired(&self) -> Interest {
        Interest::NONE
    }
    fn locality(&self) -> Locality {
        Locality::Program
    }
    fn enter_program(&mut self, _node: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_program(&mut self, _node: &mut Program<'a>, _cx: &mut Cx<'a, '_>) {}
    fn enter_statements(&mut self, _node: &mut AVec<'a, Statement<'a>>, _cx: &mut Cx<'a, '_>) {}
    fn exit_statements(&mut self, _node: &mut AVec<'a, Statement<'a>>, _cx: &mut Cx<'a, '_>) {}
    fn enter_statement(&mut self, _node: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_statement(&mut self, _node: &mut Statement<'a>, _cx: &mut Cx<'a, '_>) {}
    fn enter_expression(&mut self, _node: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_expression(&mut self, _node: &mut Expression<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_function(&mut self, _node: &mut Function<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_arrow_function_expression(
        &mut self,
        _node: &mut ArrowFunctionExpression<'a>,
        _cx: &mut Cx<'a, '_>,
    ) {
    }
    fn exit_variable_declaration(
        &mut self,
        _node: &mut VariableDeclaration<'a>,
        _cx: &mut Cx<'a, '_>,
    ) {
    }
    fn exit_object_property(&mut self, _node: &mut ObjectProperty<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_method_definition(&mut self, _node: &mut MethodDefinition<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_property_definition(
        &mut self,
        _node: &mut PropertyDefinition<'a>,
        _cx: &mut Cx<'a, '_>,
    ) {
    }
    fn exit_template_literal(&mut self, _node: &mut TemplateLiteral<'a>, _cx: &mut Cx<'a, '_>) {}
    fn exit_assignment_target(&mut self, _node: &mut AssignmentTarget<'a>, _cx: &mut Cx<'a, '_>) {}
}

pub type PassCtor<'a> = fn() -> Box<dyn Pass<'a> + 'a>;
