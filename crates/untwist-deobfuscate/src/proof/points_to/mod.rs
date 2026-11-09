mod calls;
mod expr;
mod generate;
mod model;
mod object;
mod query;
mod solve;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashMap;

use super::aliased::AliasSite;
use generate::Gen;

pub const BUILTIN_ALIAS_BITS: &str = "inline-decoded-strings/builtin-alias-bits";

fn solved<'s>(
    program: &Program<'_>,
    scoping: &'s Scoping,
    (decoded, skip): (&'s FxHashMap<Span, String>, &'s [Span]),
    (dump, shuffle): (bool, Option<u64>),
) -> Gen<'s> {
    let mut generator = Gen::new(program, scoping, decoded, skip);
    if dump {
        generator.dump = Some(Vec::new());
    }
    generator.run(program);
    generator.g.solve(shuffle);
    generator
}

pub fn points_to_sites(
    program: &Program<'_>,
    scoping: &Scoping,
    decoded: &FxHashMap<Span, String>,
    skip: &[Span],
) -> Vec<AliasSite> {
    solved(program, scoping, (decoded, skip), (false, None)).sites()
}

#[doc(hidden)]
pub fn points_to_dump(
    program: &Program<'_>,
    scoping: &Scoping,
    decoded: &FxHashMap<Span, String>,
    (within, shuffle): (Span, Option<u64>),
) -> String {
    let generator = solved(program, scoping, (decoded, &[]), (true, shuffle));
    let mut lines: Vec<(Span, String)> = Vec::new();
    for (span, node) in generator.dump.iter().flatten() {
        if span.start < within.start || span.end > within.end {
            continue;
        }
        let text = program
            .source_text
            .get(span.start as usize..span.end as usize)
            .unwrap_or("");
        let text: String = text.chars().take(60).collect();
        let set = generator.describe(*node).join(", ");
        lines.push((
            *span,
            format!("{}..{} {text} -> {{{set}}}", span.start, span.end),
        ));
    }
    lines.sort_by(|left, right| {
        (left.0.start, left.0.end, &left.1).cmp(&(right.0.start, right.0.end, &right.1))
    });
    lines.dedup();
    let mut out: Vec<String> = lines.into_iter().map(|(_, line)| line).collect();
    for site in generator.sites() {
        out.push(format!("site {}..{}", site.span().start, site.span().end));
    }
    if generator.g.exhausted() {
        out.push("exhausted".to_string());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests;
