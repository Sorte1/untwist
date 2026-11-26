use std::collections::{BTreeMap, BTreeSet};

pub struct FingerprintRow {
    pub hash: u64,
    pub rt: Option<String>,
    pub nodes: usize,
    pub name: String,
    pub used_by_component: bool,
}

fn chosen(rows: &[FingerprintRow]) -> Vec<Option<String>> {
    let mut per_rt: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        if let Some(rt) = &row.rt {
            per_rt.entry(rt.clone()).or_default().push(index);
        }
    }
    let mut out = vec![None; rows.len()];
    for (rt, indexes) in per_rt {
        let picked: Vec<usize> = indexes
            .iter()
            .copied()
            .filter(|index| rows[*index].used_by_component)
            .collect();
        match picked.as_slice() {
            [only] => out[*only] = Some(rt),
            [] => {}
            _ => eprintln!("{rt} has {} candidates, left out", indexes.len()),
        }
    }
    out
}

#[doc(hidden)]
pub fn table(args: &[String], rows: &dyn Fn(&str) -> Vec<FingerprintRow>) -> Vec<(u64, String)> {
    let mut labels: BTreeMap<u64, BTreeSet<Option<String>>> = BTreeMap::new();
    for arg in args {
        let (only_exports, arg) = match arg.strip_prefix("exports:") {
            Some(rest) => (true, rest),
            None => (false, arg.as_str()),
        };
        let (labeled, mangled) = arg.split_once('=').unwrap_or((arg, arg));
        let named = rows(labeled);
        let plain = rows(mangled);
        if named.len() != plain.len() {
            panic!("{labeled} and {mangled} hold different function counts");
        }
        let picks = chosen(&named);
        let mut misaligned = 0;
        for ((left, right), rt) in named.iter().zip(&plain).zip(picks) {
            if only_exports && !rt.as_deref().is_some_and(|rt| rt.starts_with("Export")) {
                continue;
            }
            if rt.is_none() && left.rt.is_some() {
                continue;
            }
            let rt = match left.nodes == right.nodes {
                true => rt,
                false => {
                    misaligned += 1;
                    None
                }
            };
            labels.entry(left.hash).or_default().insert(rt.clone());
            labels.entry(right.hash).or_default().insert(rt);
        }
        eprintln!(
            "{labeled}: {} functions, {misaligned} misaligned",
            named.len()
        );
    }
    if std::env::var_os("EXPLAIN").is_some() {
        for (hash, set) in &labels {
            if set.len() > 1 {
                eprintln!("conflict {hash:016x}: {set:?}");
            }
        }
    }
    let mut kept: Vec<(u64, String)> = Vec::new();
    for (hash, set) in &labels {
        if let [Some(rt)] = set.iter().collect::<Vec<_>>().as_slice() {
            kept.push((*hash, rt.clone()));
        }
    }
    kept
}
