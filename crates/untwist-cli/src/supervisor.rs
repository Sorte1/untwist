use std::io::{self, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::args::Args;
use crate::budget::TIME_LEFT;
use crate::channel::{Event, Record, Relay, RelayEnd, Stream};
use crate::hooks::SupervisorHook;
use crate::reruns::{next, Action, Death, Phase, Reruns};
use crate::worker::{OVER_MEMORY, OVER_TIME, REFUSE, SERIAL, UNPACK_LOST, WORKER};

const HELPER_STACK: usize = 64 * 1024;
const DRAIN_BUFFER: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Frame(Stream),
    Raw,
}

type Queue = Vec<(Source, Vec<u8>)>;

#[derive(Default)]
struct Held {
    emitted: bool,
    queue: Queue,
    late_raw: Vec<u8>,
}

impl Held {
    fn keep(&mut self, source: Source, chunk: &[u8]) {
        match self.queue.last_mut() {
            Some((last, bytes)) if *last == source => bytes.extend_from_slice(chunk),
            _ => self.queue.push((source, chunk.to_vec())),
        }
    }
}

type SharedHeld = Arc<Mutex<Held>>;

fn locked(held: &SharedHeld) -> MutexGuard<'_, Held> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

fn helper(body: impl FnOnce() + Send + 'static) -> io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .stack_size(HELPER_STACK)
        .spawn(body)
}

fn drain(mut pipe: ChildStderr, held: SharedHeld) {
    let mut buffer = [0; DRAIN_BUFFER];
    loop {
        let read = match pipe.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        };
        let mut held = locked(&held);
        if held.emitted {
            held.late_raw.extend_from_slice(&buffer[..read]);
        } else {
            held.keep(Source::Raw, &buffer[..read]);
        }
    }
}

fn release(queue: Queue, frames: bool) -> Result<(), (Stream, io::Error)> {
    for (source, bytes) in queue {
        match source {
            Source::Raw => {
                let _ = io::stderr().write_all(&bytes);
            }
            Source::Frame(stream) if frames => {
                write_to(stream, &bytes).map_err(|error| (stream, error))?;
            }
            Source::Frame(_) => {}
        }
    }
    Ok(())
}

fn release_late_raw(held: &SharedHeld) {
    let late = std::mem::take(&mut locked(held).late_raw);
    let _ = io::stderr().write_all(&late);
}

fn write_to(stream: Stream, bytes: &[u8]) -> io::Result<()> {
    match stream {
        Stream::Out => {
            let mut out = io::stdout().lock();
            out.write_all(bytes)?;
            out.flush()
        }
        Stream::Err => io::stderr().write_all(bytes),
    }
}

fn stream_name(stream: Option<Stream>) -> &'static str {
    match stream {
        Some(Stream::Err) => "stderr",
        _ => "stdout",
    }
}

struct Run<'s> {
    phase: Phase,
    held: SharedHeld,
    failed: Option<Stream>,
    stdin: &'s mut Option<Arc<[u8]>>,
}

impl Run<'_> {
    fn on_event(&mut self, event: Event<'_>) -> io::Result<()> {
        match event {
            Event::Record(record) => self.on_record(record),
            Event::Body(stream, chunk) if self.phase.emitted => {
                write_to(stream, chunk).inspect_err(|_| self.failed = Some(stream))
            }
            Event::Body(stream, chunk) => {
                locked(&self.held).keep(Source::Frame(stream), chunk);
                Ok(())
            }
        }
    }

    fn on_record(&mut self, record: Record) -> io::Result<()> {
        match record {
            Record::Begin(index) => {
                self.phase
                    .begun
                    .insert(usize::try_from(index).unwrap_or(usize::MAX));
            }
            Record::Finish(index) => {
                self.phase
                    .begun
                    .remove(&usize::try_from(index).unwrap_or(usize::MAX));
            }
            Record::AfterInputs => self.phase.after_inputs = true,
            Record::Unpack => self.phase.unpack = true,
            Record::Emit => return self.on_emit(),
            Record::Stdout(_) | Record::Stderr(_) => {}
        }
        Ok(())
    }

    fn on_emit(&mut self) -> io::Result<()> {
        self.phase.emitted = true;
        *self.stdin = None;
        let queue = {
            let mut held = locked(&self.held);
            held.emitted = true;
            std::mem::take(&mut held.queue)
        };
        release(queue, true).map_err(|(stream, error)| {
            self.failed = Some(stream);
            error
        })
    }
}

enum Ended {
    Exited {
        death: Death,
        phase: Phase,
        held: SharedHeld,
    },
    RelayFailed(String),
    NoWorker(io::Error),
}

fn executable(hook: SupervisorHook) -> io::Result<PathBuf> {
    if hook == SupervisorHook::FailExe {
        return Err(io::Error::other(
            "the current executable is unknown (test hook)",
        ));
    }
    std::env::current_exe()
}

fn spawn(
    exe: &PathBuf,
    reruns: &Reruns,
    hook: SupervisorHook,
    (piped, time_left): (bool, Option<Duration>),
) -> io::Result<Child> {
    if hook == SupervisorHook::FailSpawn {
        return Err(io::Error::other("spawning is refused (test hook)"));
    }
    let mut argv = std::env::args_os();
    let mut command = Command::new(exe);
    if let Some(name) = argv.next() {
        command.arg0(name);
    }
    command
        .args(argv)
        .env(WORKER, "1")
        .env_remove(REFUSE)
        .env_remove(SERIAL)
        .env_remove(UNPACK_LOST)
        .env_remove(OVER_MEMORY)
        .env_remove(OVER_TIME)
        .env_remove(TIME_LEFT)
        .stdin(if piped { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(list) = reruns.refused_list() {
        command.env(REFUSE, list);
    }
    for (name, hit) in [
        (OVER_MEMORY, untwist::BudgetHit::Memory),
        (OVER_TIME, untwist::BudgetHit::Time),
    ] {
        if let Some(list) = reruns.over_list(hit) {
            command.env(name, list);
        }
    }
    if let Some(left) = time_left {
        command.env(TIME_LEFT, left.as_millis().to_string());
    }
    if reruns.serial {
        command.env(SERIAL, "1");
    }
    match (reruns.unpack_lost, reruns.unpack_over) {
        (true, Some(hit)) => {
            command.env(UNPACK_LOST, hit.as_str());
        }
        (true, None) => {
            command.env(UNPACK_LOST, "1");
        }
        (false, _) => {}
    }
    command.spawn()
}

fn feed_stdin(
    child: &mut Child,
    stdin: &Option<Arc<[u8]>>,
    hook: SupervisorHook,
) -> io::Result<Option<JoinHandle<()>>> {
    let (Some(mut pipe), Some(bytes)) = (child.stdin.take(), stdin.clone()) else {
        return Ok(None);
    };
    if hook == SupervisorHook::FailWriter {
        return Err(io::Error::other(
            "the thread that writes its stdin could not start (test hook)",
        ));
    }
    helper(move || {
        let _ = pipe.write_all(&bytes);
    })
    .map(Some)
}

fn relay(child: &mut Child, run: &mut Run<'_>) -> RelayEnd {
    match child.stdout.take() {
        Some(pipe) => Relay::new(pipe).run(|event| run.on_event(event)),
        None => RelayEnd::Eof,
    }
}

fn supervise(mut child: Child, stdin: &mut Option<Arc<[u8]>>, hook: SupervisorHook) -> Ended {
    let writer = match feed_stdin(&mut child, stdin, hook) {
        Ok(writer) => writer,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Ended::NoWorker(error);
        }
    };
    let held = SharedHeld::default();
    let drainer = child.stderr.take().and_then(|pipe| {
        let held = Arc::clone(&held);
        helper(move || drain(pipe, held)).ok()
    });
    let mut run = Run {
        phase: Phase::default(),
        held,
        failed: None,
        stdin,
    };
    let end = relay(&mut child, &mut run);
    if end.kills_the_worker() {
        let _ = child.kill();
    }
    let status = child.wait();
    for handle in [writer, drainer].into_iter().flatten() {
        let _ = handle.join();
    }
    let death = match status {
        Ok(status) => Death::from_parts(status.code(), status.signal()),
        Err(_) => Death::Exit(1),
    };
    match verdict(end, run.failed, death) {
        Ok(death) => Ended::Exited {
            death,
            phase: run.phase,
            held: run.held,
        },
        Err(message) => Ended::RelayFailed(message),
    }
}

const UNREADABLE: &str = "error: the worker process sent an unreadable record";

fn verdict(end: RelayEnd, failed: Option<Stream>, death: Death) -> Result<Death, String> {
    let died = !matches!(death, Death::Exit(_));
    match end {
        RelayEnd::Eof => Ok(death),
        RelayEnd::Truncated | RelayEnd::ReadFailed(_) if died => Ok(death),
        RelayEnd::Truncated | RelayEnd::Malformed => Err(UNREADABLE.into()),
        RelayEnd::ReadFailed(error) => Err(format!("error: read from the worker process: {error}")),
        RelayEnd::Sink(error) => Err(format!("error: write to {}: {error}", stream_name(failed))),
    }
}

fn read_stdin(args: &Args) -> io::Result<Option<Arc<[u8]>>> {
    if !args.files.is_empty() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    Ok(Some(Arc::from(bytes)))
}

fn fallback(args: &Args, stdin: Option<Arc<[u8]>>, reruns: &Reruns, error: &io::Error) -> u8 {
    let _ = writeln!(
        io::stderr(),
        "warning: could not start a worker process ({error}); running in this process, where an input the depth scan misreads can still crash it"
    );
    let isolation = untwist::Isolation {
        progress: None,
        refused: reruns.refused.iter().copied().collect(),
        over_budget: reruns
            .over_budget
            .iter()
            .map(|(at, hit)| (*at, *hit))
            .collect(),
        unpack_lost: reruns.unpack_lost,
        unpack_over: reruns.unpack_over,
    };
    crate::in_process(args, stdin.map(|bytes| bytes.to_vec()), false, isolation)
}

fn take_queue(held: &SharedHeld) -> Queue {
    std::mem::take(&mut locked(held).queue)
}

fn conclude(death: Death, phase: &Phase, held: &SharedHeld, reruns: &Reruns) -> Result<u8, Reruns> {
    match next(reruns, phase, death) {
        Action::Rerun(rerun) => Err(rerun),
        Action::Exit(code) => {
            if let Err((stream, error)) = release(take_queue(held), true) {
                let name = stream_name(Some(stream));
                let _ = writeln!(io::stderr(), "error: write to {name}: {error}");
                return Ok(1);
            }
            release_late_raw(held);
            Ok(code)
        }
        Action::Fail { message, code } => {
            let _ = release(take_queue(held), false);
            release_late_raw(held);
            let _ = writeln!(io::stderr(), "{message}");
            Ok(code)
        }
    }
}

pub fn run(args: &Args, hook: SupervisorHook) -> u8 {
    if crate::preflight_warning(args, &mut io::stderr()).is_err() {
        return 1;
    }
    let mut stdin = match read_stdin(args) {
        Ok(stdin) => stdin,
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: read stdin: {error}");
            return 1;
        }
    };
    let mut reruns = Reruns::new(args.files.len().max(1));
    let exe = match executable(hook) {
        Ok(exe) => exe,
        Err(error) => return fallback(args, stdin, &reruns, &error),
    };
    let started = Instant::now();
    loop {
        let time_left = args
            .timeout
            .map(|timeout| timeout.saturating_sub(started.elapsed()));
        let child = match spawn(&exe, &reruns, hook, (stdin.is_some(), time_left)) {
            Ok(child) => child,
            Err(error) => return fallback(args, stdin, &reruns, &error),
        };
        match supervise(child, &mut stdin, hook) {
            Ended::RelayFailed(message) => {
                let _ = writeln!(io::stderr(), "{message}");
                return 1;
            }
            Ended::NoWorker(error) => return fallback(args, stdin, &reruns, &error),
            Ended::Exited { death, phase, held } => match conclude(death, &phase, &held, &reruns) {
                Ok(code) => return code,
                Err(rerun) => reruns = rerun,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{verdict, UNREADABLE};
    use crate::channel::{write_record, Record, Relay, RelayEnd, HEADER};
    use crate::reruns::{next, Action, Death, Phase, Reruns};
    use std::io;

    const SIGABRT: i32 = 6;
    const SIGKILL: i32 = 9;

    fn cut_stream(frames: &[(Record, &[u8])], cut_by: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (record, body) in frames {
            write_record(&mut bytes, *record, body).unwrap();
        }
        bytes.truncate(bytes.len() - cut_by);
        bytes
    }

    fn relayed(bytes: &[u8]) -> (Phase, RelayEnd) {
        let mut phase = Phase::default();
        let end = Relay::new(bytes).run(|event| {
            match event {
                crate::channel::Event::Record(Record::Begin(index)) => {
                    phase.begun.insert(index as usize);
                }
                crate::channel::Event::Record(Record::Finish(index)) => {
                    phase.begun.remove(&(index as usize));
                }
                crate::channel::Event::Record(Record::Emit) => phase.emitted = true,
                _ => {}
            }
            Ok(())
        });
        (phase, end)
    }

    #[test]
    fn a_crash_inside_a_header_is_judged_by_the_table() {
        let bytes = cut_stream(
            &[
                (Record::Begin(9), b""),
                (Record::Begin(10), b""),
                (Record::Finish(9), b""),
                (Record::Finish(10), b""),
            ],
            HEADER - 2,
        );
        let (phase, end) = relayed(&bytes);
        assert!(matches!(end, RelayEnd::Truncated));
        let death = verdict(end, None, Death::Crash(SIGKILL)).unwrap();
        let Action::Rerun(rerun) = next(&Reruns::new(12), &phase, death) else {
            panic!("a rerun");
        };
        assert_eq!(rerun.refused.into_iter().collect::<Vec<_>>(), [10]);
    }

    #[test]
    fn a_crash_inside_an_output_body_died_while_writing() {
        let bytes = cut_stream(
            &[(Record::Emit, b""), (Record::Stdout(8), b"var a;\n\n")],
            3,
        );
        let (phase, end) = relayed(&bytes);
        assert!(matches!(end, RelayEnd::Truncated));
        let death = verdict(end, None, Death::Crash(SIGABRT)).unwrap();
        let Action::Fail { message, code } = next(&Reruns::new(1), &phase, death) else {
            panic!("a failure");
        };
        assert_eq!(code, 1);
        assert_eq!(
            message,
            "error: the worker process died while writing its output (signal 6)"
        );
    }

    #[test]
    fn a_cut_stream_from_a_normal_exit_is_unreadable() {
        for code in [0, 1, 3] {
            let judged = verdict(RelayEnd::Truncated, None, Death::Exit(code));
            assert_eq!(judged, Err(UNREADABLE.to_string()));
        }
        let judged = verdict(RelayEnd::Malformed, None, Death::Crash(SIGKILL));
        assert_eq!(judged, Err(UNREADABLE.to_string()));
    }

    #[test]
    fn a_read_error_is_a_death_once_the_worker_is_killed() {
        let failed = RelayEnd::ReadFailed(io::Error::other("gone"));
        assert_eq!(
            verdict(failed, None, Death::Crash(SIGKILL)),
            Ok(Death::Crash(SIGKILL))
        );
        let failed = RelayEnd::ReadFailed(io::Error::other("gone"));
        assert!(verdict(failed, None, Death::Exit(0)).is_err());
        assert_eq!(
            verdict(RelayEnd::Eof, None, Death::Exit(3)),
            Ok(Death::Exit(3))
        );
    }
}
