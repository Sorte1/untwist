pub(crate) mod args;
pub(crate) mod budget;
pub(crate) mod channel;
pub(crate) mod hooks;
mod output;
mod preflight;
pub(crate) mod reruns;
mod rescue;
mod stdout;
#[cfg(unix)]
mod supervisor;
mod wasm_supply;
pub(crate) mod worker;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use args::{Args, ReportFormat};
use clap::Parser;
use hooks::Role;
use untwist::{
    printable, Isolation, Output, OutputConflict, Pipeline, PlanRefusal, StopReason, DEOBFUSCATED,
};

pub(crate) struct Session<'a> {
    pub(crate) out: &'a mut dyn Write,
    pub(crate) err: &'a mut dyn Write,
    pub(crate) stdin: Option<Vec<u8>>,
    pub(crate) preflight: bool,
    pub(crate) options: untwist::Options,
    pub(crate) emitted: &'a dyn Fn(),
    pub(crate) inputs_read: &'a dyn Fn(&Arc<[String]>),
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
        writeln!(err, "{}", printable(diagnostic))?;
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
            StopReason::Budget => writeln!(
                err,
                "warning: {} stopped after {} iterations because the run reached its budget; output may not be fully simplified",
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
    for (index, hit) in &result.budget_hits {
        writeln!(
            err,
            "warning: input {} reached its budget ({}); its passes stopped early and the output is the tree as far as they got",
            index + 1,
            hit.as_str()
        )?;
    }
    if result.unpack_panicked {
        writeln!(
            err,
            "warning: unpacking hit an internal error; the inputs were written without a bundle; please report it"
        )?;
    }
    if let Some(hit) = result.unpack_budget {
        writeln!(
            err,
            "warning: unpacking reached the run's budget ({}); the inputs were written without a bundle",
            hit.as_str()
        )?;
    }
    Ok(())
}

const STDERR: &str = "write to stderr";

fn report_wasm(err: &mut dyn Write, result: &Output) -> io::Result<()> {
    for note in result.notes.iter().filter(|note| note.pass == "wasm") {
        match note.kind {
            untwist::NoteKind::Refusal => writeln!(err, "warning: {}", printable(&note.message))?,
            untwist::NoteKind::Progress => writeln!(err, "note: {}", printable(&note.message))?,
        }
    }
    if result.wasm_unchecked {
        writeln!(
            err,
            "note: the output may hold embedded WebAssembly; pass --wasm-check auto or on to extract it"
        )?;
    }
    for reference in &result.wasm_unresolved {
        writeln!(
            err,
            "note: wasm module {} was not resolved; it is loaded from {}; save it under that name and pass --wasm-dir, or pass --fetch-wasm",
            printable(&reference.name),
            printable(&reference.url)
        )?;
    }
    Ok(())
}

enum ResolvedOutput {
    None,
    Conflict,
    Refused,
    Written {
        written: output::Written,
        failed: bool,
    },
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
    let is_home = homes(|name| std::env::var_os(name))
        .into_iter()
        .filter_map(|home| home.canonicalize().ok())
        .any(|home| home == canonical);
    Ok(is_home)
}

fn homes(read: impl Fn(&str) -> Option<std::ffi::OsString>) -> Vec<PathBuf> {
    let drive_and_path = read("HOMEDRIVE")
        .zip(read("HOMEPATH"))
        .map(|(drive, path)| {
            let mut joined = drive;
            joined.push(path);
            joined
        });
    [read("HOME"), read("USERPROFILE"), drive_and_path]
        .into_iter()
        .flatten()
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .collect()
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
            PlanRefusal::InputFile(path) => {
                writeln!(
                    err,
                    "warning: refusing to write {path:?}: it is one of the input files"
                )?;
            }
            PlanRefusal::Failed {
                path,
                action,
                reason,
            } => {
                writeln!(
                    err,
                    "warning: could not {action} {path:?}: {}",
                    printable(reason)
                )?;
            }
        }
    }
    Ok(())
}

fn conflict(err: &mut dyn Write, error: anyhow::Error) -> Result<ResolvedOutput> {
    let Some(conflict) = error.downcast_ref::<OutputConflict>() else {
        return Err(error);
    };
    writeln!(err, "error: {conflict} (use --force)").context(STDERR)?;
    Ok(ResolvedOutput::Conflict)
}

fn resolve_output(err: &mut dyn Write, args: &Args, result: &Output) -> Result<ResolvedOutput> {
    let Some(dir) = &args.output else {
        return Ok(ResolvedOutput::None);
    };
    if is_protected_dir(dir)? {
        writeln!(
            err,
            "error: refusing to write into {} (current directory, filesystem root, or the home directory)",
            printable(&dir.display().to_string())
        )
        .context(STDERR)?;
        return Ok(ResolvedOutput::Refused);
    }
    let report = match result.write_to_with(dir, &args.files, args.force) {
        Ok(report) => report,
        Err(error) => return conflict(err, error),
    };
    report_refusals(err, &report.refused).context(STDERR)?;
    let failed = report
        .refused
        .iter()
        .any(|refusal| matches!(refusal, PlanRefusal::Failed { .. }));
    Ok(ResolvedOutput::Written {
        written: output::Written {
            path: Some(report.first.unwrap_or_else(|| dir.join(DEOBFUSCATED))),
            files: report.files,
            unwritten_modules: report.unwritten_modules,
        },
        failed,
    })
}

fn print_stdout(
    out: &mut dyn Write,
    (report, limits): (Option<ReportFormat>, budget::Limits),
    result: &Output,
    written: &output::Written,
) -> Result<()> {
    let printed = if matches!(report, Some(ReportFormat::Json)) {
        stdout::write_line(out, &output::to_json(result, written, limits)?)
    } else if written.path.is_none() && result.single_file {
        stdout::write_line(out, &result.concatenated())
    } else if written.path.is_none() {
        stdout::write_codes(out, &result.codes)
    } else {
        Ok(())
    };
    printed
        .and_then(|()| out.flush())
        .context("write to stdout")
}

fn exit_code(result: &Output) -> u8 {
    let panicked = !result.panicked.is_empty() || result.unpack_panicked || result.wasm_panicked;
    if result.parse_errors > 0 || panicked {
        return 1;
    }
    if !result.depth_refused.is_empty()
        || !result.budget_hits.is_empty()
        || result.unpack_budget.is_some()
    {
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

const INLINE_INPUT_MAP: &str = "inline";

fn input_source_maps(args: &Args, sources: &[String]) -> Result<Vec<Option<String>>> {
    let Some(given) = &args.input_source_map else {
        return Ok(Vec::new());
    };
    if given == INLINE_INPUT_MAP {
        return Ok(sources
            .iter()
            .map(|source| Output::input_source_map(source))
            .collect());
    }
    if sources.len() > 1 {
        anyhow::bail!("--input-source-map takes a file only with a single input; use inline");
    }
    let text = std::fs::read_to_string(given).with_context(|| format!("read {given}"))?;
    if untwist::source_map::parse(&text).is_none() {
        anyhow::bail!("{given} is not a source map");
    }
    Ok(vec![Some(text)])
}

fn check_source_map_target(args: &Args) -> Result<()> {
    if args.source_map == Some(untwist::MapLink::File) && args.output.is_none() {
        anyhow::bail!(
            "--source-map file needs --output DIR; use --source-map inline to append the map to stdout"
        );
    }
    Ok(())
}

fn run(args: &Args, session: &mut Session<'_>) -> Result<u8> {
    check_source_map_target(args)?;
    if session.preflight {
        preflight_warning(args, session.err)?;
    }
    let sources: Arc<[String]> = read_inputs(args, session.stdin.take())?.into();
    (session.inputs_read)(&sources);
    run_on(args, session, &sources)
}

fn run_on(args: &Args, session: &mut Session<'_>, sources: &[String]) -> Result<u8> {
    let borrowed: Vec<&str> = sources.iter().map(String::as_str).collect();
    let mut options = std::mem::take(&mut session.options);
    options.input_source_maps = input_source_maps(args, sources)?;
    let mut result = Pipeline::new(options).run_many(&borrowed);
    result.source_map_link = args.source_map;
    if args.source_map.is_some() && args.output.is_none() {
        result.codes = result.inline_linked_codes(&args.files);
    }
    (session.emitted)();
    let err = &mut *session.err;
    wasm_supply::settle(args, &mut result, err)?;
    report_diagnostics(err, &result).context(STDERR)?;
    report_wasm(err, &result).context(STDERR)?;

    let report = args
        .report
        .or_else(|| args.verbose.then_some(ReportFormat::Text));

    let (written, write_failed) = match resolve_output(err, args, &result)? {
        ResolvedOutput::Conflict => return Ok(2),
        ResolvedOutput::Refused => return Ok(1),
        ResolvedOutput::Written { written, failed } => (written, failed),
        ResolvedOutput::None => (output::Written::default(), false),
    };

    print_stdout(session.out, (report, args.limits()), &result, &written)?;
    let err = &mut *session.err;
    if matches!(report, Some(ReportFormat::Text)) {
        err.write_all(output::text_report(&result, args.limits()).as_bytes())
            .context(STDERR)?;
    }
    Ok(match exit_code(&result) {
        0 if write_failed => 1,
        code => code,
    })
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
const GLIBC_THRESHOLD_RAISING_BLOCK_BYTES: usize = 20 * 1024 * 1024;

fn reported(err: &mut dyn Write, outcome: Result<u8>) -> u8 {
    match outcome {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", printable(&format!("{error:#}")));
            1
        }
    }
}

pub(crate) fn finish(args: &Args, session: &mut Session<'_>) -> u8 {
    let outcome = run(args, session);
    reported(session.err, outcome)
}

pub(crate) fn finish_on(args: &Args, session: &mut Session<'_>, sources: &[String]) -> u8 {
    let outcome = run_on(args, session, sources);
    reported(session.err, outcome)
}

pub(crate) fn in_process(
    args: &Args,
    stdin: Option<Vec<u8>>,
    preflight: bool,
    isolation: Isolation,
) -> u8 {
    rescue::guarded(args, isolation, |options, guard| {
        let mut session = Session {
            out: &mut io::stdout(),
            err: &mut io::stderr(),
            stdin,
            preflight,
            options,
            emitted: guard.emitted,
            inputs_read: guard.inputs_read,
        };
        finish(args, &mut session)
    })
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

#[global_allocator]
static ALLOCATOR: budget::Counting = budget::counting();

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
    fn every_home_variable_names_a_protected_directory() {
        let read = |name: &str| {
            let value = match name {
                "HOME" => "/home/u",
                "USERPROFILE" => "C:\\Users\\u",
                "HOMEDRIVE" => "D:",
                "HOMEPATH" => "\\Users\\v",
                _ => return None,
            };
            Some(std::ffi::OsString::from(value))
        };
        let homes = super::homes(read);
        assert_eq!(
            homes,
            [
                std::path::PathBuf::from("/home/u"),
                std::path::PathBuf::from("C:\\Users\\u"),
                std::path::PathBuf::from("D:\\Users\\v"),
            ]
        );
        assert!(super::homes(|_| None).is_empty());
        let drive_only = |name: &str| (name == "HOMEDRIVE").then(|| "D:".into());
        assert!(super::homes(drive_only).is_empty());
    }

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
    fn a_panic_in_the_wasm_step_makes_the_exit_code_non_zero() {
        let mut out = untwist::Pipeline::new(untwist::Options::default()).run("");
        out.wasm_panicked = true;
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
