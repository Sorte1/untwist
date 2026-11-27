mod hazards;
mod rewrite;
mod shape;
mod sites;

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{
    build_scoping, has_module_syntax, is_foreign_global, is_shared_global, may_call_eval,
};

use self::rewrite::{Restore, Rewrite};
use self::shape::{member, Ctx, Key, Machine};
use self::sites::{Kind, Sites};
use super::refs::{inside, names_observed, scan, shared_elsewhere, unpatched_outside, Refs};
use crate::calls::agnostic::{global_reach, ReachWatch};

pub const PASS: &str = "inline-js-confuser-dispatcher";

const HINT: &str = "apply";

const NON_CALL_NAME: &str = "fn";

static RELIED_ON: Reliance = Reliance {
    roots: &["Object", "Array", "Function", "Symbol"],
    members: &[],
};

#[derive(Default)]
pub struct InlineDispatcher {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineDispatcher::default())
}

struct Candidate {
    machine: Machine,
    top: bool,
    declarations: Vec<Span>,
}

struct Lists<'s> {
    cx: Ctx<'s>,
    found: Vec<Candidate>,
    stubs: FxHashSet<SymbolId>,
}

fn creates_null_object(cx: &Ctx<'_>, init: Option<&Expression<'_>>) -> bool {
    let Some(Expression::CallExpression(call)) = init.map(Expression::without_parentheses) else {
        return false;
    };
    let Some((object, Key::Str(create))) = member(&call.callee) else {
        return false;
    };
    let global_object = matches!(object.without_parentheses(), Expression::Identifier(id) if id.name == "Object")
        && cx.symbol(object).is_none();
    global_object
        && create == "create"
        && !call.optional
        && matches!(call.arguments.as_slice(), [Argument::NullLiteral(_)])
}

fn declarations(
    cx: &Ctx<'_>,
    statements: &[Statement<'_>],
    machine: &Machine,
) -> Option<Vec<Span>> {
    let mut spans = Vec::new();
    let (mut cache, mut payload) = (false, false);
    for statement in statements {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        let symbols: Vec<Option<SymbolId>> = declaration
            .declarations
            .iter()
            .map(|d| match &d.id {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            })
            .collect();
        let ours = |s: &Option<SymbolId>| *s == Some(machine.cache) || *s == Some(machine.payload);
        if !symbols.iter().any(ours) {
            continue;
        }
        if declaration.kind != VariableDeclarationKind::Var || !symbols.iter().all(ours) {
            return None;
        }
        for (declarator, symbol) in declaration.declarations.iter().zip(&symbols) {
            match *symbol == Some(machine.cache) {
                true if !cache && creates_null_object(cx, declarator.init.as_ref()) => cache = true,
                false if !payload && declarator.init.is_none() => payload = true,
                _ => return None,
            }
        }
        spans.push(declaration.span);
    }
    (cache && payload).then_some(spans)
}

impl Lists<'_> {
    fn list(&mut self, statements: &[Statement<'_>], top: bool) {
        for statement in statements {
            let Some(machine) = shape::dispatcher(&self.cx, statement) else {
                continue;
            };
            if let Some(declarations) = declarations(&self.cx, statements, &machine) {
                self.found.push(Candidate {
                    machine,
                    top,
                    declarations,
                });
            }
        }
    }
}

impl<'a> VisitJs<'a> for Lists<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.list(&it.body, true);
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.list(&it.statements, false);
        walk_js::walk_function_body(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let empty = it.params.items.is_empty()
            && it.params.rest.is_none()
            && it
                .body
                .as_ref()
                .is_some_and(|b| b.statements.is_empty() && b.directives.is_empty());
        if let (true, true, Some(symbol)) = (
            empty,
            it.is_declaration(),
            it.id.as_ref().and_then(|id| id.symbol_id.get()),
        ) {
            self.stubs.insert(symbol);
        }
        walk_js::walk_function(self, it, flags);
    }
}

struct Planned {
    candidate: Candidate,
    sites: Sites,
    non_call: FxHashSet<String>,
}

fn sorted(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_unstable_by_key(|s| (s.start, s.end));
    spans
}

fn stable(symbol: SymbolId, scoping: &Scoping) -> bool {
    scoping.symbol_redeclarations(symbol).is_empty()
}

fn accounted(refs: &Refs<'_>, symbol: SymbolId, expected: Vec<Span>) -> bool {
    let found: Vec<Span> = refs.of(symbol).iter().map(|r| r.span).collect();
    sorted(found) == sorted(expected)
}

fn fn_name_free(scoping: &Scoping, machine: &Machine) -> bool {
    !scoping
        .root_unresolved_references()
        .contains_key(NON_CALL_NAME)
        && scoping.symbol_ids().all(|symbol| {
            scoping.symbol_name(symbol) != NON_CALL_NAME
                || inside(machine.span, scoping.symbol_span(symbol))
        })
}

fn non_call_kept(
    machine: &Machine,
    non_call: &FxHashSet<String>,
    stub: bool,
    scoping: &Scoping,
) -> bool {
    if non_call.is_empty() {
        return false;
    }
    let lengths_hold = machine
        .entries
        .iter()
        .filter(|e| non_call.contains(&e.key))
        .all(|e| stub || machine.lengths.get(&e.key).copied().unwrap_or(0.0) == 0.0);
    non_call.len() > 1 || !lengths_hold || !fn_name_free(scoping, machine)
}

fn check(
    program: &Program<'_>,
    scoping: &Scoping,
    refs: &Refs<'_>,
    candidate: Candidate,
    stubs: &FxHashSet<SymbolId>,
) -> Result<Planned, &'static str> {
    let machine = &candidate.machine;
    let sites = sites::scan(program, Ctx { scoping }, machine);
    let symbols = [machine.dispatcher, machine.payload, machine.cache];
    if !symbols.iter().all(|s| stable(*s, scoping)) {
        return Err("its names are declared twice");
    }
    if refs.of(machine.dispatcher).iter().any(|r| r.write)
        || !accounted(refs, machine.dispatcher, sites.dispatcher_refs.clone())
    {
        return Err("the dispatcher is used other than through its call forms");
    }
    let payload: Vec<Span> = machine
        .payload_refs
        .iter()
        .chain(&sites.payload_refs)
        .copied()
        .collect();
    if !accounted(refs, machine.payload, payload) {
        return Err("the payload variable is used other than to pass arguments");
    }
    let cache = refs.of(machine.cache);
    if cache.is_empty() || cache.iter().any(|r| !inside(machine.span, r.span)) {
        return Err("the function cache is used outside the dispatcher");
    }
    let keys: FxHashSet<&str> = machine.entries.iter().map(|e| e.key.as_str()).collect();
    let used = sites
        .standalone
        .iter()
        .map(|s| s.key.as_str())
        .chain(sites.pairs.iter().map(|p| p.key.as_str()));
    if used.clone().any(|key| !keys.contains(key)) {
        return Err("a call names a function the dispatcher does not hold");
    }
    let non_call: FxHashSet<String> = sites
        .standalone
        .iter()
        .filter(|s| s.kind == Kind::NonCall)
        .map(|s| s.key.clone())
        .collect();
    let stub = stubs.contains(&machine.setter);
    if non_call_kept(machine, &non_call, stub, scoping) {
        return Err("a function it hands out as a value has a different name or length than the function it would become");
    }
    Ok(Planned {
        candidate,
        sites,
        non_call,
    })
}

struct Analysis {
    planned: Vec<Planned>,
    notes: Vec<String>,
}

fn analyse(program: &Program<'_>, scoping: &Scoping) -> Analysis {
    let mut analysis = Analysis {
        planned: Vec::new(),
        notes: Vec::new(),
    };
    if has_module_syntax(program) || !program.source_text.contains(HINT) {
        return analysis;
    }
    let mut lists = Lists {
        cx: Ctx { scoping },
        found: Vec::new(),
        stubs: FxHashSet::default(),
    };
    lists.visit_program(program);
    if lists.found.is_empty() {
        return analysis;
    }
    let note = |reason: &str| format!("js-confuser dispatcher kept: {reason}");
    let refs = scan(program, scoping);
    if refs.has_with || may_call_eval(scoping) {
        analysis
            .notes
            .push(note("the program has a with statement or may call eval"));
        return analysis;
    }
    let spans: Vec<Span> = lists.found.iter().map(|c| c.machine.span).collect();
    for candidate in lists.found {
        let span = candidate.machine.span;
        if spans
            .iter()
            .any(|outer| *outer != span && inside(*outer, span))
        {
            continue;
        }
        match check(program, scoping, &refs, candidate, &lists.stubs) {
            Ok(planned) => analysis.planned.push(planned),
            Err(reason) => analysis.notes.push(note(reason)),
        }
    }
    analysis
}

fn base_name(key: &str) -> String {
    match key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !key.is_empty()
    {
        true => format!("fn_{key}"),
        false => "dispatched".to_string(),
    }
}

fn names<'a>(planned: &Planned, cx: &mut Cx<'a, '_>) -> Option<Vec<String>> {
    planned
        .candidate
        .machine
        .entries
        .iter()
        .map(|entry| match planned.non_call.contains(&entry.key) {
            true => Some(NON_CALL_NAME.to_string()),
            false => cx.invent_name(&base_name(&entry.key)),
        })
        .collect()
}

fn shared(
    program: &Program<'_>,
    scoping: &Scoping,
    nested_root: bool,
    (planned, fresh): (&Planned, &[String]),
) -> bool {
    let machine = &planned.candidate.machine;
    shared_elsewhere(
        program,
        scoping,
        nested_root,
        &[machine.dispatcher, machine.payload, machine.cache],
    ) || fresh
        .iter()
        .any(|name| is_shared_global(name) || is_foreign_global(name))
}

fn reached(trial: &Program<'_>, scoping: &Scoping, removed: &[String], fresh: &[String]) -> bool {
    let names: Vec<&str> = removed.iter().chain(fresh).map(String::as_str).collect();
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &[],
        observe: true,
    };
    let reach = global_reach(trial, scoping, (None, &[]), watch);
    reach.reach.is_some() || reach.reach_by_code.is_some() || names_observed(&reach)
}

fn removed_names(scoping: &Scoping, planned: &Planned) -> Vec<String> {
    let machine = &planned.candidate.machine;
    [machine.dispatcher, machine.payload, machine.cache]
        .iter()
        .map(|s| scoping.symbol_name(*s).to_string())
        .collect()
}

fn rewrite<'b, 'a>(
    builder: &'b oxc_ast::builder::AstBuilder<'a>,
    accepted: &[(&Planned, Vec<String>)],
) -> (Rewrite<'b, 'a>, usize) {
    let mut rewrite = Rewrite {
        builder,
        standalone: FxHashMap::default(),
        pairs: FxHashMap::default(),
        machines: FxHashMap::default(),
        removals: FxHashSet::default(),
        applied: 0,
        failed: false,
    };
    let mut expected = 0;
    for (planned, names) in accepted {
        let machine = &planned.candidate.machine;
        let name_of = |key: &str| {
            machine
                .entries
                .iter()
                .position(|e| e.key == key)
                .map(|at| names[at].clone())
                .unwrap_or_default()
        };
        for site in &planned.sites.standalone {
            rewrite
                .standalone
                .insert(site.span, (name_of(&site.key), site.kind));
            expected += 1;
        }
        for pair in &planned.sites.pairs {
            rewrite
                .pairs
                .entry(pair.sequence)
                .or_default()
                .push((pair.index, name_of(&pair.key)));
            expected += 1;
        }
        for span in &planned.candidate.declarations {
            rewrite.removals.insert(*span);
            expected += 1;
        }
        rewrite.machines.insert(
            machine.span,
            Restore {
                fns_span: machine.fns_span,
                names: names.clone(),
                exact: machine
                    .entries
                    .iter()
                    .map(|e| planned.non_call.contains(&e.key))
                    .collect(),
            },
        );
        expected += 1;
    }
    (rewrite, expected)
}

fn apply<'a>(
    program: &mut Program<'a>,
    cx: &Cx<'a, '_>,
    accepted: &[(&Planned, Vec<String>)],
) -> Option<usize> {
    let (mut rewrite, expected) = rewrite(cx.ast(), accepted);
    rewrite.visit_program(program);
    let complete = !rewrite.failed
        && rewrite.applied == expected
        && rewrite.standalone.is_empty()
        && rewrite.pairs.is_empty()
        && rewrite.machines.is_empty()
        && rewrite.removals.is_empty();
    complete.then_some(expected)
}

fn judged_names(index: usize, planned: &Planned, taken: &FxHashSet<&str>) -> Option<Vec<String>> {
    planned
        .candidate
        .machine
        .entries
        .iter()
        .enumerate()
        .map(|(at, entry)| match planned.non_call.contains(&entry.key) {
            true => Some(NON_CALL_NAME.to_string()),
            false => {
                let name = format!("__jsc_judged_{index}_{at}");
                (!taken.contains(name.as_str())).then_some(name)
            }
        })
        .collect()
}

fn removed_and_fresh(
    scoping: &Scoping,
    accepted: &[(&Planned, Vec<String>)],
) -> (Vec<String>, Vec<String>) {
    accepted
        .iter()
        .filter(|(planned, _)| planned.candidate.top)
        .fold(
            (Vec::new(), Vec::new()),
            |(mut removed, mut fresh), (planned, names)| {
                removed.extend(removed_names(scoping, planned));
                fresh.extend(names.iter().cloned());
                (removed, fresh)
            },
        )
}

pub(in crate::js_confuser) fn undone<'a>(
    program: &Program<'a>,
    scoping: &Scoping,
    cx: &Cx<'a, '_>,
) -> Option<(Program<'a>, Scoping)> {
    let analysis = analyse(program, scoping);
    if analysis.planned.is_empty() {
        return None;
    }
    let unresolved = scoping.root_unresolved_references();
    let taken: FxHashSet<&str> = scoping
        .symbol_names()
        .chain(unresolved.keys().map(|k| k.as_str()))
        .collect();
    let mut accepted = Vec::new();
    for (index, planned) in analysis.planned.iter().enumerate() {
        let names = judged_names(index, planned, &taken)?;
        if planned.candidate.top && shared(program, scoping, cx.opts.nested_root, (planned, &names))
        {
            return None;
        }
        accepted.push((planned, names));
    }
    let mut trial = program.clone_in(cx.allocator());
    apply(&mut trial, cx, &accepted)?;
    let restored = build_scoping(&trial).scoping;
    if !cx.opts.unsafe_transforms && !unpatched_outside(&trial, &restored, &RELIED_ON, &[]) {
        return None;
    }
    let (removed, fresh) = removed_and_fresh(scoping, &accepted);
    if !removed.is_empty() && reached(&trial, &restored, &removed, &fresh) {
        return None;
    }
    Some((trial, restored))
}

impl<'a> Pass<'a> for InlineDispatcher {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn reset(&mut self) {
        self.settled = false;
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.settled {
            return;
        }
        self.settled = true;
        let analysis = analyse(program, cx.scoping());
        for note in &analysis.notes {
            cx.note(note.clone(), None);
        }
        let mut accepted = Vec::new();
        for planned in &analysis.planned {
            let Some(fresh) = names(planned, cx) else {
                cx.note(
                    "js-confuser dispatcher kept: no free name for a restored function",
                    None,
                );
                continue;
            };
            if planned.candidate.top
                && shared(
                    program,
                    cx.scoping(),
                    cx.opts.nested_root,
                    (planned, &fresh),
                )
            {
                cx.note("js-confuser dispatcher kept: another input names its machinery or a name it would add", None);
                continue;
            }
            accepted.push((planned, fresh));
        }
        if accepted.is_empty() {
            return;
        }
        let mut trial = program.clone_in(cx.allocator());
        if apply(&mut trial, cx, &accepted).is_none() {
            cx.note(
                "js-confuser dispatcher kept: the restored functions could not be put in place",
                None,
            );
            return;
        }
        let restored = build_scoping(&trial).scoping;
        if !cx.opts.unsafe_transforms && !unpatched_outside(&trial, &restored, &RELIED_ON, &[]) {
            cx.note("js-confuser dispatcher kept: the program may replace a standard built-in the dispatcher uses (Object, Array, Function, Symbol)", None);
            return;
        }
        let (removed, fresh) = removed_and_fresh(cx.scoping(), &accepted);
        if !removed.is_empty() && reached(&trial, &restored, &removed, &fresh) {
            cx.note("js-confuser dispatcher kept: code may reach or observe its names through the global object or code text", None);
            return;
        }
        let Some(applied) = apply(program, cx, &accepted) else {
            return;
        };
        *cx.scoping_mut() = build_scoping(program).scoping;
        cx.binding_moved();
        cx.changed_by(applied);
        cx.note(
            format!(
                "{} js-confuser dispatcher(s) inlined back into functions",
                accepted.len()
            ),
            None,
        );
    }
}
