use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use oxc_allocator::Allocator;
use serde_json::Value;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::shape::UnitKind;
use untwist_rename::library::table::{exact_key, loose_key, Role, RowData as Row};
use untwist_rename::library::unit_records;

use crate::library_table::{npm_key, parse_npm_key, Build, Manifest, Observed, MIN_UNIT_TOKENS};

pub(crate) struct WholeFile {
    pub(crate) key: String,
    specifier: Option<String>,
}

fn whole_file_row(text: &str, file: &WholeFile, build: &Build, manifest: &Manifest) -> Option<Row> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let sem = build_scoping(&parsed.program);
    let record = unit_records(&parsed.program, &sem.scoping)
        .into_iter()
        .find(|record| record.kind == UnitKind::Program)?;
    if record.loose.0 < MIN_UNIT_TOKENS || build.obfuscated {
        return None;
    }
    let (label, versions, module) = match manifest.npm {
        true => {
            let owner = parse_npm_key(&file.key)?;
            (owner.label, owner.version, owner.module)
        }
        false => (
            manifest.label.clone(),
            build.version.clone(),
            file.key.clone(),
        ),
    };
    let role = match file.specifier {
        Some(_) => Role::Complete,
        None => Role::Module,
    };
    Some(Row {
        loose: loose_key(record.loose),
        exact: exact_key(record.exact),
        label,
        versions,
        module,
        role,
        own: file.specifier.clone(),
        inner: Vec::new(),
        outer: Vec::new(),
        specific: record.specific,
    })
}

pub(crate) fn nested_module_rows(
    out: &Path,
    build: &Build,
    manifest: &Manifest,
    files: &BTreeMap<PathBuf, WholeFile>,
) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for (path, file) in files {
        if path.parent() == Some(out) || !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(path)?;
        rows.extend(whole_file_row(&text, file, build, manifest));
    }
    Ok(rows)
}

pub(crate) fn lane_stable(rows: Vec<Row>) -> Vec<Observed> {
    let mut exacts: BTreeMap<(String, String), BTreeSet<u64>> = BTreeMap::new();
    for row in &rows {
        exacts
            .entry((row.label.clone(), row.module.clone()))
            .or_default()
            .insert(row.exact);
    }
    rows.into_iter()
        .map(|mut row| {
            let stable = exacts[&(row.label.clone(), row.module.clone())].len() == 1;
            if row.role == Role::Complete && !stable {
                row.role = Role::Module;
                row.own = None;
            }
            Observed { known: true, row }
        })
        .collect()
}

pub(crate) fn whole_files(
    out: &Path,
    build: &Build,
    attributor_files: &BTreeMap<String, String>,
    owners: Option<&BTreeMap<String, String>>,
) -> BTreeMap<PathBuf, WholeFile> {
    let Ok(text) = std::fs::read_to_string(out.join("bundle.json")) else {
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return BTreeMap::new();
    };
    value["modules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|module| {
            let id = module["id"].as_str()?;
            let path = module["path"].as_str()?;
            let key = module_key(id, build, attributor_files, owners)?;
            Some((
                out.join(path),
                WholeFile {
                    key,
                    specifier: build.complete.get(id).cloned(),
                },
            ))
        })
        .collect()
}

fn module_key(
    id: &str,
    build: &Build,
    attributor_files: &BTreeMap<String, String>,
    owners: Option<&BTreeMap<String, String>>,
) -> Option<String> {
    if let Some(owners) = owners {
        return npm_key(id, owners);
    }
    let marker = format!("{}/", build.dir);
    let at = id.find(&marker)?;
    attributor_files.get(&id[at + marker.len()..]).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(exact: u64, module: &str, role: Role) -> Row {
        Row {
            loose: 1,
            exact,
            label: "ieee754".to_string(),
            versions: "1.2.1".to_string(),
            module: module.to_string(),
            role,
            own: (role == Role::Complete).then(|| format!("ieee754/{module}")),
            inner: Vec::new(),
            outer: Vec::new(),
            specific: 3,
        }
    }

    #[test]
    fn a_file_whose_lanes_disagree_is_not_complete() {
        let rows = vec![
            row(1, "index.js", Role::Complete),
            row(1, "index.js", Role::Complete),
            row(2, "other.js", Role::Complete),
            row(3, "other.js", Role::Complete),
        ];
        let roles: Vec<(Role, Option<String>)> = lane_stable(rows)
            .into_iter()
            .map(|item| (item.row.role, item.row.own))
            .collect();
        assert_eq!(
            roles,
            [
                (Role::Complete, Some("ieee754/index.js".to_string())),
                (Role::Complete, Some("ieee754/index.js".to_string())),
                (Role::Module, None),
                (Role::Module, None),
            ]
        );
    }
}
