pub mod helpers;
pub mod seal;
pub mod shape;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::collect::Collected;
use crate::generate::{lower_camel, uid_candidates};
use crate::human::looks_human_written;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::simple::is_renameable;
use crate::smart::smart_config;
use crate::{RenameConfig, RenameMode};
use helpers::{Helper, HelperFinder, COMMONJS_MIN};
use oxc_ast_visit::VisitJs;
use oxc_syntax::scope::ScopeId;
use seal::sealed;
pub use shape::{wrappers, Holder, Wrapper};

pub const PASS: &str = "name-commonjs-wrappers";
const FALLBACK_STEM: &str = "module";
const REQUIRE: &str = "require";
const HAS_REQUIRED: &str = "hasRequired";
const FACTORY_PREFIX: &str = "require_";
const MODULE_PARAM: &str = "module";
const EXPORTS_PARAM: &str = "exports";
const SUFFIX_LIMIT: u32 = 64;

pub struct NameWrappers {
    done: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(NameWrappers { done: false })
}

impl<'a> Pass<'a> for NameWrappers {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Suppress,
            OuterRule::UsedEnclosing,
            decide,
        );
    }
}

fn config() -> RenameConfig {
    smart_config(RenameConfig {
        mode: RenameMode::Smart,
        ..RenameConfig::default()
    })
}

fn upper_first(stem: &str) -> String {
    let mut chars = stem.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn after_prefix(name: &str, prefix: &str) -> Option<String> {
    let rest = name.strip_prefix(prefix)?;
    rest.chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
        .then(|| lower_camel(rest))
}

fn holder_stem(name: &str) -> Option<String> {
    let trimmed = name
        .trim_end_matches(|ch: char| ch.is_ascii_digit())
        .strip_suffix('$')
        .unwrap_or(name);
    (trimmed.chars().count() > 2 && looks_human_written(trimmed)).then(|| lower_camel(trimmed))
}

fn evidenced_stem(scoping: &Scoping, wrapper: &Wrapper) -> Option<String> {
    let function = wrapper
        .function
        .and_then(|symbol| after_prefix(scoping.symbol_name(symbol), REQUIRE));
    function
        .or_else(|| after_prefix(scoping.symbol_name(wrapper.flag), HAS_REQUIRED))
        .or_else(|| {
            wrapper
                .holder
                .and_then(|holder| holder_stem(scoping.symbol_name(holder.symbol())))
        })
}

fn stem(scoping: &Scoping, wrapper: &Wrapper, number: usize) -> String {
    evidenced_stem(scoping, wrapper).unwrap_or_else(|| format!("{FALLBACK_STEM}{number}"))
}

struct Wanted {
    symbol: SymbolId,
    base: String,
    function: bool,
}

impl Wanted {
    fn of(symbol: SymbolId, base: String) -> Self {
        Self {
            symbol,
            base,
            function: false,
        }
    }
}

fn snake_stem(name: &str) -> Option<String> {
    let rest = name.strip_prefix(FACTORY_PREFIX)?;
    let mut out = String::new();
    let mut upper = false;
    for ch in rest.chars() {
        match ch {
            '_' => upper = !out.is_empty(),
            _ if upper => {
                out.extend(ch.to_uppercase());
                upper = false;
            }
            _ => out.push(ch),
        }
    }
    (!out.is_empty() && looks_human_written(&out)).then_some(out)
}

fn factory_wanted(scoping: &Scoping, wrapper: &Wrapper, number: usize) -> Vec<Wanted> {
    let Some((exports, module)) = wrapper.factory else {
        return Vec::new();
    };
    let stem = snake_stem(scoping.symbol_name(wrapper.flag))
        .unwrap_or_else(|| format!("{FALLBACK_STEM}{number}"));
    let mut out = vec![Wanted::of(
        wrapper.flag,
        format!("{REQUIRE}{}", upper_first(&stem)),
    )];
    out.extend(exports.map(|exports| Wanted::of(exports, EXPORTS_PARAM.to_string())));
    out.extend(module.map(|module| Wanted::of(module, MODULE_PARAM.to_string())));
    out
}

fn wanted(
    scoping: &Scoping,
    wrapper: &Wrapper,
    number: usize,
    calls: &dyn Fn(Option<ScopeId>) -> bool,
) -> Vec<Wanted> {
    if wrapper.factory.is_some() {
        return factory_wanted(scoping, wrapper, number);
    }
    let stem = stem(scoping, wrapper, number);
    let upper = upper_first(&stem);
    let mut out = vec![Wanted::of(wrapper.flag, format!("{HAS_REQUIRED}{upper}"))];
    out.extend(
        wrapper
            .holder
            .map(|holder| Wanted::of(holder.symbol(), stem.clone())),
    );
    if let Some(function) = wrapper.function.filter(|_| calls(wrapper.scope)) {
        out.push(Wanted {
            symbol: function,
            base: format!("{REQUIRE}{upper}"),
            function: true,
        });
    }
    if let Some((module, exports)) = wrapper.params {
        out.push(Wanted::of(module, MODULE_PARAM.to_string()));
        out.extend(exports.map(|exports| Wanted::of(exports, EXPORTS_PARAM.to_string())));
    }
    out
}

fn candidates(base: &str) -> impl Iterator<Item = String> + '_ {
    std::iter::once(base.to_string())
        .chain((2..SUFFIX_LIMIT).map(move |number| format!("{base}{number}")))
}

fn renameable(collected: &Collected, wanted: &Wanted) -> bool {
    let Some(&at) = collected.by_symbol.get(&wanted.symbol) else {
        return false;
    };
    let info = &collected.bindings[at];
    let name_only = wanted.function && collected.name_pinned.contains(&wanted.symbol);
    is_renameable(info, collected, config()) || (name_only && !looks_human_written(&info.name))
}

pub fn may_hold_wrappers(program: &Program<'_>) -> bool {
    let mut helpers = HelperFinder::default();
    helpers.visit_program(program);
    !helpers.found.is_empty() || shape::may_hold_wrappers(program)
}

fn helper_wanted(helper: &Helper, calls: &dyn Fn(Option<ScopeId>) -> bool) -> Option<Wanted> {
    let function = helper.scope.is_some();
    (!function || calls(helper.scope)).then(|| Wanted {
        symbol: helper.symbol,
        base: helper.name.to_string(),
        function,
    })
}

pub fn decide(program: &Program<'_>, collected: &Collected, scoping: &Scoping) -> RenamePlan {
    let mut helpers = HelperFinder::default();
    helpers.visit_program(program);
    let factories = helpers
        .found
        .iter()
        .filter(|helper| helper.name == COMMONJS_MIN)
        .map(|helper| helper.symbol)
        .collect();
    let found = wrappers(program, scoping, factories);
    if found.is_empty() && helpers.found.is_empty() {
        return RenamePlan::new();
    }
    let callbacks: Vec<ScopeId> = found
        .iter()
        .filter(|wrapper| wrapper.factory.is_some())
        .filter_map(|wrapper| wrapper.scope)
        .collect();
    let sealed = sealed(program, scoping, &callbacks);
    let calls = |scope: Option<ScopeId>| scope.is_some_and(|scope| sealed.contains(scope));
    let planned: Vec<Wanted> = helpers
        .found
        .iter()
        .filter_map(|helper| helper_wanted(helper, &calls))
        .chain(
            found
                .iter()
                .enumerate()
                .flat_map(|(index, wrapper)| wanted(scoping, wrapper, index + 1, &calls)),
        )
        .filter(|wanted| sealed.hides(scoping, wanted.symbol) && renameable(collected, wanted))
        .collect();
    let mut oracle = NameOracle::new(scoping, collected);
    oracle.defer(planned.iter().map(|wanted| wanted.symbol));
    for wanted in &planned {
        if oracle.is_assigned(wanted.symbol) {
            continue;
        }
        if take(&mut oracle, wanted).is_none() {
            return aborted(scoping.symbol_name(wanted.symbol));
        }
    }
    let mut plan = oracle.into_plan().without_unchanged(scoping);
    plan.notes
        .push(found_note(found.len(), helpers.found.len(), plan.len()));
    plan
}

fn take(oracle: &mut NameOracle<'_>, wanted: &Wanted) -> Option<String> {
    oracle
        .take(wanted.symbol, candidates(&wanted.base))
        .or_else(|| {
            let current = oracle.current_name(wanted.symbol).to_string();
            oracle.take(wanted.symbol, uid_candidates(&current))
        })
}

fn aborted(name: &str) -> RenamePlan {
    let mut plan = RenamePlan::new();
    plan.notes.push(format!(
        "ran out of names for {name}; left the commonjs wrappers unnamed"
    ));
    plan
}

pub fn found_note(count: usize, helpers: usize, named: usize) -> String {
    format!("found {count} commonjs wrappers and {helpers} interop helpers; named {named} bindings")
}
