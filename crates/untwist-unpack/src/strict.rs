use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use untwist_core::ast_util::respan::SYNTHETIC_BASE;

use crate::model::{Bundle, Module};

struct SloppyFunctions<'t> {
    text: &'t str,
    strict: bool,
    found: Option<String>,
}

impl SloppyFunctions<'_> {
    fn line(&self, span: Span) -> usize {
        let end = (span.start as usize).min(self.text.len());
        let before = self.text.get(..end).unwrap_or_default();
        before.bytes().filter(|byte| *byte == b'\n').count() + 1
    }

    fn describe(&self, name: Option<&str>, span: Span) -> String {
        let line = self.line(span);
        match name {
            Some(name) => format!("the function {name} on line {line}"),
            None => format!("a function on line {line}"),
        }
    }

    fn inside(&mut self, strict: bool, walk: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.strict, strict);
        walk(self);
        self.strict = outer;
    }
}

impl<'a> VisitJs<'a> for SloppyFunctions<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.found.is_none() {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.found.is_none() {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.found.is_some() || it.body.is_none() {
            return;
        }
        let strict = self.strict || it.has_use_strict_directive();
        if !strict {
            let name = it.id.as_ref().map(|id| id.name.as_str());
            self.found = Some(self.describe(name, it.span));
            return;
        }
        self.inside(true, |this| walk_js::walk_function(this, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if self.found.is_some() {
            return;
        }
        let strict = self.strict || it.has_use_strict_directive();
        if !strict {
            self.found = Some(self.describe(None, it.span));
            return;
        }
        self.inside(true, |this| {
            walk_js::walk_arrow_function_expression(this, it)
        });
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if self.found.is_none() {
            self.inside(true, |this| walk_js::walk_class(this, it));
        }
    }
}

pub fn first_sloppy_function(program: &Program<'_>) -> Option<String> {
    let mut sloppy = SloppyFunctions {
        text: program.source_text,
        strict: program.source_type.is_module() || program.has_use_strict_directive(),
        found: None,
    };
    sloppy.visit_program(program);
    sloppy.found
}

struct StrictRegions {
    regions: Vec<Region>,
}

impl StrictRegions {
    fn own(&mut self, span: Span) {
        self.regions.push(Region { span, whole: false });
    }
}

impl<'a> VisitJs<'a> for StrictRegions {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        match it.has_use_strict_directive() {
            true => self.own(it.span),
            false => walk_js::walk_function(self, it, flags),
        }
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        match it.has_use_strict_directive() {
            true => self.own(it.span),
            false => walk_js::walk_arrow_function_expression(self, it),
        }
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.own(it.span);
    }
}

fn wholly_strict(program: &Program<'_>) -> bool {
    program.source_type.is_module() || program.has_use_strict_directive()
}

fn strict_regions(program: &Program<'_>) -> Vec<Region> {
    if wholly_strict(program) {
        return vec![Region {
            span: program.span,
            whole: true,
        }];
    }
    let mut found = StrictRegions {
        regions: Vec::new(),
    };
    found.visit_program(program);
    found.regions
}

#[derive(Clone, Copy)]
struct Region {
    span: Span,
    whole: bool,
}

impl Region {
    fn encloses(self, span: Span) -> bool {
        self.span.contains_inclusive(span) && (self.whole || self.span != span)
    }
}

fn inside(module: &Module<'_>, regions: &[Region]) -> bool {
    let mut spans = module
        .program()
        .body
        .iter()
        .map(GetSpan::span)
        .filter(|span| !span.is_empty() && span.start < SYNTHETIC_BASE)
        .peekable();
    spans.peek().is_some() && spans.all(|span| regions.iter().any(|region| region.encloses(span)))
}

pub fn mark_strict_sources(program: &Program<'_>, bundle: &mut Bundle<'_>) {
    if wholly_strict(program) {
        for module in bundle.modules.values_mut() {
            module.strict_source = true;
        }
        return;
    }
    let regions = strict_regions(program);
    if regions.is_empty() {
        return;
    }
    for module in bundle.modules.values_mut() {
        module.strict_source |= inside(module, &regions);
    }
}

pub fn mark_from_inputs(programs: &[Program<'_>], bundle: &mut Bundle<'_>) {
    let mut inputs = bundle
        .inputs
        .iter()
        .filter_map(|input| programs.get(*input));
    let held = match bundle.inputs.is_empty() {
        true => programs.iter().all(wholly_strict),
        false => inputs.all(wholly_strict),
    };
    if held && !programs.is_empty() {
        for module in bundle.modules.values_mut() {
            module.strict_source = true;
        }
        return;
    }
    let regions: Vec<(usize, Vec<Region>)> = programs
        .iter()
        .map(|program| {
            (
                program.source_text.as_ptr() as usize,
                strict_regions(program),
            )
        })
        .filter(|(_, regions)| !regions.is_empty())
        .collect();
    for module in bundle.modules.values_mut() {
        let text = module.program().source_text.as_ptr() as usize;
        let found = regions.iter().find(|(input, _)| *input == text);
        if let Some((_, regions)) = found {
            module.strict_source |= inside(module, regions);
        }
    }
}

pub fn written_as_module(module: &Module<'_>) -> bool {
    let path = module.path.as_str();
    path.ends_with(".mjs")
        || (!path.ends_with(".cjs")
            && module
                .program()
                .body
                .iter()
                .any(Statement::is_module_declaration))
}

fn written_strict(module: &Module<'_>) -> bool {
    module.program().has_use_strict_directive()
        || module.path.as_str().ends_with(".json")
        || written_as_module(module)
}

pub fn written_as_strict(module: &Module<'_>) -> bool {
    module.strict_source || written_strict(module)
}

fn use_strict(alloc: &Allocator) -> Option<Directive<'_>> {
    let parsed = untwist_core::parse::parse_program(alloc, "\"use strict\";");
    parsed.program.directives.into_iter().next()
}

pub fn keep_strictness<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) {
    for module in bundle.modules.values_mut() {
        if !module.strict_source || written_strict(module) {
            continue;
        }
        if let Some(directive) = use_strict(alloc) {
            module.program_mut().directives.insert(0, directive);
        }
    }
}

pub fn sloppy_written_module(bundle: &Bundle<'_>) -> Option<String> {
    bundle
        .modules
        .iter()
        .find(|(_, module)| !written_strict(module))
        .map(|(id, _)| id.clone())
}
