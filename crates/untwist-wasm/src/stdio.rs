use std::fmt::Write as _;

use untwist::{printable, Output, StopReason};

pub fn stdout_of(result: &Output) -> String {
    let body = if result.single_file {
        result.concatenated().into_owned()
    } else {
        result.codes.join("\n\n")
    };
    body + "\n"
}

fn stop_warning(stage: &str, iterations: u32, stop: StopReason) -> Option<String> {
    match stop {
        StopReason::IterationCap => Some(format!(
            "warning: {stage} stopped at the iteration cap ({iterations}); output may not be fully simplified"
        )),
        StopReason::Cycle => Some(format!(
            "warning: {stage} stopped after {iterations} iterations because its passes oscillate; output may not be fully simplified"
        )),
        StopReason::Budget => Some(format!(
            "warning: {stage} stopped after {iterations} iterations because the run reached its budget; output may not be fully simplified"
        )),
        StopReason::Converged => None,
    }
}

pub fn stderr_of(result: &Output) -> String {
    let mut err = String::new();
    for diagnostic in &result.parse_diagnostics {
        let _ = writeln!(err, "{}", printable(diagnostic));
    }
    for stage in &result.stages {
        if let Some(line) = stop_warning(stage.stage, stage.iterations, stage.stop) {
            let _ = writeln!(err, "{line}");
        }
    }
    for index in &result.panicked {
        let _ = writeln!(
            err,
            "warning: input {} hit an internal error part-way through a rewrite; its output may be incomplete or broken; please report it",
            index + 1
        );
    }
    for (index, note) in result.depth_refusals() {
        let _ = writeln!(
            err,
            "warning: input {} was not processed: {note}",
            index + 1
        );
    }
    for (index, hit) in &result.budget_hits {
        let _ = writeln!(
            err,
            "warning: input {} reached its budget ({}); its passes stopped early and the output is the tree as far as they got",
            index + 1,
            hit.as_str()
        );
    }
    if result.unpack_panicked {
        let _ = writeln!(
            err,
            "warning: unpacking hit an internal error; the inputs were written without a bundle; please report it"
        );
    }
    err
}

pub fn exit_code(result: &Output) -> u8 {
    let panicked = !result.panicked.is_empty() || result.unpack_panicked || result.wasm_panicked;
    if result.parse_errors > 0 || panicked {
        return 1;
    }
    if !result.depth_refused.is_empty() || !result.budget_hits.is_empty() {
        return 3;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_in_the_wasm_step_makes_the_exit_code_non_zero() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("x;");
        assert_eq!(exit_code(&out), 0);

        out.wasm_panicked = true;

        assert_eq!(exit_code(&out), 1);
    }
}
