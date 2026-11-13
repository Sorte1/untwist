use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::base_name::{base_name, structural_base, Context, FALLBACK_BASE};
use crate::collect::{BindingInfo, BindingRole, Collected, InitKind};
use crate::evidence::{collect_evidence, Evidence};
use crate::generate::{lower_camel, to_identifier, uid_candidates};
use crate::human::looks_human_written;
use crate::oracle::{NameOracle, OuterRule};
use crate::plan::{run_mode, HazardNote, RenamePlan};
use crate::simple::{is_renameable, unit_roots};
use crate::text_reads::{text_read_bindings_with, SelfReads, TextReads};
use crate::usage::{collect_naming_usage, Usage};
use crate::RenameConfig;

pub const NUMBER_LIMIT: u32 = 4096;

const SHORT_NAME: usize = 2;

const HEX_TOKEN: usize = 16;

const STRUCTURAL_FUNCTION: &str = "fn";

const CALL_RESULT: &str = "result";

const CONSTRUCTED: &str = "instance";

const CONSTRUCTOR: &str = "Ctor";

const UNNAMED_CALLABLES: [&str; 2] = ["fn", "cb"];

fn is_call_result(info: &BindingInfo) -> bool {
    matches!(&info.role, BindingRole::Variable(shape) if shape.kind == InitKind::Call)
}

fn is_constructed(info: &BindingInfo) -> bool {
    matches!(&info.role, BindingRole::Variable(shape) if shape.kind == InitKind::New)
}

pub struct Smart {
    config: RenameConfig,
    done: bool,
}

pub fn boxed<'a>(config: RenameConfig) -> Box<dyn Pass<'a> + 'a> {
    Box::new(Smart {
        config,
        done: false,
    })
}

impl<'a> Pass<'a> for Smart {
    fn name(&self) -> &'static str {
        "rename-smart"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_PROGRAM
    }

    fn exit_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let config = smart_config(self.config);
        let self_reads = lane_self_reads(cx.opts.unsafe_transforms);
        run_mode(
            &mut self.done,
            program,
            cx,
            HazardNote::Report,
            OuterRule::UsedEnclosing,
            move |program, collected, scoping| {
                decide_in(program, collected, scoping, (config, self_reads))
            },
        );
    }
}

pub fn lane_self_reads(unsafe_transforms: bool) -> SelfReads {
    if unsafe_transforms {
        return SelfReads::InstanceMethodsReadInstances;
    }
    SelfReads::Unknown
}

pub fn smart_config(config: RenameConfig) -> RenameConfig {
    RenameConfig {
        rename_all: false,
        ..config
    }
}

fn has_strong_base(info: &BindingInfo, collected: &Collected) -> bool {
    if collected.loop_counters.contains_key(&info.symbol) {
        return true;
    }
    match &info.role {
        BindingRole::ImportDefault(_)
        | BindingRole::ImportNamespace(_)
        | BindingRole::ImportNamed { .. }
        | BindingRole::PatternProperty(_) => true,
        BindingRole::Variable(shape) => shape.kind == InitKind::Require,
        _ => false,
    }
}

pub fn is_hex_token(name: &str) -> bool {
    name.len() >= HEX_TOKEN && name.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn smart_renameable(info: &BindingInfo, collected: &Collected, config: RenameConfig) -> bool {
    let hex_token = !collected.pinned.contains(&info.symbol) && is_hex_token(&info.name);
    hex_token || is_renameable(info, collected, config)
}

pub fn smart_base(info: &BindingInfo, cx: &Context<'_>, evidence: &Evidence) -> Option<String> {
    let evidenced = (!has_strong_base(info, cx.collected))
        .then(|| evidence.name(info.symbol))
        .flatten()
        .map(to_identifier)
        .filter(|name| !name.is_empty() && name != "_");
    let base = evidenced
        .or_else(|| instance_base(info, cx, evidence))
        .unwrap_or_else(|| {
            let derived = derived_base(info, cx);
            let unnamed_constructor = evidence.is_constructor(info.symbol)
                && UNNAMED_CALLABLES.contains(&derived.as_str());
            if unnamed_constructor {
                return CONSTRUCTOR.to_string();
            }
            derived
        });
    let unevidenced = base == FALLBACK_BASE
        && (info.name.chars().count() <= SHORT_NAME || is_hex_token(&info.name));
    (!unevidenced).then_some(base)
}

fn instance_base(info: &BindingInfo, cx: &Context<'_>, evidence: &Evidence) -> Option<String> {
    let BindingRole::Variable(shape) = &info.role else {
        return None;
    };
    if shape.kind != InitKind::New {
        return None;
    }
    let callee = shape.symbol?;
    let binding = cx
        .collected
        .bindings
        .get(*cx.collected.by_symbol.get(&callee)?)?;
    if !smart_renameable(binding, cx.collected, cx.config) {
        return None;
    }
    let name = to_identifier(&lower_camel(evidence.name(callee)?));
    (!name.is_empty() && name != "_").then_some(name)
}

fn derived_base(info: &BindingInfo, cx: &Context<'_>) -> String {
    let base = base_name(info, cx);
    if base == STRUCTURAL_FUNCTION && is_call_result(info) {
        return CALL_RESULT.to_string();
    }
    if base == STRUCTURAL_FUNCTION && is_constructed(info) {
        return CONSTRUCTED.to_string();
    }
    let counter = cx.collected.loop_counters.contains_key(&info.symbol);
    if counter || looks_human_written(&base) {
        return base;
    }
    structural_base(info)
        .filter(|structural| looks_human_written(structural))
        .unwrap_or_else(|| FALLBACK_BASE.to_string())
}

pub fn decide(
    program: &Program<'_>,
    collected: &Collected,
    scoping: &Scoping,
    config: RenameConfig,
) -> RenamePlan {
    decide_in(program, collected, scoping, (config, SelfReads::Unknown))
}

pub fn decide_in(
    program: &Program<'_>,
    collected: &Collected,
    scoping: &Scoping,
    settings: (RenameConfig, SelfReads),
) -> RenamePlan {
    decide_with_limit(program, collected, scoping, settings, NUMBER_LIMIT)
}

fn decide_with_limit(
    program: &Program<'_>,
    collected: &Collected,
    scoping: &Scoping,
    (config, self_reads): (RenameConfig, SelfReads),
    limit: u32,
) -> RenamePlan {
    let usage = collect_naming_usage(program, scoping);
    let evidence = collect_evidence(program, scoping);
    let text_read = match text_read_bindings_with(program, scoping, self_reads) {
        TextReads::Bindings(pinned) => pinned,
        TextReads::Unknown => return refused(),
    };
    let cx = Context {
        collected,
        usage: &usage,
        namer: None,
        config,
    };
    let candidates: Vec<&BindingInfo> = collected
        .bindings
        .iter()
        .filter(|info| !text_read.contains(&info.symbol))
        .filter(|info| smart_renameable(info, collected, config))
        .collect();
    let mut planned: Vec<(&BindingInfo, String)> = candidates
        .iter()
        .filter_map(|info| smart_base(info, &cx, &evidence).map(|base| (*info, base)))
        .collect();
    let flowed = flowed_hex_tokens(&candidates, &planned, (collected, &evidence, &usage));
    planned.extend(flowed);
    let mut oracle = NameOracle::new(scoping, collected);
    oracle.defer(planned.iter().map(|(info, _)| info.symbol));
    let units = unit_roots(scoping);
    let mut counters: FxHashMap<(ScopeId, String), u32> = FxHashMap::default();
    for (info, base) in planned {
        let unit = units.get(&info.scope).copied().unwrap_or(info.scope);
        if assign(&mut oracle, &mut counters, (unit, info), &base, limit).is_none() {
            return aborted(&info.name);
        }
    }
    oracle.into_plan()
}

fn flowed_hex_tokens<'c>(
    candidates: &[&'c BindingInfo],
    planned: &[(&BindingInfo, String)],
    (collected, evidence, usage): (&Collected, &Evidence, &Usage),
) -> Vec<(&'c BindingInfo, String)> {
    let bases: FxHashMap<SymbolId, &str> = planned
        .iter()
        .map(|(info, base)| (info.symbol, base.as_str()))
        .collect();
    let target_base = |target: SymbolId| -> Option<String> {
        if let Some(key) = usage.object_literal_key(target) {
            return Some(to_identifier(key));
        }
        if let Some(base) = bases.get(&target) {
            return Some((*base).to_string());
        }
        let binding = collected.bindings.get(*collected.by_symbol.get(&target)?)?;
        looks_human_written(&binding.name).then(|| binding.name.clone())
    };
    candidates
        .iter()
        .filter(|info| is_hex_token(&info.name) && !bases.contains_key(&info.symbol))
        .filter_map(|info| {
            let targets = evidence.flows_into(info.symbol);
            let mut names = targets.iter().map(|target| target_base(*target));
            let first = names.next()??;
            let agreed = names.all(|name| name.as_deref() == Some(first.as_str()));
            let usable = agreed && first != FALLBACK_BASE && !is_hex_token(&first);
            usable.then_some((*info, first))
        })
        .collect()
}

fn candidate(base: &str, number: u32) -> String {
    if number <= 1 {
        return base.to_string();
    }
    format!("{base}{number}")
}

fn assign(
    oracle: &mut NameOracle<'_>,
    counters: &mut FxHashMap<(ScopeId, String), u32>,
    (unit, info): (ScopeId, &BindingInfo),
    base: &str,
    limit: u32,
) -> Option<String> {
    let key = (unit, base.to_string());
    let start = counters.get(&key).copied().unwrap_or(1);
    match pick(oracle, info.symbol, base, start, limit) {
        Some((name, number)) => {
            counters.insert(key, number.saturating_add(1));
            oracle.take(info.symbol, std::iter::once(name))
        }
        None => {
            counters.insert(key, start.saturating_add(limit));
            let current = oracle.current_name(info.symbol).to_string();
            oracle.take(info.symbol, uid_candidates(&current))
        }
    }
}

fn pick(
    oracle: &NameOracle<'_>,
    symbol: SymbolId,
    base: &str,
    start: u32,
    limit: u32,
) -> Option<(String, u32)> {
    (start..start.saturating_add(limit))
        .map(|number| (candidate(base, number), number))
        .find(|(name, _)| oracle.is_available(name, symbol))
}

pub const TEXT_READ_UNKNOWN: &str = "a function reads its own source through this, callee or caller, so smart renaming left the program unrenamed";

fn refused() -> RenamePlan {
    let mut plan = RenamePlan::new();
    plan.notes.push(TEXT_READ_UNKNOWN.to_string());
    plan
}

fn aborted(name: &str) -> RenamePlan {
    let mut plan = RenamePlan::new();
    plan.notes.push(format!(
        "ran out of names for {name}; leaving the program unrenamed"
    ));
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect::collect;
    use crate::RenameMode;
    use oxc_allocator::Allocator;
    use untwist_core::parse::parse_program;
    use untwist_core::semantic::build_scoping;

    #[test]
    fn exhausting_the_numbers_and_the_fallback_aborts_the_mode() {
        let alloc = Allocator::default();
        let mut source = String::from("q;");
        for index in 2..1000 {
            source.push_str(&format!("q{index};"));
        }
        source.push_str("try { q; } catch (q) { q; }");
        let parsed = parse_program(&alloc, &source);
        let sem = build_scoping(&parsed.program);
        let collected = collect(&parsed.program, &sem.scoping, OuterRule::UsedEnclosing);
        let config = RenameConfig {
            mode: RenameMode::Smart,
            rename_all: false,
            extended: true,
        };
        let settings = (config, SelfReads::Unknown);
        let plan = decide_with_limit(&parsed.program, &collected, &sem.scoping, settings, 0);
        assert!(plan.is_empty(), "the mode must rename nothing");
        assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    }
}
