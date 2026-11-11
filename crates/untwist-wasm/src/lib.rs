pub mod analysis;
mod flags;
mod limits;
pub mod report;
mod stdio;

use limits::limits_of;
use untwist::{Isolation, Options, Pipeline};
use wasm_bindgen::prelude::wasm_bindgen;

pub use flags::options_from;
pub use stdio::{exit_code, stderr_of, stdout_of};

const USAGE_ERROR: u8 = 2;

#[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
#[global_allocator]
static ALLOCATOR: untwist::CountingAlloc<talc::wasm::WasmDynamicTalc> =
    untwist::CountingAlloc(talc::wasm::new_wasm_dynamic_allocator());

#[wasm_bindgen(getter_with_clone)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
    pub limits: String,
}

fn isolation(refused: bool) -> Isolation {
    Isolation {
        refused: if refused { vec![0] } else { Vec::new() },
        ..Isolation::default()
    }
}

fn run_with(source: &str, options: Options) -> RunOutput {
    report::clock::register();
    let before = limits::fired();
    let result = Pipeline::new(options).run(source);
    let fired = limits::fired().since(before);
    RunOutput {
        stdout: stdout_of(&result),
        stderr: stderr_of(&result),
        code: exit_code(&result),
        limits: limits_of(&result, fired),
    }
}

#[derive(serde::Serialize)]
struct PassRow {
    name: &'static str,
    stage: &'static str,
    #[serde(rename = "unsafe")]
    unsafe_only: bool,
}

#[wasm_bindgen]
pub fn pass_table() -> String {
    let rows: Vec<PassRow> = untwist::PASSES
        .iter()
        .map(|pass| PassRow {
            name: pass.name,
            stage: pass.stage,
            unsafe_only: pass.unsafe_only,
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_default()
}

#[wasm_bindgen]
pub fn nesting_budget() -> u32 {
    limits::nesting_budget()
}

#[wasm_bindgen]
pub fn shadow_stack_bytes() -> u32 {
    limits::shadow_stack_bytes()
}

#[wasm_bindgen]
pub fn call_depth_cap() -> u32 {
    limits::call_depth_cap()
}

#[wasm_bindgen]
pub fn evaluator_nesting_cap() -> u32 {
    limits::evaluator_nesting_cap()
}

#[wasm_bindgen]
pub fn run(source: &str, flags: &str, refused: bool) -> RunOutput {
    match options_from(flags) {
        Ok(options) => run_with(
            source,
            Options {
                isolation: isolation(refused),
                ..options
            },
        ),
        Err(message) => RunOutput {
            stdout: String::new(),
            stderr: format!("error: {message}\n"),
            code: USAGE_ERROR,
            limits: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn a_run_prints_the_code_with_one_trailing_newline_and_exits_zero() {
        let out = run("var a = !0;", "--no-unpack", false);
        assert_eq!(out.stdout, "var a = true;\n");
        assert_eq!(out.stderr, "");
        assert_eq!(out.code, 0);
        assert_eq!(out.limits, "");
    }

    #[test]
    fn a_refused_input_passes_through_with_the_workers_message_and_exits_three() {
        let out = run("var a = !0;", "", true);
        assert_eq!(out.stdout, "var a = !0;\n");
        assert_eq!(
            out.stderr,
            format!(
                "warning: input 1 was not processed: {}\n",
                untwist::WORKER_DIED
            )
        );
        assert_eq!(out.code, 3);
        assert_eq!(out.limits, "refused");
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        let out = run("a;", "--report json", false);
        assert_eq!(out.stdout, "");
        assert_eq!(out.stderr, "error: unsupported flag --report\n");
        assert_eq!(out.code, 2);
    }
}
