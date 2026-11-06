use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use super::lanes::SAFE;
use super::report::cause_key;
use super::{Judged, Outcome};
use crate::obf_matrix::expect::Verdict;
use crate::obf_matrix::notes::class_of;

pub const SPANS_HOOK: &str = "UNTWIST_TEST_REFUSAL_SPANS";
const SPAN_PREFIX: &str = "refusal_span: ";
const NAMES_SPAN: &str = "MACHINERY_NAME_OBSERVED";
const SOURCE_SPAN: &str = "global source: ";
const SNIPPET_CHARS: usize = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub line: usize,
    pub column: usize,
    pub what: String,
}

pub fn spans_of(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter_map(|line| line.strip_prefix(SPAN_PREFIX))
        .map(String::from)
        .collect()
}

pub fn parse_site(text: &str) -> Option<Site> {
    let (at, what) = text.split_once(' ')?;
    let (line, column) = at.split_once(':')?;
    Some(Site {
        line: line.parse().ok()?,
        column: column.parse().ok()?,
        what: what.to_string(),
    })
}

pub fn snippet(source: &str, site: &Site) -> String {
    let Some(line) = source.lines().nth(site.line.saturating_sub(1)) else {
        return String::new();
    };
    line.chars()
        .skip(site.column.saturating_sub(1))
        .take(SNIPPET_CHARS)
        .collect::<String>()
        .replace('\t', " ")
}

fn first_blocker(sites: &[Site], names: bool) -> Option<&Site> {
    let wanted = |site: &&Site| !site.what.starts_with(SOURCE_SPAN);
    if names {
        if let Some(site) = sites.iter().find(|site| site.what == NAMES_SPAN) {
            return Some(site);
        }
    }
    sites.iter().find(wanted)
}

fn global_source(sites: &[Site]) -> Option<&Site> {
    sites.iter().find(|site| site.what.starts_with(SOURCE_SPAN))
}

fn cause_of(judged: &[&Judged]) -> Option<String> {
    judged.iter().find_map(|j| match &j.verdict {
        Verdict::ExpectedFail(reason) => Some(cause_key(reason).to_string()),
        Verdict::Fail(_) => Some("unexpected".to_string()),
        _ => None,
    })
}

fn located(source: &str, site: Option<&Site>) -> (String, String) {
    site.map_or_else(
        || ("-".to_string(), "-".to_string()),
        |site| {
            (
                format!("{}:{} {}", site.line, site.column, site.what),
                snippet(source, site),
            )
        },
    )
}

pub fn row(source: &str, cause: &str, note: &str, spans: &[String]) -> String {
    let sites: Vec<Site> = spans.iter().filter_map(|s| parse_site(s)).collect();
    let names = note.contains("may observe the names");
    let (blocker, blocker_text) = located(source, first_blocker(&sites, names));
    let (origin, origin_text) = located(source, global_source(&sites));
    let note = if note == "-" {
        note.to_string()
    } else {
        class_of(note)
    };
    format!("{cause}\t{note}\t{blocker}\t{blocker_text}\t{origin}\t{origin_text}")
}

pub fn write(root: &Path, path: &Path, judged: &[Judged], outcomes: &[Outcome]) -> Result<()> {
    let mut by_file: BTreeMap<&str, Vec<&Judged>> = BTreeMap::new();
    for j in judged.iter().filter(|j| j.lane == SAFE) {
        by_file.entry(j.file.as_str()).or_default().push(j);
    }
    let mut text = String::from("file\tcause\tnote\tspan\tsource\tglobal source\tsource\n");
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();
    for outcome in outcomes {
        let Some(run) = outcome.lanes.get(SAFE) else {
            continue;
        };
        let Some(cause) = by_file
            .get(outcome.entry.file.as_str())
            .and_then(|judged| cause_of(judged))
        else {
            continue;
        };
        let input = outcome.entry.path(root);
        let source =
            std::fs::read_to_string(&input).with_context(|| input.display().to_string())?;
        let note = run.first_note().unwrap_or("-");
        let line = row(&source, &cause, note, &run.spans);
        text.push_str(&format!("{}\t{line}\n", outcome.entry.file));
        *causes.entry(cause).or_default() += 1;
    }
    std::fs::write(path, text).with_context(|| path.display().to_string())?;
    for (cause, count) in &causes {
        println!("blockers: {count:>4} {cause}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_line_parses_into_its_line_column_and_label() {
        let site = parse_site("3:14 global source: a free name").unwrap();
        assert_eq!((site.line, site.column), (3, 14));
        assert_eq!(site.what, "global source: a free name");
        assert_eq!(parse_site("synthetic GLOBAL_REACH"), None);
        let stderr = "note: x: y\nrefusal_span: 1:2 GLOBAL_REACH\n";
        assert_eq!(spans_of(stderr), ["1:2 GLOBAL_REACH"]);
    }

    #[test]
    fn the_snippet_starts_at_the_column_and_is_cut_short() {
        let source = format!("a;\n  b.c({});\n", "x".repeat(80));
        let site = parse_site("2:3 GLOBAL_REACH").unwrap();
        let text = snippet(&source, &site);
        assert!(text.starts_with("b.c(x"));
        assert_eq!(text.chars().count(), SNIPPET_CHARS);
        assert_eq!(snippet(&source, &parse_site("9:1 x").unwrap()), "");
    }

    #[test]
    fn a_names_refusal_shows_the_observed_call_and_the_global_source() {
        let source = "require('a');\nf(x);\n";
        let spans = [
            "2:1 write by an unknown key".to_string(),
            "2:1 MACHINERY_NAME_OBSERVED".to_string(),
            "1:1 global source: a free name that may be a host global".to_string(),
        ];
        let note = "remove-decoder-machinery: code may observe the names of the global object";
        let line = row(source, "safe-global-names", note, &spans);
        let columns: Vec<&str> = line.split('\t').collect();
        assert_eq!(columns[2], "2:1 MACHINERY_NAME_OBSERVED");
        assert_eq!(columns[3], "f(x);");
        assert_eq!(columns[5], "require('a');");
        let reach = row(source, "safe-global-key-reach", "-", &spans);
        assert!(reach.split('\t').nth(2).unwrap().ends_with("unknown key"));
        assert_eq!(row(source, "c", "-", &[]), "c\t-\t-\t-\t-\t-");
    }
}
