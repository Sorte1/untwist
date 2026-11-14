use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use rustc_hash::FxHashMap;
use untwist_core::ast_util::prop::{number_key, prop_name};

use super::stem::meaningful;
use crate::matcher::{argument_expression, call_parts};
use crate::model::Bundle;

const REQUIRE: &str = "require";
const WEBPACK_IMPORTED: &str = "__WEBPACK_IMPORTED_MODULE_";
const DEFAULT_SUFFIX: &str = "_default";
const SCRIPT_SUFFIX: &str = "_js";

#[derive(Default)]
pub struct Importers {
    pub reads: FxHashMap<(String, String), usize>,
    pub names: FxHashMap<String, FxHashMap<String, usize>>,
    pub paths: FxHashMap<String, FxHashMap<String, usize>>,
}

fn unique_top(votes: Option<&FxHashMap<String, usize>>) -> Option<&str> {
    let votes = votes?;
    let best = votes.values().copied().max()?;
    let mut top = votes.iter().filter(|(_, count)| **count == best);
    let (name, _) = top.next()?;
    top.next().is_none().then_some(name.as_str())
}

pub fn webpack_import_stem(name: &str) -> Option<String> {
    let (head, tail) = name.split_once(WEBPACK_IMPORTED)?;
    let index = tail.trim_end_matches(DEFAULT_SUFFIX).strip_suffix("__")?;
    if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let stem = head.trim_start_matches('_');
    let stem = stem
        .strip_suffix(SCRIPT_SUFFIX)
        .unwrap_or(stem)
        .trim_end_matches('_');
    (!stem.is_empty()
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
    .then(|| stem.to_string())
}

impl Importers {
    pub fn read(&self, id: &str, name: &str) -> usize {
        self.reads
            .get(&(id.to_string(), name.to_string()))
            .copied()
            .unwrap_or_default()
    }

    pub fn name(&self, id: &str) -> Option<&str> {
        unique_top(self.names.get(id))
    }

    pub fn path(&self, id: &str) -> Option<&str> {
        unique_top(self.paths.get(id))
    }
}

fn required(expr: &Expression<'_>) -> Option<String> {
    let (callee, arguments) = call_parts(expr.without_parentheses())?;
    if !matches!(callee, Expression::Identifier(id) if id.name == REQUIRE) || arguments.len() != 1 {
        return None;
    }
    match argument_expression(&arguments[0])? {
        Expression::NumericLiteral(literal) => Some(number_key(literal.value)),
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        _ => None,
    }
}

struct Scan<'i> {
    found: &'i mut Importers,
    bound: FxHashMap<String, String>,
}

impl Scan<'_> {
    fn vote(&mut self, target: String, name: &str) {
        if let Some(stem) = webpack_import_stem(name) {
            *self
                .found
                .paths
                .entry(target)
                .or_default()
                .entry(stem)
                .or_default() += 1;
            return;
        }
        if meaningful(name) {
            *self
                .found
                .names
                .entry(target)
                .or_default()
                .entry(name.to_string())
                .or_default() += 1;
        }
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (Some(id), Some(target)) = (
            it.id.get_binding_identifier(),
            it.init.as_ref().and_then(required),
        ) {
            self.bound.insert(id.name.to_string(), target.clone());
            self.vote(target, id.name.as_str());
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(target) = required(&it.right) {
            let name = match &it.left {
                AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.to_string()),
                other => other
                    .as_member_expression()
                    .and_then(|member| member.static_property_name())
                    .and_then(|name| name.as_str())
                    .map(str::to_string),
            };
            if let Some(name) = name {
                self.vote(target, &name);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if let (Some(target), Some(key)) = (required(&it.value), prop_name(&it.key)) {
            self.vote(target, &key);
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let target = match &it.object {
            Expression::Identifier(id) => self.bound.get(id.name.as_str()).cloned(),
            other => required(other),
        };
        if let Some(target) = target {
            *self
                .found
                .reads
                .entry((target, it.property.name.to_string()))
                .or_default() += 1;
        }
        walk_js::walk_static_member_expression(self, it);
    }
}

pub fn scan(bundle: &Bundle<'_>) -> Importers {
    let mut found = Importers::default();
    for module in bundle.modules.values() {
        let mut scan = Scan {
            found: &mut found,
            bound: FxHashMap::default(),
        };
        scan.visit_program(module.program());
    }
    found.names.retain(|id, _| bundle.modules.contains_key(id));
    found.paths.retain(|id, _| bundle.modules.contains_key(id));
    found
}
