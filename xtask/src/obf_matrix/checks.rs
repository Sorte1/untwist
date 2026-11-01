use std::collections::BTreeMap;

use super::cell::{all_cells, Behaviour, Cell, Observation, Ran};
use super::residue::Measured;

pub const MIN_MODULES: usize = 5;

pub type Observations = BTreeMap<Cell, Observation>;

pub struct Subject<'a> {
    pub cells: &'a Observations,
    pub expected: Option<&'a Option<Ran>>,
    pub realistic: bool,
    pub bundle: bool,
}

pub type Failures = Vec<(Cell, &'static str, Option<String>)>;

fn residues(o: &Observation) -> (Option<Measured>, Option<Measured>) {
    (o.program, o.modules)
}

fn render(o: &Observation) -> String {
    let side = |m: Option<Measured>| m.map_or("-".to_string(), Measured::render);
    format!("program[{}] modules[{}]", side(o.program), side(o.modules))
}

fn compare(this: &Observation, other: &Observation, label: &str) -> Option<String> {
    (residues(this) != residues(other))
        .then(|| format!("{} vs {label} {}", render(this), render(other)))
}

fn runs(o: &Observation) -> Option<String> {
    (!o.runs()).then(|| {
        format!(
            "exit {:?}{} parses {}",
            o.exit,
            if o.timed_out { " (timed out)" } else { "" },
            o.parses()
        )
    })
}

fn rename(cells: &Observations, cell: Cell, o: &Observation) -> Option<String> {
    if cell.rename == "none" {
        return None;
    }
    let reference = cells.get(&cell.with_rename("none"))?;
    compare(o, reference, "rename none")
}

fn output(cells: &Observations, cell: Cell, o: &Observation) -> Option<String> {
    if cell.output == "stdout" {
        return None;
    }
    let stdout = cells.get(&cell.with_output("stdout"))?;
    let mut problems = Vec::new();
    if let (Some(mine), Some(theirs)) = (o.program, stdout.program) {
        if mine != theirs {
            problems.push(format!(
                "program {} vs stdout {}",
                mine.render(),
                theirs.render()
            ));
        }
    }
    if cell.output == "dir" {
        if let Some(single) = cells.get(&cell.with_output("single-file")) {
            if single.modules.is_some()
                && (o.modules != single.modules || o.module_set != single.module_set)
            {
                problems.push(format!(
                    "modules {} {:?} vs single-file {} {:?}",
                    o.modules.map_or("-".into(), Measured::render),
                    o.module_set.len(),
                    single.modules.map_or("-".into(), Measured::render),
                    single.module_set.len()
                ));
            }
        }
    }
    if o.bundle != stdout.bundle {
        problems.push(format!(
            "bundle {:?} vs stdout {:?}",
            o.bundle, stdout.bundle
        ));
    }
    (!problems.is_empty()).then(|| problems.join("; "))
}

fn unsafe_lane(cells: &Observations, cell: Cell, o: &Observation) -> Option<String> {
    if cell.lane != "unsafe" {
        return None;
    }
    let safe = cells.get(&cell.with_lane("safe"))?;
    (!o.residue().at_most(safe.residue())).then(|| {
        format!(
            "unsafe {} vs safe {}",
            o.residue().render(),
            safe.residue().render()
        )
    })
}

fn node(expected: &Option<Ran>, o: &Observation) -> Option<Option<String>> {
    match &o.behaviour {
        Behaviour::NotChecked | Behaviour::Skipped(_) => None,
        Behaviour::Ran(actual) if actual == expected => Some(None),
        Behaviour::Ran(actual) => Some(Some(format!(
            "node {} vs expected {}",
            describe(actual),
            describe(expected)
        ))),
    }
}

fn describe(ran: &Option<Ran>) -> String {
    match ran {
        None => "timed out".to_string(),
        Some(r) => format!(
            "exit {:?}, {} line(s), first {:?}",
            r.exit,
            r.stdout.lines().count(),
            r.stdout.lines().next().unwrap_or("")
        ),
    }
}

fn decoded(o: &Observation) -> Option<String> {
    (!o.residue().is_clean()).then(|| format!("residue {}", o.residue().render()))
}

fn whole_run<'a>(cells: &'a Observations, cell: Cell, o: &'a Observation) -> &'a Observation {
    if cell.output == "single-file" && o.program.is_none() {
        return cells.get(&cell.with_output("dir")).unwrap_or(o);
    }
    o
}

fn unpacked(cell: Cell, o: &Observation) -> Option<String> {
    let modules = if cell.output == "stdout" {
        MIN_MODULES
    } else {
        o.module_set.len()
    };
    (o.bundle == "none" || modules < MIN_MODULES).then(|| {
        format!(
            "bundle {:?}, {} module file(s)",
            o.bundle,
            o.module_set.len()
        )
    })
}

pub fn evaluate(subject: &Subject) -> Failures {
    let mut out = Failures::new();
    for cell in all_cells() {
        let Some(o) = subject.cells.get(&cell) else {
            continue;
        };
        out.push((cell, "runs", runs(o)));
        out.push((cell, "rename", rename(subject.cells, cell, o)));
        out.push((cell, "output", output(subject.cells, cell, o)));
        out.push((cell, "unsafe", unsafe_lane(subject.cells, cell, o)));
        if let Some(expected) = subject.expected {
            if let Some(result) = node(expected, o) {
                out.push((cell, "node", result));
            }
        }
        if subject.realistic {
            let whole = whole_run(subject.cells, cell, o);
            out.push((cell, "decoded", decoded(whole)));
            if subject.bundle {
                out.push((cell, "unpacked", unpacked(cell, o)));
            }
        }
    }
    out
}
