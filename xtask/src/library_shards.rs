use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use untwist_rename::library::collection::{shard_bloom_id, Bloom, Shard};
use untwist_rename::library::table::{Role, RowData as Row, Table};
use untwist_rename::library::{parse_ranges, version_key};

use crate::library_collection::{range_of, versions_of, write_content};

pub(crate) const SPLIT_BYTES: usize = 256 * 1024;

pub(crate) fn major_key(version: &str) -> Vec<u64> {
    let parts = version_key(version);
    let lead = parts
        .iter()
        .position(|part| *part != 0)
        .unwrap_or(parts.len().saturating_sub(1));
    parts.into_iter().take(lead + 1).collect()
}

fn highest_major(row: &Row) -> Vec<u64> {
    parse_ranges(&row.versions)
        .into_iter()
        .map(|(_, high)| major_key(&high))
        .max()
        .unwrap_or_default()
}

pub(crate) fn split_by_major(rows: &[Row]) -> Vec<Vec<Row>> {
    let mut groups: BTreeMap<Vec<u64>, Vec<Row>> = BTreeMap::new();
    for row in rows {
        groups
            .entry(highest_major(row))
            .or_default()
            .push(row.clone());
    }
    groups.into_values().collect()
}

fn shard_of(out: &Path, label: &str, rows: &[Row]) -> Result<(Shard, Bloom)> {
    let bytes = crate::library_gz::bytes(&Table::from_rows(rows).write(label));
    let (file, sha256) = write_content(out, &bytes, "utl")?;
    let loose: Vec<u32> = rows
        .iter()
        .filter(|row| row.role != Role::Order)
        .map(|row| row.loose)
        .collect();
    let shard = Shard {
        file,
        bytes: bytes.len() as u64,
        sha256,
        versions: range_of(&versions_of(rows)),
        rows: rows.len() as u64,
    };
    Ok((shard, Bloom::with_keys(&loose)))
}

pub(crate) type Blooms = Vec<(String, Bloom)>;

pub(crate) fn package_shards(
    out: &Path,
    label: &str,
    rows: &[Row],
) -> Result<(Vec<Shard>, Blooms)> {
    let whole = crate::library_gz::bytes(&Table::from_rows(rows).write(label));
    let groups = match whole.len() > SPLIT_BYTES {
        true => split_by_major(rows),
        false => vec![rows.to_vec()],
    };
    if groups.len() < 2 {
        let (shard, bloom) = shard_of(out, label, rows)?;
        return Ok((vec![shard], vec![(label.to_string(), bloom)]));
    }
    let mut shards = Vec::new();
    let mut blooms = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let (shard, bloom) = shard_of(out, label, group)?;
        shards.push(shard);
        blooms.push((shard_bloom_id(label, index), bloom));
    }
    Ok((shards, blooms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(exact: u64, versions: &str) -> Row {
        Row {
            loose: exact as u32,
            exact,
            label: "three".to_string(),
            versions: versions.to_string(),
            module: "build/three.module.js".to_string(),
            role: Role::Function,
            own: None,
            inner: Vec::new(),
            outer: Vec::new(),
            specific: 3,
        }
    }

    #[test]
    fn the_major_of_a_zero_version_is_its_first_non_zero_part() {
        assert_eq!(major_key("3.38.1"), [3]);
        assert_eq!(major_key("0.150.0"), [0, 150]);
        assert_eq!(major_key("0.0.7"), [0, 0, 7]);
    }

    #[test]
    fn a_row_goes_to_the_shard_of_its_newest_major() {
        let rows = vec![
            row(1, "0.120.1~0.150.0"),
            row(2, "0.120.1"),
            row(3, "0.150.0,0.187.0"),
        ];
        let groups = split_by_major(&rows);
        let exacts: Vec<Vec<u64>> = groups
            .iter()
            .map(|group| group.iter().map(|row| row.exact).collect())
            .collect();
        assert_eq!(exacts, [vec![2], vec![1], vec![3]]);
    }
}
