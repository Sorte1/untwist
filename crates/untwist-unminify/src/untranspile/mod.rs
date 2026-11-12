use std::cell::Cell;

use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::{
    Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass, PassCtor, StatementKind,
    StatementVariants,
};
use untwist_core::semantic::{has_module_syntax, program_is_strict, scope_hazard};

mod async_fn;
mod builtins;
mod class;
mod class_ctor;
mod class_members;
mod class_native;
mod class_proto;
mod class_super;
mod define_chain;
mod destructure;
mod fingerprint;
mod generator;
pub mod identify;
mod machine;
mod machine_check;
mod machine_emit;
mod machine_read;
mod machine_tslib;
pub mod merge_member_declarations;
pub mod shapes;
mod spread;
mod spread_array;
mod spread_object;
mod structure;
mod structure_tidy;
mod sweep;
mod trust;
mod values;

use crate::notes::{
    UNTRANSPILE_ASYNC_KEPT, UNTRANSPILE_BUILTINS_WRITTEN, UNTRANSPILE_CLASS_PROTOTYPE_OPEN,
    UNTRANSPILE_HELPER_UNTRUSTED, UNTRANSPILE_MACHINE_KEPT, UNTRANSPILE_OBJECT_REST_KEPT,
    UNTRANSPILE_SCOPE_HAZARD, UNTRANSPILE_UNKNOWN_HELPER,
};
use builtins::Builtins;
use destructure::Site;
use identify::{Helper, Table};
use shapes::Kind;
use spread::Gate;

pub const PASS: &str = "untranspile-helpers";

const ASKING_ROUNDS: u8 = 2;

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::ArrowFunctionExpression,
    ExpressionKind::CallExpression,
    ExpressionKind::ClassExpression,
    ExpressionKind::SequenceExpression,
]);

const ACTED_ON_STATEMENTS: StatementVariants =
    StatementVariants::of(&[StatementKind::ClassDeclaration]);

#[derive(Clone, Copy)]
struct SweepRules {
    global: bool,
}

pub struct UntranspileHelpers {
    table: Table,
    scopes: Vec<ScopeId>,
    hazard: Option<bool>,
    module: bool,
    identified: bool,
    asked: Cell<bool>,
    asking_rounds: u8,
    builtins: Builtins,
    partial: bool,
    gate: Gate<'static>,
    dormant: bool,
    sites: Vec<Site>,
    object_rest: Vec<Kind>,
    open_classes: bool,
    open_waiting: bool,
    prototypes: class_proto::Prototypes,
    strict: bool,
    class_pending: u32,
    waiting: bool,
    promise_written: bool,
    keys: define_chain::Keys,
    define_ready: bool,
    run_lists: Vec<usize>,
    root_run: bool,
    define_fired: usize,
    define_quiet: bool,
    trust: Option<trust::Trust>,
    wants_trust: bool,
    identified_late: bool,
}

impl Default for UntranspileHelpers {
    fn default() -> Self {
        Self {
            table: Table::default(),
            scopes: Vec::new(),
            hazard: None,
            module: false,
            identified: false,
            asked: Cell::new(false),
            asking_rounds: 0,
            builtins: Builtins::Unknown,
            partial: false,
            gate: Gate::SHUT,
            dormant: false,
            sites: Vec::new(),
            object_rest: Vec::new(),
            open_classes: false,
            open_waiting: false,
            prototypes: class_proto::Prototypes::default(),
            strict: false,
            class_pending: 0,
            waiting: false,
            promise_written: false,
            keys: define_chain::Keys::default(),
            define_ready: false,
            run_lists: Vec::new(),
            root_run: false,
            define_fired: 0,
            define_quiet: false,
            trust: None,
            wants_trust: false,
            identified_late: false,
        }
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UntranspileHelpers::default())
}

pub fn untranspile_ctors<'a>() -> Vec<PassCtor<'a>> {
    vec![boxed]
}

fn unknown_helper(name: &str, span: Span, cx: &mut Cx<'_, '_>) {
    if let Some(kind) = shapes::named_kind(name) {
        cx.note(
            format!("{UNTRANSPILE_UNKNOWN_HELPER}: {kind:?}"),
            Some(span),
        );
    }
}

fn may_be_helper_name(name: &str) -> bool {
    name.as_bytes().first() == Some(&b'_')
}

fn is_named_helper(name: &str) -> bool {
    may_be_helper_name(name) && shapes::named_kind(name).is_some()
}

fn is_function_like(init: &Expression<'_>) -> bool {
    matches!(
        init.without_parentheses(),
        Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::LogicalExpression(_)
    )
}

fn sweep_shape(definition: &str, safety: &str) -> &'static str {
    match (definition, safety) {
        ("function", "safe") => "sweep/function/safe",
        ("function", _) => "sweep/function/unsafe",
        (_, "safe") => "sweep/declarator/safe",
        _ => "sweep/declarator/unsafe",
    }
}

fn note_named_declarators(node: &VariableDeclaration<'_>, cx: &mut Cx<'_, '_>) {
    for declarator in &node.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            continue;
        };
        if !is_named_helper(&id.name) || !identify::may_declare_helper(declarator) {
            continue;
        }
        let unknown = identify::declarator(declarator, cx.scoping(), true).is_none();
        if unknown && declarator.init.as_ref().is_some_and(is_function_like) {
            unknown_helper(id.name.as_str(), declarator.span, cx);
        }
    }
}

fn note_named_function(node: &Function<'_>, cx: &mut Cx<'_, '_>) {
    let Some(id) = node.id.as_ref().filter(|id| is_named_helper(&id.name)) else {
        return;
    };
    if identify::function_declaration(node, cx.scoping()).is_none() {
        unknown_helper(id.name.as_str(), node.span, cx);
    }
}

impl UntranspileHelpers {
    fn hazarded(&self) -> bool {
        self.hazard == Some(true)
    }

    fn open_gate(&self, unsafe_transforms: bool) -> Gate<'static> {
        if self.hazarded() {
            return Gate::SHUT;
        }
        Gate {
            safe: self.builtins.allows_safe(unsafe_transforms),
            unsafe_: unsafe_transforms && self.builtins.allows_unsafe(),
            asked: None,
            asks_unsafe: false,
        }
    }

    fn inert(&self, unsafe_transforms: bool) -> bool {
        if self.hazard != Some(false) || !self.gate.shut() || identify::recording() {
            return false;
        }
        match self.builtins {
            Builtins::Unknown => {
                !unsafe_transforms && self.asking_rounds > ASKING_ROUNDS && !self.asked.get()
            }
            _ => true,
        }
    }

    fn learn_builtins(&mut self, program: &Program<'_>, cx: &mut Cx<'_, '_>) {
        let rescan = self.partial && self.asked.get();
        if (self.builtins != Builtins::Unknown && !rescan) || !self.identified || self.hazarded() {
            return;
        }
        let unsafe_transforms = cx.opts.unsafe_transforms;
        if !unsafe_transforms && !self.asked.get() {
            return;
        }
        let scanned = builtins::scan(
            program,
            cx.scoping(),
            self.module,
            unsafe_transforms,
            &builtins::RELIED_ON,
        );
        self.builtins = scanned.verdict;
        self.partial = false;
        self.promise_written = scanned.promise_written;
        if self.builtins == Builtins::Written {
            cx.note(UNTRANSPILE_BUILTINS_WRITTEN, None);
        }
    }

    fn insert_trusted(&mut self, found: (SymbolId, Helper), declared: bool, cx: &mut Cx<'_, '_>) {
        let symbol = found.0;
        let scoping = cx.scoping();
        let global = !self.module && scoping.symbol_scope_id(symbol) == scoping.root_scope_id();
        if !declared && !global {
            self.insert(found, cx);
            return;
        }
        let Some(trust) = self.trust.as_ref() else {
            self.wants_trust = true;
            cx.request_round();
            return;
        };
        let trusted =
            (!declared || trust.settled(symbol)) && trust.global_kept(symbol, self.module, scoping);
        if trusted {
            self.identified_late |= !self.identified;
            self.insert(found, cx);
        } else {
            cx.note(UNTRANSPILE_HELPER_UNTRUSTED, None);
        }
    }

    fn insert(&mut self, (symbol, helper): (SymbolId, Helper), cx: &Cx<'_, '_>) {
        let scope = cx.scoping().symbol_scope_id(symbol);
        if !self.scopes.contains(&scope) {
            self.scopes.push(scope);
        }
        self.table.insert(symbol, helper);
        self.identified = true;
        cx.walk_whole_round();
    }

    fn sweep_rules(&self, scope: ScopeId, cx: &Cx<'_, '_>) -> Option<SweepRules> {
        if self.table.is_empty() || self.gate.shut() || !self.scopes.contains(&scope) {
            return None;
        }
        let at_script_root = !self.module && scope == cx.scoping().root_scope_id();
        if at_script_root && !self.gate.unsafe_ {
            return None;
        }
        Some(SweepRules {
            global: at_script_root,
        })
    }

    fn sweep<'a>(
        &self,
        statements: &mut AVec<'a, Statement<'a>>,
        scope: ScopeId,
        cx: &mut Cx<'a, '_>,
    ) {
        let Some(rules) = self.sweep_rules(scope, cx) else {
            return;
        };
        let swept = sweep::sweep(statements, &self.table, scope, cx);
        let safety = if rules.global { "unsafe" } else { "safe" };
        for _ in 0..swept.functions {
            shapes::fire(PASS, sweep_shape("function", safety));
        }
        for _ in 0..swept.declarators {
            shapes::fire(PASS, sweep_shape("declarator", safety));
        }
        cx.changed_by(swept.functions + swept.declarators);
    }

    fn collect_sites(&mut self, node: &VariableDeclaration<'_>, cx: &Cx<'_, '_>) {
        for declarator in &node.declarations {
            let Some((temp, safety)) =
                destructure::candidate(declarator, &self.table, cx.scoping())
            else {
                continue;
            };
            if self.builtins == Builtins::Unknown {
                if self.asking_rounds <= ASKING_ROUNDS {
                    Gate::asking(&self.asked, cx.opts.unsafe_transforms).allows(safety);
                }
            } else if self.gate.allows(safety) {
                self.sites.push(Site {
                    span: node.span,
                    temp,
                });
            }
        }
    }

    fn rewrite_sites<'a>(
        &mut self,
        statements: &mut AVec<'a, Statement<'a>>,
        span: Span,
        cx: &mut Cx<'a, '_>,
    ) {
        let mut sites = std::mem::take(&mut self.sites);
        let from = destructure::covered_suffix(&sites, span);
        for site in sites.drain(from..) {
            let rewritten =
                destructure::rewrite_site(statements, site, &self.table, &self.gate, cx);
            if let Some(shape) = rewritten {
                shapes::fire(PASS, shape);
                cx.changed();
            }
        }
        self.sites = sites;
    }

    fn note_object_rest(&mut self, cx: &mut Cx<'_, '_>) {
        if !self.dormant {
            self.object_rest.clear();
            for kind in self
                .table
                .kinds()
                .filter(|kind| shapes::is_object_rest(*kind))
            {
                if !self.object_rest.contains(&kind) {
                    self.object_rest.push(kind);
                }
            }
            self.object_rest.sort_unstable();
        }
        for kind in &self.object_rest {
            cx.note(format!("{UNTRANSPILE_OBJECT_REST_KEPT}: {kind:?}"), None);
        }
    }

    #[inline(never)]
    fn class_site<'a>(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let outcome = class::rewrite(
            node,
            self.strict,
            &self.table,
            &self.gate,
            &self.prototypes,
            cx,
        );
        if outcome.is_some() {
            self.class_pending -= 1;
        }
        match outcome {
            Some(class::Outcome::Fired(shape, fields)) => {
                shapes::fire(PASS, shape);
                for _ in 0..fields {
                    shapes::fire(PASS, "class/field/unsafe");
                }
                cx.changed();
                true
            }
            Some(class::Outcome::RefusedOpen) => {
                self.open_classes = true;
                false
            }
            Some(class::Outcome::Refused) | None => false,
        }
    }

    fn holds_extends(&self) -> bool {
        self.table
            .symbols()
            .any(|symbol| self.table.kind(symbol) == Some(Kind::Extends))
    }

    fn native_on(&self) -> bool {
        self.gate.unsafe_ && self.table.has_field_definers()
    }

    fn fire_native(moved: &class_native::Moved, cx: &mut Cx<'_, '_>) {
        let shape = if moved.derived {
            "class/native/derived/unsafe"
        } else {
            "class/native/base/unsafe"
        };
        shapes::fire(PASS, shape);
        for _ in 0..moved.fields {
            shapes::fire(PASS, "class/field/unsafe");
        }
        cx.changed();
    }

    #[inline(never)]
    fn native_site<'a>(&mut self, class: &mut Class<'a>, cx: &mut Cx<'a, '_>) {
        if self.native_on() {
            if let Some(moved) = class_native::rewrite(class, &self.table, cx) {
                Self::fire_native(&moved, cx);
            }
            return;
        }
        if self.may_wait(cx) && class_native::has_run(class, &self.table, cx.scoping()) {
            self.waiting = true;
        }
    }

    fn may_wait(&self, cx: &Cx<'_, '_>) -> bool {
        cx.opts.unsafe_transforms && self.builtins == Builtins::Unknown && !self.hazarded()
    }

    fn waits_for_builtins(&self) -> bool {
        self.builtins == Builtins::Unknown
            && self.identified
            && !self.hazarded()
            && (self.waiting || self.asked.get())
    }

    fn note_class_ctor(&mut self, node: &Function<'_>, cx: &Cx<'_, '_>) {
        if !self.table.has_classes()
            || !class_ctor::checks_its_class(node, &self.table, cx.scoping())
            || !(self.strict || node.has_use_strict_directive() || class::context_strict(cx))
        {
            return;
        }
        if self.gate.unsafe_ {
            self.class_pending = self.class_pending.saturating_add(1);
        } else {
            self.waiting = true;
        }
    }

    fn machines_on(&self) -> bool {
        self.table.has_machines() || self.table.has_runners()
    }

    #[inline(never)]
    fn machine_site<'a>(&mut self, node: &mut Function<'a>, cx: &mut Cx<'a, '_>) {
        let generator_site = generator::is_site(node, &self.table, cx.scoping());
        let async_site = !generator_site && async_fn::is_site(node, &self.table, cx.scoping());
        if !self.gate.unsafe_ {
            if (generator_site || async_site) && self.may_wait(cx) {
                self.waiting = true;
            }
            return;
        }
        let span = node.span;
        let shape = if generator_site {
            generator::rewrite(node, &self.table, cx)
        } else if async_site && !self.promise_written {
            async_fn::rewrite_function(node, &self.table, cx)
        } else {
            None
        };
        match shape {
            Some(shape) => {
                shapes::fire(PASS, shape);
                cx.changed();
            }
            None if generator_site => cx.note(UNTRANSPILE_MACHINE_KEPT, Some(span)),
            None if async_site && !self.promise_written => {
                cx.note(UNTRANSPILE_ASYNC_KEPT, Some(span))
            }
            None => {}
        }
    }

    #[inline(never)]
    fn arrow_site<'a>(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::ArrowFunctionExpression(arrow) = node else {
            return;
        };
        if !self.gate.unsafe_ || self.promise_written {
            return;
        }
        if let Some(shape) = async_fn::rewrite_arrow(arrow, &self.table, cx) {
            shapes::fire(PASS, shape);
            cx.changed();
        }
    }

    fn define_live(&self) -> bool {
        self.builtins != Builtins::Unknown && self.gate.safe
    }

    fn defines(&self) -> bool {
        !self.define_quiet && self.table.has_field_definers()
    }

    #[inline(never)]
    fn define_site<'a>(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) -> bool {
        let known = self.builtins != Builtins::Unknown;
        if self.hazarded() || (known && !self.gate.safe) {
            return false;
        }
        let site = define_chain::Site {
            table: &self.table,
            module: self.module,
            live: known,
        };
        match site.rewrite(node, &mut self.keys, cx) {
            define_chain::Outcome::Fired(shape) => {
                shapes::fire(PASS, shape);
                cx.changed();
                self.define_fired += 1;
                true
            }
            define_chain::Outcome::Ready => {
                self.define_ready = true;
                false
            }
            define_chain::Outcome::Kept => {
                if known
                    && define_chain::in_slot(cx.tcx.parent())
                    && site.runs_from(node, &self.keys, cx.scoping())
                {
                    self.hint_run(cx);
                }
                false
            }
        }
    }

    fn hint_run(&mut self, cx: &Cx<'_, '_>) {
        let Some((depth, root)) = define_chain::list_owner(cx) else {
            return;
        };
        self.root_run |= root;
        if !self.run_lists.contains(&depth) {
            self.run_lists.push(depth);
        }
    }

    fn statement_runs<'a>(&mut self, node: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.dormant || !self.define_live() || !self.defines() {
            return;
        }
        let site = define_chain::Site {
            table: &self.table,
            module: self.module,
            live: true,
        };
        let fired = site.statements(node, &mut self.keys, cx);
        for _ in 0..fired {
            shapes::fire(PASS, define_chain::RUN);
        }
        cx.changed_by(fired);
        self.define_fired += fired;
    }

    fn define_allowed(&mut self, program: &Program<'_>, cx: &Cx<'_, '_>) -> bool {
        if self.builtins != Builtins::Unknown {
            return self.builtins.allows_safe(cx.opts.unsafe_transforms);
        }
        if cx.opts.unsafe_transforms || self.asked.get() {
            return true;
        }
        let intact = builtins::intact(program, cx.scoping(), self.module, &builtins::RELIED_ON);
        self.builtins = if intact {
            Builtins::Intact
        } else {
            Builtins::Aliased
        };
        self.partial = !intact;
        intact
    }

    fn settle_define(&mut self, program: &Program<'_>, cx: &mut Cx<'_, '_>) {
        let ready = std::mem::take(&mut self.define_ready);
        let fired = std::mem::take(&mut self.define_fired);
        if !self.defines() || self.hazarded() {
            return;
        }
        self.keys.arm();
        let proven = self.ask_define(ready, program, cx);
        if !(ready || proven || fired > 0) {
            self.define_quiet = true;
        }
    }

    fn ask_define(&mut self, ready: bool, program: &Program<'_>, cx: &mut Cx<'_, '_>) -> bool {
        if !ready && !self.keys.waiting() {
            return false;
        }
        if !self.define_allowed(program, cx) {
            return false;
        }
        let proven = self.keys.waiting() && self.keys.prove(program, cx.scoping());
        if ready || proven {
            cx.request_round();
        }
        proven
    }

    fn identify_declarators(&mut self, node: &VariableDeclaration<'_>, cx: &mut Cx<'_, '_>) {
        for declarator in &node.declarations {
            if !identify::may_declare_helper(declarator) {
                continue;
            }
            let shared_trusted = self.module || cx.opts.unsafe_transforms;
            match identify::declarator(declarator, cx.scoping(), shared_trusted) {
                Some(found) => self.insert_trusted(found, true, cx),
                None if identify::declarator(declarator, cx.scoping(), true).is_some() => {
                    cx.note(UNTRANSPILE_HELPER_UNTRUSTED, None);
                }
                None => {
                    if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                        (&declarator.id, &declarator.init)
                    {
                        if may_be_helper_name(&id.name) && is_function_like(init) {
                            unknown_helper(id.name.as_str(), declarator.span, cx);
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Pass<'a> for UntranspileHelpers {
    fn name(&self) -> &'static str {
        PASS
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM
            | Interest::EXIT_PROGRAM
            | Interest::EXIT_FUNCTION
            | Interest::EXIT_VARIABLE_DECLARATION
            | Interest::EXIT_STATEMENTS
            | Interest::EXIT_STATEMENT
            | Interest::EXIT_EXPRESSION)
            .on_expressions(ACTED_ON_EXPRESSIONS)
            .on_statements(ACTED_ON_STATEMENTS)
    }

    fn locality(&self) -> Locality {
        if self.dormant || !self.identified {
            Locality::Local
        } else {
            Locality::Program
        }
    }

    fn retired(&self) -> Interest {
        if self.dormant {
            Interest::EXIT_EXPRESSION | Interest::EXIT_STATEMENT | Interest::EXIT_STATEMENTS
        } else if self.identified && (self.define_quiet || !self.define_live()) {
            Interest::EXIT_STATEMENTS
        } else {
            Interest::NONE
        }
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.table.clear();
        self.scopes.clear();
        self.module = has_module_syntax(program);
        self.strict = program_is_strict(program);
        self.trust = self.wants_trust.then(|| {
            let shared = self.module || cx.opts.unsafe_transforms;
            trust::gather(program, cx.scoping(), self.module, shared)
        });
        self.class_pending = 0;
        self.waiting = false;
        self.keys.begin_round();
        self.define_ready = false;
        self.run_lists.clear();
        self.root_run = false;
        self.define_fired = 0;
        if self.hazard.is_none() {
            self.hazard = Some(scope_hazard(cx.scoping()).is_some());
        }
        self.learn_builtins(program, cx);
        if self.identified && self.builtins == Builtins::Unknown {
            self.asking_rounds = self.asking_rounds.saturating_add(1);
        }
        self.gate = self.open_gate(cx.opts.unsafe_transforms);
        self.dormant = self.inert(cx.opts.unsafe_transforms);
        self.prototypes = if self.gate.unsafe_ && std::mem::take(&mut self.open_waiting) {
            class_proto::gather(program, cx.scoping())
        } else {
            class_proto::Prototypes::default()
        };
    }

    fn exit_program(&mut self, node: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.hazarded() && !self.table.is_empty() {
            cx.note(UNTRANSPILE_SCOPE_HAZARD, None);
        } else if !self.hazarded() {
            self.note_object_rest(cx);
        }
        if self.gate.unsafe_ && self.holds_extends() {
            self.open_classes = true;
        }
        let refused_open = std::mem::take(&mut self.open_classes);
        if refused_open {
            cx.note(UNTRANSPILE_CLASS_PROTOTYPE_OPEN, None);
        }
        self.open_waiting = (refused_open || !self.gate.unsafe_)
            && self.table.kinds().any(|kind| kind == Kind::CreateClassOpen);
        if std::mem::take(&mut self.root_run) {
            self.statement_runs(&mut node.body, cx);
        }
        if !self.sites.is_empty() {
            self.rewrite_sites(&mut node.body, node.span, cx);
        }
        let late = std::mem::take(&mut self.identified_late);
        if self.waits_for_builtins() || (late && self.may_wait(cx)) {
            cx.request_round();
        }
        self.settle_define(node, cx);
        let root = cx.scoping().root_scope_id();
        self.sweep(&mut node.body, root, cx);
        self.table = Table::default();
        self.scopes = Vec::new();
        self.sites = Vec::new();
    }

    fn exit_function(&mut self, node: &mut Function<'a>, cx: &mut Cx<'a, '_>) {
        if self.gate.unsafe_ || self.may_wait(cx) {
            self.note_class_ctor(node, cx);
        }
        if self.dormant {
            if node.is_declaration() {
                note_named_function(node, cx);
            }
            return;
        }
        if self.machines_on() {
            self.machine_site(node, cx);
        }
        if let (Some(scope), Some(body)) = (node.scope_id.get(), node.body.as_mut()) {
            if !self.sites.is_empty() {
                self.rewrite_sites(&mut body.statements, node.span, cx);
            }
            if self.scopes.contains(&scope) {
                self.sweep(&mut body.statements, scope, cx);
            }
        }
        if !node.is_declaration() {
            return;
        }
        match identify::function_declaration(node, cx.scoping()) {
            Some(found) => self.insert_trusted(found, false, cx),
            None => {
                if let Some(id) = node.id.as_ref().filter(|id| may_be_helper_name(&id.name)) {
                    unknown_helper(id.name.as_str(), node.span, cx);
                }
            }
        }
    }

    fn exit_variable_declaration(
        &mut self,
        node: &mut VariableDeclaration<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        if self.dormant {
            note_named_declarators(node, cx);
            return;
        }
        if node.declarations.iter().any(identify::may_declare_helper) {
            self.identify_declarators(node, cx);
        }
        if !self.define_quiet && (self.keys.armed() || self.table.has_field_definers()) {
            self.keys.record(node, cx.scoping());
        }
        if self.table.has_destructurers() {
            self.collect_sites(node, cx);
        }
    }

    fn exit_statements(&mut self, node: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.run_lists.is_empty() {
            return;
        }
        let depth = cx.tcx.ancestors_depth();
        let Some(at) = self.run_lists.iter().position(|listed| *listed == depth) else {
            return;
        };
        self.run_lists.swap_remove(at);
        self.statement_runs(node, cx);
    }

    fn exit_statement(&mut self, node: &mut Statement<'a>, cx: &mut Cx<'a, '_>) {
        if let Statement::ClassDeclaration(class) = node {
            if self.table.has_field_definers() {
                self.native_site(class, cx);
            }
        }
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if matches!(node, Expression::ArrowFunctionExpression(_)) {
            if self.table.has_runners() {
                self.arrow_site(node, cx);
            }
            return;
        }
        if let Expression::ClassExpression(class) = node {
            if self.table.has_field_definers() {
                self.native_site(class, cx);
            }
            return;
        }
        if self.class_pending > 0 && class::may_be_class(node) && self.class_site(node, cx) {
            return;
        }
        if self.defines()
            && define_chain::may_be_site(node, &self.table, cx.scoping())
            && self.define_site(node, cx)
        {
            return;
        }
        if !self.table.has_consumers() {
            return;
        }
        if self.builtins == Builtins::Unknown {
            if self.asking_rounds > ASKING_ROUNDS {
                return;
            }
            let asking = Gate::asking(&self.asked, cx.opts.unsafe_transforms);
            spread::rewrite(node, &self.table, &asking, cx);
            return;
        }
        if self.gate.shut() {
            return;
        }
        if let Some(shape) = spread::rewrite(node, &self.table, &self.gate, cx) {
            shapes::fire(PASS, shape);
            cx.changed();
        }
    }
}
