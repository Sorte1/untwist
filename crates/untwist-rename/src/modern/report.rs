use super::baseline::Baseline;
use super::plan::{Installer, Keep};

pub const REMOVED: &str = "removed core-js ";
pub const KEPT: &str = "kept core-js ";
pub const REWROTE: &str = "rewrote ";

pub fn assumption(baseline: &Baseline) -> String {
    format!(
        "the output assumes an engine at least as new as {} (--modern {}), with its own built-ins, read when the code runs and not replaced after core-js loaded; core-js polyfills of features native there are removed",
        baseline.describe(),
        baseline.text()
    )
}

fn reason(keep: &Keep) -> String {
    match keep {
        Keep::Proposal => "a proposal, not native in the baseline".to_string(),
        Keep::Needed {
            engine,
            needs: Some(needs),
            baseline,
        } => {
            format!(
                "core-js still replaces it below {engine} {needs} (baseline {engine} {baseline})"
            )
        }
        Keep::Needed {
            engine,
            needs: None,
            ..
        } => {
            format!("core-js-compat records no native support for it in {engine}")
        }
        Keep::Unlisted => "core-js-compat has no data for it".to_string(),
        Keep::Foreign => "it calls code that is not recognised as core-js".to_string(),
        Keep::ExportsRead => "other code reads its exports".to_string(),
        Keep::NotRun => "it is not run as a statement".to_string(),
        Keep::SharedState => "it shares state with other code".to_string(),
        Keep::TextRead => "code may read its function text".to_string(),
        Keep::RunsKept => "it runs a core-js module that is kept".to_string(),
    }
}

fn group(keep: &Keep) -> &'static str {
    match keep {
        Keep::Proposal => "proposals",
        Keep::Needed { .. } => "still needed in the baseline",
        Keep::Unlisted => "without core-js-compat data",
        Keep::Foreign => "need code not recognised as core-js",
        Keep::ExportsRead => "have exports other code reads",
        Keep::NotRun => "not run as a statement",
        Keep::SharedState => "share state with other code",
        Keep::TextRead => "may have their function text read",
        Keep::RunsKept => "run a kept core-js module",
    }
}

fn modules(installer: &Installer) -> String {
    installer
        .modules
        .iter()
        .map(|module| format!("{module}.js"))
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn installer_note(installer: &Installer) -> String {
    let label = format!("{}: {}", installer.range, modules(installer));
    match &installer.kept {
        None if installer.with_others => {
            format!("{REMOVED}{label} (run only by removed modules)")
        }
        None => format!("{REMOVED}{label} (native in the baseline)"),
        Some(keep) => format!("{KEPT}{label}: {}", reason(keep)),
    }
}

pub fn rewrite_note(kind: &str, count: usize) -> String {
    format!("{REWROTE}{count} core-js helper calls: {kind}")
}

pub fn is_detail(message: &str) -> bool {
    [REMOVED, KEPT, REWROTE]
        .iter()
        .any(|prefix| message.starts_with(prefix))
}

fn kept_groups(kept: &[&Installer]) -> String {
    let mut groups: Vec<(&'static str, usize)> = Vec::new();
    for keep in kept.iter().filter_map(|installer| installer.kept.as_ref()) {
        let name = group(keep);
        match groups.iter_mut().find(|(known, _)| *known == name) {
            Some((_, count)) => *count += 1,
            None => groups.push((name, 1)),
        }
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group.1));
    groups
        .iter()
        .map(|(name, count)| format!("{count} {name}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub struct Totals {
    pub removed_helpers: usize,
    pub kept_helpers: usize,
    pub rewritten: usize,
}

pub fn summary(removed: &[&Installer], kept: &[&Installer], totals: &Totals) -> String {
    let mut text = format!(
        "removed {} of {} core-js polyfill modules and {} helper modules",
        removed.len(),
        removed.len() + kept.len(),
        totals.removed_helpers,
    );
    if !kept.is_empty() {
        text.push_str(&format!(", kept {} ({})", kept.len(), kept_groups(kept)));
    }
    if totals.rewritten > 0 {
        text.push_str(&format!(
            "; rewrote {} helper calls to the native form",
            totals.rewritten
        ));
    }
    text.push_str(&format!(
        "; {} helper modules are still used",
        totals.kept_helpers
    ));
    text
}

pub fn banner(removed: usize, baseline: &Baseline) -> String {
    format!(
        "// untwist --modern {}: {removed} core-js polyfills of features native in {} were removed",
        baseline.text(),
        baseline.describe()
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Removed,
    Kept,
    Rewrote,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detail<'m> {
    pub status: Status,
    pub reason: &'m str,
    pub item: String,
}

pub fn detail(message: &str) -> Option<Detail<'_>> {
    if let Some(rest) = message.strip_prefix(REMOVED) {
        let (label, reason) = rest.strip_suffix(')')?.rsplit_once(" (")?;
        return Some(Detail {
            status: Status::Removed,
            reason,
            item: format!("core-js {label}"),
        });
    }
    if let Some(rest) = message.strip_prefix(KEPT) {
        let (label, reason) = rest.rsplit_once(": ")?;
        return Some(Detail {
            status: Status::Kept,
            reason,
            item: format!("core-js {label}"),
        });
    }
    let rest = message.strip_prefix(REWROTE)?;
    let (count, kind) = rest.split_once(" core-js helper calls: ")?;
    Some(Detail {
        status: Status::Rewrote,
        reason: "helper calls rewritten to the native form",
        item: format!("{count} \u{d7} {kind}"),
    })
}
