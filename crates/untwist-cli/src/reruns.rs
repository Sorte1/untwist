use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use untwist::BudgetHit;

#[cfg(target_os = "linux")]
const SIGBUS: i32 = 7;
#[cfg(not(target_os = "linux"))]
const SIGBUS: i32 = 10;
const CRASH_SIGNALS: [i32; 5] = [4, 6, SIGBUS, 9, 11];
const SPAWNS_BEYOND_INPUTS: usize = 3;
const SIGNAL_EXIT_BASE: i32 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Death {
    Exit(u8),
    Crash(i32),
    Over(BudgetHit),
    Stopped(i32),
}

#[derive(Clone, Copy)]
enum Cause {
    Signal(i32),
    Budget(BudgetHit),
}

impl fmt::Display for Cause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cause::Signal(signal) => write!(formatter, "signal {signal}"),
            Cause::Budget(hit) => formatter.write_str(hit.as_str()),
        }
    }
}

impl Death {
    pub fn from_parts(code: Option<i32>, signal: Option<i32>) -> Death {
        if let Some(code) = code {
            let code = u8::try_from(code).unwrap_or(1);
            return crate::budget::hit_of_exit(code).map_or(Death::Exit(code), Death::Over);
        }
        let signal = signal.unwrap_or(0);
        if CRASH_SIGNALS.contains(&signal) {
            return Death::Crash(signal);
        }
        Death::Stopped(signal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reruns {
    pub refused: BTreeSet<usize>,
    pub over_budget: BTreeMap<usize, BudgetHit>,
    pub serial: bool,
    pub unpack_lost: bool,
    pub unpack_over: Option<BudgetHit>,
    pub spawns: usize,
    inputs: usize,
}

impl Reruns {
    pub fn new(inputs: usize) -> Self {
        Self {
            refused: BTreeSet::new(),
            over_budget: BTreeMap::new(),
            serial: false,
            unpack_lost: false,
            unpack_over: None,
            spawns: 1,
            inputs,
        }
    }

    pub fn most_spawns(&self) -> usize {
        self.inputs + SPAWNS_BEYOND_INPUTS
    }

    pub fn refused_list(&self) -> Option<String> {
        if self.refused.is_empty() {
            return None;
        }
        let indices: Vec<String> = self.refused.iter().map(usize::to_string).collect();
        Some(indices.join(","))
    }

    pub fn over_list(&self, hit: BudgetHit) -> Option<String> {
        let indices: Vec<String> = self
            .over_budget
            .iter()
            .filter(|(_, over)| **over == hit)
            .map(|(index, _)| index.to_string())
            .collect();
        (!indices.is_empty()).then(|| indices.join(","))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Phase {
    pub begun: BTreeSet<usize>,
    pub after_inputs: bool,
    pub unpack: bool,
    pub emitted: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Exit(u8),
    Rerun(Reruns),
    Fail { message: String, code: u8 },
}

fn fail(message: String) -> Action {
    Action::Fail { message, code: 1 }
}

fn served_after_inputs(state: &Reruns, phase: &Phase) -> Option<Vec<usize>> {
    if !phase.after_inputs {
        return None;
    }
    let served: Vec<usize> = (0..state.inputs)
        .filter(|index| !state.refused.contains(index))
        .collect();
    (!served.is_empty()).then_some(served)
}

pub fn next(state: &Reruns, phase: &Phase, death: Death) -> Action {
    let signal = match death {
        Death::Exit(code) => return Action::Exit(code),
        Death::Stopped(signal) => {
            let code = u8::try_from(SIGNAL_EXIT_BASE + signal).unwrap_or(u8::MAX);
            let message = format!("error: the worker process was stopped by signal {signal}");
            return Action::Fail { message, code };
        }
        Death::Crash(signal) => Cause::Signal(signal),
        Death::Over(hit) => Cause::Budget(hit),
    };
    if phase.emitted {
        return fail(format!(
            "error: the worker process died while writing its output ({signal})"
        ));
    }
    let mut rerun = state.clone();
    if phase.begun.len() == 1 || (state.serial && !phase.begun.is_empty()) {
        if phase
            .begun
            .iter()
            .any(|index| state.refused.contains(index))
        {
            return kept_dying(signal);
        }
        rerun.refused.extend(phase.begun.iter().copied());
        if let Cause::Budget(hit) = signal {
            rerun
                .over_budget
                .extend(phase.begun.iter().map(|index| (*index, hit)));
        }
    } else if phase.begun.len() > 1 {
        rerun.serial = true;
    } else if phase.unpack && !state.unpack_lost {
        rerun.unpack_lost = true;
        if let Cause::Budget(hit) = signal {
            rerun.unpack_over = Some(hit);
        }
    } else if let Some(served) = served_after_inputs(state, phase) {
        rerun.refused.extend(served);
    } else {
        return fail(format!(
            "error: the worker process died outside any input ({signal})"
        ));
    }
    rerun.spawns += 1;
    if rerun.spawns > state.most_spawns() {
        return kept_dying(signal);
    }
    Action::Rerun(rerun)
}

fn kept_dying(signal: Cause) -> Action {
    fail(format!(
        "error: the worker process kept dying after every rerun ({signal})"
    ))
}

#[cfg(test)]
mod tests {
    use super::{next, Action, Death, Phase, Reruns, SIGBUS};
    use std::collections::BTreeSet;

    const SIGABRT: i32 = 6;
    const SIGSEGV: i32 = 11;
    const SIGKILL: i32 = 9;
    const SIGTERM: i32 = 15;
    const SIGINT: i32 = 2;

    fn phase(begun: &[usize], unpack: bool, emitted: bool) -> Phase {
        Phase {
            begun: begun.iter().copied().collect(),
            after_inputs: unpack,
            unpack,
            emitted,
        }
    }

    fn after_inputs() -> Phase {
        Phase {
            after_inputs: true,
            ..Phase::default()
        }
    }

    #[test]
    fn deaths_are_read_from_the_exit_code_or_the_signal() {
        assert_eq!(Death::from_parts(Some(3), None), Death::Exit(3));
        assert_eq!(Death::from_parts(Some(0), None), Death::Exit(0));
        for signal in [4, SIGABRT, SIGBUS, SIGKILL, SIGSEGV] {
            assert_eq!(Death::from_parts(None, Some(signal)), Death::Crash(signal));
        }
        for signal in [SIGINT, SIGTERM, 1, 13] {
            assert_eq!(
                Death::from_parts(None, Some(signal)),
                Death::Stopped(signal)
            );
        }
    }

    #[test]
    fn a_normal_exit_of_any_code_is_passed_through() {
        for code in [0, 1, 2, 3, 101] {
            let action = next(&Reruns::new(2), &phase(&[0], true, true), Death::Exit(code));
            assert_eq!(action, Action::Exit(code));
        }
    }

    #[test]
    fn a_budget_death_refuses_the_input_and_names_the_budget() {
        let Action::Rerun(rerun) = next(
            &Reruns::new(3),
            &phase(&[1], false, false),
            Death::Over(untwist::BudgetHit::Memory),
        ) else {
            panic!("a rerun");
        };
        assert_eq!(rerun.refused, BTreeSet::from([1]));
        assert_eq!(
            rerun.over_list(untwist::BudgetHit::Memory).as_deref(),
            Some("1")
        );
        assert_eq!(rerun.over_list(untwist::BudgetHit::Time), None);
        assert_eq!(
            Death::from_parts(Some(i32::from(crate::budget::EXIT_OVER_TIME)), None),
            Death::Over(untwist::BudgetHit::Time)
        );
        let Action::Fail { message, .. } = next(
            &Reruns::new(1),
            &phase(&[0], false, true),
            Death::Over(untwist::BudgetHit::Time),
        ) else {
            panic!("a failure");
        };
        assert!(message.ends_with("(budget-time)"), "{message}");
    }

    #[test]
    fn a_crash_with_one_input_in_progress_refuses_it() {
        let Action::Rerun(rerun) = next(
            &Reruns::new(4),
            &phase(&[2], false, false),
            Death::Crash(SIGSEGV),
        ) else {
            panic!("a rerun");
        };
        assert_eq!(rerun.refused, BTreeSet::from([2]));
        assert_eq!(
            (rerun.serial, rerun.unpack_lost, rerun.spawns),
            (false, false, 2)
        );
        assert_eq!(rerun.refused_list().as_deref(), Some("2"));
    }

    #[test]
    fn a_crash_with_several_in_progress_reruns_serially_first() {
        let Action::Rerun(rerun) = next(
            &Reruns::new(4),
            &phase(&[0, 1, 3], false, false),
            Death::Crash(SIGKILL),
        ) else {
            panic!("a rerun");
        };
        assert!(rerun.serial && rerun.refused.is_empty());
        let Action::Rerun(again) = next(&rerun, &phase(&[1], false, false), Death::Crash(SIGABRT))
        else {
            panic!("a second rerun");
        };
        assert_eq!(again.refused, BTreeSet::from([1]));
    }

    #[test]
    fn a_crash_in_unpack_reruns_without_it_once() {
        let Action::Rerun(rerun) = next(
            &Reruns::new(2),
            &phase(&[], true, false),
            Death::Crash(SIGABRT),
        ) else {
            panic!("a rerun");
        };
        assert!(rerun.unpack_lost);
        let unpack_alone = Phase {
            unpack: true,
            ..Phase::default()
        };
        let again = next(&rerun, &unpack_alone, Death::Crash(SIGABRT));
        assert!(matches!(again, Action::Fail { code: 1, .. }));
    }

    #[test]
    fn a_budget_kill_in_unpack_reruns_without_it_as_a_budget_stop() {
        let Action::Rerun(rerun) = next(
            &Reruns::new(2),
            &phase(&[], true, false),
            Death::Over(untwist::BudgetHit::Memory),
        ) else {
            panic!("a rerun");
        };
        assert!(rerun.unpack_lost && rerun.refused.is_empty());
        assert_eq!(rerun.unpack_over, Some(untwist::BudgetHit::Memory));
    }

    #[test]
    fn a_crash_after_every_input_finished_refuses_the_inputs_it_serves() {
        let mut state = Reruns::new(3);
        state.refused.insert(1);
        let Action::Rerun(rerun) = next(&state, &after_inputs(), Death::Crash(SIGKILL)) else {
            panic!("a rerun");
        };
        assert_eq!(rerun.refused, BTreeSet::from([0, 1, 2]));
        assert_eq!((rerun.serial, rerun.unpack_lost), (false, false));
        let again = next(&rerun, &after_inputs(), Death::Crash(SIGKILL));
        assert!(matches!(again, Action::Fail { code: 1, .. }));
    }

    #[test]
    fn a_crash_in_unpack_drops_the_bundle_before_it_refuses_the_inputs() {
        let Action::Rerun(lost) = next(
            &Reruns::new(2),
            &phase(&[], true, false),
            Death::Crash(SIGKILL),
        ) else {
            panic!("a rerun");
        };
        assert!(lost.unpack_lost && lost.refused.is_empty());
        let Action::Rerun(refused) = next(&lost, &after_inputs(), Death::Crash(SIGKILL)) else {
            panic!("a second rerun");
        };
        assert_eq!(refused.refused, BTreeSet::from([0, 1]));
    }

    #[test]
    fn crashes_that_blame_no_input_fail_without_a_rerun() {
        let written = next(
            &Reruns::new(2),
            &phase(&[], true, true),
            Death::Crash(SIGABRT),
        );
        assert_eq!(
            written,
            Action::Fail {
                message: "error: the worker process died while writing its output (signal 6)"
                    .to_string(),
                code: 1
            }
        );
        let outside = next(
            &Reruns::new(2),
            &phase(&[], false, false),
            Death::Crash(SIGSEGV),
        );
        assert_eq!(
            outside,
            Action::Fail {
                message: "error: the worker process died outside any input (signal 11)".to_string(),
                code: 1
            }
        );
    }

    #[test]
    fn another_signal_stops_with_its_own_exit_code() {
        let stopped = next(
            &Reruns::new(1),
            &phase(&[0], false, false),
            Death::Stopped(SIGTERM),
        );
        assert_eq!(
            stopped,
            Action::Fail {
                message: "error: the worker process was stopped by signal 15".to_string(),
                code: 143
            }
        );
    }

    fn worst_case(state: &Reruns, inputs: usize, deaths: &mut usize) {
        let live: Vec<usize> = (0..inputs).filter(|i| !state.refused.contains(i)).collect();
        let mut phases: Vec<Phase> = vec![
            phase(&[], false, false),
            phase(&[], false, true),
            after_inputs(),
        ];
        if !state.unpack_lost {
            phases.push(phase(&[], true, false));
        }
        for mask in 1_usize..(1 << live.len()) {
            let begun: Vec<usize> = (0..live.len())
                .filter(|bit| mask & (1 << bit) != 0)
                .map(|bit| live[bit])
                .collect();
            if !state.serial || begun.len() == 1 {
                phases.push(phase(&begun, false, false));
            }
        }
        for each in phases {
            *deaths += 1;
            assert!(each
                .begun
                .iter()
                .all(|index| !state.refused.contains(index)));
            if let Action::Rerun(rerun) = next(state, &each, Death::Crash(SIGSEGV)) {
                assert!(rerun.spawns <= inputs + 3, "{rerun:?}");
                assert!(rerun.refused.len() <= inputs);
                worst_case(&rerun, inputs, deaths);
            }
        }
    }

    #[test]
    fn every_death_sequence_of_a_four_input_run_spawns_at_most_seven_workers() {
        let mut deaths = 0;
        worst_case(&Reruns::new(4), 4, &mut deaths);
        assert!(deaths > 1_000, "{deaths}");
    }
}
