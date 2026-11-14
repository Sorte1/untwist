use std::time::{Duration, Instant};

use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use untwist_rename::library::table::Role;
use untwist_rename::modern::collect::Facts;
use untwist_rename::modern::plan::{Graph, Source, Unit};

fn chain(length: usize) -> Vec<Unit> {
    (0..length)
        .map(|index| {
            let start = u32::try_from(index * 10).unwrap();
            Unit {
                span: Span::new(start, start + 5),
                modules: vec!["internals/x".to_string()],
                versions: ("3.0.0".to_string(), "3.0.0".to_string()),
                range: "3.0".to_string(),
                role: Role::Wrapper,
                source: Source::Matched,
                binding: Some(SymbolId::from_usize(index)),
                outer: vec![SymbolId::from_usize(index + 1)],
            }
        })
        .collect()
}

#[test]
fn a_chain_ending_in_foreign_code_is_open_all_the_way_down() {
    let units = chain(200_000);
    let facts = Facts::default();
    let started = Instant::now();
    let closed = Graph::new(&units, &facts).closed();
    assert!(closed.iter().all(|closed| !closed));
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_chain_ending_in_a_unit_is_closed() {
    let mut units = chain(1_000);
    if let Some(last) = units.last_mut() {
        last.outer.clear();
    }
    let facts = Facts::default();
    let closed = Graph::new(&units, &facts).closed();
    assert!(closed.iter().all(|closed| *closed));
}
