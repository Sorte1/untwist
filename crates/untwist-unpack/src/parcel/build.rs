use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, SourceType, Span};

use super::layout::{Def, Hoisted, Layout, Registry};
use super::links::ParcelView;
use super::{candidates, moved_helpers, ENTRY_ID, GLOBAL_PARAM, HOISTED_NAMES, REGISTRY_NAMES};
use crate::esbuild::mentions::names_anywhere;
use crate::matcher::{expression_statement, param_names};
use crate::model::{module_from_body_with, Bodies, Bundle, BundleKind, Module, Runtime};
use crate::rename::{rename_factory_parameters, rename_wrapper_bindings};
use crate::runtime::{js_string, runtime_module, synthetic_statements};

fn prepend<'a>(program: &mut Program<'a>, statements: AVec<'a, Statement<'a>>) {
    let tail: Vec<Statement<'a>> = program.body.drain(..).collect();
    program.body.extend(statements);
    program.body.extend(tail);
}

fn renamed<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    from: &[&str],
    to: &[&str],
) -> Vec<(usize, String)> {
    let to = &to[..from.len().min(to.len())];
    if from == to {
        return Vec::new();
    }
    rename_factory_parameters(alloc, program, (from, to), false)
}

fn global_prologue<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    fresh: &[(usize, String)],
) -> Option<()> {
    let at = REGISTRY_NAMES.len() - 2;
    let name = fresh
        .iter()
        .find(|(index, _)| *index == at)
        .map_or(GLOBAL_PARAM, |(_, name)| name.as_str());
    if names_anywhere(program, name) {
        prepend(
            program,
            synthetic_statements(alloc, format!("var {name} = globalThis;"))?,
        );
    }
    Some(())
}

fn registry_module<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    def: &Def<'_, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<(Program<'a>, String)> {
    let body = def.body()?;
    let mut program = module_from_body_with(alloc, source, body, bodies);
    let params = param_names(&def.function.params);
    let require = params
        .first()
        .map_or_else(String::new, |name| name.to_string());
    if matches!(bodies, Bodies::Planned(_)) {
        return Some((program, require));
    }
    program.directives = body.directives.clone_in(alloc);
    if params.len() > 1 {
        let fresh = renamed(alloc, &mut program, &params[1..], &REGISTRY_NAMES[1..]);
        if params.len() == REGISTRY_NAMES.len() {
            global_prologue(alloc, &mut program, &fresh)?;
        }
    }
    rename_wrapper_bindings(alloc, &mut program, &params);
    Some((program, require))
}

fn synthetic_entry<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Registry<'_, '_>,
) -> Option<Module<'a>> {
    let mut text: String = layout
        .entries
        .iter()
        .map(|id| format!("require({});\n", js_string(id)))
        .collect();
    if let Some(main) = &layout.main {
        text.push_str(&format!("module.exports = require({});\n", js_string(main)));
    }
    let program = plain_program(
        alloc,
        source,
        Span::default(),
        synthetic_statements(alloc, text)?,
    );
    Some(Module::new(ENTRY_ID.to_string(), program, true))
}

fn plain_program<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    span: Span,
    body: AVec<'a, Statement<'a>>,
) -> Program<'a> {
    let builder = AstBuilder::new(alloc);
    Program::new(
        span,
        SourceType::cjs(),
        source.source_text,
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        body,
        &builder,
    )
}

fn registry<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Registry<'_, 'a>,
    bodies: &mut Bodies<'a>,
    chunk: bool,
) -> Option<Bundle<'a>> {
    let direct = (layout.entries.len() == 1 && layout.main.as_ref() == layout.entries.first())
        .then(|| layout.entries[0].clone())
        .filter(|_| !chunk);
    let entry = (!chunk).then(|| direct.clone().unwrap_or_else(|| ENTRY_ID.to_string()));
    let mut bundle = Bundle::new(BundleKind::Parcel, entry);
    for def in &layout.defs {
        let (program, require) = registry_module(alloc, source, def, bodies)?;
        let mut module = Module::new(
            def.id.clone(),
            program,
            direct.as_deref() == Some(def.id.as_str()),
        );
        module.dependencies = def.deps.clone();
        bundle.fresh_requires.push((def.id.clone(), require));
        bundle.modules.insert(def.id.clone(), module);
    }
    if direct.is_none() && !chunk && !matches!(bodies, Bodies::Planned(_)) {
        bundle.modules.insert(
            ENTRY_ID.to_string(),
            synthetic_entry(alloc, source, layout)?,
        );
        bundle
            .fresh_requires
            .push((ENTRY_ID.to_string(), "require".to_string()));
    }
    Some(bundle)
}

fn kept_sequence<'a>(
    alloc: &'a Allocator,
    statement: &Statement<'a>,
    removed: &[Option<usize>],
) -> Option<Statement<'a>> {
    let Some(Expression::SequenceExpression(sequence)) = expression_statement(statement) else {
        return None;
    };
    let mut kept: Vec<Expression<'a>> = sequence
        .expressions
        .iter()
        .enumerate()
        .filter(|(index, _)| removed.binary_search(&Some(*index)).is_err())
        .map(|(_, expr)| expr.clone_in(alloc))
        .collect();
    let mut out = statement.clone_in(alloc);
    let Statement::ExpressionStatement(target) = &mut out else {
        return None;
    };
    if kept.len() == 1 {
        target.expression = kept.pop()?;
    } else if let Expression::SequenceExpression(sequence) = &mut target.expression {
        sequence.expressions = AVec::from_iter_in(kept, &alloc);
    }
    Some(out)
}

fn removed_spans(top: &[Statement<'_>], layout: &Hoisted<'_, '_>, moved: &[usize]) -> Vec<Span> {
    let mut spans: Vec<Span> = moved.iter().map(|&index| top[index].span()).collect();
    for def in &layout.defs {
        let span = match (def.element, expression_statement(&top[def.at])) {
            (Some(at), Some(Expression::SequenceExpression(sequence))) => {
                sequence.expressions[at].span()
            }
            _ => top[def.at].span(),
        };
        spans.push(span);
    }
    spans
}

fn inside_any(sorted: &[Span], inner: Span) -> bool {
    let after = sorted.partition_point(|span| span.start <= inner.start);
    after > 0 && inner.end <= sorted[after - 1].end
}

fn entry_program<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Hoisted<'_, 'a>,
    moved: &[usize],
) -> Program<'a> {
    let top = &source.body;
    let mut body = AVec::with_capacity_in(top.len(), &alloc);
    for (index, statement) in top.iter().enumerate() {
        if moved.binary_search(&index).is_ok() {
            continue;
        }
        let removed: Vec<Option<usize>> = layout
            .defs_at(index)
            .iter()
            .map(|def| def.element)
            .collect();
        if removed.is_empty() {
            body.push(statement.clone_in(alloc));
        } else if !removed.contains(&None) {
            body.extend(kept_sequence(alloc, statement, &removed));
        }
    }
    let mut spans = removed_spans(top, layout, moved);
    spans.sort_unstable_by_key(|span| span.start);
    let comments = source
        .comments
        .iter()
        .filter(|comment| !inside_any(&spans, comment.span))
        .cloned();
    let mut program = plain_program(alloc, source, source.span, body);
    program.comments = AVec::from_iter_in(comments, &alloc);
    program.hashbang = source.hashbang.clone_in(alloc);
    program
}

fn runtime_of<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    moved: &[usize],
) -> Option<Module<'a>> {
    let statements = AVec::from_iter_in(
        moved
            .iter()
            .map(|&index| source.body[index].clone_in(alloc)),
        &alloc,
    );
    runtime_module(alloc, source, BundleKind::Parcel, statements).map(|(module, _)| module)
}

fn hoisted<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Hoisted<'_, 'a>,
    bodies: &mut Bodies<'a>,
    chunk: bool,
) -> Option<Bundle<'a>> {
    let entry = (!chunk).then(|| ENTRY_ID.to_string());
    let mut bundle = Bundle::new(BundleKind::Parcel, entry);
    let mut programs = Vec::with_capacity(layout.defs.len());
    for def in &layout.defs {
        programs.push(module_from_body_with(alloc, source, def.body()?, bodies));
    }
    if matches!(bodies, Bodies::Planned(_)) {
        return Some(bundle);
    }
    let top = &source.body;
    let moved = moved_helpers(
        top,
        &candidates(top, layout),
        programs.iter().flat_map(|p| p.body.iter()),
    );
    if !chunk {
        let mut entry = entry_program(alloc, source, layout, &moved);
        rename_wrapper_bindings(alloc, &mut entry, &[]);
        bundle.modules.insert(
            ENTRY_ID.to_string(),
            Module::new(ENTRY_ID.to_string(), entry, true),
        );
    }
    if !moved.is_empty() {
        let runtime = runtime_of(alloc, source, &moved)?;
        bundle.modules.insert(runtime.id.clone(), runtime);
    }
    for (def, mut program) in layout.defs.iter().zip(programs) {
        let body = def.body()?;
        program.directives = body.directives.clone_in(alloc);
        let params = param_names(&def.function.params);
        renamed(alloc, &mut program, &params, &HOISTED_NAMES);
        rename_wrapper_bindings(alloc, &mut program, &params);
        bundle
            .modules
            .insert(def.id.clone(), Module::new(def.id.clone(), program, false));
    }
    bundle.runtime = Some(Runtime {
        require_name: layout.require.name.to_string(),
        helpers: Vec::new(),
    });
    Some(bundle)
}

pub fn bundle<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    layout: &Layout<'_, 'a>,
    bodies: &mut Bodies<'a>,
    view: Option<&ParcelView>,
) -> Option<Bundle<'a>> {
    let chunk = view.is_some_and(|view| view.chunk);
    match layout {
        Layout::Registry(registry_layout) => {
            registry(alloc, source, registry_layout, bodies, chunk)
        }
        Layout::Hoisted(hoisted_layout) => hoisted(alloc, source, hoisted_layout, bodies, chunk),
    }
}
