use indexmap::IndexMap;
use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{SourceType, Span};
use rustc_hash::FxHashMap;

use crate::path::{module_id_path, PosixPath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleKind {
    Webpack,
    Browserify,
    Esbuild,
    Rollup,
    SystemJs,
    Amd,
    Umd,
    Parcel,
    Metro,
    Turbopack,
    Closure,
}

impl BundleKind {
    pub const ALL: [BundleKind; 11] = [
        Self::Webpack,
        Self::Browserify,
        Self::Esbuild,
        Self::Rollup,
        Self::SystemJs,
        Self::Amd,
        Self::Umd,
        Self::Parcel,
        Self::Metro,
        Self::Turbopack,
        Self::Closure,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Webpack => "webpack",
            Self::Browserify => "browserify",
            Self::Esbuild => "esbuild",
            Self::Rollup => "rollup",
            Self::SystemJs => "systemjs",
            Self::Amd => "amd",
            Self::Umd => "umd",
            Self::Parcel => "parcel",
            Self::Metro => "metro",
            Self::Turbopack => "turbopack",
            Self::Closure => "closure",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RequireParam {
    #[default]
    Default,
    Fresh,
    Absent,
}

pub struct Module<'a> {
    pub id: String,
    pub path: PosixPath,
    pub is_entry: bool,
    pub dependencies: Vec<(String, String)>,
    pub unsafe_path_source: Option<String>,
    pub runtime_require: RequireParam,
    program: Program<'a>,
}

impl<'a> Module<'a> {
    pub fn new(id: String, program: Program<'a>, is_entry: bool) -> Self {
        let mut module = Self {
            id,
            path: PosixPath::default(),
            is_entry,
            dependencies: Vec::new(),
            unsafe_path_source: None,
            runtime_require: RequireParam::Default,
            program,
        };
        module.assign_default_path(is_entry);
        module
    }

    pub fn assign_default_path(&mut self, is_entry: bool) {
        let derived = module_id_path(&self.id, is_entry);
        let (path, unsafe_path_source) = match PosixPath::parse(&derived) {
            Some(path) => (path, None),
            None => (fallback_path(&self.id), Some(derived)),
        };
        self.path = path;
        self.unsafe_path_source = unsafe_path_source;
    }

    pub fn program(&self) -> &Program<'a> {
        &self.program
    }

    pub fn program_mut(&mut self) -> &mut Program<'a> {
        &mut self.program
    }

    pub fn is_esm(&self) -> bool {
        self.program.source_type.is_module()
    }
}

pub fn fallback_path(id: &str) -> PosixPath {
    let sanitized: String = id
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    PosixPath::parse(&format!("_unsafe_{sanitized}.js"))
        .unwrap_or_else(|| PosixPath::parse("_unsafe.js").unwrap_or_default())
}

pub struct Helper<'a> {
    pub name: String,
    pub statement: Statement<'a>,
    pub movable: bool,
}

pub struct Runtime<'a> {
    pub require_name: String,
    pub helpers: Vec<Helper<'a>>,
}

pub struct Bundle<'a> {
    pub kind: BundleKind,
    pub entry_id: Option<String>,
    pub entry_candidates: usize,
    pub chunk_ids: Vec<String>,
    pub modules: IndexMap<String, Module<'a>>,
    pub runtime: Option<Runtime<'a>>,
    pub fresh_requires: Vec<(String, String)>,
    pub link: Option<String>,
    pub fixed_paths: Vec<String>,
}

impl<'a> Bundle<'a> {
    pub fn new(kind: BundleKind, entry_id: Option<String>) -> Self {
        Self {
            kind,
            entry_id,
            entry_candidates: 0,
            chunk_ids: Vec::new(),
            modules: IndexMap::new(),
            runtime: None,
            fresh_requires: Vec::new(),
            link: None,
            fixed_paths: Vec::new(),
        }
    }

    pub fn entry(&self) -> Option<&Module<'a>> {
        let id = self.entry_id.as_deref()?;
        self.modules.get(id)
    }

    pub fn path_of(&self, id: &str) -> Option<&PosixPath> {
        self.modules.get(id).map(|module| &module.path)
    }
}

pub type BodyKey = (u32, u32);

pub enum Bodies<'a> {
    Cloned,
    Planned(Vec<BodyKey>),
    Taken(FxHashMap<BodyKey, AVec<'a, Statement<'a>>>),
}

pub fn body_key(body: &FunctionBody<'_>) -> BodyKey {
    (body.span.start, body.span.end)
}

fn cloned<'a>(alloc: &'a Allocator, body: &FunctionBody<'a>) -> AVec<'a, Statement<'a>> {
    AVec::from_iter_in(
        body.statements.iter().map(|stmt| stmt.clone_in(alloc)),
        &alloc,
    )
}

impl<'a> Bodies<'a> {
    pub(crate) fn planned_len(&self) -> usize {
        match self {
            Bodies::Planned(keys) => keys.len(),
            Bodies::Cloned | Bodies::Taken(_) => 0,
        }
    }

    pub(crate) fn forget_planned_after(&mut self, len: usize) {
        if let Bodies::Planned(keys) = self {
            keys.truncate(len);
        }
    }

    pub(crate) fn statements(
        &mut self,
        alloc: &'a Allocator,
        body: &FunctionBody<'a>,
    ) -> AVec<'a, Statement<'a>> {
        match self {
            Bodies::Cloned => cloned(alloc, body),
            Bodies::Planned(keys) => {
                keys.push(body_key(body));
                AVec::new_in(&alloc)
            }
            Bodies::Taken(pool) => pool
                .remove(&body_key(body))
                .unwrap_or_else(|| cloned(alloc, body)),
        }
    }
}

fn comments_within(comments: &[Comment], span: Span) -> impl Iterator<Item = Comment> + '_ {
    let first = comments.partition_point(|comment| comment.span.start < span.start);
    comments[first..]
        .iter()
        .take_while(move |comment| comment.span.end <= span.end)
        .cloned()
}

pub fn module_from_body<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    body: &FunctionBody<'a>,
) -> Program<'a> {
    module_from_body_with(alloc, source, body, &mut Bodies::Cloned)
}

pub fn module_from_body_with<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    body: &FunctionBody<'a>,
    bodies: &mut Bodies<'a>,
) -> Program<'a> {
    let builder = AstBuilder::new(alloc);
    let statements = bodies.statements(alloc, body);
    let comments = match source.comments.is_empty() {
        true => AVec::new_in(&builder),
        false => AVec::from_iter_in(comments_within(&source.comments, body.span), &builder),
    };
    Program::new(
        body.span,
        SourceType::cjs(),
        source.source_text,
        comments,
        None,
        AVec::new_in(&builder),
        statements,
        &builder,
    )
}
