use serde::Serialize;
use untwist::{stage_rows, Output, RefusalSite, Snapshot, SnapshotCode, StageRow};
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::{Note, NoteKind};

use super::detect::{families, FamilyRow};
use super::diagnostics::{stderr_diagnostics, Diagnostic};
use crate::limits;

pub const SCHEMA: &str = "untwist.report/1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    version: &'static str,
    build: String,
    nesting_budget: u32,
    shadow_stack_bytes: u32,
    call_depth_cap: u32,
    evaluator_nesting_cap: u32,
    step_budget: u64,
    analysis_cap_bytes: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteRow<'a> {
    level: &'static str,
    pass: &'static str,
    kind: &'static str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_span: Option<[u32; 2]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SiteRow<'a> {
    what: &'a str,
    layer: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_span: Option<[u32; 2]>,
}

#[derive(Serialize)]
struct SnapshotRow {
    stage: &'static str,
    round: u32,
    layer: u32,
    kept: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Limits<'a> {
    fired: Vec<&'a str>,
    nesting_budget: u32,
    fuel_cap: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_iterations: Option<u32>,
    heap_peak: u64,
}

#[derive(Serialize)]
struct Detection<'a> {
    bundle: Option<&'a str>,
    modules: usize,
    families: Vec<FamilyRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Sizes {
    input_bytes: usize,
    output_bytes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleRow<'a> {
    id: &'a str,
    path: &'a str,
    is_entry: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<[u32; 2]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleRow<'a> {
    kind: &'a str,
    entry_id: Option<&'a str>,
    chunk_ids: &'a [String],
    modules: Vec<ModuleRow<'a>>,
}

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    engine: Engine,
    stages: Vec<StageRow<'a>>,
    notes: Vec<NoteRow<'a>>,
    #[serde(rename = "refusalSites", skip_serializing_if = "Vec::is_empty")]
    refusal_sites: Vec<SiteRow<'a>>,
    diagnostics: Vec<Diagnostic<'a>>,
    limits: Limits<'a>,
    detection: Detection<'a>,
    sizes: Sizes,
    stats: &'a untwist::RunStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle: Option<BundleRow<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    snapshots: Vec<SnapshotRow>,
}

pub struct RunFacts<'a> {
    pub source: &'a str,
    pub stdout: &'a str,
    pub stderr: &'a str,
    pub limits: &'a str,
    pub fuel: Option<u64>,
    pub max_iterations: Option<u32>,
}

fn build_id() -> String {
    option_env!("UNTWIST_BUILD_ID")
        .unwrap_or("dev")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect()
}

pub fn engine() -> Engine {
    Engine {
        version: env!("CARGO_PKG_VERSION"),
        build: build_id(),
        nesting_budget: limits::nesting_budget(),
        shadow_stack_bytes: limits::shadow_stack_bytes(),
        call_depth_cap: limits::call_depth_cap(),
        evaluator_nesting_cap: limits::evaluator_nesting_cap(),
        step_budget: untwist_eval::budget::STEP_BUDGET,
        analysis_cap_bytes: crate::analysis::ANALYSIS_CAP_BYTES,
    }
}

fn input_span(note: &Note, source: &str, index: &Utf16Index) -> Option<[u32; 2]> {
    let span = note.span?;
    let (start, end) = (span.start, span.end);
    let fits = start <= end && end < SYNTHETIC_BASE && usize::try_from(end).ok()? <= source.len();
    fits.then(|| Some([index.at(start)?, index.at(end)?]))?
}

fn note_rows<'a>(notes: &'a [Note], source: &str) -> Vec<NoteRow<'a>> {
    let index = Utf16Index::new(source, notes.iter().any(|note| note.span.is_some()));
    notes
        .iter()
        .map(|note| {
            let refusal = note.kind == NoteKind::Refusal;
            NoteRow {
                level: if refusal { "warning" } else { "info" },
                pass: note.pass,
                kind: if refusal { "refusal" } else { "progress" },
                message: &note.message,
                input_span: input_span(note, source, &index),
            }
        })
        .collect()
}

fn site_span(site: &RefusalSite, source: &str, index: &Utf16Index) -> Option<[u32; 2]> {
    let at = site.at.filter(|_| site.layer == 0)?;
    let fits = at.start <= at.end
        && at.end < SYNTHETIC_BASE
        && usize::try_from(at.end).ok()? <= source.len();
    fits.then(|| Some([index.at(at.start)?, index.at(at.end)?]))?
}

fn site_rows<'a>(sites: &'a [RefusalSite], source: &str) -> Vec<SiteRow<'a>> {
    let index = Utf16Index::new(source, !sites.is_empty());
    sites
        .iter()
        .map(|site| SiteRow {
            what: &site.what,
            layer: site.layer,
            input_span: site_span(site, source, &index),
        })
        .collect()
}

fn snapshot_rows(snapshots: &[Snapshot]) -> Vec<SnapshotRow> {
    snapshots
        .iter()
        .map(|snapshot| SnapshotRow {
            stage: snapshot.stage,
            round: snapshot.round,
            layer: snapshot.layer,
            kept: match snapshot.code {
                SnapshotCode::Text(_) => "text",
                SnapshotCode::Unchanged => "unchanged",
                SnapshotCode::OverCap => "over-cap",
            },
        })
        .collect()
}

fn bundle_row<'a>(output: &'a Output, stdout: &str) -> Option<BundleRow<'a>> {
    let bundle = output.bundle.as_ref()?;
    let spans_hold = !output.single_file && output.codes.len() == 1;
    let any_span = spans_hold && bundle.modules.iter().any(|module| module.span.is_some());
    let index = Utf16Index::new(stdout, any_span);
    let span_of = |span: Option<(u32, u32)>| -> Option<[u32; 2]> {
        let (start, end) = span.filter(|_| spans_hold)?;
        Some([index.at(start)?, index.at(end)?])
    };
    Some(BundleRow {
        kind: &bundle.kind,
        entry_id: bundle.entry_id.as_deref(),
        chunk_ids: &bundle.chunk_ids,
        modules: bundle
            .modules
            .iter()
            .map(|module| ModuleRow {
                id: &module.id,
                path: &module.path,
                is_entry: module.is_entry,
                span: span_of(module.span),
            })
            .collect(),
    })
}

fn diagnostic_rows<'a>(
    output: &Output,
    facts: &RunFacts<'a>,
    notes: &[NoteRow<'a>],
) -> Vec<Diagnostic<'a>> {
    let mut rows = stderr_diagnostics(output, facts.source, facts.stderr);
    rows.extend(
        notes
            .iter()
            .map(|note| Diagnostic::new(note.level, note.pass, note.message, note.input_span)),
    );
    rows
}

pub fn report_json(output: &Output, facts: &RunFacts<'_>) -> String {
    let bundle = output.bundle.as_ref();
    let notes = note_rows(&output.notes, facts.source);
    let report = Report {
        schema: SCHEMA,
        engine: engine(),
        stages: stage_rows(&output.stages),
        diagnostics: diagnostic_rows(output, facts, &notes),
        notes,
        refusal_sites: site_rows(&output.refusal_sites, facts.source),
        limits: Limits {
            fired: facts
                .limits
                .split(',')
                .filter(|name| !name.is_empty())
                .collect(),
            nesting_budget: limits::nesting_budget(),
            fuel_cap: facts.fuel.unwrap_or(untwist_eval::budget::STEP_BUDGET),
            max_iterations: facts.max_iterations,
            heap_peak: output.heap_peak,
        },
        detection: Detection {
            bundle: bundle.map(|b| b.kind.as_str()),
            modules: bundle.map_or(0, |b| b.modules.len()),
            families: families(&output.stages),
        },
        sizes: Sizes {
            input_bytes: facts.source.len(),
            output_bytes: facts.stdout.len(),
        },
        stats: &output.stats,
        bundle: bundle_row(output, facts.stdout),
        snapshots: snapshot_rows(&output.snapshots),
    };
    serde_json::to_string(&report).unwrap_or_default()
}

pub fn bare_json() -> String {
    #[derive(Serialize)]
    struct Bare {
        schema: &'static str,
        engine: Engine,
    }
    serde_json::to_string(&Bare {
        schema: SCHEMA,
        engine: engine(),
    })
    .unwrap_or_default()
}

pub struct Utf16Index {
    wide: Vec<Wide>,
}

struct Wide {
    start: u32,
    end: u32,
    units_after: u32,
}

impl Utf16Index {
    pub fn new(text: &str, needed: bool) -> Self {
        let mut wide = Vec::new();
        if needed {
            let mut units = 0u32;
            for (byte, ch) in text.char_indices() {
                units = units.saturating_add(u32::try_from(ch.len_utf16()).unwrap_or(2));
                if !ch.is_ascii() {
                    let start = u32::try_from(byte).unwrap_or(u32::MAX);
                    let end = start.saturating_add(u32::try_from(ch.len_utf8()).unwrap_or(4));
                    wide.push(Wide {
                        start,
                        end,
                        units_after: units,
                    });
                }
            }
        }
        Self { wide }
    }

    pub fn at(&self, byte: u32) -> Option<u32> {
        let before = self.wide.partition_point(|wide| wide.start < byte);
        let Some(last) = before.checked_sub(1).and_then(|index| self.wide.get(index)) else {
            return Some(byte);
        };
        (byte >= last.end).then(|| last.units_after + (byte - last.end))
    }
}

#[cfg(test)]
mod tests {
    use super::Utf16Index;

    #[test]
    fn byte_offsets_map_to_utf16_units_and_mid_character_offsets_to_none() {
        let text = "aé😀b";
        let index = Utf16Index::new(text, true);
        assert_eq!(index.at(0), Some(0));
        assert_eq!(index.at(1), Some(1));
        assert_eq!(index.at(3), Some(2));
        assert_eq!(index.at(7), Some(4));
        assert_eq!(index.at(8), Some(5));
        assert_eq!(index.at(2), None);
        assert_eq!(index.at(5), None);
        assert_eq!(text.encode_utf16().count(), 5);
    }
}
