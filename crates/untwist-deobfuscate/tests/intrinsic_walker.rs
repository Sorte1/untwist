use std::collections::{BTreeMap, BTreeSet};

use untwist_deobfuscate::calls::agnostic::{key_roles, QUIET_INTRINSICS};
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const WALKER: &str = include_str!("data/intrinsic_walker.js");
const CLASSES: &str = include_str!("data/intrinsics.tsv");

const ACCESSOR_SUFFIXES: [&str; 2] = [" [get]", " [set]"];

fn walked(node: &std::path::Path) -> BTreeMap<String, Vec<String>> {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("walk.js");
    std::fs::write(&script, WALKER).unwrap();
    let out = run(node, &script).unwrap().expect("the walker finished");
    assert_eq!(out.exit, Some(0), "the walker failed: {}", out.stderr);
    out.stdout
        .lines()
        .map(|line| {
            let (path, keys) = line.split_once('\t').unwrap_or((line, ""));
            let keys = keys
                .split(',')
                .filter(|key| !key.is_empty())
                .map(str::to_string)
                .collect();
            (path.to_string(), keys)
        })
        .collect()
}

fn classes() -> BTreeMap<&'static str, &'static str> {
    CLASSES
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect()
}

fn plain(keys: &[String]) -> impl Iterator<Item = &str> {
    keys.iter()
        .map(String::as_str)
        .filter(|key| !ACCESSOR_SUFFIXES.iter().any(|suffix| key.ends_with(suffix)))
}

fn base(key: &str) -> &str {
    ACCESSOR_SUFFIXES
        .iter()
        .find_map(|suffix| key.strip_suffix(suffix))
        .unwrap_or(key)
}

fn misjudged(path: &str, class: &str, keys: &[String]) -> Option<String> {
    let all_plain = |role: fn(&str) -> bool| {
        let names: Vec<&str> = plain(keys).collect();
        !names.is_empty() && names.iter().all(|name| role(name))
    };
    let held = match class {
        "pure" | "slot" | "own" | "index" | "accessor" => true,
        "definer"
            if ACCESSOR_SUFFIXES
                .iter()
                .any(|suffix| path.ends_with(suffix)) =>
        {
            keys.iter().all(|key| key_roles(base(key)).accessor_definer)
        }
        "definer" => all_plain(|name| key_roles(name).definer),
        "reader" => all_plain(|name| key_roles(name).reader),
        "mutator" => all_plain(|name| key_roles(name).mutator),
        "runner" => all_plain(|name| key_roles(name).runner),
        "forwarder" => all_plain(|name| key_roles(name).forwards),
        "construct" => all_plain(|name| key_roles(name).construct),
        other => return Some(format!("{path}: unknown class {other}")),
    };
    (!held).then(|| format!("{path}: class {class}, but (A) does not judge its keys {keys:?} so"))
}

#[test]
fn every_intrinsic_function_in_node_s_realm_is_classified() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let walked = walked(&node);
    let classes = classes();
    let mut wrong: Vec<String> = Vec::new();
    for (path, keys) in &walked {
        match classes.get(path.as_str()) {
            None => wrong.push(format!("{path}: unclassified (keys {keys:?})")),
            Some(class) => wrong.extend(misjudged(path, class, keys)),
        }
    }
    let found: BTreeSet<&str> = walked.keys().map(String::as_str).collect();
    for path in classes.keys() {
        if !found.contains(path) {
            wrong.push(format!("{path}: classified but not in node's realm"));
        }
    }
    assert!(
        walked.len() > 500,
        "the walker found {} functions",
        walked.len()
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_quiet_intrinsic_is_a_classified_function_that_runs_no_code_of_its_own() {
    let classes = classes();
    let wrong: Vec<String> = QUIET_INTRINSICS
        .iter()
        .filter(|path| !matches!(classes.get(*path), Some(&"pure" | &"construct")))
        .map(|path| format!("{path}: {:?}", classes.get(path)))
        .collect();
    assert!(wrong.is_empty(), "{wrong:?}");
}
