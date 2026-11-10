use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_str::Str;
use rustc_hash::FxHashMap;

use super::build::relink_dynamic;
use super::DYNAMIC;
use crate::model::Bundle;
use crate::specifier::ModulePaths;

fn remap<'a>(
    builder: &AstBuilder<'a>,
    literal: &mut StringLiteral<'a>,
    targets: &FxHashMap<String, String>,
) {
    let path = literal
        .value
        .as_str()
        .and_then(|source| targets.get(source));
    if let Some(path) = path {
        literal.value = Str::from_str_in(path, builder).into();
        literal.raw = None;
    }
}

fn statements<'a>(
    builder: &AstBuilder<'a>,
    list: &mut [Statement<'a>],
    targets: &FxHashMap<String, String>,
) {
    for statement in list {
        match statement {
            Statement::ImportDeclaration(declaration) => {
                remap(builder, &mut declaration.source, targets)
            }
            Statement::ExportFromDeclaration(declaration) => {
                remap(builder, &mut declaration.source, targets)
            }
            _ => {}
        }
    }
}

pub fn link<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) {
    let paths = ModulePaths::of(bundle);
    let builder = AstBuilder::new(alloc);
    for module in bundle.modules.values_mut() {
        if module.dependencies.is_empty() {
            continue;
        }
        let from = module.path.as_str().to_string();
        let targets: FxHashMap<String, String> = module
            .dependencies
            .iter()
            .filter_map(|(_, id)| Some((id.clone(), paths.specifier(&from, paths.get(id)?))))
            .collect();
        let dynamic = module.dependencies.iter().any(|(kind, _)| kind == DYNAMIC);
        let program = module.program_mut();
        statements(&builder, &mut program.body, &targets);
        if dynamic {
            relink_dynamic(alloc, program, &targets);
        }
    }
}
