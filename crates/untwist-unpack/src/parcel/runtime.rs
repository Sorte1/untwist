use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;

use crate::canon::{function_shape, statement_shape};

const KNOWN: [&str; 9] = [
    include_str!("runtimes/dev.txt"),
    include_str!("runtimes/dev-unminified.txt"),
    include_str!("runtimes/dev-default.txt"),
    include_str!("runtimes/min.txt"),
    include_str!("runtimes/min-prepared.txt"),
    include_str!("runtimes/min-unminified.txt"),
    include_str!("runtimes/min-default.txt"),
    include_str!("runtimes/dev-unsafe.txt"),
    include_str!("runtimes/min-unsafe.txt"),
];

const KEY: &str = "%KEY%";
const KNOWN_DEFINITIONS: [&str; 5] = [
    include_str!("runtimes/hoisted.txt"),
    include_str!("runtimes/hoisted-default.txt"),
    include_str!("runtimes/hoisted-min.txt"),
    include_str!("runtimes/hoisted-min-prepared.txt"),
    include_str!("runtimes/hoisted-min-default.txt"),
];

const LEGACY: [&str; 6] = [
    include_str!("runtimes/legacy-dev.txt"),
    include_str!("runtimes/legacy-dev-default.txt"),
    include_str!("runtimes/legacy-dev-unsafe.txt"),
    include_str!("runtimes/legacy-min.txt"),
    include_str!("runtimes/legacy-min-default.txt"),
    include_str!("runtimes/legacy-min-unsafe.txt"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    Current,
    Legacy,
}

pub fn proven(function: &Function<'_>, source_text: &str) -> Option<Generation> {
    let text = function_shape(function, source_text)?;
    if KNOWN.contains(&text.as_str()) {
        return Some(Generation::Current);
    }
    LEGACY
        .contains(&text.as_str())
        .then_some(Generation::Legacy)
}

const STORES: [&str; 2] = ["$parcel$inits", "$parcel$modules"];

#[derive(Default)]
struct Stores {
    bound: usize,
    read: usize,
}

impl<'a> VisitJs<'a> for Stores {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.read += usize::from(STORES.contains(&it.name.as_str()));
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.bound += usize::from(STORES.contains(&it.name.as_str()));
    }
}

fn empty_object(declarator: &VariableDeclarator<'_>) -> bool {
    matches!(
        &declarator.init,
        Some(Expression::ObjectExpression(object)) if object.properties.is_empty()
    )
}

fn empty_stores<'k>(top: &'k [Statement<'_>]) -> Vec<&'k str> {
    let mut names: Vec<&str> = top
        .iter()
        .filter_map(|statement| match statement {
            Statement::VariableDeclaration(declaration) => Some(&declaration.declarations),
            _ => None,
        })
        .flatten()
        .filter(|declarator| empty_object(declarator))
        .filter_map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .filter(|name| STORES.contains(name))
        .collect();
    names.sort_unstable();
    names
}

fn stores_untouched(top: &[Statement<'_>], definition: usize) -> bool {
    let mut stores = Stores::default();
    for (index, statement) in top.iter().enumerate() {
        if index != definition {
            stores.visit_statement(statement);
        }
    }
    stores.read == 0 && stores.bound == STORES.len() && empty_stores(top) == STORES
}

pub fn defined(top: &[Statement<'_>], definition: usize, key: &str, source_text: &str) -> bool {
    let text = statement_shape(&top[definition], source_text);
    let known = KNOWN_DEFINITIONS
        .iter()
        .any(|known| known.replace(KEY, key) == text);
    known && stores_untouched(top, definition)
}
