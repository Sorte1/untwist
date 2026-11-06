use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::member_parts;
use untwist_core::pass::{Note, NoteKind};

use crate::model::Bundle;
use crate::path::PosixPath;

const PASS_NAME: &str = "unpack";
const NODE_MODULES: &str = "node_modules";

#[derive(Clone, Debug, PartialEq)]
pub enum MappingMatcher {
    StringLiteral(String),
    NumericLiteral(f64),
    Identifier(String),
    Member { object: String, property: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mapping {
    pub path: String,
    pub matcher: MappingMatcher,
}

impl MappingMatcher {
    fn matches(&self, expr: &Expression<'_>) -> bool {
        match self {
            Self::StringLiteral(value) => matches!(
                expr,
                Expression::StringLiteral(literal)
                    if literal.value == value.as_str()
            ),
            Self::NumericLiteral(value) => {
                matches!(expr, Expression::NumericLiteral(literal) if literal.value == *value)
            }
            Self::Identifier(name) => {
                matches!(expr, Expression::Identifier(id) if id.name.as_str() == name)
            }
            Self::Member { object, property } => match member_parts(expr) {
                Some((target, name)) => {
                    name == property.as_str()
                        && matches!(target, Expression::Identifier(id) if id.name.as_str() == object)
                }
                None => false,
            },
        }
    }
}

fn note(message: String) -> Note {
    Note {
        pass: PASS_NAME,
        kind: NoteKind::Refusal,
        message,
        span: None,
    }
}

fn resolve_mapping_path(key: &str) -> Option<PosixPath> {
    if key.starts_with('.') {
        PosixPath::parse(key)
    } else {
        PosixPath::parse(&format!("{NODE_MODULES}/{key}"))
    }
}

struct Match<'m> {
    mappings: &'m [Mapping],
    used: &'m FxHashSet<usize>,
    found: Option<usize>,
}

impl<'a> VisitJs<'a> for Match<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.found.is_some() {
            return;
        }
        for (index, mapping) in self.mappings.iter().enumerate() {
            if !self.used.contains(&index) && mapping.matcher.matches(it) {
                self.found = Some(index);
                return;
            }
        }
        walk_js::walk_expression(self, it);
    }
}

pub fn apply_mappings(bundle: &mut Bundle<'_>, mappings: &[Mapping]) -> Vec<Note> {
    let mut notes = Vec::new();
    if mappings.is_empty() {
        return notes;
    }
    let mut used: FxHashSet<usize> = FxHashSet::default();
    for module in bundle.modules.values_mut() {
        let mut matcher = Match {
            mappings,
            used: &used,
            found: None,
        };
        matcher.visit_program(module.program());
        let Some(index) = matcher.found else {
            continue;
        };
        used.insert(index);
        match resolve_mapping_path(&mappings[index].path) {
            Some(path) => {
                module.path = path;
                module.unsafe_path_source = None;
            }
            None => notes.push(note(crate::notes::mapping_unsafe_path(
                &mappings[index].path,
            ))),
        }
    }
    for (index, mapping) in mappings.iter().enumerate() {
        if !used.contains(&index) {
            notes.push(note(crate::notes::mapping_unmatched(&mapping.path)));
        }
    }
    notes
}
