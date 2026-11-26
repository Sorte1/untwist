use std::collections::{BTreeMap, BTreeSet};

use untwist_svelte::FingerprintRow;

fn rows(path: &str) -> Vec<FingerprintRow> {
    let code = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"));
    untwist_svelte::fingerprint_rows(&code)
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if std::env::var_os("DUMP").is_some() {
        for path in &args {
            for row in rows(path) {
                println!("{path}\t{}\t{:016x}\t{}", row.name, row.hash, row.nodes);
            }
        }
        return;
    }
    let mut labels: BTreeMap<u64, BTreeSet<Option<String>>> = BTreeMap::new();
    for arg in &args {
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
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, rt) in &kept {
        *count.entry(rt.as_str()).or_default() += 1;
    }
    eprintln!("{} rows: {count:?}", kept.len());
    println!("use crate::runtime::Rt;\n");
    println!("const TABLE: &[(u64, Rt)] = &[");
    for (hash, rt) in &kept {
        println!("    (0x{hash:016x}, Rt::{rt}),");
    }
    println!("];");
    print!("{}", include_str!("lookup.rs.in"));
}
