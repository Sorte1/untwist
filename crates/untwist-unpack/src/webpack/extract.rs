use indexmap::IndexMap;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_span::GetSpan;

use crate::matcher::param_names;
use crate::model::{module_from_body_with, Bodies, Module, RequireParam};
use crate::rename::rename_factory_parameters;
use crate::webpack::container::module_factories;
use crate::UnpackOptions;

pub const WEBPACK_PARAMS: [&str; 3] = ["module", "exports", "require"];
const REQUIRE_PARAM: usize = 2;

const MODULE_MARKER_COMMENT: &str = "/***/";

pub fn extract_modules<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    container: &Expression<'a>,
    entry_id: Option<&str>,
) -> IndexMap<String, Module<'a>> {
    extract_modules_with(
        alloc,
        source,
        (container, entry_id),
        &mut Bodies::Cloned,
        UnpackOptions::default(),
    )
    .0
}

pub type Extracted<'a> = (IndexMap<String, Module<'a>>, Vec<(String, String)>);

fn require_param(params: usize, fresh: Option<String>) -> RequireParam {
    match (params > REQUIRE_PARAM, fresh) {
        (false, _) => RequireParam::Absent,
        (true, Some(_)) => RequireParam::Fresh,
        (true, None) => RequireParam::Default,
    }
}

pub fn extract_modules_with<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    (container, entry_id): (&Expression<'a>, Option<&str>),
    bodies: &mut Bodies<'a>,
    options: UnpackOptions,
) -> Extracted<'a> {
    let mut modules = IndexMap::new();
    let mut fresh_requires = Vec::new();
    for (id, factory) in module_factories(container) {
        let mut program = module_from_body_with(alloc, source, factory.body, bodies);
        if !options.conservative {
            program.directives = factory.body.directives.clone_in(alloc);
        }
        let params = param_names(factory.params);
        let fresh = rename_factory_parameters(
            alloc,
            &mut program,
            (&params, &WEBPACK_PARAMS),
            options.conservative,
        );
        let fresh = fresh
            .into_iter()
            .find_map(|(index, name)| (index == REQUIRE_PARAM).then_some(name));
        let is_entry = entry_id == Some(id.as_str());
        let mut module = Module::new(id.clone(), program, is_entry);
        module.runtime_require = require_param(params.len(), fresh.clone());
        if let Some(name) = fresh {
            fresh_requires.push((id.clone(), name));
        }
        modules.insert(id, module);
    }
    (modules, fresh_requires)
}

pub fn strip_trailing_marker_comment(program: &mut Program<'_>) {
    let last_statement_end = program.body.last().map_or(0, |stmt| stmt.span().end);
    let is_marker = program.comments.last().is_some_and(|comment| {
        comment.span.start >= last_statement_end
            && comment.span.source_text(program.source_text) == MODULE_MARKER_COMMENT
    });
    if is_marker {
        program.comments.pop();
    }
}
