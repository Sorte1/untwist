use serde::Serialize;
use untwist::{Output, StopReason};

use super::schema::Utf16Index;

pub const UNATTRIBUTED: &str = "stderr";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic<'a> {
    level: &'static str,
    pass: &'static str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_span: Option<[u32; 2]>,
}

impl<'a> Diagnostic<'a> {
    pub fn new(
        level: &'static str,
        pass: &'static str,
        message: &'a str,
        input_span: Option<[u32; 2]>,
    ) -> Self {
        Self {
            level,
            pass,
            message,
            input_span,
        }
    }
}

struct Slot {
    level: &'static str,
    pass: &'static str,
    lines: usize,
    input_span: Option<[u32; 2]>,
}

fn warning(pass: &'static str) -> Slot {
    Slot {
        level: "warning",
        pass,
        lines: 1,
        input_span: None,
    }
}

struct ParseLocator<'s> {
    source: &'s str,
    line_starts: Vec<usize>,
    index: Utf16Index,
}

impl<'s> ParseLocator<'s> {
    fn new(source: &'s str, needed: bool) -> Self {
        let line_starts = if needed {
            std::iter::once(0)
                .chain(source.match_indices('\n').map(|(at, _)| at + 1))
                .collect()
        } else {
            Vec::new()
        };
        Self {
            source,
            line_starts,
            index: Utf16Index::new(source, needed),
        }
    }

    fn locate(&self, text: &str) -> Option<[u32; 2]> {
        let (line, column) = suffix_line_column(text)?;
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let skipped = column.checked_sub(1)?;
        let rest = self.source.get(start..)?;
        let bytes: usize = rest.chars().take(skipped).map(char::len_utf8).sum();
        let taken = rest.chars().take(skipped).count();
        if taken != skipped {
            return None;
        }
        let at = self.index.at(u32::try_from(start + bytes).ok()?)?;
        Some([at, at])
    }
}

fn suffix_line_column(text: &str) -> Option<(usize, usize)> {
    let inner = text.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let (line, column) = inner.get(open + 1..)?.split_once(':')?;
    Some((line.parse().ok()?, column.parse().ok()?))
}

fn slots(output: &Output, source: &str) -> Vec<Slot> {
    let parse_level = if output.parse_errors > 0 {
        "error"
    } else {
        "warning"
    };
    let locator = ParseLocator::new(source, !output.parse_diagnostics.is_empty());
    let parse = output.parse_diagnostics.iter().map(|text| Slot {
        level: parse_level,
        pass: "parser",
        lines: text.lines().count().max(1),
        input_span: locator.locate(text),
    });
    let stops = output
        .stages
        .iter()
        .filter(|stage| stage.stop != StopReason::Converged)
        .map(|stage| warning(stage.stage));
    let panics = output.panicked.iter().map(|_| warning("pipeline"));
    let refusals = output
        .depth_refusals()
        .into_iter()
        .map(|_| warning("pipeline"));
    let unpack = output.unpack_panicked.then(|| warning("unpack"));
    parse
        .chain(stops)
        .chain(panics)
        .chain(refusals)
        .chain(unpack)
        .collect()
}

fn take_lines<'s>(rest: &mut &'s str, count: usize) -> Option<&'s str> {
    let mut end = 0;
    for _ in 0..count {
        let tail = rest.get(end..).filter(|tail| !tail.is_empty())?;
        end += tail.find('\n').map_or(tail.len(), |at| at + 1);
    }
    let taken = rest.get(..end)?;
    *rest = rest.get(end..)?;
    Some(taken.strip_suffix('\n').unwrap_or(taken))
}

pub fn stderr_diagnostics<'a>(
    output: &Output,
    source: &str,
    stderr: &'a str,
) -> Vec<Diagnostic<'a>> {
    let mut rest = stderr;
    let mut rows: Vec<Diagnostic<'a>> = slots(output, source)
        .into_iter()
        .map_while(|slot| {
            take_lines(&mut rest, slot.lines)
                .map(|message| Diagnostic::new(slot.level, slot.pass, message, slot.input_span))
        })
        .collect();
    rows.extend(
        rest.lines()
            .filter(|line| !line.is_empty())
            .map(|line| Diagnostic::new("warning", UNATTRIBUTED, line, None)),
    );
    rows
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{suffix_line_column, UNATTRIBUTED};
    use crate::report::run_report;

    fn diagnostics(source: &str, flags: &str, refused: bool) -> (String, Vec<Value>) {
        let out = run_report(source, flags, refused);
        let report: Value = serde_json::from_str(&out.report_json()).unwrap_or_default();
        let rows = report["diagnostics"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        (out.stderr(), rows)
    }

    fn message(row: &Value) -> &str {
        row["message"].as_str().unwrap_or_default()
    }

    fn assert_covers(source: &str, flags: &str, refused: bool) -> Vec<Value> {
        let (stderr, rows) = diagnostics(source, flags, refused);
        let joined = rows.iter().map(message).collect::<Vec<_>>().join("\n");
        let mut from = 0;
        for line in stderr.lines() {
            let found = joined.get(from..).and_then(|rest| rest.find(line));
            assert!(
                found.is_some(),
                "stderr line {line:?} missing from {joined:?}"
            );
            from += found.unwrap_or_default() + line.len();
        }
        assert!(
            rows.iter().all(|row| row["pass"] != UNATTRIBUTED),
            "unattributed rows in {rows:?}"
        );
        rows
    }

    #[test]
    fn a_parse_error_is_an_error_row_at_its_input_location() {
        let rows = assert_covers("var a = 1;\nvar é = ;", "", false);
        let first = rows.first().cloned().unwrap_or_default();
        assert_eq!(first["level"], "error");
        assert_eq!(first["pass"], "parser");
        let span = first["inputSpan"].as_array().cloned().unwrap_or_default();
        let at = span.first().and_then(Value::as_u64).unwrap_or(u64::MAX);
        assert_eq!(at, 19);
        assert_eq!(span.get(1).and_then(Value::as_u64), Some(at));
    }

    #[test]
    fn a_stage_stopped_at_the_iteration_cap_is_a_warning_of_that_stage() {
        let source = include_str!(
            "../../../../fixtures/reference/deobfuscate/obfuscator.io-control-flow.js"
        );
        let rows = assert_covers(source, "--max-iterations 1", false);
        let stop = rows
            .iter()
            .find(|row| message(row).contains("stopped at the iteration cap"));
        let stop = stop.cloned().unwrap_or_default();
        assert_eq!(stop["level"], "warning");
        assert!(message(&stop).contains(stop["pass"].as_str().unwrap_or("?")));
    }

    #[test]
    fn a_refused_input_and_its_notes_are_listed_in_stderr_order_then_the_notes() {
        let rows = assert_covers("var a = !0;", "", true);
        assert_eq!(
            rows.first().map(|row| row["pass"].clone()),
            Some("pipeline".into())
        );
        assert!(rows.len() >= 2);
        assert_covers("var a = !0;", "--no-unpack", false);
    }

    #[test]
    fn only_a_trailing_line_and_column_pair_is_read_as_a_location() {
        assert_eq!(suffix_line_column("Unexpected token (3:14)"), Some((3, 14)));
        assert_eq!(suffix_line_column("Unexpected token"), None);
        assert_eq!(suffix_line_column("Expected (x:1)"), None);
    }
}
