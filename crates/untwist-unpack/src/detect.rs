use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_js_mut, VisitJs, VisitJsMut};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::browserify::detect::{detect_browserify_with, is_browserify};
use crate::frame::Frame;
use crate::links::Links;
use crate::model::{body_key, Bodies, BodyKey, Bundle, BundleKind};
use crate::webpack::chunk::{detect_webpack_chunk_with, is_webpack_chunk};
use crate::webpack::detect4::{detect_webpack4_with, is_webpack4};
use crate::webpack::detect5::{detect_webpack5_with, is_webpack5};
use crate::UnpackOptions;

trait Finder<'a> {
    fn found(&self) -> bool;
    fn statements(&mut self, body: &[Statement<'a>]);
    fn expression(&mut self, expr: &Expression<'a>);
}

struct Extract<'a, 'p, 'b> {
    alloc: &'a Allocator,
    source: &'p Program<'a>,
    bodies: &'b mut Bodies<'a>,
    options: UnpackOptions,
    bundle: Option<Bundle<'a>>,
}

impl<'a> Finder<'a> for Extract<'a, '_, '_> {
    fn found(&self) -> bool {
        self.bundle.is_some()
    }

    fn statements(&mut self, body: &[Statement<'a>]) {
        let (alloc, source, options) = (self.alloc, self.source, self.options);
        self.bundle = attempt(self.bodies, |bodies| {
            detect_webpack5_with(alloc, source, body, bodies, options)
        });
    }

    fn expression(&mut self, expr: &Expression<'a>) {
        let (alloc, source, options) = (self.alloc, self.source, self.options);
        let bodies = &mut *self.bodies;
        self.bundle = attempt(bodies, |bodies| {
            detect_webpack4_with(alloc, source, expr, bodies, options)
        })
        .or_else(|| {
            attempt(bodies, |bodies| {
                detect_webpack_chunk_with(alloc, source, expr, bodies, options)
            })
        })
        .or_else(|| {
            attempt(bodies, |bodies| {
                detect_browserify_with(alloc, source, expr, bodies, options)
            })
        });
    }
}

fn attempt<'a>(
    bodies: &mut Bodies<'a>,
    detect: impl FnOnce(&mut Bodies<'a>) -> Option<Bundle<'a>>,
) -> Option<Bundle<'a>> {
    let planned = bodies.planned_len();
    let bundle = detect(bodies);
    if bundle.is_none() {
        bodies.forget_planned_after(planned);
    }
    bundle
}

#[derive(Default)]
struct Probe {
    found: bool,
}

impl<'a> Finder<'a> for Probe {
    fn found(&self) -> bool {
        self.found
    }

    fn statements(&mut self, body: &[Statement<'a>]) {
        self.found = is_webpack5(body);
    }

    fn expression(&mut self, expr: &Expression<'a>) {
        self.found = is_webpack4(expr) || is_webpack_chunk(expr) || is_browserify(expr);
    }
}

struct Detector<F> {
    finder: F,
}

impl<F> Detector<F> {
    fn try_statements<'a>(&mut self, body: &[Statement<'a>]) -> bool
    where
        F: Finder<'a>,
    {
        if !self.finder.found() {
            self.finder.statements(body);
        }
        self.finder.found()
    }

    fn try_expression<'a>(&mut self, expr: &Expression<'a>) -> bool
    where
        F: Finder<'a>,
    {
        if !self.finder.found() {
            self.finder.expression(expr);
        }
        self.finder.found()
    }
}

impl<'a, F: Finder<'a>> VisitJs<'a> for Detector<F> {
    fn visit_program(&mut self, it: &Program<'a>) {
        if self.try_statements(&it.body) {
            return;
        }
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        if self.try_statements(&it.statements) {
            return;
        }
        walk_js::walk_function_body(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        if self.try_statements(&it.body) {
            return;
        }
        walk_js::walk_block_statement(self, it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.try_expression(it) {
            return;
        }
        walk_js::walk_expression(self, it);
    }
}

type KindProbe = fn(&Frame<'_, '_>) -> bool;
type KindDetect = for<'k, 'a> fn(
    &'a Allocator,
    &'k Program<'a>,
    &Frame<'k, 'a>,
    &mut Bodies<'a>,
    &Links,
) -> Option<Bundle<'a>>;
type Kind = (BundleKind, KindProbe, KindDetect);

macro_rules! unlinked {
    ($name:ident, $detect:path) => {
        fn $name<'k, 'a>(
            alloc: &'a Allocator,
            source: &'k Program<'a>,
            frame: &Frame<'k, 'a>,
            bodies: &mut Bodies<'a>,
            _links: &Links,
        ) -> Option<Bundle<'a>> {
            $detect(alloc, source, frame, bodies)
        }
    };
}

unlinked!(closure_detect, crate::closure::detect_with);
unlinked!(systemjs_detect, crate::systemjs::detect_with);
unlinked!(esbuild_detect, crate::esbuild::detect_with);
unlinked!(metro_detect, crate::metro::detect_with);
unlinked!(amd_detect, crate::amd::detect_with);
unlinked!(umd_detect, crate::umd::detect_with);
unlinked!(rollup_detect, crate::rollup::detect_with);

const KINDS: [Kind; 8] = [
    (BundleKind::Closure, crate::closure::probe, closure_detect),
    (
        BundleKind::SystemJs,
        crate::systemjs::probe,
        systemjs_detect,
    ),
    (BundleKind::Esbuild, crate::esbuild::probe, esbuild_detect),
    (BundleKind::Metro, crate::metro::probe, metro_detect),
    (
        BundleKind::Parcel,
        crate::parcel::probe,
        crate::parcel::detect_with,
    ),
    (
        BundleKind::Turbopack,
        crate::turbopack::probe,
        crate::turbopack::detect_with,
    ),
    (BundleKind::Amd, crate::amd::probe, amd_detect),
    (BundleKind::Umd, crate::umd::probe, umd_detect),
];

const HOISTED: Kind = (BundleKind::Rollup, crate::rollup::probe, rollup_detect);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Top,
    Nested,
}

fn kinds(options: UnpackOptions, scope: Scope) -> impl Iterator<Item = Kind> {
    let nested = scope == Scope::Nested;
    KINDS
        .into_iter()
        .filter(move |(kind, _, _)| !nested || !matches!(kind, BundleKind::Amd | BundleKind::Umd))
        .chain((options.split_hoisted && !nested).then_some(HOISTED))
}

fn extract_kinds<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
    bodies: &mut Bodies<'a>,
    (options, links, scope): (UnpackOptions, &Links, Scope),
) -> Option<Bundle<'a>> {
    let frame = Frame::of(program);
    kinds(options, scope).find_map(|(_, _, detect)| {
        attempt(bodies, |bodies| {
            detect(alloc, program, &frame, bodies, links)
        })
    })
}

fn extract_with<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
    bodies: &mut Bodies<'a>,
    (options, links, scope): (UnpackOptions, &Links, Scope),
) -> Option<Bundle<'a>> {
    if scope == Scope::Nested {
        return extract_kinds(alloc, program, bodies, (options, links, scope));
    }
    let mut detector = Detector {
        finder: Extract {
            alloc,
            source: program,
            bodies,
            options,
            bundle: None,
        },
    };
    detector.visit_program(program);
    let Extract { bodies, bundle, .. } = detector.finder;
    bundle.or_else(|| extract_kinds(alloc, program, bodies, (options, links, scope)))
}

fn cloned<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
    context: (UnpackOptions, &Links, Scope),
) -> Option<Bundle<'a>> {
    extract_with(alloc, program, &mut Bodies::Cloned, context)
}

fn refeedable(bundle: &Bundle<'_>) -> bool {
    let wrapper = matches!(
        bundle.kind,
        BundleKind::Umd | BundleKind::Amd | BundleKind::SystemJs
    );
    wrapper
        && bundle.modules.len() == 1
        && bundle.modules.values().all(|module| {
            module.dependencies.is_empty()
                && !module
                    .program()
                    .body
                    .iter()
                    .any(Statement::is_module_declaration)
        })
}

fn refeed<'a>(
    alloc: &'a Allocator,
    mut bundle: Bundle<'a>,
    options: UnpackOptions,
    taken: bool,
) -> Bundle<'a> {
    if !refeedable(&bundle) {
        return bundle;
    }
    let Some(module) = bundle.modules.values_mut().next() else {
        return bundle;
    };
    let none = Links::default();
    let context = (options, &none, Scope::Nested);
    let program = module.program_mut();
    let nested = if taken {
        taking(alloc, program, context)
    } else {
        cloned(alloc, program, context)
    };
    nested.unwrap_or(bundle)
}

pub fn unpack_program<'a>(alloc: &'a Allocator, program: &Program<'a>) -> Option<Bundle<'a>> {
    unpack_program_with(alloc, program, UnpackOptions::default())
}

pub fn unpack_program_with<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
    options: UnpackOptions,
) -> Option<Bundle<'a>> {
    let bundle = cloned(alloc, program, (options, &Links::default(), Scope::Top))?;
    Some(refeed(alloc, bundle, options, false))
}

pub fn refused_kind(program: &Program<'_>, options: UnpackOptions) -> Option<BundleKind> {
    let frame = Frame::of(program);
    kinds(options, Scope::Top)
        .find(|(_, probe, _)| probe(&frame))
        .map(|(kind, _, _)| kind)
}

struct Taker<'a> {
    alloc: &'a Allocator,
    wanted: FxHashSet<BodyKey>,
    taken: FxHashMap<BodyKey, AVec<'a, Statement<'a>>>,
}

impl<'a> VisitJsMut<'a> for Taker<'a> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if !self.wanted.is_empty() {
            walk_js_mut::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if !self.wanted.is_empty() {
            walk_js_mut::walk_expression(self, it);
        }
    }

    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        let key = body_key(it);
        if !self.wanted.remove(&key) {
            walk_js_mut::walk_function_body(self, it);
            return;
        }
        let statements = std::mem::replace(&mut it.statements, AVec::new_in(&self.alloc));
        self.taken.insert(key, statements);
    }
}

struct Restorer<'a> {
    pool: FxHashMap<BodyKey, AVec<'a, Statement<'a>>>,
}

impl<'a> VisitJsMut<'a> for Restorer<'a> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if !self.pool.is_empty() {
            walk_js_mut::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if !self.pool.is_empty() {
            walk_js_mut::walk_expression(self, it);
        }
    }

    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        match self.pool.remove(&body_key(it)) {
            Some(statements) => it.statements = statements,
            None => walk_js_mut::walk_function_body(self, it),
        }
    }
}

pub fn unpack_program_taking<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
) -> Option<Bundle<'a>> {
    unpack_program_taking_with(alloc, program, UnpackOptions::default())
}

pub fn unpack_program_taking_with<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    options: UnpackOptions,
) -> Option<Bundle<'a>> {
    unpack_program_linked(alloc, program, options, &Links::default())
}

pub fn unpack_program_linked<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    options: UnpackOptions,
    links: &Links,
) -> Option<Bundle<'a>> {
    let bundle = taking(alloc, program, (options, links, Scope::Top))?;
    Some(refeed(alloc, bundle, options, true))
}

fn taking<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    context: (UnpackOptions, &Links, Scope),
) -> Option<Bundle<'a>> {
    let mut planned = Bodies::Planned(Vec::new());
    extract_with(alloc, program, &mut planned, context)?;
    let Bodies::Planned(keys) = planned else {
        return None;
    };
    let mut taker = Taker {
        alloc,
        wanted: keys.into_iter().collect(),
        taken: FxHashMap::default(),
    };
    taker.visit_program(program);
    let held = taker.taken.len();
    let mut taken = Bodies::Taken(taker.taken);
    if let Some(bundle) = extract_with(alloc, program, &mut taken, context) {
        return Some(bundle);
    }
    let Bodies::Taken(pool) = taken else {
        return None;
    };
    if pool.len() != held {
        return None;
    }
    let mut restorer = Restorer { pool };
    restorer.visit_program(program);
    cloned(alloc, program, context)
}

pub fn holds_a_bundle(program: &Program<'_>) -> bool {
    holds_a_bundle_with(program, UnpackOptions::default())
}

pub fn holds_a_bundle_with(program: &Program<'_>, options: UnpackOptions) -> bool {
    let frame = Frame::of(program);
    if kinds(options, Scope::Top).any(|(_, probe, _)| probe(&frame)) {
        return true;
    }
    let mut detector = Detector {
        finder: Probe::default(),
    };
    detector.visit_program(program);
    detector.finder.found
}

#[cfg(test)]
mod tests {
    use untwist_core::parse::parse_program;
    use untwist_core::print::pretty_codegen;

    use super::*;

    #[test]
    fn restoring_the_taken_bodies_gives_back_the_program() {
        let source = include_str!("../../../fixtures/reference/unpack/webpack-object.js");
        let alloc = Allocator::default();
        let mut parsed = parse_program(&alloc, source);
        let before = pretty_codegen(&parsed.program);
        let mut planned = Bodies::Planned(Vec::new());
        let none = Links::default();
        assert!(extract_with(
            &alloc,
            &parsed.program,
            &mut planned,
            (UnpackOptions::default(), &none, Scope::Top)
        )
        .is_some());
        let Bodies::Planned(keys) = planned else {
            panic!("the plan changed kind");
        };
        let mut taker = Taker {
            alloc: &alloc,
            wanted: keys.into_iter().collect(),
            taken: FxHashMap::default(),
        };
        taker.visit_program(&mut parsed.program);
        assert_ne!(pretty_codegen(&parsed.program), before);
        let mut restorer = Restorer { pool: taker.taken };
        restorer.visit_program(&mut parsed.program);
        assert!(restorer.pool.is_empty());
        assert_eq!(pretty_codegen(&parsed.program), before);
    }
}
