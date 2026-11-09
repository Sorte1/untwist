use std::ffi::OsStr;

pub const TEST_HOOKS: bool = untwist::TEST_HOOKS;
pub const SUPERVISOR: &str = "UNTWIST_TEST_SUPERVISOR";
pub const LADDER_REFUSE: &str = "UNTWIST_TEST_LADDER_REFUSE";
pub const LADDER_FIRST: &str = "UNTWIST_TEST_LADDER_FIRST";
pub const SCAN_OFF: &str = "UNTWIST_TEST_SCAN_OFF";
pub const HEAP_BUDGET: &str = "UNTWIST_TEST_HEAP_BUDGET";
pub const ADMIT_PANIC: &str = "UNTWIST_TEST_ADMIT_PANIC";
pub const REFUSAL_SPANS: &str = "UNTWIST_TEST_REFUSAL_SPANS";
pub const POINTS_TO: &str = "UNTWIST_TEST_POINTS_TO";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorHook {
    On,
    Off,
    FailExe,
    FailSpawn,
    FailWriter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Worker,
    Supervisor(SupervisorHook),
    InProcess,
}

pub fn supervisor_hook_from(value: Option<&OsStr>) -> SupervisorHook {
    match value.and_then(OsStr::to_str) {
        Some("off") => SupervisorHook::Off,
        Some("fail-exe") if TEST_HOOKS => SupervisorHook::FailExe,
        Some("fail-spawn") if TEST_HOOKS => SupervisorHook::FailSpawn,
        Some("fail-writer") if TEST_HOOKS => SupervisorHook::FailWriter,
        _ => SupervisorHook::On,
    }
}

pub fn supervisor_hook() -> SupervisorHook {
    supervisor_hook_from(std::env::var_os(SUPERVISOR).as_deref())
}

pub fn role(worker: bool, hook: SupervisorHook) -> Role {
    if worker {
        return Role::Worker;
    }
    if hook == SupervisorHook::Off || cfg!(not(unix)) {
        return Role::InProcess;
    }
    Role::Supervisor(hook)
}

fn number(value: Option<&OsStr>) -> Option<usize> {
    value?.to_str()?.trim().parse().ok()
}

fn flag(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| value == "1")
}

pub fn test_hooks_from(read: impl Fn(&str) -> Option<std::ffi::OsString>) -> untwist::TestHooks {
    untwist::TestHooks {
        ladder_refuse: number(read(LADDER_REFUSE).as_deref()).unwrap_or(0),
        ladder_first: number(read(LADDER_FIRST).as_deref()).filter(|bytes| *bytes > 0),
        scan_off: flag(read(SCAN_OFF).as_deref()),
        heap_budget: number(read(HEAP_BUDGET).as_deref())
            .filter(|bytes| *bytes > 0)
            .map(|bytes| bytes as u64),
        admit_panic: number(read(ADMIT_PANIC).as_deref()),
        layer_stub: None,
        refusal_spans: flag(read(REFUSAL_SPANS).as_deref()),
        points_to: flag(read(POINTS_TO).as_deref()),
    }
}

pub fn test_hooks() -> untwist::TestHooks {
    if !TEST_HOOKS {
        return untwist::TestHooks::default();
    }
    test_hooks_from(|name| std::env::var_os(name))
}

#[cfg(test)]
mod tests {
    use super::{
        role, supervisor_hook_from, test_hooks_from, Role, SupervisorHook, ADMIT_PANIC,
        HEAP_BUDGET, LADDER_FIRST, LADDER_REFUSE, POINTS_TO, REFUSAL_SPANS, SCAN_OFF,
    };
    use std::ffi::{OsStr, OsString};

    #[test]
    fn a_worker_never_supervises_whatever_the_hook_says() {
        for hook in [
            SupervisorHook::On,
            SupervisorHook::Off,
            SupervisorHook::FailExe,
            SupervisorHook::FailSpawn,
        ] {
            assert_eq!(role(true, hook), Role::Worker);
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_supervisor_runs_unless_the_hook_turns_it_off() {
        assert_eq!(
            role(false, SupervisorHook::On),
            Role::Supervisor(SupervisorHook::On)
        );
        assert_eq!(role(false, SupervisorHook::Off), Role::InProcess);
        assert_eq!(
            role(false, SupervisorHook::FailSpawn),
            Role::Supervisor(SupervisorHook::FailSpawn)
        );
    }

    #[test]
    fn the_supervisor_hook_is_read_from_its_variable() {
        let read = |value: &str| supervisor_hook_from(Some(OsStr::new(value)));
        assert_eq!(supervisor_hook_from(None), SupervisorHook::On);
        assert_eq!(read("off"), SupervisorHook::Off);
        assert_eq!(read("fail-exe"), SupervisorHook::FailExe);
        assert_eq!(read("fail-spawn"), SupervisorHook::FailSpawn);
        assert_eq!(read("fail-writer"), SupervisorHook::FailWriter);
        assert_eq!(read("on"), SupervisorHook::On);
    }

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
    fn no_variables_mean_no_hooks() {
        assert_eq!(
            test_hooks_from(read_from(&[])),
            untwist::TestHooks::default()
        );
    }

    #[test]
    fn the_hooks_are_read_from_their_variables() {
        let hooks = test_hooks_from(read_from(&[
            (LADDER_REFUSE, "2"),
            (LADDER_FIRST, "67108864"),
            (SCAN_OFF, "1"),
            (HEAP_BUDGET, "1048576"),
            (ADMIT_PANIC, "3"),
            (REFUSAL_SPANS, "1"),
            (POINTS_TO, "1"),
        ]));
        assert_eq!(hooks.admit_panic, Some(3));
        assert!(hooks.refusal_spans);
        assert!(hooks.points_to);
        assert_eq!(hooks.ladder_refuse, 2);
        assert_eq!(hooks.ladder_first, Some(64 << 20));
        assert!(hooks.scan_off);
        assert_eq!(hooks.heap_budget, Some(1 << 20));
    }

    #[test]
    fn malformed_values_are_ignored() {
        let hooks = test_hooks_from(read_from(&[
            (LADDER_REFUSE, "two"),
            (LADDER_FIRST, "0"),
            (SCAN_OFF, "yes"),
            (HEAP_BUDGET, "0"),
        ]));
        assert_eq!(hooks, untwist::TestHooks::default());
    }
}
