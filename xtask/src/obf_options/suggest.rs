use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};

use super::manifest::Entry;
use super::refusals;
use super::report::cause_key;
use super::{Judged, Outcome};
use crate::obf_matrix::expect::{glob, Mark, Rule, Verdict};

const CHECKS: [&str; 4] = ["runs", "machinery", "simplified", "node"];
const LANES: [&str; 2] = ["safe", "unsafe"];
const UNKNOWN_REASON: &str = "TODO: name the cause";

const SPELLED: &str = "spelled-decoder-name";
const BUILTIN_REPLACED: &str = "builtin-replaced-decoded-key";
const BUILTIN_ALIAS: &str = "builtin-alias-patched";
const KEY_REACH: &str = "safe-global-key-reach";
const NAMES: &str = "safe-global-names";
const STEP_BUDGET: &str = "safe-decode-step-budget";
const DOMAIN_LOCK: &str = "safe-domain-lock-reach";
const DEBUG_KEPT: &str = "safe-debug-protection-kept";
const DEAD_VARS: &str = "safe-dead-code-vars";
const OBJECT_KEYS: &str = "object-keys-computed-key";
const DECODE_MISS: &str = "unexplained-decode-miss";
const STATEFUL_ESCAPE: &str = "stateful-decoder-escape";
const ESCAPE_NOTE: &str = "a stateful decoder is referenced outside the calls untwist proves";
const NO_ARRAY: &str = "missed-no-string-array";

const REACH_NOTE: &str = "may reach the string array's function by a key";
const NAMES_NOTE: &str = "may observe the names of the global object";

const DEFAULT_REASONS: [(&str, &str); 13] = [
    (
        NO_ARRAY,
        "MISSED: find-string-array finds no string array although the input has one, so every decoder call is left and no pass says why",
    ),
    (
        BUILTIN_REPLACED,
        "a call or write on a built-in whose key is itself a decoder call is judged as a possible built-in patch, so a stateful base64 or rc4 unit is refused",
    ),
    (
        DECODE_MISS,
        "MISSED: the unit keeps every reference undecoded and no pass says why",
    ),
    (
        STATEFUL_ESCAPE,
        "a stateful base64 or rc4 decoder is also reached through calls whose arguments are not proven (wrappers reading locals of an enclosing function, or the decoder handed on as a value), so decoding the collected calls ahead of those could change its state, and the unit's calls are left alone",
    ),
    (
        STEP_BUDGET,
        "safe lane: decoding the unit runs out of the evaluator's step budget",
    ),
    (
        "memo-reset-gate",
        "javascript-obfuscator 5 base64 and rc4 decoders carry a memo reset gate (array[0] kept under a memo key) that the stateful-decoder proof does not accept, so decoding bails and the string array, decoders and rotator stay",
    ),
    (
        KEY_REACH,
        "safe lane, premise 2: the program may hold the global object and reaches keys the check cannot resolve, so the unit's call sites are left alone",
    ),
    (
        NAMES,
        "safe lane, premise 2: code may observe the global object's names (a non-strict this, require), so the decoded unit's string array, decoders and rotator are kept as written",
    ),
    (
        DOMAIN_LOCK,
        "safe lane: the domain lock's own for-in scan of the global object is a global reach, so the unit is refused before the lock is removed",
    ),
    (
        DEBUG_KEPT,
        "safe lane: a reference to the debug protection function survives outside the statements the pass removes, so it stays and its Function global getter then blocks the unit",
    ),
    (
        DEAD_VARS,
        "safe lane: folding a dead-code-injection branch keeps the var it hoisted, an unused declaration the unsafe lane drops",
    ),
    (
        OBJECT_KEYS,
        "transformObjectKeys' var o = {}; o[k] = v; x = o sequence is not merged back into one literal when a key is computed (o[tag] = ...)",
    ),
    (
        SPELLED,
        "the string array function or decoder has a name that a string or property name in the program also spells, so find-decoders leaves the unit undecoded",
    ),
];

type Key = (String, &'static str, &'static str);

pub struct Failed<'j> {
    pub detail: &'j str,
    pub note: &'j str,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Shape {
    pub domain_lock: bool,
    pub key_shaped: bool,
    pub version_matrix: bool,
}

impl Shape {
    fn of(entry: &Entry) -> Shape {
        Shape {
            domain_lock: entry.cell.starts_with("domain-lock"),
            key_shaped: entry.source.ends_with("objects.js") || entry.source.ends_with("mixed.js"),
            version_matrix: entry.dir != super::manifest::DIR,
        }
    }
}

pub struct Suggestion {
    pub text: String,
    pub rules: usize,
    pub unclassified: Vec<String>,
    pub uncovered: usize,
}

fn by_note(note: &str) -> Option<&'static str> {
    if note.contains(super::lanes::NO_ARRAY) {
        return Some(NO_ARRAY);
    }
    if note.contains("spells this function's name") {
        return Some(SPELLED);
    }
    if note.contains("replaces a standard built-in") {
        return Some(BUILTIN_REPLACED);
    }
    refusals::refusal_of(note).filter(|cause| *cause == BUILTIN_ALIAS)
}

fn only_class(detail: &str, class: &str) -> bool {
    detail
        .strip_prefix(class)
        .and_then(|rest| rest.strip_prefix('='))
        .is_some_and(|count| !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()))
}

fn later_cause(lane: &str, notes: &[String]) -> &'static str {
    let safe = lane == "safe";
    for note in notes {
        if let Some(cause) = by_note(note).or_else(|| refusals::refusal_of(note)) {
            return cause;
        }
        if safe && note.contains("step budget exhausted") {
            return STEP_BUDGET;
        }
        if safe && note.contains(REACH_NOTE) {
            return KEY_REACH;
        }
        if safe && note.contains(NAMES_NOTE) {
            return NAMES;
        }
    }
    if notes.iter().any(|note| note.contains(ESCAPE_NOTE)) {
        return STATEFUL_ESCAPE;
    }
    DECODE_MISS
}

fn machinery_cause(
    shape: Shape,
    lane: &str,
    failed: &Failed<'_>,
    notes: &[String],
) -> Option<&'static str> {
    let (detail, note) = (failed.detail, failed.note);
    let safe = lane == "safe";
    if by_note(note) == Some(SPELLED) {
        return Some(SPELLED);
    }
    let aliased = notes
        .iter()
        .any(|note| by_note(note) == Some(BUILTIN_ALIAS));
    let reach = notes.iter().any(|note| note.contains(REACH_NOTE));
    if safe && shape.domain_lock && reach && !aliased {
        return Some(DOMAIN_LOCK);
    }
    if safe && shape.key_shaped && note.contains(REACH_NOTE) {
        return Some(KEY_REACH);
    }
    if let Some(cause) = refused_without_array(note, notes) {
        return Some(cause);
    }
    if let Some(cause) = by_note(note) {
        return Some(cause);
    }
    if note.contains(ESCAPE_NOTE) || note.contains("decoder machinery kept") {
        return Some(later_cause(lane, notes));
    }
    if safe && detail.contains("debug_protection") && !note.contains(NAMES_NOTE) {
        return Some(DEBUG_KEPT);
    }
    if safe && note.contains(REACH_NOTE) {
        return Some(KEY_REACH);
    }
    if safe && note.contains(NAMES_NOTE) {
        return Some(NAMES);
    }
    if safe && only_class(detail, "dead_declarations") {
        return Some(DEAD_VARS);
    }
    if !shape.version_matrix {
        return None;
    }
    last_resort(lane, detail, note, notes)
}

fn refused_without_array(note: &str, notes: &[String]) -> Option<&'static str> {
    if !note.contains(super::lanes::NO_ARRAY) {
        return None;
    }
    notes.iter().find_map(|later| refusals::refusal_of(later))
}

fn last_resort(lane: &str, detail: &str, note: &str, notes: &[String]) -> Option<&'static str> {
    if note.contains(refusals::INTERPRETED) {
        return Some(later_cause(lane, notes));
    }
    if note.is_empty() || note.contains(refusals::GUARD_ON_ONE_LINE) {
        return refusals::residue_of(detail);
    }
    refusals::refusal_of(note).or_else(|| {
        let later = later_cause(lane, notes);
        (later != DECODE_MISS).then_some(later)
    })
}

pub fn cause_of(
    shape: Shape,
    lane: &str,
    checks: &BTreeMap<&str, Failed<'_>>,
    notes: &[String],
) -> Option<&'static str> {
    if let Some(failed) = checks.get("machinery") {
        return machinery_cause(shape, lane, failed, notes);
    }
    let only_simplified = checks.len() == 1
        && checks
            .get("simplified")
            .is_some_and(|failed| failed.detail.starts_with("object_key_assignments"));
    only_simplified.then_some(OBJECT_KEYS)
}

fn failed(verdict: &Verdict) -> Option<bool> {
    match verdict {
        Verdict::Allowed(_) => None,
        Verdict::Fail(_) | Verdict::ExpectedFail(_) => Some(true),
        Verdict::Pass | Verdict::UnexpectedPass(_) => Some(false),
    }
}

struct Truth {
    causes: BTreeMap<Key, Option<&'static str>>,
    unclassified: Vec<String>,
}

fn failing_of<'j>(all: &[&'j Judged]) -> BTreeMap<&'static str, Failed<'j>> {
    all.iter()
        .filter(|j| failed(&j.verdict) == Some(true))
        .map(|j| {
            let failed = Failed {
                detail: j.detail.as_deref().unwrap_or(""),
                note: j.note.as_deref().unwrap_or(""),
            };
            (j.check, failed)
        })
        .collect()
}

fn lane_cause(
    outcome: &Outcome,
    lane: &str,
    all: &[&Judged],
) -> Result<Option<&'static str>, String> {
    let failing = failing_of(all);
    if failing.is_empty() {
        return Ok(None);
    }
    let notes = outcome
        .lanes
        .get(lane)
        .map_or(&[][..], |run| run.notes.as_slice());
    match cause_of(Shape::of(&outcome.entry), lane, &failing, notes) {
        Some(cause) => Ok(Some(cause)),
        None => {
            let names: Vec<&str> = failing.keys().copied().collect();
            Err(format!("{} {lane} {}", outcome.entry.file, names.join(",")))
        }
    }
}

fn truth(judged: &[Judged], outcomes: &[Outcome]) -> Truth {
    let entries: BTreeMap<&str, &Outcome> = outcomes
        .iter()
        .map(|outcome| (outcome.entry.file.as_str(), outcome))
        .collect();
    let mut checked: BTreeMap<(&str, &'static str), Vec<&Judged>> = BTreeMap::new();
    for j in judged.iter().filter(|j| LANES.contains(&j.lane)) {
        checked
            .entry((j.file.as_str(), j.lane))
            .or_default()
            .push(j);
    }
    let mut truth = Truth {
        causes: BTreeMap::new(),
        unclassified: Vec::new(),
    };
    for ((file, lane), all) in checked {
        let Some(outcome) = entries.get(file) else {
            continue;
        };
        let cause = lane_cause(outcome, lane, &all).unwrap_or_else(|unclassified| {
            truth.unclassified.push(unclassified);
            None
        });
        for j in all {
            if let Some(failed) = failed(&j.verdict) {
                let value = if failed { cause } else { None };
                truth
                    .causes
                    .insert((file.to_string(), lane, j.check), value);
            }
        }
    }
    truth
}

struct Builder<'t> {
    truth: &'t BTreeMap<Key, Option<&'static str>>,
    covered: BTreeSet<Key>,
    reasons: &'t BTreeMap<String, String>,
    lines: Vec<String>,
}

impl Builder<'_> {
    fn try_rule(&mut self, fixture_glob: &str, lane_glob: &str, check: &str) -> bool {
        let caught: Vec<&Key> = self
            .truth
            .keys()
            .filter(|key| {
                !self.covered.contains(*key)
                    && key.2 == check
                    && glob(fixture_glob, &key.0)
                    && glob(lane_glob, key.1)
            })
            .collect();
        let causes: BTreeSet<Option<&str>> = caught.iter().map(|key| self.truth[*key]).collect();
        let [Some(cause)] = causes.into_iter().collect::<Vec<_>>()[..] else {
            return false;
        };
        let lanes: BTreeSet<&str> = caught.iter().map(|key| key.1).collect();
        let lane = match lanes.iter().next() {
            Some(only) if !fixture_glob.contains('*') && lanes.len() == 1 => *only,
            _ => lane_glob,
        };
        let reason = self
            .reasons
            .get(cause)
            .map_or(UNKNOWN_REASON, String::as_str);
        self.lines.push(format!(
            "{fixture_glob}\t{lane}\t{check}\txfail\t{cause}: {reason}"
        ));
        let caught: Vec<Key> = caught.into_iter().cloned().collect();
        self.covered.extend(caught);
        true
    }

    fn try_group(&mut self, fixture_glob: &str, check: &str) {
        if !self.try_rule(fixture_glob, "*", check) {
            for lane in LANES {
                self.try_rule(fixture_glob, lane, check);
            }
        }
    }
}

fn reasons_of(rules: &[Rule]) -> BTreeMap<String, String> {
    let mut reasons: BTreeMap<String, String> = DEFAULT_REASONS
        .iter()
        .copied()
        .chain(refusals::reasons())
        .map(|(key, reason)| (key.to_string(), reason.to_string()))
        .collect();
    let mut kept = BTreeSet::new();
    for rule in rules.iter().filter(|rule| rule.mark == Mark::ExpectedFail) {
        let key = cause_key(&rule.reason);
        let Some((_, reason)) = rule.reason.split_once(": ") else {
            continue;
        };
        if kept.insert(key.to_string()) {
            reasons.insert(key.to_string(), reason.to_string());
        }
    }
    reasons
}

impl Builder<'_> {
    fn group(&mut self, outcomes: &[Outcome]) {
        let cells: BTreeSet<&str> = outcomes.iter().map(|o| o.entry.cell.as_str()).collect();
        let programs: BTreeSet<&str> = outcomes
            .iter()
            .filter_map(|o| o.entry.file.split_once("--").map(|(_, program)| program))
            .collect();
        let globs = cells
            .into_iter()
            .map(|cell| format!("{cell}--*"))
            .chain(programs.into_iter().map(|program| format!("*--{program}")));
        for glob in globs {
            for check in CHECKS {
                self.try_group(&glob, check);
            }
        }
    }

    fn cover_fixtures(&mut self) -> usize {
        let failing: Vec<Key> = self
            .truth
            .iter()
            .filter(|(_, cause)| cause.is_some())
            .map(|(key, _)| key.clone())
            .collect();
        for key in &failing {
            if !self.covered.contains(key) && !self.try_rule(&key.0, "*", key.2) {
                self.try_rule(&key.0, key.1, key.2);
            }
        }
        failing
            .iter()
            .filter(|key| !self.covered.contains(*key))
            .count()
    }
}

fn assembled(current: &str, rules: &[String]) -> String {
    let kept = current
        .lines()
        .filter(|line| line.starts_with('#') || is_allow_line(line));
    let lines: Vec<&str> = kept.chain(rules.iter().map(String::as_str)).collect();
    lines.join("\n") + "\n"
}

pub fn suggest(
    current: &str,
    rules: &[Rule],
    judged: &[Judged],
    outcomes: &[Outcome],
) -> Suggestion {
    let Truth {
        causes,
        unclassified,
    } = truth(judged, outcomes);
    let reasons = reasons_of(rules);
    let mut builder = Builder {
        truth: &causes,
        covered: BTreeSet::new(),
        reasons: &reasons,
        lines: Vec::new(),
    };
    builder.group(outcomes);
    let uncovered = builder.cover_fixtures();
    Suggestion {
        text: assembled(current, &builder.lines),
        rules: builder.lines.len(),
        unclassified,
        uncovered,
    }
}

fn is_allow_line(line: &str) -> bool {
    line.split('\t').nth(3) == Some("allow")
}

pub fn write(
    path: &Path,
    current_path: &Path,
    rules: &[Rule],
    judged: &[Judged],
    outcomes: &[Outcome],
) -> Result<()> {
    let current = std::fs::read_to_string(current_path).unwrap_or_default();
    let suggestion = suggest(&current, rules, judged, outcomes);
    std::fs::write(path, &suggestion.text).with_context(|| path.display().to_string())?;
    println!(
        "suggested EXPECT: {} rules, {} unclassified, {} uncovered, written to {}",
        suggestion.rules,
        suggestion.unclassified.len(),
        suggestion.uncovered,
        path.display()
    );
    for line in suggestion.unclassified.iter().take(20) {
        println!("suggested EXPECT: unclassified {line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obf_matrix::expect::parse;
    use crate::obf_options::lanes::{Behaviour, LaneRun};
    use crate::obf_options::machinery::measure;

    const NAMES_FAILURE: &str = "note: remove-decoder-machinery: code may observe the names of the global object, where this unit's string array function or decoders are properties";

    fn entry(file: &str) -> Entry {
        Entry {
            dir: crate::obf_options::manifest::DIR.to_string(),
            file: file.to_string(),
            cell: file
                .split_once("--")
                .map_or(file, |(cell, _)| cell)
                .to_string(),
            source: "tools/obfuscated/sources/strings.js".to_string(),
            seed: 1,
            obfuscator: "javascript-obfuscator@5.9.1".to_string(),
            behaviour: "node".to_string(),
            options: "{}".to_string(),
            sha256: String::new(),
        }
    }

    fn outcome(file: &str, notes: &[&str]) -> Outcome {
        let run = || LaneRun {
            exit: Some(0),
            found: measure(""),
            behaviour: Behaviour::Same,
            notes: notes.iter().map(|note| note.to_string()).collect(),
            spans: Vec::new(),
        };
        Outcome {
            entry: entry(file),
            lanes: LANES.iter().map(|lane| (*lane, run())).collect(),
        }
    }

    fn judged(file: &str, lane: &'static str, note: Option<&str>) -> Judged {
        Judged {
            file: file.to_string(),
            cell: entry(file).cell,
            lane,
            check: "machinery",
            verdict: match note {
                Some(_) => Verdict::Fail("decoder_calls=3".to_string()),
                None => Verdict::Pass,
            },
            detail: note.map(|_| "decoder_calls=3".to_string()),
            note: note.map(str::to_string),
        }
    }

    fn safe_lane_fails(files: &[&str], note: &str) -> (Vec<Judged>, Vec<Outcome>) {
        let judged = files
            .iter()
            .flat_map(|file| {
                [
                    judged(file, "safe", Some(note)),
                    judged(file, "unsafe", None),
                ]
            })
            .collect();
        let outcomes = files.iter().map(|file| outcome(file, &[note])).collect();
        (judged, outcomes)
    }

    #[test]
    fn a_cell_failing_one_check_for_one_cause_gets_one_glob_rule() {
        let (judged, outcomes) = safe_lane_fails(&["c--a-1.js", "c--b-1.js"], NAMES_FAILURE);
        let suggestion = suggest("# header\n", &[], &judged, &outcomes);
        let rule = format!(
            "c--*\tsafe\tmachinery\txfail\t{NAMES}: {}",
            DEFAULT_REASONS
                .iter()
                .find(|(key, _)| *key == NAMES)
                .map(|(_, r)| *r)
                .unwrap_or("")
        );
        assert_eq!(suggestion.text, format!("# header\n{rule}\n"));
        assert_eq!((suggestion.rules, suggestion.uncovered), (1, 0));
    }

    #[test]
    fn a_domain_lock_cell_refused_as_a_built_in_patch_is_not_labelled_a_lock_reach() {
        let note = "note: inline-decoded-strings: the program may replace a standard built-in that the unit's decoders or rotator use through a variable, parameter, `this` or computed object that can hold it";
        let (judged, outcomes) = safe_lane_fails(&["domain-lock--a-1.js"], note);
        let suggestion = suggest("", &[], &judged, &outcomes);
        assert!(
            suggestion.text.contains(&format!("\t{BUILTIN_ALIAS}: ")),
            "{}",
            suggestion.text
        );
        assert!(
            !suggestion.text.contains(DOMAIN_LOCK),
            "{}",
            suggestion.text
        );
    }

    #[test]
    fn a_domain_lock_cell_without_a_key_reach_is_not_a_lock_reach() {
        let (judged, outcomes) = safe_lane_fails(&["domain-lock--a-1.js"], "note: something else");
        let suggestion = suggest("", &[], &judged, &outcomes);
        assert_eq!(
            suggestion.unclassified,
            vec!["domain-lock--a-1.js safe machinery".to_string()]
        );
        assert!(
            !suggestion.text.contains(DOMAIN_LOCK),
            "{}",
            suggestion.text
        );
    }

    #[test]
    fn a_domain_lock_cell_with_a_key_reach_is_a_lock_reach() {
        let note = format!("note: inline-decoded-strings: code that may hold the global object {REACH_NOTE} that may be its name");
        let (judged, outcomes) = safe_lane_fails(&["domain-lock--a-1.js"], &note);
        let suggestion = suggest("", &[], &judged, &outcomes);
        assert!(
            suggestion.text.contains(&format!("\t{DOMAIN_LOCK}: ")),
            "{}",
            suggestion.text
        );
    }

    #[test]
    fn debug_strings_left_in_a_kept_string_array_count_as_the_global_names_refusal() {
        let (mut judged, outcomes) = safe_lane_fails(&["debug--a-1.js"], NAMES_FAILURE);
        for failed in judged.iter_mut().filter(|j| j.lane == "safe") {
            failed.verdict = Verdict::Fail("debug_protection=2".to_string());
            failed.detail = Some("debug_protection=2".to_string());
        }
        let suggestion = suggest("", &[], &judged, &outcomes);
        assert!(
            suggestion.text.contains(&format!("\t{NAMES}: ")),
            "{}",
            suggestion.text
        );
        assert!(!suggestion.text.contains(DEBUG_KEPT), "{}", suggestion.text);
    }

    #[test]
    fn an_existing_reason_is_kept_for_its_cause_key() {
        let current = format!(
            "# header\nold--*\tsafe\tmachinery\txfail\t{NAMES}: the reason as worded now\n"
        );
        let rules = parse(&current).unwrap_or_default();
        let (judged, outcomes) = safe_lane_fails(&["c--a-1.js"], NAMES_FAILURE);
        let suggestion = suggest(&current, &rules, &judged, &outcomes);
        assert!(
            suggestion
                .text
                .contains(&format!("{NAMES}: the reason as worded now")),
            "{}",
            suggestion.text
        );
    }

    #[test]
    fn a_guard_printed_on_one_line_leaves_the_self_defending_residue_as_the_cause() {
        let note = format!("note: pipeline: {} 1", untwist::GUARD_ON_ONE_LINE);
        assert_eq!(
            last_resort("safe", "call_controllers=4", &note, &[]),
            Some("missed-self-defending")
        );
    }

    #[test]
    fn an_unclassified_failure_is_reported_and_left_without_a_rule() {
        let (judged, outcomes) = safe_lane_fails(&["c--a-1.js"], "note: something no cause names");
        let suggestion = suggest("# header\n", &[], &judged, &outcomes);
        assert_eq!(
            suggestion.unclassified,
            vec!["c--a-1.js safe machinery".to_string()]
        );
        assert_eq!(
            (suggestion.rules, suggestion.text.as_str()),
            (0, "# header\n")
        );
    }

    #[test]
    fn a_single_fixture_rule_names_the_one_lane_that_fails() {
        let files = ["c--a-1.js", "c--b-1.js", "d--a-1.js"];
        let (mut judged, outcomes) = safe_lane_fails(&files, NAMES_FAILURE);
        judged.retain(|j| j.file == "c--a-1.js" || j.lane == "unsafe");
        judged.push(super::tests::judged("c--b-1.js", "safe", None));
        judged.push(super::tests::judged("d--a-1.js", "safe", None));
        let suggestion = suggest("", &[], &judged, &outcomes);
        assert!(
            suggestion
                .text
                .contains("c--a-1.js\tsafe\tmachinery\txfail\t"),
            "{}",
            suggestion.text
        );
    }

    #[test]
    fn header_and_allow_lines_are_kept() {
        let current =
            "# one\n# two\nx--*\t*\tnode\tallow\twhy\nold\tsafe\truns\txfail\tgone: no longer\n";
        let suggestion = suggest(current, &[], &[], &[]);
        assert_eq!(suggestion.text, "# one\n# two\nx--*\t*\tnode\tallow\twhy\n");
    }

    #[test]
    fn version_matrix_misses_and_refusals_get_their_own_causes() {
        let cause = |detail: &str, note: &str| {
            let checks = BTreeMap::from([("machinery", Failed { detail, note })]);
            let shape = Shape {
                version_matrix: true,
                ..Shape::default()
            };
            cause_of(shape, "unsafe", &checks, &[note.to_string()])
        };
        let missed = format!("note: {}", crate::obf_options::lanes::NO_ARRAY);
        assert_eq!(cause("string_arrays=1", &missed), Some(NO_ARRAY));
        assert_eq!(
            cause(
                "string_arrays=1",
                "note: inline-decoded-strings: a decoder of the unit reads a binding outside itself other than a standard global"
            ),
            Some("decoder-reads-outer-binding")
        );
        assert_eq!(
            cause("opaque_predicates=1", ""),
            Some("missed-opaque-predicate")
        );
        assert_eq!(cause("opaque_predicates=1,decoders=1", ""), None);
        let checks = BTreeMap::from([(
            "machinery",
            Failed {
                detail: "opaque_predicates=1",
                note: "",
            },
        )]);
        assert_eq!(cause_of(Shape::default(), "unsafe", &checks, &[]), None);
    }

    #[test]
    fn a_named_refusal_behind_no_string_array_is_a_refusal_in_both_matrices() {
        let missed = format!("note: deobfuscate: {}", crate::obf_options::lanes::NO_ARRAY);
        let notes = [
            missed.clone(),
            "note: find-string-array: this plain string array may be reached by name through the global object, a with statement or a direct eval".to_string(),
        ];
        let checks = BTreeMap::from([(
            "machinery",
            Failed {
                detail: "string_arrays=1",
                note: &missed,
            },
        )]);
        let matrix = Shape {
            version_matrix: true,
            ..Shape::default()
        };
        assert_eq!(
            cause_of(matrix, "safe", &checks, &notes),
            Some("legacy-array-reached")
        );
        assert_eq!(
            cause_of(Shape::default(), "safe", &checks, &notes),
            Some("legacy-array-reached")
        );
        assert_eq!(
            cause_of(matrix, "safe", &checks, &notes[..1]),
            Some(NO_ARRAY)
        );
    }

    #[test]
    fn a_named_refusal_after_kept_machinery_is_that_refusal() {
        let kept = "note: remove-decoder-machinery: decoder machinery kept: 2 reference(s) to the string array or its decoders did not decode".to_string();
        let notes = [
            kept.clone(),
            "note: inline-decoded-strings: decoding bailed: unsupported construct: a stateful decoder keeps state that is not a proven memo or init gate".to_string(),
        ];
        let checks = BTreeMap::from([(
            "machinery",
            Failed {
                detail: "string_arrays=1,decoders=1,decoder_calls=2",
                note: &kept,
            },
        )]);
        assert_eq!(
            cause_of(Shape::default(), "unsafe", &checks, &notes),
            Some("stateful-decoder-state")
        );
        assert_eq!(
            cause_of(Shape::default(), "unsafe", &checks, &notes[..1]),
            Some(DECODE_MISS)
        );
    }

    #[test]
    fn an_unnamed_first_note_takes_a_named_refusal_after_it() {
        let alias = "note: inline-decoder-wrappers: the alias declaration has an initialiser, so the assignment to it was left in place".to_string();
        let notes = [
            alias.clone(),
            "note: inline-decoded-strings: a decoder of the unit is used other than as the callee of a call, so it may run from where no call site was collected, and the unit's call sites were left alone".to_string(),
        ];
        let checks = BTreeMap::from([(
            "machinery",
            Failed {
                detail: "string_arrays=1,decoders=1,decoder_calls=2",
                note: &alias,
            },
        )]);
        let shape = Shape {
            version_matrix: true,
            ..Shape::default()
        };
        assert_eq!(
            cause_of(shape, "unsafe", &checks, &notes),
            Some("decoder-used-as-value")
        );
    }

    #[test]
    fn an_escaping_stateful_decoder_with_no_later_refusal_is_named() {
        let kept = "note: remove-decoder-machinery: decoder machinery kept: 2 reference(s) to the string array or its decoders did not decode".to_string();
        let notes = [
            "note: inline-decoded-strings: decoding bailed: unsupported construct: a stateful decoder is referenced outside the calls untwist proves".to_string(),
            kept.clone(),
        ];
        let checks = BTreeMap::from([(
            "machinery",
            Failed {
                detail: "string_arrays=1,decoders=1,decoder_calls=2",
                note: &kept,
            },
        )]);
        assert_eq!(
            cause_of(Shape::default(), "unsafe", &checks, &notes),
            Some(STATEFUL_ESCAPE)
        );
    }
}
