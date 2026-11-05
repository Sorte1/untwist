use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use super::ledger::{self, Known, Refusal};
use super::spec::Spec;
use crate::obf_matrix::cell::{read_lossy, Cell, Ran, Runner};
use crate::obf_matrix::notes::class_of;
use crate::obf_options::lanes::{notes_of, Behaviour, LaneRun, UNSAFE};
use crate::obf_options::machinery::{measure, MACHINERY};
use crate::obf_options::suggest::{cause_of, Failed, Shape};

pub const WRONG_OUTPUT: &str = "wrong-output";
pub const UNSAFE_PREMISE: &str = "unsafe-premise-violated";
pub const PANIC: &str = "untwist-panic";
pub const TIMEOUT: &str = "untwist-timeout";
pub const ERROR: &str = "untwist-error";
pub const UNCLASSIFIED_SIMPLIFY: &str = "unclassified-simplify";
pub const DECODE_MISS: &str = "unexplained-decode-miss";
const CONSEQUENCES: [&str; 2] = [
    "decoder machinery kept",
    "stateful decoder is referenced outside",
];
const BLOCKER_WORDS: usize = 8;
const DECODE_PASSES: [&str; 2] = ["deobfuscate", "pipeline"];
const DECODE_WORDS: [&str; 7] = [
    "string-array",
    "decod",
    "rotator",
    "protection",
    "defending",
    "domain",
    "console",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Sound(String),
    Known(String),
    New(String),
}

impl Verdict {
    pub fn tag(&self) -> &'static str {
        match self {
            Verdict::Pass => "ok",
            Verdict::Sound(_) => "SOUND",
            Verdict::Known(_) => "KNOWN",
            Verdict::New(_) => "NEW",
        }
    }

    pub fn cause(&self) -> Option<&str> {
        match self {
            Verdict::Pass => None,
            Verdict::Sound(c) | Verdict::Known(c) | Verdict::New(c) => Some(c),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LaneResult {
    pub lane: &'static str,
    pub run: LaneRun,
    pub verdict: Verdict,
}

#[derive(Clone, Debug)]
pub struct SampleResult {
    pub id: String,
    pub features: Vec<String>,
    pub skipped: Option<String>,
    pub lanes: Vec<LaneResult>,
}

pub struct Rules<'a> {
    pub refusals: &'a [Refusal],
    pub known: &'a [Known],
}

fn behaviour(
    runner: &Runner,
    spec: &Spec,
    run: (&LaneRun, &Path),
    expected: &Ran,
) -> Result<Behaviour> {
    let (lane, output) = run;
    if let Some(reason) = spec.residue_only() {
        return Ok(Behaviour::Skipped(format!("residue-only: {reason}")));
    }
    if !lane.found.present(&MACHINERY).is_empty() {
        return Ok(Behaviour::Skipped("machinery left".to_string()));
    }
    if lane.exit != Some(0) {
        return Ok(Behaviour::Differs(format!(
            "deobfuscator exit {:?}",
            lane.exit
        )));
    }
    let runnable = if spec.is_module() {
        let copy = output.with_extension("mjs");
        std::fs::copy(output, &copy)?;
        copy
    } else {
        output.to_path_buf()
    };
    let ran = runner.run_node(&runnable)?;
    Ok(match ran {
        Some(r) if &r == expected => Behaviour::Same,
        Some(r) => Behaviour::Differs(format!(
            "node exit {:?} first {:?} vs original exit {:?} first {:?}",
            r.exit,
            first_difference(&r.stdout, &expected.stdout),
            expected.exit,
            first_difference(&expected.stdout, &r.stdout)
        )),
        None => Behaviour::Differs("node timed out on the output".to_string()),
    })
}

fn first_difference<'a>(text: &'a str, other: &str) -> &'a str {
    let mut theirs = other.lines();
    text.lines()
        .find(|line| theirs.next() != Some(*line))
        .unwrap_or("")
}

pub fn cause(lane: &str, run: &LaneRun, spec_shape: (Shape, bool)) -> Option<String> {
    let (shape, hazard) = spec_shape;
    let failing: Vec<(&'static str, String)> = run
        .checks()
        .into_iter()
        .filter_map(|(check, failure)| failure.map(|f| (check, f)))
        .collect();
    if failing.is_empty() {
        return None;
    }
    let failed = |check: &str| failing.iter().any(|(c, _)| *c == check);
    if failed("runs") {
        let key = if run.notes.iter().any(|n| n.contains("panicked")) {
            PANIC
        } else if run.exit.is_none() {
            TIMEOUT
        } else {
            ERROR
        };
        return Some(key.to_string());
    }
    if failed("node") {
        let key = if lane == UNSAFE && hazard {
            UNSAFE_PREMISE
        } else {
            WRONG_OUTPUT
        };
        return Some(key.to_string());
    }
    let note = run
        .notes
        .iter()
        .find(|n| is_decode_note(&class_of(n)))
        .map(String::as_str)
        .or(run.first_note())
        .unwrap_or("");
    let checks: BTreeMap<&str, Failed<'_>> = failing
        .iter()
        .map(|(check, detail)| {
            (
                *check,
                Failed {
                    detail: detail.as_str(),
                    note,
                },
            )
        })
        .collect();
    let fallback = if failed("machinery") {
        DECODE_MISS
    } else {
        UNCLASSIFIED_SIMPLIFY
    };
    let known = cause_of(shape, lane, &checks, &run.notes)
        .filter(|key| *key != DECODE_MISS)
        .map(str::to_string);
    Some(
        known
            .or_else(|| blocker_key(&run.notes, failed("machinery")))
            .unwrap_or_else(|| fallback.to_string()),
    )
}

fn is_decode_note(class: &str) -> bool {
    let pass = class.split_once(": ").map_or("", |(pass, _)| pass);
    DECODE_PASSES.contains(&pass) || DECODE_WORDS.iter().any(|w| pass.contains(w))
}

pub fn blocker_key(notes: &[String], decode_only: bool) -> Option<String> {
    let classes: Vec<String> = notes
        .iter()
        .filter(|n| !CONSEQUENCES.iter().any(|c| n.contains(c)))
        .map(|n| class_of(n))
        .collect();
    let class = classes
        .iter()
        .find(|c| is_decode_note(c))
        .or(if decode_only { None } else { classes.first() })?;
    let (pass, message) = class.split_once(": ").unwrap_or(("cli", class.as_str()));
    let words: Vec<String> = message
        .split_whitespace()
        .filter(|w| *w != "_" && *w != "N")
        .take(BLOCKER_WORDS)
        .map(|w| {
            w.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    Some(format!("{pass}/{}", words.join("-")))
}

pub fn verdict(
    rules: &Rules<'_>,
    lane: &str,
    cause: Option<String>,
    has_feature: impl Fn(&str) -> bool,
) -> Verdict {
    let Some(cause) = cause else {
        return Verdict::Pass;
    };
    let refusal = cause != WRONG_OUTPUT && cause != UNSAFE_PREMISE;
    if lane != UNSAFE && refusal && ledger::sound(rules.refusals, &cause, has_feature).is_some() {
        return Verdict::Sound(cause);
    }
    if ledger::known(rules.known, lane, &cause).is_some() {
        return Verdict::Known(cause);
    }
    Verdict::New(cause)
}

pub fn shape_of(spec: &Spec) -> (Shape, bool) {
    (
        Shape {
            domain_lock: spec.domain_lock(),
            key_shaped: spec.key_shaped(),
        },
        spec.has_hazard(),
    )
}

pub fn expected_of(runner: &Runner, spec: &Spec) -> Result<Result<Ran, String>> {
    let ran = runner.run_node(&spec.entry())?;
    Ok(match ran {
        Some(r) if r.exit == Some(0) => Ok(r),
        Some(r) => Err(format!("the original exits {:?}", r.exit)),
        None => Err("the original timed out".to_string()),
    })
}

pub fn run_lane(
    runner: &Runner,
    spec: &Spec,
    lane: &'static str,
    expected: &Ran,
    rules: &Rules<'_>,
) -> Result<LaneResult> {
    let work = spec.dir.join(lane);
    let cell = Cell {
        lane,
        rename: "none",
        output: "stdout",
    };
    let observed = runner.observe(&spec.sample(), cell, &work, false)?;
    let output = work.join("out.js");
    let mut run = LaneRun {
        exit: observed.exit,
        found: measure(&read_lossy(&output)),
        behaviour: Behaviour::Skipped(String::new()),
        notes: notes_of(&read_lossy(&work.join("err.txt"))),
    };
    run.behaviour = behaviour(runner, spec, (&run, &output), expected)?;
    let cause = cause(lane, &run, shape_of(spec));
    let verdict = verdict(rules, lane, cause, |f| spec.has_feature(f));
    Ok(LaneResult { lane, run, verdict })
}

pub fn judge_sample(
    runner: &Runner,
    spec: &Spec,
    lanes: &[&'static str],
    rules: &Rules<'_>,
) -> Result<SampleResult> {
    let mut result = SampleResult {
        id: spec.id().to_string(),
        features: spec.features().iter().map(|f| f.to_string()).collect(),
        skipped: None,
        lanes: Vec::new(),
    };
    if let Err(reason) = spec.built() {
        result.skipped = Some(reason);
        return Ok(result);
    }
    let expected = match expected_of(runner, spec)? {
        Ok(ran) => ran,
        Err(reason) => {
            result.skipped = Some(reason);
            return Ok(result);
        }
    };
    for lane in lanes {
        result
            .lanes
            .push(run_lane(runner, spec, lane, &expected, rules)?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obf_options::lanes::SAFE;

    fn lane(exit: Option<i32>, source: &str, behaviour: Behaviour, notes: &[&str]) -> LaneRun {
        LaneRun {
            exit,
            found: measure(source),
            behaviour,
            notes: notes.iter().map(|n| n.to_string()).collect(),
        }
    }

    const NAMES: &str = "note: remove-decoder-machinery: code may observe the names of the global object, where this unit's string array function or decoders are properties";

    #[test]
    fn a_clean_run_has_no_cause_and_passes() {
        let run = lane(Some(0), "console.log(1);", Behaviour::Same, &[]);
        assert_eq!(cause(SAFE, &run, (Shape::default(), false)), None);
        let rules = Rules {
            refusals: &[],
            known: &[],
        };
        assert_eq!(verdict(&rules, SAFE, None, |_| false), Verdict::Pass);
    }

    #[test]
    fn wrong_output_outranks_residue_and_an_unsafe_hazard_is_a_premise() {
        let run = lane(
            Some(0),
            "console.log(1);",
            Behaviour::Differs("d".into()),
            &[],
        );
        assert_eq!(
            cause(SAFE, &run, (Shape::default(), true)).as_deref(),
            Some(WRONG_OUTPUT)
        );
        assert_eq!(
            cause(UNSAFE, &run, (Shape::default(), true)).as_deref(),
            Some(UNSAFE_PREMISE)
        );
        assert_eq!(
            cause(UNSAFE, &run, (Shape::default(), false)).as_deref(),
            Some(WRONG_OUTPUT)
        );
        let crashed = lane(None, "", Behaviour::Differs("x".into()), &[]);
        assert_eq!(
            cause(SAFE, &crashed, (Shape::default(), false)).as_deref(),
            Some(TIMEOUT)
        );
    }

    #[test]
    fn a_refusal_is_sound_only_on_the_safe_lane_with_the_ledger_feature() {
        let refusals = vec![Refusal {
            cause: "safe-global-names".into(),
            feature: "observes-global-names".into(),
            rows: "r".into(),
            argument: "a".into(),
        }];
        let known = vec![Known {
            lane: "safe".into(),
            cause: "safe-global-names".into(),
            reason: "open".into(),
        }];
        let rules = Rules {
            refusals: &refusals,
            known: &known,
        };
        let key = || Some("safe-global-names".to_string());
        let with = |f: &str| f == "observes-global-names";
        assert!(matches!(
            verdict(&rules, SAFE, key(), with),
            Verdict::Sound(_)
        ));
        assert!(matches!(
            verdict(&rules, SAFE, key(), |_| false),
            Verdict::Known(_)
        ));
        assert!(matches!(
            verdict(&rules, UNSAFE, key(), with),
            Verdict::New(_)
        ));
    }

    #[test]
    fn an_unknown_blocker_is_keyed_by_its_note_and_consequences_are_skipped() {
        let notes = vec![
            "note: remove-decoder-machinery: decoder machinery kept: 4 reference(s)".to_string(),
            "note: inline-decoded-strings: a decoder of the unit is used other than as the callee of a call, so".to_string(),
        ];
        assert_eq!(
            blocker_key(&notes, true).as_deref(),
            Some("inline-decoded-strings/a-decoder-of-the-unit-is-used-other")
        );
        assert_eq!(blocker_key(&notes[..1], false), None);
        let other =
            vec!["note: inline-object-props: the object literal defines a twice".to_string()];
        assert_eq!(blocker_key(&other, true), None);
        assert_eq!(
            blocker_key(&other, false).as_deref(),
            Some("inline-object-props/the-object-literal-defines-twice")
        );
    }

    #[test]
    fn a_names_note_on_left_machinery_is_classified() {
        let run = lane(
            Some(0),
            "function _0xd3f4(){var a=['log','hi'];_0xd3f4=function(){return a;};return _0xd3f4();}function _0xcdc4(b,c){b=b-0xaa;var d=_0xd3f4();return d[b];}console[_0xcdc4(0xaa)](_0xcdc4(0xab));",
            Behaviour::Skipped("machinery left".into()),
            &[NAMES],
        );
        assert_eq!(
            cause(SAFE, &run, (Shape::default(), false)).as_deref(),
            Some("safe-global-names")
        );
    }
}
