use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::judge::SampleResult;
use crate::obf_matrix::notes::class_of;

const EXAMPLES: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cluster {
    pub lane: &'static str,
    pub cause: String,
    pub tag: &'static str,
    pub samples: Vec<String>,
    pub notes: BTreeMap<String, usize>,
    pub repro: Option<PathBuf>,
}

impl Cluster {
    pub fn top_note(&self) -> Option<&str> {
        self.notes
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(note, _)| note.as_str())
    }

    pub fn is_new(&self) -> bool {
        self.tag == "NEW"
    }
}

pub fn clusters(results: &[SampleResult]) -> Vec<Cluster> {
    let mut by_key: BTreeMap<(&'static str, String), Cluster> = BTreeMap::new();
    for result in results {
        for lane in &result.lanes {
            let Some(cause) = lane.verdict.cause() else {
                continue;
            };
            let cluster = by_key
                .entry((lane.lane, cause.to_string()))
                .or_insert_with(|| Cluster {
                    lane: lane.lane,
                    cause: cause.to_string(),
                    tag: lane.verdict.tag(),
                    samples: Vec::new(),
                    notes: BTreeMap::new(),
                    repro: None,
                });
            cluster.samples.push(result.id.clone());
            if let Some(note) = lane.run.first_note() {
                *cluster.notes.entry(class_of(note)).or_default() += 1;
            }
        }
    }
    let mut out: Vec<Cluster> = by_key.into_values().collect();
    out.sort_by(|a, b| {
        b.is_new()
            .cmp(&a.is_new())
            .then(b.samples.len().cmp(&a.samples.len()))
            .then(a.lane.cmp(b.lane))
            .then(a.cause.cmp(&b.cause))
    });
    out
}

pub struct Totals {
    pub samples: usize,
    pub judged: usize,
    pub skipped: BTreeMap<String, usize>,
    pub lanes: BTreeMap<(&'static str, &'static str), usize>,
}

fn skip_class(reason: &str) -> String {
    reason.chars().take(60).collect()
}

pub fn totals(results: &[SampleResult]) -> Totals {
    let mut totals = Totals {
        samples: results.len(),
        judged: 0,
        skipped: BTreeMap::new(),
        lanes: BTreeMap::new(),
    };
    for result in results {
        if let Some(reason) = &result.skipped {
            *totals.skipped.entry(skip_class(reason)).or_default() += 1;
            continue;
        }
        totals.judged += 1;
        for lane in &result.lanes {
            *totals
                .lanes
                .entry((lane.lane, lane.verdict.tag()))
                .or_default() += 1;
        }
    }
    totals
}

pub fn render(clusters: &[Cluster], totals: &Totals) -> String {
    let mut lines =
        vec!["cluster\tlane\tverdict\tsamples\tcause\ttop-note\texamples\trepro".to_string()];
    for c in clusters {
        let examples: Vec<&str> = c
            .samples
            .iter()
            .take(EXAMPLES)
            .map(String::as_str)
            .collect();
        lines.push(format!(
            "cluster\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            c.lane,
            c.tag,
            c.samples.len(),
            c.cause,
            c.top_note().unwrap_or("-"),
            examples.join(","),
            c.repro
                .as_ref()
                .map_or("-".to_string(), |p| p.display().to_string())
        ));
    }
    for (reason, n) in &totals.skipped {
        lines.push(format!("skipped\t{n}\t{reason}"));
    }
    let lanes: Vec<String> = totals
        .lanes
        .iter()
        .map(|((lane, tag), n)| format!("{lane} {tag} {n}"))
        .collect();
    lines.push(format!(
        "obf-fuzz: {} samples, {} judged, {}",
        totals.samples,
        totals.judged,
        lanes.join(", ")
    ));
    lines.join("\n") + "\n"
}

pub fn render_samples(results: &[SampleResult]) -> String {
    let mut lines = vec!["sample\tlane\tverdict\tcause\tfeatures\tfirst-note".to_string()];
    for result in results {
        if let Some(reason) = &result.skipped {
            lines.push(format!(
                "{}\t-\tskipped\t-\t{}\t{reason}",
                result.id,
                result.features.join(",")
            ));
            continue;
        }
        for lane in &result.lanes {
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                result.id,
                lane.lane,
                lane.verdict.tag(),
                lane.verdict.cause().unwrap_or("-"),
                result.features.join(","),
                lane.run.first_note().unwrap_or("-").replace('\t', " ")
            ));
        }
    }
    lines.join("\n") + "\n"
}

pub fn write(path: &Path, text: &str) -> Result<()> {
    std::fs::write(path, text).with_context(|| path.display().to_string())
}

pub fn new_failures(clusters: &[Cluster]) -> usize {
    clusters
        .iter()
        .filter(|c| c.is_new())
        .map(|c| c.samples.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obf_fuzz::judge::{LaneResult, Verdict};
    use crate::obf_options::lanes::{Behaviour, LaneRun, SAFE, UNSAFE};
    use crate::obf_options::machinery::measure;

    fn lane(lane: &'static str, verdict: Verdict, note: Option<&str>) -> LaneResult {
        LaneResult {
            lane,
            run: LaneRun {
                exit: Some(0),
                found: measure(""),
                behaviour: Behaviour::Same,
                notes: note.map(|n| vec![n.to_string()]).unwrap_or_default(),
            },
            verdict,
        }
    }

    fn sample(id: &str, lanes: Vec<LaneResult>) -> SampleResult {
        SampleResult {
            id: id.to_string(),
            features: Vec::new(),
            skipped: None,
            lanes,
        }
    }

    #[test]
    fn failures_group_by_lane_and_cause_with_new_clusters_first() {
        let key = || Verdict::Known("safe-global-key-reach".into());
        let results = vec![
            sample(
                "a",
                vec![
                    lane(SAFE, key(), Some("note: p: may reach x")),
                    lane(UNSAFE, Verdict::Pass, None),
                ],
            ),
            sample(
                "b",
                vec![
                    lane(SAFE, key(), Some("note: p: may reach x")),
                    lane(UNSAFE, Verdict::New("wrong-output".into()), None),
                ],
            ),
            sample(
                "c",
                vec![
                    lane(SAFE, Verdict::Sound("safe-global-names".into()), None),
                    lane(UNSAFE, Verdict::Pass, None),
                ],
            ),
        ];
        let got = clusters(&results);
        let keys: Vec<(&str, &str, usize)> = got
            .iter()
            .map(|c| (c.lane, c.cause.as_str(), c.samples.len()))
            .collect();
        assert_eq!(
            keys,
            vec![
                (UNSAFE, "wrong-output", 1),
                (SAFE, "safe-global-key-reach", 2),
                (SAFE, "safe-global-names", 1),
            ]
        );
        assert_eq!(got[1].top_note(), Some("p: may reach x"));
        assert_eq!(new_failures(&got), 1);
    }

    #[test]
    fn totals_count_skips_and_lane_verdicts() {
        let mut skipped = sample("s", Vec::new());
        skipped.skipped = Some("not built: boom".into());
        let results = vec![skipped, sample("a", vec![lane(SAFE, Verdict::Pass, None)])];
        let t = totals(&results);
        assert_eq!((t.samples, t.judged), (2, 1));
        assert_eq!(t.lanes[&(SAFE, "ok")], 1);
        let text = render(&clusters(&results), &t);
        assert!(text.contains("skipped\t1\tnot built: boom"), "{text}");
    }
}
