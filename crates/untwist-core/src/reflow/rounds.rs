use std::cell::RefCell;

use super::{reflow_once, GuardRecord};

thread_local! {
    static WITNESS: RefCell<Option<Witnessed>> = const { RefCell::new(None) };
}

#[derive(Debug, Default)]
pub struct Witnessed {
    pub compared: usize,
    pub in_rounds: usize,
    pub placed: usize,
    pub reparsed: usize,
    pub differing: Vec<String>,
}

type Finished = (String, GuardRecord, Option<bool>);

pub fn recording<R>(body: impl FnOnce() -> R) -> (R, Witnessed) {
    let outer = WITNESS.with(|slot| slot.replace(Some(Witnessed::default())));
    let value = body();
    let witnessed = WITNESS.with(|slot| slot.replace(outer));
    (value, witnessed.unwrap_or_default())
}

pub(super) fn is_recording() -> bool {
    WITNESS.with(|slot| slot.borrow().is_some())
}

pub(super) fn witness(before: String, code: &str, record: &GuardRecord) {
    let (rounds, settled, expected) = in_rounds(before);
    WITNESS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(witnessed) = slot.as_mut() else {
            return;
        };
        witnessed.compared += 1;
        witnessed.in_rounds += usize::from(rounds > 1);
        if settled != code || expected != *record {
            witnessed.differing.push(difference(&settled, code, rounds));
        }
    });
}

fn in_rounds(code: String) -> (usize, String, GuardRecord) {
    let (mut code, mut changed, record) = reflow_once::<true>(code, None, false, None);
    let mut rounds = 0;
    while changed {
        rounds += 1;
        (code, changed, _) = reflow_once::<false>(code, None, false, None);
    }
    (rounds, code, record)
}

fn difference(rounds: &str, one_parse: &str, count: usize) -> String {
    let at = rounds
        .bytes()
        .zip(one_parse.bytes())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| rounds.len().min(one_parse.len()));
    let from = at.saturating_sub(40);
    let excerpt = |text: &str| {
        text.get(from..(at + 40).min(text.len()))
            .unwrap_or_default()
            .to_string()
    };
    format!(
        "after {count} rounds, at byte {at}: rounds {:?}, one parse {:?}",
        excerpt(rounds),
        excerpt(one_parse)
    )
}

fn placed_difference(expected: &Finished, finished: &Finished, lending: bool) -> Option<String> {
    let held = expected.2.filter(|_| lending);
    if held != finished.2 {
        return Some(format!(
            "the bundle check answers {:?} on the program and {held:?} on the reparsed text",
            finished.2
        ));
    }
    if expected.1 != finished.1 {
        return Some("the placed reflow records other guards than the reparsed text".to_string());
    }
    (expected.0 != finished.0)
        .then(|| difference(&expected.0, &finished.0, 0).replacen("rounds", "reparsed", 1))
}

pub(super) fn witness_placed(
    placed: bool,
    lending: bool,
    expected: &Finished,
    finished: &Finished,
) {
    let differs = placed_difference(expected, finished, lending);
    WITNESS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(witnessed) = slot.as_mut() else {
            return;
        };
        match placed {
            true => witnessed.placed += 1,
            false => witnessed.reparsed += 1,
        }
        witnessed.differing.extend(differs);
    });
}
