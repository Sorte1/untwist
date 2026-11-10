use std::ffi::OsString;
use std::io::Write;
use std::num::NonZeroUsize;

use untwist::{Isolation, Progress, ProgressSink};

use crate::args::Args;
use crate::channel::{cut_record, open_channel, write_marker, FramedWriter, Record, Stream};
use crate::hooks::TEST_HOOKS;

pub const WORKER: &str = "UNTWIST_INTERNAL_WORKER";
pub const REFUSE: &str = "UNTWIST_INTERNAL_REFUSE";
pub const SERIAL: &str = "UNTWIST_INTERNAL_SERIAL";
pub const UNPACK_LOST: &str = "UNTWIST_INTERNAL_UNPACK_LOST";
pub const OVER_MEMORY: &str = "UNTWIST_INTERNAL_OVER_MEMORY";
pub const OVER_TIME: &str = "UNTWIST_INTERNAL_OVER_TIME";
pub const ABORT: &str = "UNTWIST_TEST_WORKER_ABORT";
const MID_RECORD: &str = "mid-record:";
#[cfg(target_os = "linux")]
const OOM_SCORE_ADJ: &str = "/proc/self/oom_score_adj";
#[cfg(target_os = "linux")]
const OOM_SCORE_ADJ_MAX: &str = "1000";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Abort {
    #[default]
    Never,
    AfterBegin(Vec<usize>),
    AfterInputs,
    AfterUnpack,
    AfterEmit,
    MidRecord(usize),
    MidOutput,
}

impl Abort {
    fn parse(value: Option<OsString>) -> Self {
        let Some(value) = value.and_then(|value| value.into_string().ok()) else {
            return Abort::Never;
        };
        if let Some(index) = value.strip_prefix(MID_RECORD) {
            return index.trim().parse().map_or(Abort::Never, Abort::MidRecord);
        }
        match value.as_str() {
            "after-inputs" => Abort::AfterInputs,
            "unpack" => Abort::AfterUnpack,
            "emit" => Abort::AfterEmit,
            "mid-output" => Abort::MidOutput,
            list => indices(list).map_or(Abort::Never, Abort::AfterBegin),
        }
    }

    fn cuts(&self, progress: Progress) -> bool {
        matches!((self, progress), (Abort::MidRecord(at), Progress::InputFinished(index)) if *at == index)
    }

    fn after(&self, progress: Progress) -> bool {
        match (self, progress) {
            (Abort::AfterBegin(indices), Progress::InputBegins(index)) => indices.contains(&index),
            (Abort::AfterInputs, Progress::InputsFinished) => true,
            (Abort::AfterUnpack, Progress::UnpackBegins) => true,
            _ => false,
        }
    }
}

fn indices(list: &str) -> Option<Vec<usize>> {
    list.split(',')
        .map(|index| index.trim().parse().ok())
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rerun {
    pub refused: Vec<usize>,
    pub over_budget: Vec<(usize, untwist::BudgetHit)>,
    pub serial: bool,
    pub unpack_lost: bool,
    pub unpack_over: Option<untwist::BudgetHit>,
    pub abort: Abort,
}

fn over_hit(value: &OsString) -> Option<untwist::BudgetHit> {
    [untwist::BudgetHit::Memory, untwist::BudgetHit::Time]
        .into_iter()
        .find(|hit| value.as_os_str() == hit.as_str())
}

fn is_one(value: Option<OsString>) -> bool {
    value.is_some_and(|value| value == "1")
}

impl Rerun {
    pub fn from(read: impl Fn(&str) -> Option<OsString>) -> Self {
        let refused = read(REFUSE)
            .and_then(|value| value.into_string().ok())
            .and_then(|list| indices(&list))
            .unwrap_or_default();
        let over = |name: &str, hit: untwist::BudgetHit| {
            read(name)
                .and_then(|value| value.into_string().ok())
                .and_then(|list| indices(&list))
                .unwrap_or_default()
                .into_iter()
                .map(move |index| (index, hit))
        };
        let over_budget = over(OVER_MEMORY, untwist::BudgetHit::Memory)
            .chain(over(OVER_TIME, untwist::BudgetHit::Time))
            .collect();
        Self {
            refused,
            over_budget,
            serial: is_one(read(SERIAL)),
            unpack_lost: read(UNPACK_LOST).is_some_and(|value| !value.is_empty()),
            unpack_over: read(UNPACK_LOST).and_then(|value| over_hit(&value)),
            abort: Abort::parse(read(ABORT).filter(|_| TEST_HOOKS)),
        }
    }
}

pub fn is_worker() -> bool {
    is_one(std::env::var_os(WORKER))
}

fn marker(record: Record) {
    if write_marker(record).is_err() {
        std::process::exit(1);
    }
}

fn record_of(progress: Progress) -> Option<Record> {
    let index = |index: usize| u64::try_from(index).unwrap_or(u64::MAX);
    match progress {
        Progress::InputBegins(at) => Some(Record::Begin(index(at))),
        Progress::InputFinished(at) => Some(Record::Finish(index(at))),
        Progress::InputsFinished => Some(Record::AfterInputs),
        Progress::UnpackBegins => Some(Record::Unpack),
        Progress::StageBegins(_) => None,
    }
}

fn isolation(rerun: Rerun) -> (Isolation, bool) {
    let emit_aborts = rerun.abort == Abort::AfterEmit;
    let abort = rerun.abort;
    let progress = ProgressSink::new(move |progress| {
        let Some(record) = record_of(progress) else {
            return;
        };
        if abort.cuts(progress) {
            cut_record(record, &[]);
        }
        marker(record);
        if abort.after(progress) {
            std::process::abort();
        }
    });
    let isolation = Isolation {
        progress: Some(progress),
        refused: rerun.refused,
        over_budget: rerun.over_budget,
        unpack_lost: rerun.unpack_lost,
        unpack_over: rerun.unpack_over,
    };
    (isolation, emit_aborts)
}

fn allocate_as_the_in_process_preflight_does(args: &Args) {
    let _ = crate::preflight_warning(args, &mut std::io::sink());
}

#[cfg(target_os = "linux")]
fn be_the_first_process_the_oom_killer_picks() {
    let _ = std::fs::write(OOM_SCORE_ADJ, OOM_SCORE_ADJ_MAX);
}

#[cfg(not(target_os = "linux"))]
fn be_the_first_process_the_oom_killer_picks() {}

fn arm_budgets(args: &Args, options: &mut untwist::Options) {
    let limits = args.limits();
    let left = crate::budget::time_left_from(std::env::var_os(crate::budget::TIME_LEFT));
    options.timeout = left.or(limits.timeout);
    crate::budget::arm_hard_memory(&limits);
    crate::budget::arm_watchdog(options.timeout);
}

pub fn run(args: &Args) -> u8 {
    be_the_first_process_the_oom_killer_picks();
    let mut options = crate::base_options(args);
    arm_budgets(args, &mut options);
    let _stdout_buffered_in_in_process_order = std::io::stdout();
    if let Err(error) = open_channel() {
        let _ = writeln!(
            std::io::stderr(),
            "error: open the worker's channel: {error}"
        );
        return 1;
    }
    allocate_as_the_in_process_preflight_does(args);
    let rerun = Rerun::from(|name| std::env::var_os(name));
    if rerun.serial {
        options.jobs = NonZeroUsize::new(1);
    }
    let output_cut = rerun.abort == Abort::MidOutput;
    let (isolation, emit_aborts) = isolation(rerun);
    options.isolation = isolation;
    let emitted = move || {
        crate::budget::disarm_watchdog();
        marker(Record::Emit);
        if emit_aborts {
            std::process::abort();
        }
    };
    let mut session = crate::Session {
        out: &mut FramedWriter::new(Stream::Out).cut_midway(output_cut),
        err: &mut FramedWriter::new(Stream::Err),
        stdin: None,
        preflight: false,
        options,
        emitted: &emitted,
    };
    crate::finish(args, &mut session)
}

#[cfg(test)]
mod tests {
    use super::{Abort, Rerun, ABORT, REFUSE, SERIAL, UNPACK_LOST};
    use std::ffi::OsString;
    use untwist::Progress;

    fn read_from(
        pairs: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn no_rerun_variables_mean_a_first_run() {
        assert_eq!(Rerun::from(read_from(&[])), Rerun::default());
    }

    #[test]
    fn the_rerun_variables_are_read() {
        let rerun = Rerun::from(read_from(&[
            (REFUSE, "0,2"),
            (SERIAL, "1"),
            (UNPACK_LOST, "1"),
            (ABORT, "3,1"),
        ]));
        assert_eq!(rerun.refused, [0, 2]);
        assert!(rerun.serial && rerun.unpack_lost);
        assert_eq!(rerun.abort, Abort::AfterBegin(vec![3, 1]));
        assert!(rerun.abort.after(Progress::InputBegins(1)));
        assert!(!rerun.abort.after(Progress::InputBegins(2)));
        assert!(!rerun.abort.after(Progress::InputFinished(1)));
    }

    #[test]
    fn malformed_rerun_variables_are_ignored() {
        let rerun = Rerun::from(read_from(&[
            (REFUSE, "0,x"),
            (SERIAL, "yes"),
            (ABORT, "-1"),
        ]));
        assert_eq!(rerun, Rerun::default());
    }

    #[test]
    fn the_abort_hook_names_a_phase() {
        let unpack = Rerun::from(read_from(&[(ABORT, "unpack")])).abort;
        assert!(unpack.after(Progress::UnpackBegins));
        let finished = Rerun::from(read_from(&[(ABORT, "after-inputs")])).abort;
        assert!(finished.after(Progress::InputsFinished));
        assert!(!finished.after(Progress::UnpackBegins));
        assert_eq!(
            Rerun::from(read_from(&[(ABORT, "emit")])).abort,
            Abort::AfterEmit
        );
        let cut = Rerun::from(read_from(&[(ABORT, "mid-record:10")])).abort;
        assert_eq!(cut, Abort::MidRecord(10));
        assert!(cut.cuts(Progress::InputFinished(10)));
        assert!(!cut.cuts(Progress::InputBegins(10)));
        assert!(!cut.after(Progress::InputFinished(10)));
        assert_eq!(
            Rerun::from(read_from(&[(ABORT, "mid-output")])).abort,
            Abort::MidOutput
        );
        assert_eq!(
            Rerun::from(read_from(&[(ABORT, "mid-record:x")])).abort,
            Abort::Never
        );
    }
}
