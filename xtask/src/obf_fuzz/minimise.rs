use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::generate::rebuild;
use super::judge::{expected_of, run_lane, Rules};
use crate::obf_matrix::cell::Runner;

pub fn ddmin<T: Clone>(items: &[T], mut fails: impl FnMut(&[T]) -> bool) -> Vec<T> {
    let mut current = items.to_vec();
    let mut parts = 2usize;
    while current.len() >= 2 {
        let size = current.len().div_ceil(parts);
        let chunks: Vec<Vec<T>> = current.chunks(size).map(<[T]>::to_vec).collect();
        let mut reduced = false;
        for chunk in &chunks {
            if chunk.len() < current.len() && fails(chunk) {
                current = chunk.clone();
                parts = 2;
                reduced = true;
                break;
            }
        }
        if !reduced && chunks.len() > 2 {
            for skip in 0..chunks.len() {
                let rest: Vec<T> = chunks
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != skip)
                    .flat_map(|(_, c)| c.clone())
                    .collect();
                if fails(&rest) {
                    current = rest;
                    parts = (parts - 1).max(2);
                    reduced = true;
                    break;
                }
            }
        }
        if !reduced {
            if parts >= current.len() {
                break;
            }
            parts = (parts * 2).min(current.len());
        }
    }
    current
}

pub fn drop_each<T: Clone>(items: &[T], mut fails: impl FnMut(&[T]) -> bool) -> Vec<T> {
    let mut current = items.to_vec();
    let mut index = 0;
    while index < current.len() {
        let mut candidate = current.clone();
        candidate.remove(index);
        if fails(&candidate) {
            current = candidate;
        } else {
            index += 1;
        }
    }
    current
}

pub struct Target<'a> {
    pub lane: &'static str,
    pub cause: &'a str,
}

struct Probe<'a> {
    root: &'a Path,
    runner: &'a Runner,
    node: &'a Path,
    rules: &'a Rules<'a>,
    work: PathBuf,
    target: Target<'a>,
    probes: usize,
}

impl Probe<'_> {
    fn fails(&mut self, raw: &Value) -> bool {
        self.probes += 1;
        let dir = self.work.join(format!("probe-{}", self.probes));
        let outcome = (|| -> Result<bool> {
            let spec = rebuild(self.root, self.node, raw, &dir)?;
            if spec.built().is_err() {
                return Ok(false);
            }
            let Ok(expected) = expected_of(self.runner, &spec)? else {
                return Ok(false);
            };
            let lane = run_lane(self.runner, &spec, self.target.lane, &expected, self.rules)?;
            Ok(lane.verdict.cause() == Some(self.target.cause))
        })();
        let _ = std::fs::remove_dir_all(&dir);
        outcome.unwrap_or(false)
    }
}

fn with(raw: &Value, key: &str, value: Value) -> Value {
    let mut out = raw.clone();
    out[key] = value;
    out
}

fn reduce_list(probe: &mut Probe<'_>, raw: Value, key: &str, delta: bool) -> Value {
    let items = raw[key].as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        return raw;
    }
    let mut test = |subset: &[Value]| probe.fails(&with(&raw, key, Value::Array(subset.to_vec())));
    let kept = if delta {
        ddmin(&items, &mut test)
    } else {
        drop_each(&items, &mut test)
    };
    with(&raw, key, Value::Array(kept))
}

fn reduce_options(probe: &mut Probe<'_>, raw: Value) -> Value {
    let Some(options) = raw["options"].as_object().cloned() else {
        return raw;
    };
    let entries: Vec<(String, Value)> = options.into_iter().collect();
    let object = |subset: &[(String, Value)]| Value::Object(subset.iter().cloned().collect());
    let kept = drop_each(&entries, |subset| {
        probe.fails(&with(&raw, "options", object(subset)))
    });
    with(&raw, "options", object(&kept))
}

fn reduce_flags(probe: &mut Probe<'_>, raw: Value) -> Value {
    let mut current = raw;
    let plain = serde_json::json!({ "strict": false, "iife": false });
    for (key, value) in [("terser", Value::Bool(false)), ("envelope", plain)] {
        if current[key] != value {
            let candidate = with(&current, key, value);
            if probe.fails(&candidate) {
                current = candidate;
            }
        }
    }
    current
}

pub struct Minimised {
    pub dir: PathBuf,
    pub probes: usize,
    pub chunks: usize,
}

fn digest(raw: &Value) -> String {
    Sha256::digest(raw.to_string().as_bytes())
        .iter()
        .take(4)
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn minimise(
    (root, runner, node): (&Path, &Runner, &Path),
    rules: &Rules<'_>,
    raw: &Value,
    target: Target<'_>,
    out: &Path,
) -> Result<Minimised> {
    let work = tempfile::Builder::new()
        .prefix("obf-fuzz-min-")
        .tempdir_in(out)?;
    let cause = target.cause.to_string();
    let lane = target.lane;
    let mut probe = Probe {
        root,
        runner,
        node,
        rules,
        work: work.path().to_path_buf(),
        target,
        probes: 0,
    };
    let mut current = raw.clone();
    current["wiring"] = current["applied"].clone();
    current = reduce_list(&mut probe, current, "chunks", true);
    current = reduce_options(&mut probe, current);
    current = reduce_list(&mut probe, current, "wiring", false);
    current = reduce_flags(&mut probe, current);
    let dir = out.join(format!(
        "{lane}-{}-{}",
        cause.replace('/', "--"),
        digest(&current)
    ));
    rebuild(root, node, &current, &dir)?;
    Ok(Minimised {
        dir,
        probes: probe.probes,
        chunks: current["chunks"].as_array().map_or(0, Vec::len),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddmin_finds_the_two_items_that_together_fail() {
        let items: Vec<u32> = (0..16).collect();
        let mut calls = 0;
        let kept = ddmin(&items, |subset| {
            calls += 1;
            subset.contains(&3) && subset.contains(&11)
        });
        assert_eq!(kept, vec![3, 11]);
        assert!(calls < 60, "{calls} probes");
    }

    #[test]
    fn ddmin_keeps_a_single_culprit_and_never_returns_a_passing_set() {
        let items: Vec<u32> = (0..7).collect();
        assert_eq!(ddmin(&items, |s| s.contains(&6)), vec![6]);
        assert_eq!(ddmin(&items, |s| s.len() == 7), items);
    }

    #[test]
    fn drop_each_removes_every_item_the_failure_does_not_need() {
        let items = vec!["a", "b", "c", "d"];
        assert_eq!(
            drop_each(&items, |s| s.contains(&"b") && s.contains(&"d")),
            vec!["b", "d"]
        );
        assert!(drop_each(&items, |_| true).is_empty());
    }
}
