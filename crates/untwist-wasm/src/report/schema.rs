use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use untwist::{stage_rows, Output, RefusalSite, Snapshot, SnapshotCode, StageRow};
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::{Note, NoteKind};

use super::detect::{families, FamilyRow};
use super::diagnostics::{stderr_diagnostics, Diagnostic};
use crate::limits;

pub const SCHEMA: &str = "untwist.report/1";
pub const SITE_ROWS: usize = 1000;

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
struct ReasonRow<'a> {
    what: &'a str,
    places: usize,
}

#[derive(Default)]
struct Sites<'a> {
    rows: Vec<SiteRow<'a>>,
    total: usize,
    reasons: Vec<ReasonRow<'a>>,
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
    #[serde(rename = "assumedSplit", skip_serializing_if = "std::ops::Not::not")]
    assumed_split: bool,
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
struct LibraryRow {
    library: String,
    range: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    disagree: bool,
    modules: usize,
}

#[derive(Serialize)]
struct ModernGroup<'a> {
    status: &'static str,
    reason: &'a str,
    items: Vec<String>,
}

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    engine: Engine,
    stages: Vec<StageRow<'a>>,
    notes: Vec<NoteRow<'a>>,
    #[serde(rename = "refusalSites", skip_serializing_if = "Vec::is_empty")]
    refusal_sites: Vec<SiteRow<'a>>,
    #[serde(rename = "refusalSitesTotal", skip_serializing_if = "Option::is_none")]
    refusal_sites_total: Option<usize>,
    #[serde(rename = "refusalReasons", skip_serializing_if = "Vec::is_empty")]
    refusal_reasons: Vec<ReasonRow<'a>>,
    diagnostics: Vec<Diagnostic<'a>>,
    limits: Limits<'a>,
    detection: Detection<'a>,
    sizes: Sizes,
    stats: &'a untwist::RunStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle: Option<BundleRow<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    snapshots: Vec<SnapshotRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    libraries: Vec<LibraryRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    modern: Vec<ModernGroup<'a>>,
}

pub struct RunFacts<'a> {
    pub source: &'a str,
    pub stdout: &'a str,
    pub stderr: &'a str,
    pub limits: &'a str,
    pub fuel: Option<u64>,
    pub max_iterations: Option<u32>,
    pub assumed_split: bool,
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

fn rank(what: &str) -> u8 {
    if what.starts_with("BUILTIN_PATCHED") {
        0
    } else if what == "GLOBAL_REACH" {
        1
    } else {
        2
    }
}

#[derive(PartialEq, Eq, Hash)]
enum Place<'a> {
    Span([u32; 2]),
    Elsewhere(u32, &'a str),
}

fn place<'a>(row: &SiteRow<'a>) -> Place<'a> {
    row.input_span
        .map_or(Place::Elsewhere(row.layer, row.what), Place::Span)
}

fn reason_rows<'a>(rows: &[SiteRow<'a>]) -> Vec<ReasonRow<'a>> {
    let mut reasons: Vec<ReasonRow<'a>> = Vec::new();
    let mut at: FxHashMap<&str, usize> = FxHashMap::default();
    for row in rows {
        let index = *at.entry(row.what).or_insert_with(|| {
            reasons.push(ReasonRow {
                what: row.what,
                places: 0,
            });
            reasons.len() - 1
        });
        reasons[index].places += 1;
    }
    reasons
}

fn in_input_order<'a>(rows: Vec<SiteRow<'a>>) -> Vec<SiteRow<'a>> {
    let mut order: Vec<(u32, u32)> = rows
        .iter()
        .zip(0u32..)
        .map(|(row, index)| (row.input_span.map_or(u32::MAX, |span| span[0]), index))
        .collect();
    order.sort_unstable();
    let mut slots: Vec<Option<SiteRow<'a>>> = rows.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|(_, index)| slots.get_mut(index as usize)?.take())
        .collect()
}

fn site_rows<'a>(sites: &'a [RefusalSite], source: &str) -> Sites<'a> {
    if sites.is_empty() {
        return Sites::default();
    }
    let index = Utf16Index::new(source, true);
    let mut seen = FxHashSet::default();
    let ranked: Vec<SiteRow<'a>> = (0..=2)
        .flat_map(|wanted| sites.iter().filter(move |site| rank(&site.what) == wanted))
        .map(|site| SiteRow {
            what: &site.what,
            layer: site.layer,
            input_span: site_span(site, source, &index),
        })
        .filter(|row| seen.insert(place(row)))
        .collect();
    let mut rows = in_input_order(ranked);
    let total = rows.len();
    let reasons = reason_rows(&rows);
    rows.truncate(SITE_ROWS);
    Sites {
        rows,
        total,
        reasons,
    }
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

fn library_rows(output: &Output) -> Vec<LibraryRow> {
    output
        .libraries()
        .into_iter()
        .map(|summary| LibraryRow {
            library: summary.library,
            range: summary.range,
            disagree: summary.disagree,
            modules: summary.modules,
        })
        .collect()
}

fn is_modern_detail(pass: &str, message: &str) -> bool {
    pass == untwist::modern_report::NAME && untwist::modern_report::is_detail(message)
}

fn status_name(status: untwist::modern_report::Status) -> &'static str {
    match status {
        untwist::modern_report::Status::Kept => "kept",
        untwist::modern_report::Status::Removed => "removed",
        untwist::modern_report::Status::Rewrote => "rewrote",
    }
}

fn modern_groups(notes: &[Note]) -> Vec<ModernGroup<'_>> {
    let mut groups: Vec<ModernGroup<'_>> = Vec::new();
    let details = notes
        .iter()
        .filter(|note| note.pass == untwist::modern_report::NAME)
        .filter_map(|note| untwist::modern_report::detail(&note.message));
    for detail in details {
        let status = status_name(detail.status);
        match groups
            .iter_mut()
            .find(|group| group.status == status && group.reason == detail.reason)
        {
            Some(group) => group.items.push(detail.item),
            None => groups.push(ModernGroup {
                status,
                reason: detail.reason,
                items: vec![detail.item],
            }),
        }
    }
    let rank = |status: &str| match status {
        "kept" => 0,
        "removed" => 1,
        _ => 2,
    };
    groups.sort_by(|left, right| {
        rank(left.status)
            .cmp(&rank(right.status))
            .then(right.items.len().cmp(&left.items.len()))
    });
    groups
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
            .filter(|note| !is_modern_detail(note.pass, note.message))
            .map(|note| Diagnostic::new(note.level, note.pass, note.message, note.input_span)),
    );
    rows
}

pub fn report_json(output: &Output, facts: &RunFacts<'_>) -> String {
    let bundle = output.bundle.as_ref();
    let notes = note_rows(&output.notes, facts.source);
    let sites = site_rows(&output.refusal_sites, facts.source);
    let report = Report {
        schema: SCHEMA,
        engine: engine(),
        stages: stage_rows(&output.stages),
        diagnostics: diagnostic_rows(output, facts, &notes),
        notes,
        refusal_sites: sites.rows,
        refusal_sites_total: (sites.total > 0).then_some(sites.total),
        refusal_reasons: sites.reasons,
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
            assumed_split: facts.assumed_split,
        },
        sizes: Sizes {
            input_bytes: facts.source.len(),
            output_bytes: facts.stdout.len(),
        },
        stats: &output.stats,
        bundle: bundle_row(output, facts.stdout),
        snapshots: snapshot_rows(&output.snapshots),
        libraries: library_rows(output),
        modern: modern_groups(&output.notes),
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
    use untwist::{RefusalSite, SiteAt};

    use super::{site_rows, Utf16Index, SITE_ROWS};

    fn site(what: &str, start: u32) -> RefusalSite {
        RefusalSite {
            input: 0,
            layer: 0,
            at: Some(SiteAt {
                start,
                end: start + 1,
                line: 1,
                column: 1,
            }),
            what: what.to_string(),
        }
    }

    #[test]
    fn sites_are_listed_once_per_place_in_input_order_and_capped_with_full_counts() {
        let source = "x".repeat(2 * SITE_ROWS);
        let extra = 200;
        let count = u32::try_from(SITE_ROWS + extra).unwrap_or(u32::MAX);
        let mut sites: Vec<RefusalSite> = (0..count)
            .rev()
            .map(|at| site("code from text", at))
            .collect();
        sites.push(site("code from text", 5));
        sites.push(site("GLOBAL_REACH", 7));
        let listed = site_rows(&sites, &source);
        assert_eq!(listed.total, SITE_ROWS + extra);
        assert_eq!(listed.rows.len(), SITE_ROWS);
        let starts: Vec<u32> = listed
            .rows
            .iter()
            .filter_map(|row| row.input_span.map(|span| span[0]))
            .collect();
        assert!(
            starts.windows(2).all(|pair| pair[0] < pair[1]),
            "{starts:?}"
        );
        assert_eq!(listed.rows[7].what, "GLOBAL_REACH");
        let reasons: Vec<(&str, usize)> = listed
            .reasons
            .iter()
            .map(|reason| (reason.what, reason.places))
            .collect();
        assert_eq!(
            reasons,
            [
                ("code from text", SITE_ROWS + extra - 1),
                ("GLOBAL_REACH", 1)
            ]
        );
    }

    #[test]
    fn no_sites_give_no_rows_and_no_counts() {
        let listed = site_rows(&[], "x");
        assert!(listed.rows.is_empty() && listed.reasons.is_empty());
        assert_eq!(listed.total, 0);
    }

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
