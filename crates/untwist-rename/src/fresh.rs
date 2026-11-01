use std::sync::atomic::{AtomicUsize, Ordering};

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use untwist_core::semantic::compare::{first_difference, with_fresh_scoping};

use crate::collect::{collect, Collected};
use crate::oracle::OuterRule;
use crate::plan::RenamePlan;
use crate::usage::{collect_usage, Usage};

pub const STALE: &str = "the rename pass was handed a stale scoping";

static SCOPINGS: AtomicUsize = AtomicUsize::new(0);
static PLANS: AtomicUsize = AtomicUsize::new(0);

pub fn scopings_checked() -> usize {
    SCOPINGS.load(Ordering::Relaxed)
}

pub fn plans_checked() -> usize {
    PLANS.load(Ordering::Relaxed)
}

pub fn assert_fresh(program: &Program<'_>, held: &Scoping) {
    if let Some(difference) = first_difference(program, held) {
        panic!("{STALE}: {difference}");
    }
    SCOPINGS.fetch_add(1, Ordering::Relaxed);
}

pub fn assert_same_plan<'a, F>(
    program: &Program<'a>,
    (rule, keep): (OuterRule, bool),
    plan: &RenamePlan,
    decide: &F,
) where
    F: Fn(&Program<'a>, &Collected, &Scoping) -> RenamePlan,
{
    let replayed = with_fresh_scoping(program, |program, fresh| {
        let mut collected = collect(program, fresh, rule);
        crate::collect::pin_script_globals(&mut collected, fresh, keep);
        decide(program, &collected, fresh)
    });
    if &replayed != plan {
        panic!(
            "{STALE}: the plan names {} symbols, a fresh scoping's plan {}",
            plan.len(),
            replayed.len()
        );
    }
    PLANS.fetch_add(1, Ordering::Relaxed);
}

pub fn assert_same_unique_plan<F>(
    program: &Program<'_>,
    (plan, keep): (&RenamePlan, bool),
    decide: F,
) where
    F: FnOnce(&Collected, &Usage, &Scoping) -> RenamePlan,
{
    let replayed = with_fresh_scoping(program, |program, fresh| {
        let mut collected = collect(program, fresh, OuterRule::UsedEnclosing);
        crate::collect::pin_script_globals(&mut collected, fresh, keep);
        let usage = collect_usage(program, fresh);
        decide(&collected, &usage, fresh)
    });
    if &replayed != plan {
        panic!(
            "{STALE}: the unique plan names {} symbols, a fresh scoping's plan {}",
            plan.len(),
            replayed.len()
        );
    }
    PLANS.fetch_add(1, Ordering::Relaxed);
}
