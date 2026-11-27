use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, Expression, Statement};
use oxc_ast_visit::{walk, Visit};
use serde::Serialize;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::{
    intersect_ranges, version_key, whole_module_matches, Ranges, WholeModule,
};

use crate::bundle::BundleOutput;

pub const PACKAGE_JSON: &str = "package.json";
const REQUIRE: &str = "require";
const SCRIPT_SUFFIX: &str = ".js";
const NODE_MODULES: &str = "node_modules/";
const MODULE_STATE: [&str; 4] = [
    "__webpack_require__",
    "__webpack_modules__",
    "require.main",
    "module.parent",
];
const BUNDLE_STATE: [&str; 2] = ["require.cache", "module.hot"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeptReason {
    SideEffects,
    Format,
    Ambiguous,
}

impl KeptReason {
    pub fn as_str(self) -> &'static str {
        match self {
            KeptReason::SideEffects => "side-effects",
            KeptReason::Format => "format",
            KeptReason::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Replaced {
    pub module: String,
    pub package: String,
    pub version: String,
    pub specifier: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub module: String,
    pub package: String,
    pub reason: KeptReason,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportsReport {
    pub replaced: Vec<Replaced>,
    pub kept: Vec<Kept>,
    pub pins: BTreeMap<String, String>,
}

impl ImportsReport {
    pub fn package_json(&self) -> Option<String> {
        if self.pins.is_empty() {
            return None;
        }
        let manifest = serde_json::json!({
            "private": true,
            "dependencies": self.pins,
        });
        serde_json::to_string_pretty(&manifest)
            .ok()
            .map(|text| text + "\n")
    }
}

struct Candidate {
    index: usize,
    whole: WholeModule,
    specifier: String,
    requires: Vec<usize>,
}

#[derive(Default)]
struct Requires {
    literal: Vec<String>,
    dynamic: bool,
    esm: bool,
}

impl<'a> Visit<'a> for Requires {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if matches!(
            it,
            Statement::ImportDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::ExportDefaultDeclaration(_)
                | Statement::ExportNamedDeclaration(_)
        ) {
            self.esm = true;
        }
        walk::walk_statement(self, it);
    }

    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        if matches!(&it.callee, Expression::Identifier(id) if id.name == REQUIRE) {
            match it.arguments.as_slice() {
                [Argument::StringLiteral(literal)] => {
                    self.literal
                        .push(untwist_core::ast_util::js_text::js_text(literal.value).into_owned());
                }
                _ => self.dynamic = true,
            }
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &oxc_ast::ast::ImportExpression<'a>) {
        self.esm = true;
        walk::walk_import_expression(self, it);
    }
}

fn normalize(base: &str, specifier: &str) -> Option<String> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None;
    }
    let mut parts: Vec<&str> = base.split('/').collect();
    parts.pop();
    for part in specifier.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

fn package_path(specifier: &str) -> Option<String> {
    let plain = !specifier.is_empty()
        && !specifier.starts_with(['.', '/'])
        && !specifier.contains(':')
        && specifier
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    plain.then(|| format!("{NODE_MODULES}{specifier}"))
}

fn resolve(paths: &BTreeMap<&str, usize>, base: &str, specifier: &str) -> Option<usize> {
    let joined = normalize(base, specifier).or_else(|| package_path(specifier))?;
    [
        joined.clone(),
        format!("{joined}{SCRIPT_SUFFIX}"),
        format!("{joined}/index{SCRIPT_SUFFIX}"),
    ]
    .iter()
    .find_map(|path| paths.get(path.as_str()).copied())
}

enum Scan {
    None,
    Kept(Kept),
    Candidate(Candidate),
}

fn scan_module(bundle: &BundleOutput, paths: &BTreeMap<&str, usize>, index: usize) -> Scan {
    let module = &bundle.modules[index];
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &module.code);
    if has_error(&parsed.diagnostics) {
        return Scan::None;
    }
    let sem = build_scoping(&parsed.program);
    let mut wholes = whole_module_matches(&parsed.program, &sem.scoping);
    wholes.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.module.cmp(&b.module)));
    wholes.dedup_by(|a, b| a.label == b.label && a.module == b.module);
    let Some(first) = wholes.first().cloned() else {
        return Scan::None;
    };
    let kept = |reason| {
        Scan::Kept(Kept {
            module: module.path.clone(),
            package: first.label.clone(),
            reason,
        })
    };
    if wholes.len() > 1 {
        return kept(KeptReason::Ambiguous);
    }
    let mut requires = Requires::default();
    requires.visit_program(&parsed.program);
    let Some(specifier) = first.specifier.clone() else {
        return kept(KeptReason::Format);
    };
    if requires.esm {
        return kept(KeptReason::Format);
    }
    let bundler_state = MODULE_STATE.iter().any(|name| module.code.contains(name));
    let resolved: Option<Vec<usize>> = requires
        .literal
        .iter()
        .map(|specifier| resolve(paths, &module.path, specifier))
        .collect();
    match (requires.dynamic || bundler_state, resolved) {
        (false, Some(requires)) => Scan::Candidate(Candidate {
            index,
            whole: first,
            specifier,
            requires,
        }),
        _ => kept(KeptReason::SideEffects),
    }
}

fn highest(ranges: &Ranges) -> Option<String> {
    ranges
        .iter()
        .map(|(_, high)| high.clone())
        .max_by(|a, b| version_key(a).cmp(&version_key(b)))
}

fn settle(
    mut candidates: Vec<Candidate>,
    kept: &mut Vec<Kept>,
    bundle: &BundleOutput,
) -> Vec<(Candidate, String)> {
    loop {
        let present: BTreeSet<usize> = candidates.iter().map(|candidate| candidate.index).collect();
        let (ready, broken): (Vec<Candidate>, Vec<Candidate>) = candidates
            .into_iter()
            .partition(|candidate| candidate.requires.iter().all(|dep| present.contains(dep)));
        let mut windows: BTreeMap<String, Option<Ranges>> = BTreeMap::new();
        for candidate in &ready {
            let window = windows
                .entry(candidate.whole.label.clone())
                .or_insert_with(|| Some(candidate.whole.ranges.clone()));
            *window = window
                .take()
                .map(|known| intersect_ranges(&known, &candidate.whole.ranges))
                .filter(|ranges| !ranges.is_empty());
        }
        let pinned: BTreeMap<String, String> = windows
            .into_iter()
            .filter_map(|(label, window)| Some((label, highest(&window?)?)))
            .collect();
        let (fits, ambiguous): (Vec<Candidate>, Vec<Candidate>) = ready
            .into_iter()
            .partition(|candidate| pinned.contains_key(&candidate.whole.label));
        let dropped = broken.len() + ambiguous.len();
        let reason_of = |candidate: &Candidate, reason| Kept {
            module: bundle.modules[candidate.index].path.clone(),
            package: candidate.whole.label.clone(),
            reason,
        };
        kept.extend(
            broken
                .iter()
                .map(|candidate| reason_of(candidate, KeptReason::SideEffects)),
        );
        kept.extend(
            ambiguous
                .iter()
                .map(|candidate| reason_of(candidate, KeptReason::Ambiguous)),
        );
        if dropped == 0 {
            return fits
                .into_iter()
                .map(|candidate| {
                    let version = pinned[&candidate.whole.label].clone();
                    (candidate, version)
                })
                .collect();
        }
        candidates = fits;
    }
}

fn shim(label: &str, version: &str, specifier: &str) -> String {
    format!(
        "// {label} {version}: replaced by the package, see package.json\nmodule.exports = require({});\n",
        serde_json::to_string(specifier).unwrap_or_default()
    )
}

pub fn apply(bundle: &mut BundleOutput) -> ImportsReport {
    if BUNDLE_STATE.iter().any(|name| {
        bundle
            .modules
            .iter()
            .any(|module| module.code.contains(name))
    }) {
        return ImportsReport::default();
    }
    let paths: BTreeMap<&str, usize> = bundle
        .modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.path.as_str(), index))
        .collect();
    let mut kept = Vec::new();
    let mut candidates = Vec::new();
    for index in 0..bundle.modules.len() {
        match scan_module(bundle, &paths, index) {
            Scan::None => {}
            Scan::Kept(found) => kept.push(found),
            Scan::Candidate(candidate) => candidates.push(candidate),
        }
    }
    let settled = settle(candidates, &mut kept, bundle);
    let mut report = ImportsReport {
        kept,
        ..ImportsReport::default()
    };
    for (candidate, version) in settled {
        let module = &mut bundle.modules[candidate.index];
        module.code = shim(&candidate.whole.label, &version, &candidate.specifier);
        module.map = None;
        report
            .pins
            .insert(candidate.whole.label.clone(), version.clone());
        report.replaced.push(Replaced {
            module: module.path.clone(),
            package: candidate.whole.label,
            version,
            specifier: candidate.specifier,
        });
    }
    report.kept.sort_by(|a, b| a.module.cmp(&b.module));
    report.replaced.sort_by(|a, b| a.module.cmp(&b.module));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_specifier_resolves_against_the_module_directory() {
        assert_eq!(
            normalize("node_modules/a/lib/x.js", "../y").as_deref(),
            Some("node_modules/a/y")
        );
        assert_eq!(normalize("x.js", "./y.js").as_deref(), Some("y.js"));
        assert_eq!(normalize("x.js", "../y.js"), None);
        assert_eq!(normalize("x.js", "lodash"), None);
        assert_eq!(
            package_path("base64-js/index.js").as_deref(),
            Some("node_modules/base64-js/index.js")
        );
        assert_eq!(package_path("../x"), None);
        assert_eq!(package_path("node:fs"), None);
        assert_eq!(package_path("a//b"), None);
    }

    #[test]
    fn the_pinned_version_is_the_highest_the_window_allows() {
        let ranges = vec![
            ("4.17.15".to_string(), "4.17.9".to_string()),
            ("4.17.10".to_string(), "4.17.21".to_string()),
        ];
        assert_eq!(highest(&ranges).as_deref(), Some("4.17.21"));
    }

    #[test]
    fn a_report_without_pins_writes_no_package_json() {
        assert_eq!(ImportsReport::default().package_json(), None);
        let report = ImportsReport {
            pins: BTreeMap::from([("ieee754".to_string(), "1.2.1".to_string())]),
            ..ImportsReport::default()
        };
        let text = report.package_json().expect("a manifest");
        assert!(text.contains("\"ieee754\": \"1.2.1\""), "{text}");
    }
}
