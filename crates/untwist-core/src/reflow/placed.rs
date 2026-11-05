use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_ast_visit::VisitJsMut;

mod lines;
#[cfg(feature = "test-hooks")]
mod mutants;
mod targets;
mod walk;

#[cfg(feature = "test-hooks")]
pub use mutants::{flip_bundle_answer_for_tests, misplace_an_edit_for_tests};

use super::guards::{splice_guards, Guard, GuardRecord};
#[cfg(debug_assertions)]
use super::rounds;
use super::{apply, reflow_recorded_in, Edit, Scan};
use crate::print::pretty_codegen;
use lines::{bracketed, site, Placed};
use targets::located;
use walk::{restore, Event, Placer};

pub type Check<'c> = &'c mut dyn FnMut(&Program<'_>) -> bool;

type Finished = (String, GuardRecord, Option<bool>);

struct Plan {
    edits: Vec<Edit>,
    guards: Vec<Guard>,
    record: GuardRecord,
}

pub struct Printed {
    code: String,
    plan: Option<Plan>,
    held: Option<bool>,
    #[cfg(debug_assertions)]
    expected: Option<Box<Finished>>,
}

#[cfg(feature = "test-hooks")]
fn misplaced(mut edits: Vec<Edit>) -> Vec<Edit> {
    if mutants::misplaced() {
        if let Some(Edit::Collapse(from, _)) = edits.first_mut() {
            *from += 1;
        }
    }
    edits
}

#[cfg(not(feature = "test-hooks"))]
fn misplaced(edits: Vec<Edit>) -> Vec<Edit> {
    edits
}

#[cfg(feature = "test-hooks")]
fn answered(held: Option<bool>) -> Option<bool> {
    held.map(|held| held ^ mutants::flipped())
}

#[cfg(not(feature = "test-hooks"))]
fn answered(held: Option<bool>) -> Option<bool> {
    held
}

impl Plan {
    fn apply(self, code: String) -> (String, GuardRecord) {
        if self.edits.is_empty() && self.guards.is_empty() {
            return (code, self.record);
        }
        let (code, edits) = if self.guards.is_empty() {
            (code, self.edits)
        } else {
            let (code, edits, _) = splice_guards(code, self.edits, self.guards);
            (code, edits)
        };
        (apply(code, misplaced(edits)).0, self.record)
    }
}

fn reparsed(code: String, check: Option<Check<'_>>) -> Finished {
    let mut held = None;
    let (code, record) = match check {
        Some(check) => {
            let mut lend = |program: &Program<'_>| held = Some(check(program));
            reflow_recorded_in(code, None, Some(&mut lend))
        }
        None => reflow_recorded_in(code, None, None),
    };
    (code, record, held)
}

fn replay(code: &str, placer: &Placer<'_>, sites: &[Placed], guards: Vec<Guard>) -> Plan {
    let mut scan = Scan::<true> {
        code,
        cascade: true,
        edits: Vec::new(),
        guards: Vec::new(),
        record: GuardRecord::default(),
        searched_until: 0,
        candidates: Vec::new(),
        broken: Vec::new(),
        removed_breaks: 0,
    };
    let mut marks = Vec::new();
    for event in &placer.events {
        match *event {
            Event::Enter => marks.push(scan.marks()),
            Event::Exit(index) => {
                if let (Some(mark), Some(Placed::Site(site))) = (marks.pop(), sites.get(index)) {
                    scan.consider(site, mark);
                }
            }
            Event::Candidate => scan.candidates.push(0),
        }
    }
    scan.guards = guards;
    scan.resolve();
    Plan {
        edits: scan.edits,
        guards: scan.guards,
        record: placer.record.clone(),
    }
}

fn plan(code: &str, placer: Placer<'_>, commented: bool) -> Option<Plan> {
    let doubt = &placer.doubt;
    if doubt.target || (doubt.unbroken && commented) {
        return None;
    }
    let bytes = code.as_bytes();
    let found = bracketed(code, placer.shapes.len())?;
    let sites = placer
        .shapes
        .iter()
        .zip(found)
        .map(|(shape, found)| site(bytes, shape, found))
        .collect::<Option<Vec<_>>>()?;
    let mut placer = placer;
    let targets = std::mem::take(&mut placer.targets);
    let guards = located(code, targets)?;
    Some(replay(code, &placer, &sites, guards))
}

fn reborrowed<'s>(check: &'s mut Option<Check<'_>>) -> Option<Check<'s>> {
    match check {
        Some(check) => Some(&mut **check),
        None => None,
    }
}

#[cfg(debug_assertions)]
fn expected(program: &Program<'_>, check: Option<Check<'_>>) -> Option<Box<Finished>> {
    rounds::is_recording().then(|| Box::new(reparsed(pretty_codegen(program), check)))
}

pub fn print_placing<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    mut check: Option<Check<'_>>,
) -> Printed {
    #[cfg(debug_assertions)]
    let expected = expected(program, reborrowed(&mut check));
    let held = reborrowed(&mut check).map(|check| check(program));
    let commented = !program.comments.is_empty();
    let mut placer = Placer::new(alloc);
    placer.visit_program(program);
    let code = pretty_codegen(program);
    let printed = Printed {
        code,
        plan: None,
        held,
        #[cfg(debug_assertions)]
        expected,
    };
    match plan(&printed.code, placer, commented) {
        Some(plan) => Printed {
            plan: Some(plan),
            ..printed
        },
        None => printed.unplaced_from(program),
    }
}

impl Printed {
    pub fn unplaced(code: String) -> Self {
        Printed {
            code,
            plan: None,
            held: None,
            #[cfg(debug_assertions)]
            expected: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.code
    }

    pub fn is_placed(&self) -> bool {
        self.plan.is_some()
    }

    fn unplaced_from(self, program: &mut Program<'_>) -> Self {
        restore(program);
        Printed {
            code: pretty_codegen(program),
            plan: None,
            held: None,
            ..self
        }
    }

    pub fn unplace(self, program: &mut Program<'_>) -> Self {
        match self.plan {
            Some(_) => self.unplaced_from(program),
            None => self,
        }
    }

    pub fn finish(self, check: Option<Check<'_>>) -> Finished {
        let lending = check.is_some();
        #[cfg(debug_assertions)]
        let placed = self.plan.is_some();
        let finished = match self.plan {
            Some(plan) => {
                let (code, record) = plan.apply(self.code);
                (code, record, answered(self.held.filter(|_| lending)))
            }
            None => reparsed(self.code, check),
        };
        #[cfg(debug_assertions)]
        if let Some(expected) = self.expected {
            rounds::witness_placed(placed, lending, &expected, &finished);
        }
        finished
    }
}
