mod body;
mod order;

use oxc_ast::ast::{
    BindingPattern, IdentifierReference, Program, Statement, VariableDeclarationKind,
};
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::PassOptions;
use untwist_core::semantic::{has_module_syntax, is_constant_binding};
use untwist_eval::fold::Const;
use untwist_eval::value::JsString;

use super::eval::{literal_argument, MAX_STRING};
use crate::bindings::{block_function_names, resolved_symbol};
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::mutants::straight_line_guards_dropped;
use body::{Func, State};
use order::{Fold, Walk};

pub(super) struct Plan {
    pub(super) folds: Vec<(Span, Const)>,
    pub(super) removals: Vec<Span>,
}

fn state<'p>(statement: &'p Statement<'p>, scoping: &Scoping) -> Option<(SymbolId, State)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let symbol = id.symbol_id.get()?;
    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }
    let init = match &declarator.init {
        Some(init) => literal_argument(init)?,
        None => Const::Undefined,
    };
    let writable = declaration.kind != VariableDeclarationKind::Const;
    let state = State {
        statement: statement.span(),
        init,
        writable,
    };
    Some((symbol, state))
}

fn states(program: &Program<'_>, scoping: &Scoping) -> FxHashMap<SymbolId, State> {
    program
        .body
        .iter()
        .filter_map(|statement| state(statement, scoping))
        .collect()
}

struct Count<'s> {
    scoping: &'s Scoping,
    counts: FxHashMap<SymbolId, usize>,
}

impl<'a> VisitJs<'a> for Count<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = resolved_symbol(it, self.scoping) {
            if let Some(count) = self.counts.get_mut(&symbol) {
                *count += 1;
            }
        }
    }
}

fn accounted(
    program: &Program<'_>,
    scoping: &Scoping,
    funcs: &FxHashMap<SymbolId, &Func<'_>>,
    seen: &FxHashMap<SymbolId, usize>,
    states: &FxHashMap<SymbolId, State>,
) -> bool {
    let mut count = Count {
        scoping,
        counts: states.keys().map(|symbol| (*symbol, 0)).collect(),
    };
    let bodies: FxHashSet<Span> = funcs.values().map(|func| func.statement).collect();
    for statement in &program.body {
        if bodies.contains(&statement.span()) {
            count.visit_statement(statement);
        }
    }
    let total = |symbol: SymbolId| scoping.get_resolved_references(symbol).count();
    let seen_of = |symbol: &SymbolId| seen.get(symbol).copied().unwrap_or(0);
    funcs.keys().all(|symbol| seen_of(symbol) == total(*symbol))
        && count
            .counts
            .iter()
            .all(|(symbol, inside)| seen_of(symbol) + inside == total(*symbol))
}

fn unreached(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &[SymbolId],
    folds: &[Fold],
) -> bool {
    let property = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
    let mut names: Vec<&str> = symbols.iter().map(|s| scoping.symbol_name(*s)).collect();
    let mut props: Vec<&str> = symbols
        .iter()
        .filter(|symbol| scoping.symbol_flags(**symbol).intersects(property))
        .map(|symbol| scoping.symbol_name(*symbol))
        .collect();
    names.sort_unstable();
    props.sort_unstable();
    let texts: Vec<(Span, JsString)> = folds
        .iter()
        .filter_map(|fold| match &fold.value {
            Const::Str(text) => Some((fold.span, JsString::from_str(text))),
            _ => None,
        })
        .collect();
    let resolve: FxHashMap<Span, &JsString> =
        texts.iter().map(|(span, text)| (*span, text)).collect();
    let watch = ReachWatch {
        names: &names,
        props: &props,
        fns: &[],
        observe: true,
    };
    let found = global_reach(program, scoping, (Some(&resolve), &[]), watch);
    found.reach.is_none() && found.observed.is_none()
}

fn printable(value: &Const) -> bool {
    match value {
        Const::Undefined => false,
        Const::Num(number) => number.is_finite(),
        Const::Str(text) => text.len() <= MAX_STRING,
        _ => true,
    }
}

fn tracked<'f, 'p>(
    funcs: &'f [Func<'p>],
    states: FxHashMap<SymbolId, State>,
) -> (
    FxHashMap<SymbolId, &'f Func<'p>>,
    FxHashMap<SymbolId, State>,
) {
    let written: FxHashSet<SymbolId> = funcs
        .iter()
        .filter(|func| func.writes)
        .flat_map(|func| func.touched.iter().copied())
        .collect();
    let chosen: FxHashMap<SymbolId, &Func<'p>> = funcs
        .iter()
        .filter(|func| func.touched.iter().any(|symbol| written.contains(symbol)))
        .map(|func| (func.symbol, func))
        .collect();
    let touched: FxHashSet<SymbolId> = chosen
        .values()
        .flat_map(|func| func.touched.iter().copied())
        .collect();
    let states = states
        .into_iter()
        .filter(|(symbol, _)| touched.contains(symbol))
        .collect();
    (chosen, states)
}

fn unshared(scoping: &Scoping, symbols: &[SymbolId]) -> bool {
    let shared = block_function_names(scoping);
    shared.is_empty()
        || !symbols
            .iter()
            .any(|symbol| shared.contains(scoping.symbol_name(*symbol)))
}

pub(super) fn plan<'p>(
    program: &'p Program<'p>,
    scoping: &Scoping,
    opts: &PassOptions,
) -> Option<Plan> {
    let states = states(program, scoping);
    if states.is_empty() {
        return None;
    }
    let funcs: Vec<Func<'p>> = program
        .body
        .iter()
        .filter_map(|statement| body::parse(statement, scoping, &states))
        .filter(|func| is_constant_binding(scoping, func.symbol))
        .collect();
    if !funcs.iter().any(|func| func.writes) {
        return None;
    }
    let (funcs, states) = tracked(&funcs, states);
    let symbols: Vec<SymbolId> = funcs.keys().chain(states.keys()).copied().collect();
    if !unshared(scoping, &symbols) {
        return None;
    }
    let mut walk = Walk::new(scoping, &funcs, &states);
    walk.program(program)?;
    if walk.folds.is_empty() {
        return None;
    }
    let guarded = !straight_line_guards_dropped();
    if guarded && !accounted(program, scoping, &funcs, &walk.seen, &states) {
        return None;
    }
    let global = !opts.nested_root && !has_module_syntax(program);
    if guarded && global && !unreached(program, scoping, &symbols, &walk.folds) {
        return None;
    }
    let mut folds = Vec::new();
    let mut removals: Vec<Span> = Vec::new();
    for fold in walk.folds {
        match fold.statement {
            Some(statement) => removals.push(statement),
            None if printable(&fold.value) => folds.push((fold.span, fold.value)),
            None => return None,
        }
    }
    removals.extend(funcs.values().map(|func| func.statement));
    removals.extend(states.values().map(|state| state.statement));
    Some(Plan { folds, removals })
}
