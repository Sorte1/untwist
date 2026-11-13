use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::esbuild::layout::Layout;

type Time = (usize, usize);

const NEVER: Time = (usize::MAX, usize::MAX);

#[derive(Clone, Copy)]
enum Site {
    At(Time),
    Inside(usize),
}

struct Sites<'s> {
    scoping: &'s Scoping,
    factories: FxHashMap<SymbolId, usize>,
    site: Site,
    found: Vec<(usize, Site)>,
}

impl<'a> VisitJs<'a> for Sites<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        if let Some(&factory) = symbol.and_then(|symbol| self.factories.get(&symbol)) {
            self.found.push((factory, self.site));
        }
    }
}

fn hoisted(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) => true,
        Statement::ExportDeclaration(export) => {
            matches!(export.declaration, Declaration::FunctionDeclaration(_))
        }
        Statement::ExportDefaultDeclaration(export) => matches!(
            export.declaration,
            ExportDefaultDeclarationKind::FunctionDeclaration(_)
        ),
        _ => false,
    }
}

fn sites(scoping: &Scoping, list: &[Statement<'_>], layout: &Layout<'_, '_>) -> Vec<(usize, Site)> {
    let factories = layout
        .factories
        .iter()
        .enumerate()
        .filter_map(|(index, factory)| Some((factory.binding?.symbol_id.get()?, index)))
        .collect();
    let mut visitor = Sites {
        scoping,
        factories,
        site: Site::At((0, 0)),
        found: Vec::new(),
    };
    let mut next = 0;
    for (index, statement) in list.iter().enumerate() {
        let here = |at: usize| match hoisted(statement) {
            true => Site::At((0, 0)),
            false => Site::At((index, at)),
        };
        let starts = next;
        while next < layout.factories.len() && layout.factories[next].statement == index {
            next += 1;
        }
        let inside = &layout.factories[starts..next];
        let Statement::VariableDeclaration(declaration) = statement else {
            visitor.site = here(0);
            visitor.visit_statement(statement);
            continue;
        };
        let mut offset = 0;
        for (at, declarator) in declaration.declarations.iter().enumerate() {
            visitor.site = here(at);
            match inside
                .get(offset)
                .filter(|factory| factory.declarator == at)
            {
                Some(factory) => {
                    visitor.site = Site::Inside(starts + offset);
                    walk_js::walk_function_body(&mut visitor, factory.function.body);
                    offset += 1;
                }
                None => visitor.visit_variable_declarator(declarator),
            }
        }
    }
    visitor.found
}

pub fn first_call_follows_definition(
    scoping: &Scoping,
    list: &[Statement<'_>],
    layout: &Layout<'_, '_>,
) -> bool {
    let found = sites(scoping, list, layout);
    let mut earliest = vec![NEVER; layout.factories.len()];
    let mut changed = true;
    while changed {
        changed = false;
        for &(factory, site) in &found {
            let time = match site {
                Site::At(time) => time.max(layout.first_effect),
                Site::Inside(caller) => earliest[caller],
            };
            if time < earliest[factory] {
                earliest[factory] = time;
                changed = true;
            }
        }
    }
    layout
        .factories
        .iter()
        .zip(earliest)
        .all(|(factory, time)| time == NEVER || time > (factory.statement, factory.declarator))
}
