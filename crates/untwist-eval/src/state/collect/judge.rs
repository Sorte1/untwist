use oxc_ast::ast::{Expression, Function, IdentifierReference, MemberExpression};
use oxc_ast_visit::VisitJs;
use oxc_span::Span;
use rustc_hash::FxHashMap;

use super::prefix::prefix_gate_tests;
use super::scope::{declared_in_body, Binding, Declared, Frame, FrameKind, Freshness};
use super::template_arguments::ArgumentScope;
use super::uses::PendingUse;
use super::{unwrap_parens, Collected, FinalKey, MemberPeel, Trigger, TriggerKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Reach {
    Direct,
    Prototype,
    Deep,
}

pub(super) enum Deferred<'a> {
    Local {
        binding: usize,
        span: Span,
        reach: Reach,
    },
    Captured {
        binding: usize,
        span: Span,
        peel: Option<MemberPeel<'a>>,
    },
}

pub(super) struct Walker<'a, 'm> {
    pub(super) frames: Vec<Frame<'a>>,
    pub(super) bindings: Vec<Binding>,
    pub(super) binding_names: Vec<&'a str>,
    pub(super) regions: Vec<(Span, usize)>,
    pub(super) machinery: &'m [&'a str],
    pub(super) watched: &'m [&'a str],
    pub(super) local_callee: Option<usize>,
    pub(super) alias_of: Option<usize>,
    pub(super) outer_arity: Option<usize>,
    pub(super) index_read: Option<Span>,
    pub(super) inner_reads_arguments: bool,
    pub(super) inner_params: (usize, usize),
    pub(super) calling: bool,
    pub(super) pending_bare: Vec<PendingUse<'a>>,
    pub(super) pending_writes: Vec<(&'a str, &'a str, Option<usize>)>,
    pub(super) creation: Option<Span>,
    pub(super) exempt_self: Option<&'a str>,
    pub(super) call_target: Option<Span>,
    pub(super) arguments_exempt: Option<Span>,
    pub(super) deferred: Vec<Deferred<'a>>,
    pub(super) gate_refs: Vec<(usize, Span)>,
    pub(super) gate_tests: Vec<Span>,
    pub(super) captured: Vec<usize>,
    pub(super) strict: bool,
    pub(super) argument_scopes: Vec<ArgumentScope>,
    pub(super) out: Collected<'a>,
}

pub(super) fn is_this_rooted(expr: &Expression<'_>) -> bool {
    match unwrap_parens(expr) {
        Expression::ThisExpression(_) => true,
        other => match other.as_member_expression() {
            Some(member) => is_this_rooted(member.object()),
            None => false,
        },
    }
}

pub(super) fn is_bare_arguments(expr: &Expression<'_>) -> bool {
    matches!(unwrap_parens(expr), Expression::Identifier(id) if id.name.as_str() == "arguments")
}

pub(super) fn is_arguments_callee(member: &MemberExpression<'_>) -> bool {
    is_bare_arguments(member.object())
        && member.static_property_name().and_then(|name| name.as_str()) == Some("callee")
}

pub(super) fn root_of<'a, 'n>(
    member: &'n MemberExpression<'a>,
) -> Option<(&'n IdentifierReference<'a>, Reach)> {
    let object = unwrap_parens(member.object());
    if let Expression::Identifier(id) = object {
        return Some((id, Reach::Direct));
    }
    let inner = object.as_member_expression()?;
    let Expression::Identifier(id) = unwrap_parens(inner.object()) else {
        return None;
    };
    let is_prototype =
        inner.static_property_name().and_then(|name| name.as_str()) == Some("prototype");
    Some((
        id,
        if is_prototype {
            Reach::Prototype
        } else {
            Reach::Deep
        },
    ))
}

impl<'a, 'm> Walker<'a, 'm> {
    pub(super) fn new(
        machinery: &'m [&'a str],
        exempt_self: Option<&'a str>,
        call_target: Option<Span>,
    ) -> Self {
        Self {
            frames: Vec::new(),
            bindings: Vec::new(),
            binding_names: Vec::new(),
            regions: Vec::new(),
            machinery,
            watched: machinery,
            local_callee: None,
            alias_of: None,
            outer_arity: None,
            index_read: None,
            inner_reads_arguments: false,
            inner_params: (0, 0),
            calling: false,
            pending_bare: Vec::new(),
            pending_writes: Vec::new(),
            creation: None,
            exempt_self,
            call_target,
            arguments_exempt: None,
            deferred: Vec::new(),
            gate_refs: Vec::new(),
            gate_tests: Vec::new(),
            captured: Vec::new(),
            strict: false,
            argument_scopes: Vec::new(),
            out: Collected::default(),
        }
    }

    pub(super) fn enter_root(&mut self, function: &Function<'a>, kind: FrameKind) {
        let Some(body) = &function.body else {
            return;
        };
        let first = self.bindings.len();
        let outer = self.strict;
        let strict = outer || function.has_use_strict_directive();
        self.strict = strict;
        self.push_frame(
            kind,
            false,
            declared_in_body(&function.params, &body.statements, strict),
            body.span.start,
        );
        self.enter_arguments_scope(function, first);
        if kind == FrameKind::Call {
            prefix_gate_tests(&body.statements, &mut self.gate_tests);
        }
        self.visit_formal_parameters(&function.params);
        for statement in &body.statements {
            self.visit_statement(statement);
        }
        self.leave_arguments_scope();
        self.pop_frame();
        self.strict = outer;
    }

    pub(super) fn push_frame(
        &mut self,
        kind: FrameKind,
        is_arrow: bool,
        declared: Vec<Declared<'a>>,
        start: u32,
    ) {
        let mut names = self.bind_names(declared);
        if !is_arrow && !names.contains_key("arguments") {
            names.insert("arguments", self.bindings.len());
            let frame = self.frames.len();
            self.bindings
                .push(new_binding(frame, Freshness::Stale, Span::default(), 0));
            self.binding_names.push("arguments");
        }
        self.frames.push(Frame {
            names,
            kind,
            is_arrow,
            is_block: false,
            start,
        });
    }

    pub(super) fn push_block_frame(&mut self, declared: Vec<Declared<'a>>) {
        let kind = self.innermost_kind().unwrap_or(FrameKind::Nested);
        let start = self.frames.last().map_or(0, |frame| frame.start);
        let names = self.bind_names(declared);
        self.frames.push(Frame {
            names,
            kind,
            is_arrow: false,
            is_block: true,
            start,
        });
    }

    fn bind_names(&mut self, declared: Vec<Declared<'a>>) -> FxHashMap<&'a str, usize> {
        let frame = self.frames.len();
        let mut names: FxHashMap<&'a str, usize> = FxHashMap::default();
        for entry in declared {
            match names.get(entry.name) {
                Some(&existing) => {
                    self.bindings[existing].declarations += 1;
                    self.bindings[existing].fresh = Freshness::Stale;
                }
                None => {
                    names.insert(entry.name, self.bindings.len());
                    self.bindings
                        .push(new_binding(frame, entry.fresh, entry.at, 1));
                    self.binding_names.push(entry.name);
                }
            }
        }
        names
    }

    pub(super) fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub(super) fn resolve(&self, name: &str) -> Option<usize> {
        self.frames
            .iter()
            .rev()
            .find_map(|frame| frame.names.get(name).copied())
    }

    pub(super) fn innermost_kind(&self) -> Option<FrameKind> {
        self.frames.last().map(|frame| frame.kind)
    }

    pub(super) fn frame_kind_of(&self, binding: usize) -> FrameKind {
        self.frames
            .get(self.bindings[binding].frame)
            .map_or(FrameKind::Nested, |frame| frame.kind)
    }

    pub(super) fn this_is_nested(&self) -> bool {
        self.frames
            .iter()
            .rev()
            .find(|frame| !frame.is_arrow && !frame.is_block)
            .is_some_and(|frame| frame.kind == FrameKind::Nested)
    }

    pub(super) fn push_trigger(&mut self, span: Span, kind: TriggerKind<'a>) {
        self.out.triggers.push(Trigger { span, kind });
    }

    pub(super) fn in_region(&self) -> bool {
        !self.regions.is_empty()
    }

    pub(super) fn enter_region(&mut self, region: Span) {
        self.regions.push((region, self.frames.len()));
    }

    pub(super) fn leave_region(&mut self) {
        self.regions.pop();
    }

    pub(super) fn touch(&mut self, binding: usize, span: Span) {
        let entry = &mut self.bindings[binding];
        entry.first_use = entry.first_use.min(span.start);
        entry.last_use = entry.last_use.max(span.end);
    }

    pub(super) fn check_region(&mut self, binding: usize) {
        let frame = self.bindings[binding].frame;
        for &(region, depth) in &self.regions {
            if frame < depth {
                self.gate_refs.push((binding, region));
            }
        }
    }

    pub(super) fn note_reference(&mut self, id: &IdentifierReference<'a>, contained: bool) {
        let name = id.name.as_str();
        let Some(binding) = self.resolve(name) else {
            if !contained {
                self.note_bare_machinery(name, id.span);
            }
            return;
        };
        self.touch(binding, id.span);
        if self.binding_names.get(binding).copied() == Some("arguments") {
            self.note_arguments_read(binding, id.span);
        }
        self.check_template_arguments(binding, id.span);
        if !self.calling {
            self.bindings[binding].non_call_use = true;
        }
        if !contained && self.innermost_kind() != Some(FrameKind::Outer) {
            self.bindings[binding].bare_uses += 1;
        }
        if self.arguments_exempt != Some(id.span) {
            self.check_region(binding);
        }
    }

    pub(super) fn access_at(&self, span: Span) -> u32 {
        self.creation.unwrap_or(span).start
    }

    pub(super) fn creation_of(&self, span: Span, hoisted: bool) -> Span {
        match self.frames.last().filter(|_| hoisted) {
            Some(frame) => Span::new(frame.start, frame.start),
            None => span,
        }
    }

    pub(super) fn finish(mut self) -> Collected<'a> {
        self.judge_uses();
        self.judge_parameter_writes();
        let deferred = std::mem::take(&mut self.deferred);
        for entry in deferred {
            self.judge_deferred(entry);
        }
        let gate_refs = std::mem::take(&mut self.gate_refs);
        for (binding, region) in gate_refs {
            if !self.is_gate_local(binding, region) {
                self.push_trigger(region, TriggerKind::Unrecognized);
            }
        }
        self.out
    }

    fn judge_deferred(&mut self, entry: Deferred<'a>) {
        match entry {
            Deferred::Local {
                binding,
                span,
                reach,
            } => {
                if !self.is_fresh_target(binding, reach) {
                    self.push_trigger(span, TriggerKind::Unrecognized);
                }
            }
            Deferred::Captured {
                binding,
                span,
                peel,
            } => match peel.filter(|_| self.is_valid_capture(binding)) {
                Some(peel) => self.push_trigger(span, TriggerKind::CapturedMemo(peel, binding)),
                None => self.push_trigger(span, TriggerKind::Unrecognized),
            },
        }
    }

    fn is_fresh_target(&self, binding: usize, reach: Reach) -> bool {
        let entry = &self.bindings[binding];
        let reach_ok = match reach {
            Reach::Direct => true,
            Reach::Prototype => entry.fresh == Freshness::Function,
            Reach::Deep => false,
        };
        reach_ok
            && entry.fresh != Freshness::Stale
            && entry.declarations == 1
            && entry.writes == 0
            && entry.bare_uses == 0
    }

    fn is_valid_capture(&self, binding: usize) -> bool {
        let entry = &self.bindings[binding];
        let ignored = crate::mutants::captured_static_uses_ignored();
        let static_use = !ignored
            && self.out.accesses.iter().any(|access| {
                access.binding == Some(binding)
                    && (access.peel.segment.is_some()
                        || matches!(access.peel.final_key, FinalKey::Static(_)))
            });
        self.captured.contains(&binding) && entry.writes == 1 && entry.bare_uses == 0 && !static_use
    }

    fn is_gate_local(&self, binding: usize, region: Span) -> bool {
        let entry = &self.bindings[binding];
        let inside = |start: u32, end: u32| region.start <= start && end <= region.end;
        entry.fresh == Freshness::Function
            && entry.declarations == 1
            && entry.writes == 0
            && inside(entry.declared_at.start, entry.declared_at.end)
            && inside(entry.first_use, entry.last_use)
    }
}

fn new_binding(frame: usize, fresh: Freshness, at: Span, declarations: u32) -> Binding {
    Binding {
        frame,
        fresh,
        declared_at: at,
        declarations,
        writes: 0,
        bare_uses: 0,
        non_call_use: false,
        first_use: u32::MAX,
        last_use: 0,
    }
}
