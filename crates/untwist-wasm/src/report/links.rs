use oxc_allocator::Allocator;
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::ast_util::statements::statement_spans;
use untwist_core::depth::{limits_for, scan};
use untwist_core::parse::{has_error, parse_program};
use untwist_eval::stack::room;

use super::schema::Utf16Index;

pub const LINKS_CAP_BYTES: usize = 12 << 20;

fn in_source(span: (u32, u32), source: &str) -> bool {
    let (start, end) = span;
    start <= end
        && end < SYNTHETIC_BASE
        && usize::try_from(end).is_ok_and(|end| end <= source.len())
}

fn shown_spans(stdout: &str) -> Option<Vec<(u32, u32)>> {
    if stdout.len() > LINKS_CAP_BYTES || scan(stdout, &limits_for(room(), stdout.len())).is_err() {
        return None;
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, stdout);
    (!has_error(&parsed.diagnostics)).then(|| statement_spans(&parsed.program))
}

pub fn links_json(source: &str, stdout: &str, spans: &[(u32, u32)]) -> String {
    let Some(shown) = shown_spans(stdout).filter(|shown| shown.len() == spans.len()) else {
        return String::new();
    };
    let input = Utf16Index::new(source, true);
    let output = Utf16Index::new(stdout, true);
    let rows: Vec<[u32; 4]> = shown
        .iter()
        .zip(spans)
        .filter(|(_, span)| in_source(**span, source))
        .filter_map(|(&(out_start, out_end), &(in_start, in_end))| {
            Some([
                output.at(out_start)?,
                output.at(out_end)?,
                input.at(in_start)?,
                input.at(in_end)?,
            ])
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::links_json;
    use untwist_core::ast_util::respan::SYNTHETIC_BASE;

    #[test]
    fn each_printed_statement_is_paired_with_the_input_it_came_from() {
        let source = "var \u{e9} = 1; f();";
        let stdout = "var \u{e9} = 1;\nf();\n";
        let json = links_json(source, stdout, &[(0, 11), (12, 16)]);
        assert_eq!(json, "[[0,10,0,10],[11,15,11,15]]");
    }

    #[test]
    fn a_synthetic_statement_has_no_pair_and_a_count_mismatch_has_none_at_all() {
        let stdout = "a;\nb;\n";
        assert_eq!(
            links_json(
                "a;b;",
                stdout,
                &[(0, 2), (SYNTHETIC_BASE, SYNTHETIC_BASE + 2)]
            ),
            "[[0,2,0,2]]"
        );
        assert_eq!(links_json("a;b;", stdout, &[(0, 2)]), "");
        assert_eq!(links_json("a;", "a(;\n", &[(0, 2)]), "");
    }
}
