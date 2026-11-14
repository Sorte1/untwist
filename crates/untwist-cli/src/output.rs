use std::path::PathBuf;

use serde::Serialize;
use untwist::{printable, Output, StopReason};

use crate::budget::{bytes_text, Limits};

#[derive(Serialize)]
struct OutputJson<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    codes: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    files_written: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle: Option<untwist::BundleListing<'a>>,
    stages: Vec<untwist::StageRow<'a>>,
    notes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    libraries: Vec<LibraryJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    types: Option<&'a untwist::TypesReport>,
    parse_diagnostics: &'a [String],
    parse_errors: usize,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    panicked: &'a [usize],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    depth_refused: &'a [usize],
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    unpack_panicked: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    wasm_panicked: bool,
    stats: &'a untwist::RunStats,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    wasm: &'a [untwist::WasmFile],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    wasm_unresolved: &'a [untwist::WasmReference],
    budget: BudgetJson,
    #[cfg(feature = "test-hooks")]
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    refusal_span: &'a [String],
}

#[derive(Serialize)]
struct LibraryJson {
    library: String,
    range: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    disagree: bool,
    modules: usize,
}

fn libraries_json(output: &Output) -> Vec<LibraryJson> {
    output
        .libraries()
        .into_iter()
        .map(|summary| LibraryJson {
            library: summary.library,
            range: summary.range,
            disagree: summary.disagree,
            modules: summary.modules,
        })
        .collect()
}

#[derive(Serialize)]
struct BudgetJson {
    #[serde(flatten)]
    limits: Limits,
    memory_peak: u64,
    hits: Vec<HitJson>,
}

#[derive(Serialize)]
struct HitJson {
    input: usize,
    reason: &'static str,
}

fn budget_json(output: &Output, limits: Limits) -> BudgetJson {
    BudgetJson {
        limits,
        memory_peak: output.memory_peak,
        hits: output
            .budget_hits
            .iter()
            .map(|(input, hit)| HitJson {
                input: *input,
                reason: hit.as_str(),
            })
            .collect(),
    }
}

fn budget_line(output: &Output, limits: Limits) -> String {
    let memory = limits
        .max_memory
        .map_or("unlimited".to_string(), bytes_text);
    let timeout = limits.timeout.map_or("none".to_string(), |timeout| {
        format!("{:.1} s", timeout.as_secs_f64())
    });
    let mut line = format!(
        "budget: max-memory {memory}, timeout {timeout}, memory peak {}",
        bytes_text(output.memory_peak)
    );
    for (input, hit) in &output.budget_hits {
        line.push_str(&format!(", input {} {}", input + 1, hit.as_str()));
    }
    line.push('\n');
    line
}

#[derive(Default)]
pub struct Written {
    pub path: Option<PathBuf>,
    pub files: Vec<String>,
    pub unwritten_modules: Vec<usize>,
}

pub fn to_json(output: &Output, written: &Written, limits: Limits) -> serde_json::Result<String> {
    let output_path = written.path.as_deref();
    let stages = untwist::stage_rows(&output.stages);
    let notes = output
        .notes
        .iter()
        .map(|n| format!("{}: {}", n.pass, n.message))
        .collect();
    let single_file = output.single_file.then(|| output.concatenated());
    let json = OutputJson {
        code: output_path
            .is_none()
            .then(|| single_file.as_deref().unwrap_or(output.code())),
        codes: (output_path.is_none() && output.codes.len() > 1).then_some(&output.codes[..]),
        output_path: output_path.map(|path| path.display().to_string()),
        files_written: written.files.clone(),
        bundle: output
            .bundle
            .as_ref()
            .map(|bundle| bundle.listing_without(&written.unwritten_modules)),
        stages,
        notes,
        libraries: libraries_json(output),
        types: output.types.as_ref(),
        parse_diagnostics: &output.parse_diagnostics,
        parse_errors: output.parse_errors,
        panicked: &output.panicked,
        depth_refused: &output.depth_refused,
        unpack_panicked: output.unpack_panicked,
        wasm_panicked: output.wasm_panicked,
        stats: &output.stats,
        wasm: &output.wasm,
        wasm_unresolved: &output.wasm_unresolved,
        budget: budget_json(output, limits),
        #[cfg(feature = "test-hooks")]
        refusal_span: &output.refusal_spans,
    };
    serde_json::to_string_pretty(&json)
}

fn stage_label(stage: &untwist::StageReport) -> String {
    if stage.round > 1 {
        return format!("{} (round {})", stage.stage, stage.round);
    }
    stage.stage.to_string()
}

fn elapsed_of(stage: &untwist::StageReport, pass: &str) -> f64 {
    stage
        .per_pass_elapsed
        .iter()
        .find(|(name, _)| *name == pass)
        .map_or(0.0, |(_, elapsed)| elapsed.as_secs_f64() * 1000.0)
}

pub fn text_report(output: &Output, limits: Limits) -> String {
    let mut text = String::new();
    for stage in &output.stages {
        text.push_str(&format!(
            "{:<10} {:>3} iter {:>6} changes {:>8.2} ms{}\n",
            stage_label(stage),
            stage.iterations,
            stage.changes,
            stage.elapsed.as_secs_f64() * 1000.0,
            match stage.stop {
                StopReason::IterationCap => " [CAP]",
                StopReason::Cycle => " [CYCLE]",
                StopReason::Budget => " [BUDGET]",
                StopReason::Converged => "",
            }
        ));
        for (pass, count) in stage
            .per_pass
            .iter()
            .chain(stage.fixpoint_per_pass.iter())
            .filter(|(_, c)| *c > 0)
        {
            text.push_str(&format!(
                "  {:<32} {:>6} {:>10.2} ms\n",
                pass,
                count,
                elapsed_of(stage, pass)
            ));
        }
    }
    if let Some(bundle) = &output.bundle {
        text.push_str(&format!(
            "bundle: {} ({} modules, entry {})\n",
            bundle.kind,
            bundle.modules.len(),
            printable(bundle.entry_id.as_deref().unwrap_or("none"))
        ));
    }
    text.push_str(&format!("parse_errors: {}\n", output.parse_errors));
    if let Some(types) = &output.types {
        text.push_str(&types.line());
    }
    if !output.depth_refused.is_empty() {
        let indices: Vec<String> = output.depth_refused.iter().map(usize::to_string).collect();
        text.push_str(&format!("depth_refused: {}\n", indices.join(", ")));
    }
    text.push_str(&budget_line(output, limits));
    for file in &output.wasm {
        text.push_str(&format!(
            "wasm: {} ({}, {} bytes, {} imports, {} exports, {} renamed)\n",
            printable(&file.path),
            file.origin.as_str(),
            file.byte_length,
            file.imports,
            file.exports,
            file.renamed
        ));
    }
    for reference in &output.wasm_unresolved {
        text.push_str(&format!(
            "wasm_unresolved: {} {}\n",
            printable(&reference.name),
            printable(&reference.url)
        ));
    }
    for note in &output.notes {
        text.push_str(&format!(
            "note: {}: {}\n",
            note.pass,
            printable(&note.message)
        ));
    }
    #[cfg(feature = "test-hooks")]
    for span in &output.refusal_spans {
        text.push_str(&format!("refusal_span: {span}\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{to_json, Written};
    use crate::budget::Limits;

    fn compact(json: &str) -> String {
        json.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn the_json_report_carries_the_run_stats_under_the_wasm_report_names() {
        let source = "function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }\n\
            function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }\n\
            console.log(b(0x0) + b(0x1));";
        let out = untwist::Pipeline::new(untwist::Options::default()).run(source);
        let json: serde_json::Value =
            serde_json::from_str(&to_json(&out, &Written::default(), Limits::default()).unwrap())
                .unwrap();
        assert_eq!(json["stats"], serde_json::to_value(out.stats).unwrap());
        assert!(
            json["stats"]["decode"]["steps"].as_u64().unwrap() > 0,
            "{json}"
        );
        assert!(
            json["stats"]["collisions"]["boundInScope"].is_u64(),
            "{json}"
        );
    }

    #[test]
    fn the_json_report_names_the_input_that_panicked() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("");
        out.panicked = vec![1];
        let json = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(compact(&json).contains("\"panicked\":[1]"), "{json}");
    }

    #[test]
    fn the_json_report_flags_a_panicked_wasm_step_only_when_it_happened() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("x;");
        let clean = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(!clean.contains("wasm_panicked"), "{clean}");
        out.wasm_panicked = true;
        let flagged = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(
            compact(&flagged).contains("\"wasm_panicked\":true"),
            "{flagged}"
        );
    }

    #[test]
    fn the_json_report_flags_a_panicked_unpack_only_when_it_happened() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("x;");
        let clean = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(!clean.contains("unpack_panicked"), "{clean}");
        out.unpack_panicked = true;
        let flagged = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(
            compact(&flagged).contains("\"unpack_panicked\":true"),
            "{flagged}"
        );
    }

    #[test]
    fn the_reports_list_refused_inputs_only_when_there_are_some() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("x;");
        assert!(!to_json(&out, &Written::default(), Limits::default())
            .unwrap()
            .contains("depth_refused"));
        assert!(!super::text_report(&out, Limits::default()).contains("depth_refused"));
        out.depth_refused = vec![0, 3];
        let json = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(compact(&json).contains("\"depth_refused\":[0,3]"), "{json}");
        assert!(super::text_report(&out, Limits::default())
            .contains("parse_errors: 0\ndepth_refused: 0, 3\n"));
    }

    #[cfg(feature = "test-hooks")]
    #[test]
    fn the_reports_list_refusal_spans_only_when_there_are_some() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("x;");
        assert!(!to_json(&out, &Written::default(), Limits::default())
            .unwrap()
            .contains("refusal_span"));
        assert!(!super::text_report(&out, Limits::default()).contains("refusal_span"));
        out.refusal_spans = vec!["3:7 GLOBAL_REACH".to_string()];
        let json = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(
            compact(&json).contains("\"refusal_span\":[\"3:7GLOBAL_REACH\"]"),
            "{json}"
        );
        assert!(super::text_report(&out, Limits::default())
            .ends_with("refusal_span: 3:7 GLOBAL_REACH\n"));
    }

    #[test]
    fn a_clean_json_report_omits_the_panicked_key() {
        let out = untwist::Pipeline::new(untwist::Options::default()).run("");
        let json = to_json(&out, &Written::default(), Limits::default()).unwrap();
        assert!(!json.contains("\"panicked\""), "{json}");
    }
}
