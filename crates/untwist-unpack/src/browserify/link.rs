use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::{Ident, Str};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;
use untwist_core::pass::{Note, NoteKind};
use untwist_core::semantic::build_scoping;

use crate::browserify::notes;
use crate::model::{Bundle, Module, RequireParam};
use crate::runtime::{js_string, synthetic_statements};
use crate::scan::is_free;
use crate::specifier::ModulePaths;

const REQUIRE: &str = "require";
const FALSY_IDS: [&str; 2] = ["", "0"];

type Specifiers = FxHashMap<String, Option<String>>;

fn specifiers(module: &Module<'_>, paths: &ModulePaths) -> Specifiers {
    let from = module.path.as_str();
    let mut specifiers = Specifiers::default();
    for (id, request) in &module.dependencies {
        if FALSY_IDS.contains(&id.as_str()) {
            specifiers.remove(request);
        } else {
            let linked = paths.get(id).map(|path| paths.specifier(from, path));
            specifiers.insert(request.clone(), linked);
        }
    }
    specifiers
}

fn binds(scoping: &Scoping, name: &str) -> bool {
    scoping
        .symbol_ids()
        .any(|symbol| scoping.symbol_name(symbol) == name)
}

struct Requires<'a, 'r> {
    builder: AstBuilder<'a>,
    scoping: &'r Scoping,
    name: &'r str,
    specifiers: &'r Specifiers,
    paths: &'r ModulePaths,
    unmapped: Vec<String>,
}

impl<'a> Requires<'a, '_> {
    fn is_the_bundle_require(&self, callee: &IdentifierReference<'a>) -> bool {
        callee.name == self.name && is_free(Some(self.scoping), callee)
    }

    fn link(&mut self, it: &mut CallExpression<'a>) {
        let Expression::Identifier(callee) = &mut it.callee else {
            return;
        };
        if !self.is_the_bundle_require(callee) {
            return;
        }
        let Some(argument) = it.arguments.first_mut() else {
            return;
        };
        let Argument::StringLiteral(literal) = &*argument else {
            return;
        };
        let Some(request) = literal.value.as_str() else {
            return;
        };
        match self.specifiers.get(request) {
            Some(Some(specifier)) => {
                let span = argument.span();
                let text = Str::from_str_in(specifier, &self.builder);
                *argument = Argument::from(Expression::new_string_literal(
                    span,
                    text,
                    None,
                    &self.builder,
                ));
                callee.name = Ident::from_str_in(REQUIRE, &self.builder);
            }
            Some(None) => {}
            None if self.paths.contains_key(request) => self.unmapped.push(request.to_string()),
            None => {}
        }
    }
}

impl<'a> VisitMut<'a> for Requires<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        self.link(it);
        walk_mut::walk_call_expression(self, it);
    }
}

struct Linked {
    unmapped: Vec<String>,
    unbound: bool,
}

fn link_module<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    paths: &ModulePaths,
    name: &str,
) -> Linked {
    let scoping = build_scoping(module.program()).scoping;
    if name != REQUIRE && binds(&scoping, REQUIRE) {
        return Linked {
            unmapped: Vec::new(),
            unbound: true,
        };
    }
    let specifiers = specifiers(module, paths);
    let mut requires = Requires {
        builder: AstBuilder::new(alloc),
        scoping: &scoping,
        name,
        specifiers: &specifiers,
        paths,
        unmapped: Vec::new(),
    };
    requires.visit_program(module.program_mut());
    let unmapped = requires.unmapped;
    let unbound = name != REQUIRE && still_uses(module.program(), name);
    Linked { unmapped, unbound }
}

struct NameUse<'n> {
    name: &'n str,
    found: bool,
}

impl<'a> VisitJs<'a> for NameUse<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == self.name;
    }
}

fn still_uses(program: &Program<'_>, name: &str) -> bool {
    let mut uses = NameUse { name, found: false };
    uses.visit_program(program);
    uses.found
}

#[derive(Default)]
struct Returns {
    found: bool,
}

impl<'a> VisitJs<'a> for Returns {
    fn visit_return_statement(&mut self, _it: &ReturnStatement<'a>) {
        self.found = true;
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
}

fn runs_to_its_end(program: &Program<'_>) -> bool {
    let mut returns = Returns::default();
    returns.visit_program(program);
    !returns.found
}

fn later_entries(bundle: &Bundle<'_>) -> Vec<Option<String>> {
    let later = &bundle.later_entries;
    later
        .iter()
        .enumerate()
        .filter(|(at, id)| id.is_none() || (**id != bundle.entry_id && !later[..*at].contains(id)))
        .map(|(_, id)| id.clone())
        .collect()
}

fn entry_tail(
    bundle: &Bundle<'_>,
    paths: &ModulePaths,
    later: &[Option<String>],
) -> Option<String> {
    let entry = bundle.entry()?;
    let from = entry.path.as_str();
    let scoping = build_scoping(entry.program()).scoping;
    if binds(&scoping, REQUIRE) || !runs_to_its_end(entry.program()) {
        return None;
    }
    later
        .iter()
        .map(|id| {
            let path = paths.get(id.as_deref()?)?;
            Some(format!(
                "require({});\n",
                js_string(&paths.specifier(from, path))
            ))
        })
        .collect()
}

fn run_later_entries<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    paths: &ModulePaths,
) -> Option<Note> {
    let later = later_entries(bundle);
    if later.is_empty() {
        return None;
    }
    let statements =
        entry_tail(bundle, paths, &later).and_then(|text| synthetic_statements(alloc, text));
    let entry = bundle
        .entry_id
        .clone()
        .and_then(|id| bundle.modules.get_mut(&id));
    match (statements, entry) {
        (Some(statements), Some(entry)) => {
            entry.program_mut().body.extend(statements);
            None
        }
        _ => Some(note(notes::later_entries_not_run(later.len()))),
    }
}

fn note(message: String) -> Note {
    Note {
        pass: "unpack",
        kind: NoteKind::Refusal,
        message,
        span: None,
    }
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    let paths = ModulePaths::of(bundle);
    let fresh: FxHashMap<String, String> = std::mem::take(&mut bundle.fresh_requires)
        .into_iter()
        .collect();
    let mut unmapped: Vec<(String, String)> = Vec::new();
    let mut unbound: Vec<String> = Vec::new();
    let takes_the_bundle_require =
        |module: &Module<'a>| module.runtime_require != RequireParam::Absent;
    for (id, module) in bundle.modules.iter_mut() {
        if !takes_the_bundle_require(module) {
            continue;
        }
        let name = fresh.get(id).map_or(REQUIRE, String::as_str);
        let linked = link_module(alloc, module, &paths, name);
        unmapped.extend(
            linked
                .unmapped
                .into_iter()
                .map(|request| (id.clone(), request)),
        );
        if linked.unbound {
            unbound.push(id.clone());
        }
    }
    let unmapped_note = unmapped
        .first()
        .map(|(id, request)| note(notes::unmapped_module_id(id, request, unmapped.len())));
    let unbound_note = unbound
        .first()
        .map(|id| note(notes::require_left_unbound(id, unbound.len())));
    unmapped_note
        .into_iter()
        .chain(unbound_note)
        .chain(run_later_entries(alloc, bundle, &paths))
        .collect()
}
