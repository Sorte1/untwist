mod rewrite;
mod shape;
mod sites;

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{
    build_scoping, has_module_syntax, is_foreign_global, is_shared_global, may_call_eval,
};

use self::rewrite::{take_flat, Install, Sites as SiteEdits, Taken};
use self::shape::{Flat, Wrapper};
use self::sites::Sites;
use super::ast::Ctx;
use super::refs::{names_observed, scan, unpatched_outside};
use super::resolution::{holds, record_function, Expected};
use crate::calls::agnostic::{global_reach, ReachWatch};

pub const PASS: &str = "inline-js-confuser-flatten";

const HINT: &str = "(...";

static RELIED_ON: Reliance = Reliance {
    roots: &["Array", "Symbol"],
    members: &[],
};

#[derive(Default)]
pub struct InlineFlatten {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineFlatten::default())
}

struct Collect<'s> {
    cx: Ctx<'s>,
    accessors: FxHashSet<Span>,
    wrappers: Vec<Wrapper>,
}

impl<'a> VisitJs<'a> for Collect<'_> {
    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        if it.kind != MethodDefinitionKind::Method {
            self.accessors.insert(it.value.span);
        }
        walk_js::walk_method_definition(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.kind != PropertyKind::Init {
            if let Expression::FunctionExpression(function) = &it.value {
                self.accessors.insert(function.span);
            }
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if !self.accessors.contains(&it.span) {
            if let Some(wrapper) = shape::wrapper(&self.cx, it) {
                self.wrappers.push(wrapper);
            }
        }
        walk_js::walk_function(self, it, flags);
    }
}

struct Plan {
    flat_statement: Span,
    strict: bool,
    wrapper: Span,
    name: String,
    calls: bool,
    sites: Sites,
}

struct Analysis {
    plans: Vec<Plan>,
    expected: Expected,
    notes: Vec<String>,
}

fn flats<'k, 'a>(
    program: &'k Program<'a>,
    cx: &Ctx<'_>,
) -> FxHashMap<SymbolId, (Flat, &'k Function<'a>)> {
    program
        .body
        .iter()
        .filter_map(|statement| {
            let flat = shape::flat(cx, statement)?;
            let Statement::FunctionDeclaration(function) = statement else {
                return None;
            };
            Some((flat.symbol, (flat, &**function)))
        })
        .collect()
}

fn single_use(refs: &super::refs::Refs<'_>, symbol: SymbolId, scoping: &Scoping) -> bool {
    let uses = refs.of(symbol);
    uses.len() == 1 && !uses[0].write && scoping.symbol_redeclarations(symbol).is_empty()
}

fn analyse(program: &Program<'_>, scoping: &Scoping) -> Analysis {
    let mut analysis = Analysis {
        plans: Vec::new(),
        expected: Expected::default(),
        notes: Vec::new(),
    };
    if has_module_syntax(program) || !program.source_text.contains(HINT) {
        return analysis;
    }
    let cx = Ctx { scoping };
    let flats = flats(program, &cx);
    if flats.is_empty() {
        return analysis;
    }
    let mut collect = Collect {
        cx: Ctx { scoping },
        accessors: FxHashSet::default(),
        wrappers: Vec::new(),
    };
    collect.visit_program(program);
    let refs = scan(program, scoping);
    if refs.has_with || may_call_eval(scoping) {
        analysis.notes.push(
            "js-confuser flatten kept: the program has a with statement or may call eval"
                .to_string(),
        );
        return analysis;
    }
    let mut used: FxHashSet<SymbolId> = FxHashSet::default();
    for wrapper in collect.wrappers {
        let Some((flat, function)) = flats.get(&wrapper.flat) else {
            continue;
        };
        let name = scoping.symbol_name(flat.symbol).to_string();
        let reason = if !used.insert(flat.symbol) {
            Some("a flattened function has two wrappers")
        } else if !single_use(&refs, flat.symbol, scoping)
            || !single_use(&refs, wrapper.object, scoping)
            || !single_use(&refs, wrapper.args, scoping)
        {
            Some("the flattened function or its scope object is used elsewhere")
        } else if flat.strict != wrapper.scope_strict {
            Some("the flattened function and its wrapper differ in strictness")
        } else if is_shared_global(&name) || is_foreign_global(&name) {
            Some("another input names the flattened function")
        } else {
            None
        };
        if let Some(reason) = reason {
            analysis
                .notes
                .push(format!("js-confuser flatten kept: {reason}"));
            continue;
        }
        let sites = sites::scan(Ctx { scoping }, function, flat.object, &wrapper.props);
        if sites.bad {
            analysis.notes.push(
                "js-confuser flatten kept: the scope object is used other than to read, write or call a variable"
                    .to_string(),
            );
            continue;
        }
        record_function(function, scoping, &mut analysis.expected, Some(flat.object));
        analysis.plans.push(Plan {
            flat_statement: flat.span,
            strict: flat.strict,
            wrapper: wrapper.span,
            name,
            calls: wrapper.props.values().any(|p| p.kind == shape::Use::Call),
            sites,
        });
    }
    analysis
}

fn apply<'a>(
    program: &mut Program<'a>,
    builder: &AstBuilder<'a>,
    plans: &[Plan],
) -> Option<Vec<(Span, SymbolId)>> {
    let mut edits = SiteEdits {
        builder,
        expressions: FxHashMap::default(),
        targets: FxHashMap::default(),
        created: Vec::new(),
        failed: false,
    };
    for plan in plans {
        edits
            .expressions
            .extend(plan.sites.expressions.iter().map(|(k, v)| (*k, v.clone())));
        edits
            .targets
            .extend(plan.sites.targets.iter().map(|(k, v)| (*k, v.clone())));
    }
    edits.visit_program(program);
    if edits.failed || !edits.expressions.is_empty() || !edits.targets.is_empty() {
        return None;
    }
    let by_statement: FxHashMap<Span, &Plan> =
        plans.iter().map(|p| (p.flat_statement, p)).collect();
    let mut wrappers: FxHashMap<Span, Taken<'a>> = FxHashMap::default();
    let taken: Vec<Statement<'a>> = program.body.drain(..).collect();
    let mut failed = false;
    for statement in taken {
        let plan = by_statement.get(&statement.span());
        match (plan, statement) {
            (Some(plan), Statement::FunctionDeclaration(mut function)) => {
                match take_flat(&mut function, plan.strict, builder) {
                    Some(flat) => {
                        wrappers.insert(plan.wrapper, flat);
                    }
                    None => failed = true,
                }
            }
            (_, statement) => program.body.push(statement),
        }
    }
    let mut install = Install {
        builder,
        wrappers,
        failed,
    };
    install.visit_program(program);
    (!install.failed && install.wrappers.is_empty()).then_some(edits.created)
}

fn reached(trial: &Program<'_>, scoping: &Scoping, removed: &[&str]) -> bool {
    let watch = ReachWatch {
        names: removed,
        props: removed,
        fns: &[],
        observe: true,
    };
    let reach = global_reach(trial, scoping, (None, &[]), watch);
    reach.reach.is_some() || reach.reach_by_code.is_some() || names_observed(&reach)
}

impl<'a> Pass<'a> for InlineFlatten {
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
        let mut analysis = analyse(program, cx.scoping());
        for note in &analysis.notes {
            cx.note(note.clone(), None);
        }
        if analysis.plans.is_empty() {
            return;
        }
        let mut trial = program.clone_in(cx.allocator());
        let Some(created) = apply(&mut trial, cx.ast(), &analysis.plans) else {
            cx.note(
                "js-confuser flatten kept: the functions could not be put back in place",
                None,
            );
            return;
        };
        for (span, target) in created {
            let declaration = cx.scoping().symbol_span(target);
            analysis.expected.insert(span, Some(declaration));
        }
        let restored = build_scoping(&trial).scoping;
        if !analysis.expected.is_empty() && !holds(&trial, &restored, &analysis.expected) {
            cx.note("js-confuser flatten kept: a name in the flattened body would bind differently in its wrapper", None);
            return;
        }
        let calls = analysis.plans.iter().any(|p| p.calls);
        if !cx.opts.unsafe_transforms
            && calls
            && !unpatched_outside(&trial, &restored, &RELIED_ON, &[])
        {
            cx.note("js-confuser flatten kept: the program may replace a standard built-in the flattened calls use (Array, Symbol)", None);
            return;
        }
        let removed: Vec<&str> = analysis.plans.iter().map(|p| p.name.as_str()).collect();
        if reached(&trial, &restored, &removed) {
            cx.note("js-confuser flatten kept: code may reach or observe the flattened functions through the global object or code text", None);
            return;
        }
        if apply(program, cx.ast(), &analysis.plans).is_none() {
            return;
        }
        *cx.scoping_mut() = build_scoping(program).scoping;
        cx.binding_moved();
        cx.changed_by(analysis.plans.len());
        cx.note(
            format!(
                "{} js-confuser flattened function(s) put back in place",
                analysis.plans.len()
            ),
            None,
        );
    }
}
