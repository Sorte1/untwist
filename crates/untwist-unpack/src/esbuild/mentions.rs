use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use rustc_hash::{FxHashMap, FxHashSet};

struct Mentions<'w> {
    wanted: &'w FxHashMap<String, usize>,
    shared: Option<&'w FxHashMap<String, usize>>,
    found: FxHashSet<usize>,
    references: bool,
    bindings: bool,
}

impl Mentions<'_> {
    fn saw(&mut self, name: &str) {
        if let Some(&index) = self.wanted.get(name) {
            self.found.insert(index);
        } else if let Some(&index) = self.shared.and_then(|shared| shared.get(name)) {
            self.found.insert(index);
        }
    }
}

impl<'a> VisitJs<'a> for Mentions<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.references {
            self.saw(it.name.as_str());
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.bindings {
            self.saw(it.name.as_str());
        }
    }
}

pub fn mentioned<'k, 'a: 'k>(
    wanted: &FxHashMap<String, usize>,
    statements: impl IntoIterator<Item = &'k Statement<'a>>,
) -> FxHashSet<usize> {
    let mut mentions = Mentions {
        wanted,
        shared: None,
        found: FxHashSet::default(),
        references: true,
        bindings: false,
    };
    if !wanted.is_empty() {
        for statement in statements {
            mentions.visit_statement(statement);
        }
    }
    mentions.found
}

pub fn closure<'a>(
    list: &[Statement<'a>],
    candidates: &FxHashMap<String, usize>,
    seeds: FxHashSet<usize>,
) -> Vec<usize> {
    let mut moved = seeds;
    let mut pending: Vec<usize> = moved.iter().copied().collect();
    while let Some(index) = pending.pop() {
        let Some(statement) = list.get(index) else {
            continue;
        };
        for found in mentioned(candidates, std::iter::once(statement)) {
            if moved.insert(found) {
                pending.push(found);
            }
        }
    }
    let mut moved: Vec<usize> = moved.into_iter().collect();
    moved.sort_unstable();
    moved
}

fn scan_program(
    program: &Program<'_>,
    (wanted, shared): (&FxHashMap<String, usize>, Option<&FxHashMap<String, usize>>),
    references: bool,
) -> bool {
    let mut mentions = Mentions {
        wanted,
        shared,
        found: FxHashSet::default(),
        references,
        bindings: true,
    };
    if !wanted.is_empty() || shared.is_some_and(|shared| !shared.is_empty()) {
        mentions.visit_program(program);
    }
    !mentions.found.is_empty()
}

struct Prefixed<'p> {
    prefix: &'p str,
    found: FxHashSet<String>,
}

impl Prefixed<'_> {
    fn saw(&mut self, name: &str) {
        if name.starts_with(self.prefix) {
            self.found.insert(name.to_string());
        }
    }
}

impl<'a> VisitJs<'a> for Prefixed<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.saw(it.name.as_str());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.saw(it.name.as_str());
    }
}

pub fn fresh_name(program: &Program<'_>, base: &str) -> String {
    let mut prefixed = Prefixed {
        prefix: base,
        found: FxHashSet::default(),
    };
    prefixed.visit_program(program);
    (1..)
        .map(|index| match index {
            1 => base.to_string(),
            index => format!("{base}{index}"),
        })
        .find(|name| !prefixed.found.contains(name))
        .unwrap_or_default()
}

pub fn names_anywhere(program: &Program<'_>, name: &str) -> bool {
    scan_program(
        program,
        (&FxHashMap::from_iter([(name.to_string(), 0)]), None),
        true,
    )
}

pub fn binds_any<'n>(program: &Program<'_>, names: impl Iterator<Item = &'n str>) -> bool {
    binds_any_or(program, names, None)
}

pub fn binds_any_or<'n>(
    program: &Program<'_>,
    names: impl Iterator<Item = &'n str>,
    shared: Option<&FxHashMap<String, usize>>,
) -> bool {
    let wanted: FxHashMap<String, usize> = names.map(|name| (name.to_string(), 0)).collect();
    scan_program(program, (&wanted, shared), false)
}
