use super::baseline::Baseline;
use super::plan::{Installer, Keep};

pub fn assumption(baseline: &Baseline) -> String {
    format!(
        "the output assumes an engine at least as new as {} (--modern {}), with its own built-ins; core-js polyfills of features native there are removed",
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
    let label = format!("core-js {}: {}", installer.range, modules(installer));
    match &installer.kept {
        None if installer.with_others => format!("removed {label} (run only by removed modules)"),
        None => format!("removed {label} (native in the baseline)"),
        Some(keep) => format!("kept {label}: {}", reason(keep)),
    }
}

pub fn summary(
    removed: &[&Installer],
    kept: &[&Installer],
    removed_helpers: usize,
    kept_helpers: usize,
) -> String {
    format!(
        "removed {} of {} core-js polyfill modules and {removed_helpers} helper modules; {kept_helpers} helper modules are still used",
        removed.len(),
        removed.len() + kept.len(),
    )
}

pub fn banner(removed: usize, baseline: &Baseline) -> String {
    format!(
        "// untwist --modern {}: {removed} core-js polyfills of features native in {} were removed",
        baseline.text(),
        baseline.describe()
    )
}
