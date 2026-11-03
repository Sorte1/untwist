use untwist::{Isolation, Options, Pipeline, ProgressSink};
use wasm_bindgen::prelude::wasm_bindgen;

use super::clock;
use super::schema::{bare_json, report_json, RunFacts};
use super::stages;
use crate::limits::{self, limits_of};
use crate::{exit_code, isolation, options_from, stderr_of, stdout_of, USAGE_ERROR};

#[wasm_bindgen]
pub struct RunReport {
    stdout: String,
    stderr: String,
    code: u8,
    limits: String,
    report: String,
    bundle_json: Option<String>,
    modules: Vec<String>,
}

#[wasm_bindgen]
impl RunReport {
    #[wasm_bindgen(getter)]
    pub fn stdout(&self) -> String {
        self.stdout.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn stderr(&self) -> String {
        self.stderr.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn code(&self) -> u8 {
        self.code
    }

    #[wasm_bindgen(getter)]
    pub fn limits(&self) -> String {
        self.limits.clone()
    }

    pub fn report_json(&self) -> String {
        self.report.clone()
    }

    pub fn bundle_json(&self) -> Option<String> {
        self.bundle_json.clone()
    }

    pub fn module_count(&self) -> u32 {
        u32::try_from(self.modules.len()).unwrap_or(u32::MAX)
    }

    pub fn module_code(&self, index: u32) -> Option<String> {
        let index = usize::try_from(index).ok()?;
        self.modules.get(index).cloned()
    }
}

#[wasm_bindgen]
pub fn engine_info() -> String {
    serde_json::to_string(&super::schema::engine()).unwrap_or_default()
}

fn report_with(source: &str, options: Options) -> RunReport {
    clock::register();
    let (fuel, max_iterations) = (options.fuel, options.max_iterations);
    let before = limits::fired();
    let mut result = Pipeline::new(options).run(source);
    let fired = limits::fired().since(before);
    let stdout = stdout_of(&result);
    let stderr = stderr_of(&result);
    let code = exit_code(&result);
    let limits = limits_of(&result, fired);
    let facts = RunFacts {
        source,
        stdout: &stdout,
        stderr: &stderr,
        limits: &limits,
        fuel,
        max_iterations,
    };
    let report = report_json(&result, &facts);
    let bundle_json = result
        .bundle
        .as_ref()
        .map(untwist::BundleOutput::bundle_json);
    let modules = result
        .bundle
        .take()
        .map(|bundle| {
            bundle
                .modules
                .into_iter()
                .map(|module| module.code)
                .collect()
        })
        .unwrap_or_default();
    RunReport {
        stdout,
        stderr,
        code,
        limits,
        report,
        bundle_json,
        modules,
    }
}

#[wasm_bindgen]
pub fn run_report(source: &str, flags: &str, refused: bool) -> RunReport {
    report_for(source, flags, refused, None)
}

#[wasm_bindgen]
pub fn run_report_staged(source: &str, flags: &str, refused: bool) -> RunReport {
    report_for(source, flags, refused, Some(stages::sink()))
}

fn report_for(
    source: &str,
    flags: &str,
    refused: bool,
    progress: Option<ProgressSink>,
) -> RunReport {
    match options_from(flags) {
        Ok(options) => report_with(
            source,
            Options {
                isolation: Isolation {
                    progress,
                    ..isolation(refused)
                },
                ..options
            },
        ),
        Err(message) => RunReport {
            stdout: String::new(),
            stderr: format!("error: {message}\n"),
            code: USAGE_ERROR,
            limits: String::new(),
            report: bare_json(),
            bundle_json: None,
            modules: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, PoisonError};

    use untwist::{Progress, ProgressSink};

    use super::{report_for, run_report, run_report_staged};
    use crate::report::SCHEMA;
    use crate::run;

    const DECODED: &str = "function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }\n\
        function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }\n\
        console.log(b(0x0) + b(0x1));";

    fn stage_rows(report: &str) -> Vec<(String, u64)> {
        let value: serde_json::Value = serde_json::from_str(report).unwrap_or_default();
        value["stages"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .map(|row| {
                        let name = row["stage"].as_str().unwrap_or_default().to_string();
                        (name, row["changes"].as_u64().unwrap_or(0))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn stages_of(source: &str, flags: &str) -> (Vec<&'static str>, super::RunReport) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = Arc::clone(&seen);
        let sink = ProgressSink::new(move |progress| {
            if let Progress::StageBegins(name) = progress {
                into.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(name);
            }
        });
        let out = report_for(source, flags, false, Some(sink));
        let names = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
        (names, out)
    }

    #[test]
    fn a_staged_run_announces_each_stage_in_order_and_prints_what_a_plain_run_prints() {
        let (names, staged) = stages_of(DECODED, "--rename simple");
        let plain = run_report(DECODED, "--rename simple", false);
        assert_eq!(staged.stdout(), plain.stdout());
        assert_eq!(staged.stderr(), plain.stderr());
        assert_eq!(staged.code(), plain.code());
        let rows = stage_rows(&staged.report_json());
        assert!(!rows.is_empty());
        assert_eq!(rows, stage_rows(&plain.report_json()));
        assert_eq!(names.first(), Some(&"prepare"), "{names:?}");
        let at = |stage: &str| names.iter().position(|name| *name == stage);
        for stage in ["deobfuscate", "unminify", "rename", "cleanup", "unpack"] {
            assert!(at(stage).is_some(), "{stage} missing from {names:?}");
        }
        assert!(at("deobfuscate") < at("unminify"), "{names:?}");
        assert!(at("cleanup") < at("unpack"), "{names:?}");
    }

    #[test]
    fn stages_left_out_by_the_flags_are_not_announced() {
        let (names, _) = stages_of(DECODED, "--no-deobfuscate --no-unminify --no-unpack");
        for stage in ["deobfuscate", "unminify", "cleanup", "unpack", "rename"] {
            assert!(!names.contains(&stage), "{stage} in {names:?}");
        }
    }

    #[test]
    fn the_staged_run_without_a_listener_is_the_plain_run() {
        let staged = run_report_staged(DECODED, "", false);
        let plain = run_report(DECODED, "", false);
        assert_eq!(staged.stdout(), plain.stdout());
        assert_eq!(staged.code(), plain.code());
    }

    fn same(source: &str, flags: &str, refused: bool) {
        let expected = run(source, flags, refused);
        let actual = run_report(source, flags, refused);
        assert_eq!(actual.stdout(), expected.stdout);
        assert_eq!(actual.stderr(), expected.stderr);
        assert_eq!(actual.code(), expected.code);
        assert_eq!(actual.limits(), expected.limits);
    }

    #[test]
    fn the_report_run_equals_run_on_a_plain_input_a_refusal_and_a_usage_error() {
        same("var a = !0;", "--no-unpack", false);
        same("var a = !0;", "", true);
        same("a;", "--report json", false);
        same("var a = ;", "", false);
    }

    #[test]
    fn the_report_json_names_its_schema_and_the_engine() {
        let report = run_report("var a = !0;", "", false).report_json();
        assert!(report.starts_with(&format!(
            "{{\"schema\":\"{SCHEMA}\",\"engine\":{{\"version\":\""
        )));
        let value: serde_json::Value = serde_json::from_str(&report).unwrap_or_default();
        assert_eq!(value["engine"]["stepBudget"], 50_000_000);
        assert_eq!(value["sizes"]["inputBytes"], 11);
        assert_eq!(value["limits"]["fuelCap"], 50_000_000);
        assert!(value["stages"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()));
    }

    #[test]
    fn the_report_json_carries_the_same_run_stats_as_the_cli_report() {
        let source = "function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }\n\
            function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }\n\
            var x = 1; function g(p, q) { return x + p + q; }\n\
            console.log(b(0x0) + b(0x1), g(1, 2));";
        let report = run_report(source, "--rename simple", false).report_json();
        let value: serde_json::Value = serde_json::from_str(&report).unwrap_or_default();
        let native =
            untwist::Pipeline::new(crate::options_from("--rename simple").unwrap()).run(source);
        assert_eq!(value["stats"], serde_json::to_value(native.stats).unwrap());
        assert!(
            value["stats"]["decode"]["units"].as_u64().unwrap_or(0) >= 1,
            "{value}"
        );
        assert!(
            value["stats"]["decode"]["peakCallDepth"]
                .as_u64()
                .unwrap_or(0)
                >= 1,
            "{value}"
        );
    }

    #[test]
    fn a_bundle_hands_out_each_module_code_once_split() {
        let source = include_str!("../../../../fixtures/reference/unpack/webpack-5.js");
        let out = run_report(source, "", false);
        assert!(out.module_count() > 0);
        assert!(out.module_code(0).is_some());
        assert!(out.module_code(out.module_count()).is_none());
    }

    fn module_slices(source: &str) -> Vec<(String, String)> {
        let out = run_report(source, "", false);
        let report: serde_json::Value =
            serde_json::from_str(&out.report_json()).unwrap_or_default();
        let units: Vec<u16> = out.stdout().encode_utf16().collect();
        let modules = report["bundle"]["modules"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        modules
            .iter()
            .enumerate()
            .filter_map(|(index, module)| {
                let span = module["span"].as_array()?;
                let from = usize::try_from(span.first()?.as_u64()?).ok()?;
                let to = usize::try_from(span.get(1)?.as_u64()?).ok()?;
                let slice = String::from_utf16(units.get(from..to)?).ok()?;
                let code = out.module_code(u32::try_from(index).ok()?)?;
                Some((slice, code))
            })
            .collect()
    }

    fn a_string_of(code: &str) -> Option<&str> {
        let start = code.find('"')?;
        let rest = code.get(start + 1..)?;
        rest.find('"')
            .and_then(|end| rest.get(..end))
            .filter(|text| !text.is_empty())
    }

    #[test]
    fn module_spans_slice_the_processed_bundle_text_in_utf16_units() {
        let source = include_str!("../../../../fixtures/reference/unpack/webpack-5.js");
        let accented = format!("var accent = \"é😀\";\n{source}");
        assert_ne!(accented, source);
        for input in [source, accented.as_str()] {
            let slices = module_slices(input);
            assert!(slices.len() >= 2, "{} modules carried a span", slices.len());
            for (slice, code) in &slices {
                let marker = a_string_of(code).unwrap_or("");
                assert!(
                    slice.contains(marker),
                    "the slice {slice:?} lacks {marker:?} from {code:?}"
                );
                assert!(
                    slice.starts_with('{') && slice.ends_with('}'),
                    "the slice {slice:?} is not the factory body"
                );
            }
        }
    }
}
