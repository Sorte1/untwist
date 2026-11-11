use std::cell::Cell;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::Thread;
use std::time::{Duration, Instant};

use untwist::{BudgetHit, Isolation, Progress, ProgressSink};

use crate::args::Args;
use crate::budget::{die, hard_deadline};
use crate::Session;

const RUNNING: u8 = 0;
const EMITTED: u8 = 1;
const RESCUE_MEMORY: u8 = 2;
const RESCUE_TIME: u8 = 3;
const BEGUN: u8 = 1;
const FINISHED: u8 = 2;
const EXIT_GAVE_UP: u8 = 1;
const RESCUE_GRACE: Duration = Duration::from_secs(10);
const OVER_AGAIN: &str = "error: the memory budget (--max-memory) ran out again while the inputs \
     over it were being passed through; no output was written\n";
const RESCUE_LATE: &str = "error: passing the inputs over their budget through took too long; \
     no output was written\n";
const NOTHING_READ: &str = "error: an input went over its budget before the inputs were read; \
     no output was written\n";

static STATE: AtomicU8 = AtomicU8::new(RUNNING);
static UNPACK_BEGUN: AtomicBool = AtomicBool::new(false);
static GIVING_UP: AtomicBool = AtomicBool::new(false);
static RESCUER: OnceLock<Thread> = OnceLock::new();
static INPUTS: OnceLock<Box<[AtomicU8]>> = OnceLock::new();

thread_local! {
    static ORIGINAL: Cell<bool> = const { Cell::new(false) };
}

pub(crate) struct Guard<'a> {
    pub(crate) emitted: &'a dyn Fn(),
    pub(crate) inputs_read: &'a dyn Fn(&Arc<[String]>),
}

struct Plan<'a> {
    args: &'a Args,
    base: Isolation,
    read: OnceLock<(Arc<[String]>, Instant)>,
}

impl Plan<'_> {
    fn deadline(&self) -> Option<Instant> {
        let (_, read_at) = self.read.get()?;
        Some(*read_at + hard_deadline(self.args.timeout?))
    }

    fn bound(&self, hit: BudgetHit, read_at: Instant) -> Option<Duration> {
        match hit {
            BudgetHit::Time => Some(read_at.elapsed().max(RESCUE_GRACE)),
            BudgetHit::Memory => self
                .deadline()
                .map(|at| at.saturating_duration_since(Instant::now()) + RESCUE_GRACE),
        }
    }
}

pub(crate) fn guarded(
    args: &Args,
    isolation: Isolation,
    body: impl FnOnce(untwist::Options, &Guard<'_>) -> u8,
) -> u8 {
    let mut options = crate::base_options(args);
    let limits = args.limits();
    if limits.max_memory.is_none() && limits.timeout.is_none() {
        options.isolation = isolation;
        let quiet = Guard {
            emitted: &|| {},
            inputs_read: &|_| {},
        };
        return body(options, &quiet);
    }
    let plan = Plan {
        args,
        base: Isolation {
            progress: None,
            ..isolation.clone()
        },
        read: OnceLock::new(),
    };
    let _ = INPUTS.set(
        (0..args.files.len().max(1))
            .map(|_| AtomicU8::new(0))
            .collect(),
    );
    options.isolation = Isolation {
        progress: Some(tracking(isolation.progress.clone())),
        ..isolation
    };
    std::thread::scope(|scope| {
        let rescuer = scope.spawn(|| watch(&plan));
        let _ = RESCUER.set(rescuer.thread().clone());
        if let Some(bytes) = limits.max_memory {
            untwist::set_hard_memory_limit(bytes, over_memory);
        }
        ORIGINAL.with(|original| original.set(true));
        let inputs_read = |sources: &Arc<[String]>| {
            let _ = plan.read.set((sources.clone(), Instant::now()));
            wake();
        };
        let guard = Guard {
            emitted: &emitted,
            inputs_read: &inputs_read,
        };
        let code = body(options, &guard);
        emitted();
        code
    })
}

fn tracking(inner: Option<ProgressSink>) -> ProgressSink {
    ProgressSink::new(move |progress| {
        track(progress);
        if let Some(inner) = &inner {
            inner.emit(progress);
        }
    })
}

fn track(progress: Progress) {
    let mark = |index: usize, state: u8| {
        if let Some(input) = INPUTS.get().and_then(|inputs| inputs.get(index)) {
            input.store(state, Ordering::SeqCst);
        }
    };
    match progress {
        Progress::InputBegins(index) => {
            ORIGINAL.with(|original| original.set(true));
            mark(index, BEGUN);
        }
        Progress::InputFinished(index) => mark(index, FINISHED),
        Progress::UnpackBegins => UNPACK_BEGUN.store(true, Ordering::SeqCst),
        Progress::InputsFinished | Progress::StageBegins(_) => {}
    }
}

fn begun() -> Vec<usize> {
    INPUTS.get().map_or_else(Vec::new, |inputs| {
        inputs
            .iter()
            .enumerate()
            .filter(|(_, state)| state.load(Ordering::SeqCst) == BEGUN)
            .map(|(index, _)| index)
            .collect()
    })
}

fn wake() {
    if let Some(rescuer) = RESCUER.get() {
        rescuer.unpark();
    }
}

fn park_forever() -> ! {
    loop {
        std::thread::park();
    }
}

fn give_up(message: &str) -> ! {
    if !GIVING_UP.swap(true, Ordering::SeqCst) {
        let _ = io::stderr().write_all(message.as_bytes());
    }
    die(EXIT_GAVE_UP)
}

fn emitted() {
    let ended = STATE.compare_exchange(RUNNING, EMITTED, Ordering::SeqCst, Ordering::SeqCst);
    if matches!(ended, Err(RESCUE_MEMORY | RESCUE_TIME)) {
        park_forever();
    }
    wake();
}

fn over_memory() {
    match STATE.compare_exchange(RUNNING, RESCUE_MEMORY, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => {
            wake();
            park_forever();
        }
        Err(EMITTED) => {}
        Err(_) if ORIGINAL.with(Cell::get) => park_forever(),
        Err(_) => give_up(OVER_AGAIN),
    }
}

fn watch(plan: &Plan<'_>) {
    loop {
        match STATE.load(Ordering::SeqCst) {
            RUNNING => {}
            RESCUE_MEMORY => rescue(plan, BudgetHit::Memory),
            RESCUE_TIME => rescue(plan, BudgetHit::Time),
            _ => return,
        }
        let Some(at) = plan.deadline() else {
            std::thread::park();
            continue;
        };
        let now = Instant::now();
        if now < at {
            std::thread::park_timeout(at - now);
            continue;
        }
        let _ = STATE.compare_exchange(RUNNING, RESCUE_TIME, Ordering::SeqCst, Ordering::SeqCst);
    }
}

fn arm_fallback(bound: Option<Duration>) {
    let Some(bound) = bound else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("untwist-rescue-limit".to_string())
        .stack_size(64 * 1024)
        .spawn(move || {
            std::thread::sleep(bound);
            give_up(RESCUE_LATE);
        });
}

fn rescued(base: &Isolation, hit: BudgetHit, inputs: usize) -> Isolation {
    let in_progress = begun();
    let lose_unpack = in_progress.is_empty() && UNPACK_BEGUN.load(Ordering::SeqCst);
    let blamed: Vec<usize> = if !in_progress.is_empty() {
        in_progress
    } else if lose_unpack {
        Vec::new()
    } else {
        (0..inputs)
            .filter(|index| !base.refused.contains(index))
            .collect()
    };
    let mut refused = base.refused.clone();
    refused.extend(blamed.iter().copied());
    let mut over_budget = base.over_budget.clone();
    over_budget.extend(blamed.iter().map(|index| (*index, hit)));
    Isolation {
        progress: None,
        refused,
        over_budget,
        unpack_lost: base.unpack_lost || lose_unpack,
        unpack_over: base.unpack_over.or(lose_unpack.then_some(hit)),
    }
}

fn rescue(plan: &Plan<'_>, hit: BudgetHit) -> ! {
    if let Some(cap) = plan.args.limits().max_memory {
        let raised = untwist::budget_working_total().saturating_add(cap);
        untwist::set_hard_memory_limit(raised, over_memory);
    }
    let Some((sources, read_at)) = plan.read.get() else {
        give_up(NOTHING_READ);
    };
    arm_fallback(plan.bound(hit, *read_at));
    let mut options = crate::base_options(plan.args);
    options.isolation = rescued(&plan.base, hit, sources.len());
    options.timeout = match hit {
        BudgetHit::Time => Some(Duration::ZERO),
        BudgetHit::Memory => plan
            .args
            .timeout
            .map(|timeout| timeout.saturating_sub(read_at.elapsed())),
    };
    let mut session = Session {
        out: &mut io::stdout(),
        err: &mut io::stderr(),
        stdin: None,
        preflight: false,
        options,
        emitted: &|| {},
        inputs_read: &|_| {},
    };
    let code = crate::finish_on(plan.args, &mut session, sources);
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
    die(code)
}
