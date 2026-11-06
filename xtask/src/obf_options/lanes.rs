use std::path::Path;

use anyhow::Result;

use super::machinery::{measure, Found, MACHINERY, SIMPLIFY};
use super::manifest::Entry;
use crate::obf_matrix::cell::{read_lossy, Cell, Ran, Runner};
use crate::obf_matrix::notes::{class_of, is_info};

pub const SAFE: &str = "safe";
pub const UNSAFE: &str = "unsafe";
pub const WEBCRACK: &str = "webcrack";
pub const UNTWIST_LANES: [&str; 2] = [SAFE, UNSAFE];
pub const CHECKS: [&str; 4] = ["runs", "machinery", "simplified", "node"];
const NOTE_CHARS: usize = 240;
const MACHINERY_LEFT: &str = "obfuscator machinery is left in the output, so it is not run";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Behaviour {
    Same,
    Differs(String),
    Skipped(String),
}

#[derive(Clone, Debug)]
pub struct LaneRun {
    pub exit: Option<i32>,
    pub found: Found,
    pub behaviour: Behaviour,
    pub notes: Vec<String>,
    pub spans: Vec<String>,
}

impl LaneRun {
    pub fn checks(&self) -> Vec<(&'static str, Option<String>)> {
        let runs = (self.exit != Some(0) || !self.found.parses)
            .then(|| format!("exit {:?} parses {}", self.exit, self.found.parses));
        let machinery =
            (!self.found.present(&MACHINERY).is_empty()).then(|| self.found.render(&MACHINERY));
        let simplified =
            (!self.found.present(&SIMPLIFY).is_empty()).then(|| self.found.render(&SIMPLIFY));
        let mut out = vec![
            ("runs", runs),
            ("machinery", machinery),
            ("simplified", simplified),
        ];
        match &self.behaviour {
            Behaviour::Same => out.push(("node", None)),
            Behaviour::Differs(detail) => out.push(("node", Some(detail.clone()))),
            Behaviour::Skipped(_) => {}
        }
        out
    }

    pub fn perfect(&self) -> bool {
        self.checks().iter().all(|(_, failure)| failure.is_none())
    }

    pub fn first_note(&self) -> Option<&str> {
        self.notes.first().map(String::as_str)
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

pub fn behaviour_of(
    runner: &Runner,
    entry: &Entry,
    found: &Found,
    exit: Option<i32>,
    output: &Path,
    expected: &str,
) -> Result<Behaviour> {
    if let Some(reason) = entry.residue_only() {
        return Ok(Behaviour::Skipped(format!("residue-only: {reason}")));
    }
    if !found.present(&MACHINERY).is_empty() {
        return Ok(Behaviour::Skipped(MACHINERY_LEFT.to_string()));
    }
    if exit != Some(0) {
        return Ok(Behaviour::Differs(format!("deobfuscator exit {exit:?}")));
    }
    let ran = runner.run_node(output)?;
    let want = Some(Ran {
        exit: Some(0),
        stdout: expected.to_string(),
    });
    Ok(if ran == want {
        Behaviour::Same
    } else {
        Behaviour::Differs(format!(
            "node {} vs original {}",
            describe(&ran),
            describe(&want)
        ))
    })
}

pub fn notes_of(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| {
            line.starts_with("error:")
                || line.contains("panicked")
                || ((line.starts_with("note: ") || line.starts_with("warning: ")) && !is_info(line))
        })
        .map(|line| line.chars().take(NOTE_CHARS).collect())
        .collect()
}

pub fn note_classes(notes: &[String]) -> Vec<String> {
    let mut classes: Vec<String> = notes.iter().map(|n| class_of(n)).collect();
    classes.dedup();
    classes
}

pub fn observe_untwist(
    runner: &Runner,
    entry: &Entry,
    lane: &'static str,
    input: &Path,
    work: &Path,
    expected: &str,
) -> Result<LaneRun> {
    let cell = Cell {
        lane,
        rename: "none",
        output: "stdout",
    };
    let observed = runner.observe(input, cell, work, false)?;
    let output = work.join("out.js");
    let found = measure(&read_lossy(&output));
    let stderr = read_lossy(&work.join("err.txt"));
    let notes = notes_of(&stderr);
    let spans = super::blockers::spans_of(&stderr);
    let behaviour = behaviour_of(runner, entry, &found, observed.exit, &output, expected)?;
    Ok(LaneRun {
        exit: observed.exit,
        found,
        behaviour,
        notes,
        spans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(behaviour: Behaviour, source: &str) -> LaneRun {
        LaneRun {
            exit: Some(0),
            found: measure(source),
            behaviour,
            notes: Vec::new(),
            spans: Vec::new(),
        }
    }

    #[test]
    fn a_clean_same_run_is_perfect_and_skipped_behaviour_is_not_checked() {
        assert!(run(Behaviour::Same, "console.log('a');").perfect());
        let skipped = run(Behaviour::Skipped("x".into()), "console.log('a');");
        assert!(skipped.perfect());
        assert_eq!(skipped.checks().len(), 3);
    }

    #[test]
    fn residue_and_behaviour_failures_are_reported_per_check() {
        let lane = run(Behaviour::Differs("d".into()), "console['log']('a' + 'b');");
        let failed: Vec<&str> = lane
            .checks()
            .into_iter()
            .filter_map(|(check, failure)| failure.map(|_| check))
            .collect();
        assert_eq!(failed, ["simplified", "node"]);
    }

    #[test]
    fn info_notes_are_dropped() {
        let notes = notes_of(
            "note: find-string-array: string array found: 3 entries\nnote: remove-decoder-machinery: decoder machinery kept: 2 reference(s)\nstage: x\n",
        );
        assert_eq!(notes.len(), 1);
        assert_eq!(
            note_classes(&notes),
            ["remove-decoder-machinery: decoder machinery kept"]
        );
    }
}
