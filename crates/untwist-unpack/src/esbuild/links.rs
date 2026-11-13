use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use rustc_hash::FxHashMap;
use untwist_core::semantic::build_scoping;

use crate::esbuild::layout::{exported_helpers, layout};
use crate::esbuild::shape::CallForm;
use crate::frame::Frame;

#[derive(Clone, Debug, Default)]
pub struct EsbuildView {
    pub helpers: FxHashMap<String, CallForm>,
}

fn parent(name: &str) -> &str {
    name.rfind('/').map_or("", |at| &name[..at])
}

fn resolved(from: &str, specifier: &str) -> Option<String> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None;
    }
    let mut parts: Vec<&str> = parent(from)
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in specifier.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

fn unchanged(scoping: &Scoping, binding: &BindingIdentifier<'_>) -> bool {
    binding.symbol_id.get().is_some_and(|symbol| {
        scoping.symbol_redeclarations(symbol).is_empty()
            && !scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
    })
}

fn helper_chunk(program: &Program<'_>) -> Option<FxHashMap<String, CallForm>> {
    let frame = Frame::of(program);
    if layout(frame.top).is_some() {
        return None;
    }
    let exported = exported_helpers(&program.body)?;
    let scoping = build_scoping(program).scoping;
    exported
        .trusted
        .iter()
        .all(|binding| unchanged(&scoping, binding))
        .then_some(exported.helpers)
}

fn imported(
    program: &Program<'_>,
    from: &str,
    names: &[String],
    chunks: &mut FxHashMap<usize, Option<FxHashMap<String, CallForm>>>,
    programs: &[Program<'_>],
) -> FxHashMap<String, CallForm> {
    let mut helpers = FxHashMap::default();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let Some(target) = import
            .source
            .value
            .as_str()
            .and_then(|specifier| resolved(from, specifier))
        else {
            continue;
        };
        let Some(input) = names.iter().position(|name| *name == target) else {
            continue;
        };
        let chunk = chunks
            .entry(input)
            .or_insert_with(|| helper_chunk(&programs[input]));
        let Some(chunk) = chunk.as_ref() else {
            continue;
        };
        for specifier in import.specifiers.iter().flatten() {
            let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier else {
                continue;
            };
            if let Some(&form) = chunk.get(named.imported.name().as_str()) {
                helpers.insert(named.local.name.to_string(), form);
            }
        }
    }
    helpers
}

pub fn views(programs: &[Program<'_>], names: &[String]) -> Vec<(usize, EsbuildView)> {
    if names.len() != programs.len() {
        return Vec::new();
    }
    let mut chunks = FxHashMap::default();
    programs
        .iter()
        .enumerate()
        .filter_map(|(input, program)| {
            let helpers = imported(program, &names[input], names, &mut chunks, programs);
            (!helpers.is_empty()).then_some((input, EsbuildView { helpers }))
        })
        .collect()
}

fn relative_imports<'a>(list: &[Statement<'a>]) -> Vec<&'a str> {
    list.iter()
        .filter_map(|statement| match statement {
            Statement::ImportDeclaration(import)
                if import
                    .source
                    .value
                    .as_str()
                    .is_some_and(|spec| spec.starts_with("./") || spec.starts_with("../")) =>
            {
                import.specifiers.as_ref()
            }
            _ => None,
        })
        .flatten()
        .filter_map(|specifier| match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(named) => Some(named.local.name.as_str()),
            _ => None,
        })
        .collect()
}

fn calls_imported(init: &Expression<'_>, imported: &[&str]) -> bool {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return false;
    };
    matches!(&call.callee, Expression::Identifier(callee) if imported.contains(&callee.name.as_str()))
        && matches!(
            call.arguments.as_slice(),
            [Argument::ObjectExpression(_)
                | Argument::FunctionExpression(_)
                | Argument::ArrowFunctionExpression(_)]
        )
}

pub fn may_link(list: &[Statement<'_>]) -> bool {
    let imported = relative_imports(list);
    !imported.is_empty()
        && list.iter().any(|statement| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return false;
            };
            declaration
                .declarations
                .iter()
                .filter_map(|declarator| declarator.init.as_ref())
                .any(|init| calls_imported(init, &imported))
        })
}
