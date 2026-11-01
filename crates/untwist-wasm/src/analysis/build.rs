use oxc_allocator::Allocator;
use oxc_ast_visit::VisitJs;
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::{ScopeFlags, Scoping, SemanticBuilder};
use oxc_span::SourceType;

use super::collect::Collector;
use super::index::{Index, Occurrence, Problem, Scope, NONE};
use super::offsets::Utf16Map;

use oxc_diagnostics::OxcDiagnostic as Diagnostic;

fn problem(diagnostic: &Diagnostic, map: &Utf16Map, severity: &'static str) -> Problem {
    let label = diagnostic
        .labels
        .iter()
        .find(|label| label.primary())
        .or_else(|| diagnostic.labels.first());
    let (start, end) = label.map_or((0, 0), |label| {
        (label.offset(), label.offset().saturating_add(label.len()))
    });
    let [from, to] = map.span(start, end);
    Problem {
        message: diagnostic.message.to_string(),
        severity,
        from,
        to,
    }
}

fn scopes_of(scoping: &Scoping, spans: &[[u32; 2]]) -> Vec<Scope> {
    let mut scopes: Vec<Scope> = scoping
        .scope_descendants_from_root()
        .map(|id| {
            let [start, end] = spans.get(id.index()).copied().unwrap_or([0, 0]);
            let flags = scoping.scope_flags(id);
            Scope {
                parent: scoping
                    .scope_parent_id(id)
                    .map_or(NONE, |parent| u32::try_from(parent.index()).unwrap_or(NONE)),
                start,
                end,
                last_descendant: u32::try_from(id.index()).unwrap_or(NONE),
                hazard: flags.intersects(ScopeFlags::DirectEval | ScopeFlags::With),
                var_scope: flags.is_var(),
                strict: flags.is_strict_mode(),
            }
        })
        .collect();
    for id in (1..scopes.len()).rev() {
        let Some(scope) = scopes.get(id).copied() else {
            continue;
        };
        if let Some(parent) = scopes.get_mut(scope.parent as usize) {
            parent.last_descendant = parent.last_descendant.max(scope.last_descendant);
        }
    }
    scopes
}

fn by_symbol(occurrences: &[Occurrence], symbols: usize) -> (Vec<u32>, Vec<u32>) {
    let mut starts = vec![0u32; symbols + 1];
    for occ in occurrences
        .iter()
        .filter(|occ| (occ.symbol as usize) < symbols)
    {
        if let Some(count) = starts.get_mut(occ.symbol as usize + 1) {
            *count += 1;
        }
    }
    for at in 1..starts.len() {
        let before = starts.get(at - 1).copied().unwrap_or(0);
        if let Some(slot) = starts.get_mut(at) {
            *slot += before;
        }
    }
    let mut fill = starts.clone();
    let mut list = vec![0u32; occurrences.len()];
    for (at, occ) in occurrences.iter().enumerate() {
        if let Some(next) = fill.get_mut(occ.symbol as usize) {
            if let Some(slot) = list.get_mut(*next as usize) {
                *slot = u32::try_from(at).unwrap_or(NONE);
            }
            *next += 1;
        }
    }
    (starts, list)
}

fn bindings_of(scoping: &Scoping, collector: &mut Collector<'_>) -> Vec<(u32, u32, u32)> {
    let mut bindings: Vec<(u32, u32, u32)> = scoping
        .iter_bindings()
        .flat_map(|(scope, bindings)| {
            bindings
                .iter()
                .map(move |(name, symbol)| (scope, name.as_str(), *symbol))
        })
        .map(|(scope, name, symbol)| {
            (
                u32::try_from(scope.index()).unwrap_or(NONE),
                collector.names.intern(name),
                u32::try_from(symbol.index()).unwrap_or(NONE),
            )
        })
        .collect();
    bindings.sort_unstable();
    bindings
}

fn finish(mut collector: Collector<'_>, problems: Vec<Problem>) -> Index {
    let scoping = collector.scoping;
    let scopes = scopes_of(scoping, &collector.scope_spans);
    let bindings = bindings_of(scoping, &mut collector);
    let mut index = Index {
        occurrences: std::mem::take(&mut collector.occurrences),
        by_symbol_start: Vec::new(),
        by_symbol: Vec::new(),
        symbols: std::mem::take(&mut collector.symbols),
        scopes,
        scopes_by_start: Vec::new(),
        bindings,
        members: std::mem::take(&mut collector.members),
        exports: std::mem::take(&mut collector.exports),
        names: std::mem::take(&mut collector.names),
        problems,
    };
    settle(&mut index);
    index
}

fn settle(index: &mut Index) {
    index.occurrences.sort_unstable_by_key(|occ| occ.start);
    index.members.sort_unstable_by_key(|member| member.start);
    let fixed: Vec<u32> = index
        .occurrences
        .iter()
        .map(|occ| match index.symbol(occ.symbol) {
            Some(symbol) if !index.is_within(occ.scope, symbol.scope) => symbol.scope,
            _ => occ.scope,
        })
        .collect();
    for (occ, scope) in index.occurrences.iter_mut().zip(fixed) {
        occ.scope = scope;
    }
    let (starts, list) = by_symbol(&index.occurrences, index.symbols.len());
    index.by_symbol_start = starts;
    index.by_symbol = list;
    let mut order: Vec<u32> = (0..u32::try_from(index.scopes.len()).unwrap_or(0)).collect();
    order.sort_by_key(|id| index.scope(*id).map_or(0, |scope| scope.start));
    index.scopes_by_start = order;
}

fn parse_options() -> ParseOptions {
    ParseOptions {
        allow_return_outside_function: true,
        ..ParseOptions::default()
    }
}

pub fn build(text: &str) -> Index {
    let map = Utf16Map::new(text);
    let allocator = Allocator::default();
    let source_type = SourceType::unambiguous().with_jsx(true);
    let parsed = Parser::new(&allocator, text, source_type)
        .with_options(parse_options())
        .parse();
    let mut problems: Vec<Problem> = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| problem(diagnostic, &map, "error"))
        .collect();
    let program = &parsed.program;
    let semantic = SemanticBuilder::new()
        .with_check_syntax_error(parsed.diagnostics.is_empty())
        .build(program);
    problems.extend(
        semantic
            .diagnostics
            .iter()
            .map(|diagnostic| problem(diagnostic, &map, "error")),
    );
    let scoping = semantic.semantic.scoping();
    let mut collector = Collector::new(scoping, &map);
    collector.visit_program(program);
    finish(collector, problems)
}
