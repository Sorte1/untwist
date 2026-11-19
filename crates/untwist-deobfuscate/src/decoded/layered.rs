use oxc_ast::ast::{
    ArrayExpression, AssignmentExpression, AssignmentTarget, CallExpression, Expression, Function,
    Program,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use super::conflict::prototype_conflict;
use super::values::reaches_intrinsics;
use crate::bridge::DecodeRequest;
use crate::calls::agnostic::{global_reach, guard_writes, ReachWatch};
use crate::legacy_reach::{self, Layered};
use crate::proof::{alias_refusal, alias_sites, builtin_aliases, points_to_sites, UsedNames};
use crate::root_reach::root_alias_names;
use untwist_core::ast_util::global::global_is_written;
use untwist_eval::object::{ProtoRead, PROTOTYPE_LINKS};

const TOKEN_PREFIX: &str = "layered:";

const MIN_LATENT_ENTRIES: usize = 2;

#[derive(Default)]
struct Frame {
    name: Option<String>,
    replaced: bool,
    calls: bool,
    array: bool,
}

struct LatentArray<'c> {
    calls: &'c FxHashSet<Span>,
    frames: Vec<Frame>,
    found: bool,
}

impl LatentArray<'_> {
    fn entry(&self, expr: &Expression<'_>) -> Option<bool> {
        match expr.without_parentheses() {
            Expression::StringLiteral(_) => Some(false),
            Expression::CallExpression(call) => self.calls.contains(&call.span).then_some(true),
            Expression::StaticMemberExpression(member) => {
                matches!(member.object, Expression::Identifier(_)).then_some(true)
            }
            Expression::ComputedMemberExpression(member) => {
                let keyed = self.entry(&member.expression).is_some();
                (matches!(member.object, Expression::Identifier(_)) && keyed).then_some(true)
            }
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                Some(self.entry(&binary.left)? | self.entry(&binary.right)?)
            }
            _ => None,
        }
    }

    fn holds_latent_entries(&self, array: &ArrayExpression<'_>) -> bool {
        if array.elements.len() < MIN_LATENT_ENTRIES {
            return false;
        }
        let mut latent = false;
        for element in &array.elements {
            let Some(entry) = element.as_expression().and_then(|e| self.entry(e)) else {
                return false;
            };
            latent |= entry;
        }
        latent
    }

    fn close_frame(&mut self) {
        if let Some(frame) = self.frames.pop() {
            self.found |= frame.replaced && frame.calls && frame.array;
        }
    }

    fn top(&mut self) -> &mut Frame {
        if self.frames.is_empty() {
            self.frames.push(Frame::default());
        }
        let last = self.frames.len() - 1;
        &mut self.frames[last]
    }
}

impl<'a> VisitJs<'a> for LatentArray<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.found {
            return;
        }
        self.frames.push(Frame {
            name: it.id.as_ref().map(|id| id.name.to_string()),
            ..Frame::default()
        });
        walk_js::walk_function(self, it, flags);
        self.close_frame();
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(target) = &it.left {
            let replaces = matches!(
                it.right.without_parentheses(),
                Expression::FunctionExpression(_)
            );
            let frame = self.top();
            if replaces && frame.name.as_deref() == Some(target.name.as_str()) {
                frame.replaced = true;
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.calls.contains(&it.span) {
            self.top().calls = true;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        if self.holds_latent_entries(it) {
            self.top().array = true;
        }
        walk_js::walk_array_expression(self, it);
    }
}

fn holds_latent_layer(program: &Program<'_>, request: &DecodeRequest<'_>) -> bool {
    let calls: FxHashSet<Span> = request.calls.iter().map(|call| call.span).collect();
    let mut latent = LatentArray {
        calls: &calls,
        frames: vec![Frame::default()],
        found: false,
    };
    latent.visit_program(program);
    latent.close_frame();
    latent.found
}

fn root_names(
    program: &Program<'_>,
    scoping: &Scoping,
    request: &DecodeRequest<'_>,
) -> Vec<String> {
    let root = scoping.root_scope_id();
    let roots: Vec<_> = request
        .guard
        .symbols
        .iter()
        .filter(|(symbol, _)| scoping.symbol_scope_id(*symbol) == root)
        .collect();
    let targets: FxHashSet<_> = roots.iter().map(|(symbol, _)| *symbol).collect();
    let aliases = root_alias_names(program, scoping, &targets);
    roots
        .iter()
        .map(|(_, name)| *name)
        .chain(aliases)
        .map(str::to_string)
        .collect()
}

pub(super) fn speculate(
    (program, scoping): (&Program<'_>, &Scoping),
    request: &DecodeRequest<'_>,
) -> Option<String> {
    if !legacy_reach::defers() || !holds_latent_layer(program, request) {
        return None;
    }
    let names = root_names(program, scoping, request);
    let first = names.first()?;
    let token = format!("{TOKEN_PREFIX}{first}");
    let layered = Layered {
        names,
        used: None,
        points_to: false,
        both_sites: false,
        late_reads: None,
        roots: Vec::new(),
    };
    legacy_reach::defer_layered(token.clone(), layered);
    Some(token)
}

pub(super) fn speculate_aliases(
    (program, scoping): (&Program<'_>, &Scoping),
    request: &DecodeRequest<'_>,
    (speculated, points_to): (Option<String>, bool),
) -> Option<String> {
    let token = speculated.or_else(|| speculate((program, scoping), request))?;
    let used = request
        .guard
        .used
        .names
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    legacy_reach::widen_layered(&token, used, points_to);
    Some(token)
}

pub(super) enum Waive<'w> {
    Late(&'w [ProtoRead]),
    Root(&'w str),
}

pub(super) fn speculate_waived(
    (program, scoping): (&Program<'_>, &Scoping),
    request: &DecodeRequest<'_>,
    speculated: &mut Option<String>,
    waive: Waive<'_>,
) -> bool {
    if speculated.is_none() {
        *speculated = speculate((program, scoping), request);
    }
    let Some(token) = speculated else {
        return false;
    };
    match waive {
        Waive::Late(reads) => legacy_reach::widen_late(token, reads),
        Waive::Root(name) => legacy_reach::widen_root(token, name),
    }
    true
}

fn intrinsics_hold(program: &Program<'_>, scoping: &Scoping, machinery: &[Span]) -> bool {
    if global_is_written(scoping, "Object") {
        return false;
    }
    let found = guard_writes(program, scoping, (None, machinery), None);
    let known = |span: Span| {
        found
            .extra
            .iter()
            .find(|(marker, _)| *marker == span)
            .map(|(_, text)| text.clone())
    };
    !found
        .writes
        .iter()
        .any(|write| reaches_intrinsics(write, &known))
}

fn roots_hold(
    program: &Program<'_>,
    scoping: &Scoping,
    roots: &[String],
    machinery: &[Span],
) -> bool {
    if roots.iter().any(|root| root == PROTOTYPE_LINKS)
        && !intrinsics_hold(program, scoping, machinery)
    {
        return false;
    }
    let roots: Vec<&str> = roots
        .iter()
        .map(String::as_str)
        .filter(|root| *root != PROTOTYPE_LINKS)
        .collect();
    if roots.iter().any(|root| global_is_written(scoping, root)) {
        return false;
    }
    let watch = ReachWatch {
        names: &roots,
        props: &roots,
        fns: &[],
        observe: false,
    };
    global_reach(program, scoping, (None, machinery), watch)
        .reach
        .is_none()
}

fn late_writes_hold(
    program: &Program<'_>,
    scoping: &Scoping,
    reads: &[ProtoRead],
    machinery: &[Span],
) -> bool {
    let found = guard_writes(program, scoping, (None, machinery), None);
    prototype_conflict(reads, &found.writes, &FxHashMap::default()).is_none()
}

fn machinery_spans(program: &Program<'_>) -> Vec<Span> {
    crate::removal::machinery_units(program)
        .into_iter()
        .flat_map(|unit| {
            unit.array
                .into_iter()
                .chain(unit.rotator)
                .chain(unit.decoders)
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn settle(program: &Program<'_>, scoping: &Scoping) {
    let mut machinery: Option<Vec<Span>> = None;
    legacy_reach::settle_layered(|layered| {
        let machinery = machinery.get_or_insert_with(|| machinery_spans(program));
        let names: Vec<&str> = layered.names.iter().map(String::as_str).collect();
        let watch = ReachWatch {
            names: &names,
            props: &names,
            fns: &[],
            observe: false,
        };
        let found = global_reach(program, scoping, (None, machinery), watch);
        if found.reach.is_some() {
            return false;
        }
        if !roots_hold(program, scoping, &layered.roots, machinery) {
            return false;
        }
        if let Some(reads) = &layered.late_reads {
            if !late_writes_hold(program, scoping, reads, machinery) {
                return false;
            }
        }
        let Some(used) = &layered.used else {
            return true;
        };
        let used = UsedNames {
            names: used.iter().map(String::as_str).collect(),
        };
        let kinds = match layered.both_sites {
            true => vec![true, false],
            false => vec![layered.points_to],
        };
        kinds.into_iter().all(|points_to| {
            let sites = match points_to {
                true => points_to_sites(program, scoping, &FxHashMap::default(), &[]),
                false => alias_sites(
                    program,
                    &builtin_aliases(program, scoping, FxHashMap::default()),
                    &[],
                ),
            };
            alias_refusal(&sites, &used, program.source_text).is_none()
        })
    });
}

pub fn settle_observations(program: &Program<'_>, scoping: &Scoping) {
    let mut machinery: Option<Vec<Span>> = None;
    legacy_reach::settle_observations(|names| {
        let machinery = machinery.get_or_insert_with(|| machinery_spans(program));
        let watch = ReachWatch {
            names,
            props: names,
            fns: &[],
            observe: true,
        };
        global_reach(program, scoping, (None, machinery), watch)
            .observed
            .is_some()
    });
}
