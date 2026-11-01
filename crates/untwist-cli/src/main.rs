pub(crate) mod args;
pub(crate) mod channel;
pub(crate) mod hooks;
mod output;
mod preflight;
pub(crate) mod reruns;
mod stdout;
#[cfg(unix)]
mod supervisor;
pub(crate) mod worker;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use args::{Args, ReportFormat};
use clap::Parser;
use hooks::Role;
use untwist::{Isolation, Output, Pipeline, PlanRefusal, StopReason, BUNDLE_JSON, DEOBFUSCATED};

pub(crate) struct Session<'a> {
    pub(crate) out: &'a mut dyn Write,
    pub(crate) err: &'a mut dyn Write,
    pub(crate) stdin: Option<Vec<u8>>,
    pub(crate) preflight: bool,
    pub(crate) options: untwist::Options,
    pub(crate) emitted: &'a dyn Fn(),
}

fn stdin_text(held: Option<Vec<u8>>) -> io::Result<String> {
    let Some(bytes) = held else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        return Ok(buf);
    };
    String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })
}

fn read_inputs(args: &Args, held: Option<Vec<u8>>) -> Result<Vec<String>> {
    if args.files.is_empty() {
        return Ok(vec![stdin_text(held).context("read stdin")?]);
    }
    args.files
        .iter()
        .map(|path| {
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
        })
        .collect()
}

fn report_diagnostics(err: &mut dyn Write, result: &Output) -> io::Result<()> {
    for diagnostic in &result.parse_diagnostics {
        writeln!(err, "{diagnostic}")?;
    }
    for stage in &result.stages {
        match stage.stop {
            StopReason::IterationCap => writeln!(
                err,
                "warning: {} stopped at the iteration cap ({}); output may not be fully simplified",
                stage.stage, stage.iterations
            )?,
            StopReason::Cycle => writeln!(
                err,
                "warning: {} stopped after {} iterations because its passes oscillate; output may not be fully simplified",
                stage.stage, stage.iterations
            )?,
            StopReason::Converged => {}
        }
    }
    for index in &result.panicked {
        writeln!(
            err,
            "warning: input {} hit an internal error part-way through a rewrite; its output may be incomplete or broken; please report it",
            index + 1
        )?;
    }
    for (index, note) in result.depth_refusals() {
        writeln!(
            err,
            "warning: input {} was not processed: {note}",
            index + 1
        )?;
    }
    if result.unpack_panicked {
        writeln!(
            err,
            "warning: unpacking hit an internal error; the inputs were written without a bundle; please report it"
        )?;
    }
    Ok(())
}

const STDERR: &str = "write to stderr";

const OUTPUT_FILES: &[&str] = &[DEOBFUSCATED, BUNDLE_JSON];

enum ResolvedOutput {
    None,
    Conflict,
    Refused,
    Written { path: PathBuf, files: Vec<String> },
}

fn output_files_exist(dir: &Path) -> bool {
    OUTPUT_FILES.iter().any(|name| dir.join(name).is_file())
}

fn is_protected_dir(dir: &Path) -> Result<bool> {
    let Ok(canonical) = dir.canonicalize() else {
        return Ok(false);
    };
    let cwd = std::env::current_dir().context("read current directory")?;
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    if canonical == cwd || canonical.parent().is_none() {
        return Ok(true);
    }
    if let Some(home) = std::env::var_os("HOME") {
        if let Ok(home) = PathBuf::from(home).canonicalize() {
            if canonical == home {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn report_refusals(err: &mut dyn Write, refused: &[PlanRefusal]) -> io::Result<()> {
    for refusal in refused {
        match refusal {
            PlanRefusal::Unsafe(path) => {
                writeln!(
                    err,
                    "warning: refusing to write unsafe module path {path:?}"
                )?;
            }
            PlanRefusal::Collision { path, owner } => {
                writeln!(
                    err,
                    "warning: refusing to write module path {path:?}: {owner}"
                )?;
            }
            PlanRefusal::NotRemoved(path) => {
                writeln!(err, "warning: refusing to remove stale file {path:?}")?;
            }
            PlanRefusal::SuffixesExhausted(path) => {
                writeln!(
                    err,
                    "warning: refusing to write {path:?}: every numbered form of that name is already planned"
                )?;
            }
        }
    }
    Ok(())
}

fn resolve_output(err: &mut dyn Write, args: &Args, result: &Output) -> Result<ResolvedOutput> {
    let Some(dir) = &args.output else {
        return Ok(ResolvedOutput::None);
    };
    if args.force {
        if is_protected_dir(dir)? {
            writeln!(
                err,
                "error: refusing to force-clean {} (current directory, filesystem root, or $HOME)",
                dir.display()
            )
            .context(STDERR)?;
            return Ok(ResolvedOutput::Refused);
        }
    } else if dir.is_dir() && output_files_exist(dir) {
        writeln!(err, "error: output directory already exists (use --force)").context(STDERR)?;
        return Ok(ResolvedOutput::Conflict);
    }
    let report = result.write_to_with(dir, &args.files, args.force)?;
    report_refusals(err, &report.refused).context(STDERR)?;
    Ok(ResolvedOutput::Written {
        path: report.first.unwrap_or_else(|| dir.join(DEOBFUSCATED)),
        files: report.files,
    })
}

fn print_stdout(
    out: &mut dyn Write,
    report: Option<ReportFormat>,
    result: &Output,
    output_path: Option<&Path>,
    files_written: &[String],
) -> Result<()> {
    let written = if matches!(report, Some(ReportFormat::Json)) {
        stdout::write_line(out, &output::to_json(result, output_path, files_written)?)
    } else if output_path.is_none() && result.single_file {
        stdout::write_line(out, &result.concatenated())
    } else if output_path.is_none() {
        stdout::write_codes(out, &result.codes)
    } else {
        Ok(())
    };
    written
        .and_then(|()| out.flush())
        .context("write to stdout")
}

fn exit_code(result: &Output) -> u8 {
    if result.parse_errors > 0 || !result.panicked.is_empty() || result.unpack_panicked {
        return 1;
    }
    if !result.depth_refused.is_empty() {
        return 3;
    }
    0
}

pub(crate) fn preflight_warning(args: &Args, err: &mut dyn Write) -> Result<()> {
    if let Some(budget) = preflight::budget(&args.files) {
        let shape = preflight::Shape {
            width: untwist::job_width(args.jobs, args.files.len()),
            unpack: args.unpack,
        };
        if let Some(text) = preflight::warning(&budget, args.max_input_mb, shape) {
            writeln!(err, "warning: {text}").context(STDERR)?;
        }
    }
    Ok(())
}

pub(crate) fn base_options(args: &Args) -> untwist::Options {
    args.options().with_test_hooks(hooks::test_hooks())
}

fn run(args: &Args, session: &mut Session<'_>) -> Result<u8> {
    if session.preflight {
        preflight_warning(args, session.err)?;
    }
    let sources = read_inputs(args, session.stdin.take())?;
    let borrowed: Vec<&str> = sources.iter().map(String::as_str).collect();
    let result = Pipeline::new(std::mem::take(&mut session.options)).run_many(&borrowed);
    (session.emitted)();
    let err = &mut *session.err;
    report_diagnostics(err, &result).context(STDERR)?;

    let report = args
        .report
        .or_else(|| args.verbose.then_some(ReportFormat::Text));

    let (output_path, files_written) = match resolve_output(err, args, &result)? {
        ResolvedOutput::Conflict => return Ok(2),
        ResolvedOutput::Refused => return Ok(1),
        ResolvedOutput::Written { path, files } => (Some(path), files),
        ResolvedOutput::None => (None, Vec::new()),
    };

    print_stdout(
        session.out,
        report,
        &result,
        output_path.as_deref(),
        &files_written,
    )?;
    let err = &mut *session.err;
    if matches!(report, Some(ReportFormat::Text)) {
        err.write_all(output::text_report(&result).as_bytes())
            .context(STDERR)?;
    }
    Ok(exit_code(&result))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
const GLIBC_THRESHOLD_RAISING_BLOCK_BYTES: usize = 20 * 1024 * 1024;

pub(crate) fn finish(args: &Args, session: &mut Session<'_>) -> u8 {
    match run(args, session) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(session.err, "error: {error:#}");
            1
        }
    }
}

pub(crate) fn in_process(
    args: &Args,
    stdin: Option<Vec<u8>>,
    preflight: bool,
    isolation: Isolation,
) -> u8 {
    let mut options = base_options(args);
    options.isolation = isolation;
    let mut session = Session {
        out: &mut io::stdout(),
        err: &mut io::stderr(),
        stdin,
        preflight,
        options,
        emitted: &|| {},
    };
    finish(args, &mut session)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub(crate) fn raise_glibc_dynamic_mmap_and_trim_thresholds_by_freeing_one_untouched_block() {
    drop(std::hint::black_box(Vec::<u8>::with_capacity(
        GLIBC_THRESHOLD_RAISING_BLOCK_BYTES,
    )));
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
pub(crate) fn raise_glibc_dynamic_mmap_and_trim_thresholds_by_freeing_one_untouched_block() {}

#[cfg(unix)]
fn supervised(args: &Args, hook: hooks::SupervisorHook) -> u8 {
    supervisor::run(args, hook)
}

#[cfg(not(unix))]
fn supervised(args: &Args, _: hooks::SupervisorHook) -> u8 {
    in_process(args, None, true, Isolation::default())
}

fn main() -> ExitCode {
    main_with(&|| {}, &|| {})
}

pub(crate) fn main_with(parsed: &dyn Fn(), finished: &dyn Fn()) -> ExitCode {
    let role = hooks::role(worker::is_worker(), hooks::supervisor_hook());
    if !matches!(role, Role::Supervisor(_)) {
        raise_glibc_dynamic_mmap_and_trim_thresholds_by_freeing_one_untouched_block();
    }
    let args = Args::parse();
    if args.list_passes {
        print!("{}", args::pass_list());
        return ExitCode::SUCCESS;
    }
    parsed();
    let code = match role {
        Role::Worker => worker::run(&args),
        Role::Supervisor(hook) => supervised(&args, hook),
        Role::InProcess => in_process(&args, None, true, Isolation::default()),
    };
    finished();
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::exit_code;

    #[test]
    fn a_panicked_input_makes_the_exit_code_non_zero() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("");
        out.panicked = vec![0];
        assert_eq!(exit_code(&out), 1);
    }

    #[test]
    fn a_lost_bundle_makes_the_exit_code_non_zero() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("");
        out.unpack_panicked = true;
        assert_eq!(exit_code(&out), 1);
    }

    #[test]
    fn a_refused_input_exits_three_unless_something_outranks_it() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("");
        out.depth_refused = vec![0];
        assert_eq!(exit_code(&out), 3);
        out.parse_errors = 1;
        assert_eq!(exit_code(&out), 1);
        out.parse_errors = 0;
        out.unpack_panicked = true;
        assert_eq!(exit_code(&out), 1);
    }

    #[test]
    fn a_clean_run_exits_zero() {
        let out = untwist::Pipeline::new(untwist::Options::default()).run("");
        assert_eq!(exit_code(&out), 0);
    }
}
