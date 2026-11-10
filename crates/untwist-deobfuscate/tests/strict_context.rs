use std::time::{Duration, Instant};

use oxc_allocator::Allocator;
use oxc_span::GetSpan;
use untwist_core::parse::parse_program;
use untwist_deobfuscate::stamp::{context_is_strict, context_is_strict_in, find_stamped};

fn units(count: usize) -> String {
    (0..count)
        .map(|unit| {
            if unit % 3 == 0 {
                format!(
                    "function f{unit}() {{ 'use strict'; function g() {{ return {unit}; }} }}\n"
                )
            } else {
                format!("var v{unit} = function () {{ return {unit}; }};\n")
            }
        })
        .collect()
}

fn lookups(count: usize) -> Duration {
    let source = units(count);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let sorted = find_stamped(&parsed.program).top_level_sorted;
    assert!(sorted);
    (0..3)
        .map(|_| {
            let started = Instant::now();
            for statement in &parsed.program.body {
                context_is_strict_in(&parsed.program, statement.span(), sorted);
            }
            started.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
fn the_sorted_lookup_answers_as_the_full_walk_does() {
    let source = format!("'use strict';\n{}", units(30));
    for text in [source.as_str(), &units(30)] {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, text);
        let sorted = find_stamped(&parsed.program).top_level_sorted;
        let mut spans: Vec<_> = parsed.program.body.iter().map(GetSpan::span).collect();
        spans.push(oxc_span::Span::new(1, 4));
        for span in spans {
            assert_eq!(
                context_is_strict_in(&parsed.program, span, sorted),
                context_is_strict(&parsed.program, span),
                "{span:?}"
            );
        }
    }
}

#[test]
fn one_lookup_per_unit_does_not_walk_every_top_level_statement() {
    let small = lookups(2_000).max(Duration::from_micros(200));
    let large = lookups(8_000);
    let ratio = large.as_secs_f64() / small.as_secs_f64();
    assert!(
        ratio < 8.0,
        "8000 lookups took {large:?}, 2000 took {small:?}"
    );
}
