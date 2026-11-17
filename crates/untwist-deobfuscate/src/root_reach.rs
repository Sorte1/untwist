use oxc_ast::ast::{BindingPattern, IdentifierReference, Program, Statement};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::Cx;
use untwist_core::semantic::has_module_syntax;

use crate::calls::agnostic::names_reached_quietly;
use crate::decoded::syntactic_global_reach;
use crate::legacy_reach;
use crate::mutants::root_alias_reach_trusted;
use crate::removal::alias_values;
use crate::stamp::original_name;

const TOKEN_PREFIX: &str = "root-names:";

pub(crate) fn root_names_reached(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    names: &[&str],
) -> Option<Span> {
    let script = !cx.opts.nested_root && !has_module_syntax(program);
    if names.is_empty() || !script || root_alias_reach_trusted() {
        return None;
    }
    let scoping = cx.scoping();
    if cx.opts.unsafe_transforms {
        let strings = FxHashMap::default();
        return syntactic_global_reach((program, scoping), &strings, names, &[]);
    }
    names_reached_quietly(program, scoping, names)
}

pub(crate) struct ReachedRoots {
    pub bindings: FxHashSet<SymbolId>,
    pub targets: FxHashSet<SymbolId>,
}

fn root_symbols(scoping: &Scoping, symbols: &[SymbolId]) -> Vec<SymbolId> {
    let root = scoping.root_scope_id();
    symbols
        .iter()
        .copied()
        .filter(|symbol| scoping.symbol_scope_id(*symbol) == root)
        .collect()
}

fn names_of<'s>(source_text: &'s str, scoping: &'s Scoping, symbols: &[SymbolId]) -> Vec<&'s str> {
    let mut names: Vec<&str> = symbols
        .iter()
        .map(|symbol| scoping.symbol_name(*symbol))
        .chain(
            symbols
                .iter()
                .filter_map(|symbol| original_name(source_text, scoping, *symbol)),
        )
        .filter(|name| is_identifier_name(name))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

pub(crate) fn reached_roots(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    targets: &[SymbolId],
    bindings: &[SymbolId],
) -> Option<ReachedRoots> {
    if bindings.is_empty() {
        return None;
    }
    let scoping = cx.scoping();
    let targets = root_symbols(scoping, targets);
    let bindings = root_symbols(scoping, bindings);
    let target_names = names_of(program.source_text, scoping, &targets);
    let mut names = names_of(program.source_text, scoping, &bindings);
    names.extend_from_slice(&target_names);
    names.sort_unstable();
    names.dedup();
    root_names_reached(program, cx, &names)?;
    if settled_at_stage_end(cx, &names) {
        return None;
    }
    let targets_reached =
        bindings.is_empty() || root_names_reached(program, cx, &target_names).is_some();
    Some(ReachedRoots {
        bindings: bindings.into_iter().collect(),
        targets: targets.into_iter().filter(|_| targets_reached).collect(),
    })
}

fn settled_at_stage_end(cx: &Cx<'_, '_>, names: &[&str]) -> bool {
    if cx.opts.unsafe_transforms || !legacy_reach::defers() {
        return false;
    }
    let token = format!("{TOKEN_PREFIX}{}", names.join(","));
    let names = names.iter().map(|name| (*name).to_string()).collect();
    legacy_reach::defer_names(token, names);
    true
}

pub(crate) fn root_alias_names<'p>(
    program: &'p Program<'_>,
    scoping: &Scoping,
    targets: &FxHashSet<SymbolId>,
) -> Vec<&'p str> {
    if targets.is_empty() || root_alias_reach_trusted() || has_module_syntax(program) {
        return Vec::new();
    }
    let holds_a_target = |value: &IdentifierReference<'_>| {
        value
            .reference_id
            .get()
            .and_then(|reference| scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| targets.contains(&symbol))
    };
    let mut names = Vec::new();
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            let (BindingPattern::BindingIdentifier(id), Some(init)) =
                (&declarator.id, &declarator.init)
            else {
                continue;
            };
            if alias_values(init).into_iter().any(holds_a_target) {
                names.push(id.name.as_str());
            }
        }
    }
    names
}
