use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::SourceType;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::respan::fresh_spans_statement;
use untwist_core::parse::parse_program;

use crate::closed::{captured_by_statement, Captured};
use crate::model::{BundleKind, Module};
use crate::path::{relative_path, PosixPath};

pub const RUNTIME_DIR: &str = "_runtime";

pub fn runtime_id(kind: BundleKind) -> String {
    format!("{RUNTIME_DIR}/{}", kind.as_str())
}

pub fn runtime_path(kind: BundleKind) -> String {
    format!("{}.js", runtime_id(kind))
}

pub fn take_statements<'a>(
    alloc: &'a Allocator,
    list: &mut AVec<'a, Statement<'a>>,
    indices: &[usize],
) -> AVec<'a, Statement<'a>> {
    let all = std::mem::replace(list, AVec::new_in(&alloc));
    let mut taken = AVec::with_capacity_in(indices.len(), &alloc);
    for (index, statement) in all.into_iter().enumerate() {
        if indices.contains(&index) {
            taken.push(statement);
        } else {
            list.push(statement);
        }
    }
    taken
}

fn binding_names(pattern: &BindingPattern<'_>, names: &mut Vec<String>) {
    if let BindingPattern::BindingIdentifier(id) = pattern {
        names.push(id.name.to_string());
    }
}

pub fn declared_names(statements: &[Statement<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    for statement in statements {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    binding_names(&declarator.id, &mut names);
                }
            }
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    names.push(id.name.to_string());
                }
            }
            _ => {}
        }
    }
    names
}

fn declared_symbols(statement: &Statement<'_>, symbols: &mut FxHashSet<SymbolId>) {
    match statement {
        Statement::VariableDeclaration(declaration) => {
            for declarator in &declaration.declarations {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                    symbols.extend(id.symbol_id.get());
                }
            }
        }
        Statement::FunctionDeclaration(function) => {
            symbols.extend(function.id.as_ref().and_then(|id| id.symbol_id.get()));
        }
        _ => {}
    }
}

pub fn helper_symbols(list: &[Statement<'_>], indices: &[usize]) -> FxHashSet<SymbolId> {
    let mut symbols = FxHashSet::default();
    for &index in indices {
        if let Some(statement) = list.get(index) {
            declared_symbols(statement, &mut symbols);
        }
    }
    symbols
}

pub fn first_unclosed_helper(
    scoping: &Scoping,
    list: &[Statement<'_>],
    indices: &[usize],
) -> Option<(String, Captured)> {
    let helpers = helper_symbols(list, indices);
    indices.iter().find_map(|&index| {
        let statement = list.get(index)?;
        let captured = captured_by_statement(scoping, statement, &helpers)?;
        let name = declared_names(std::slice::from_ref(statement)).join(", ");
        Some((name, captured))
    })
}

pub fn js_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for unit in text.encode_utf16() {
        match unit {
            0x5c => out.push_str("\\\\"),
            0x22 => out.push_str("\\\""),
            0x20..=0x7e => out.push(char::from(unit as u8)),
            _ => out.push_str(&format!("\\u{unit:04x}")),
        }
    }
    out.push('"');
    out
}

pub fn synthetic_statements<'a>(
    alloc: &'a Allocator,
    text: String,
) -> Option<AVec<'a, Statement<'a>>> {
    let text = alloc.alloc_str(&text);
    let parsed = parse_program(alloc, text);
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    let mut statements = parsed.program.body;
    for statement in statements.iter_mut() {
        if !fresh_spans_statement(statement) {
            return None;
        }
    }
    Some(statements)
}

pub fn runtime_module<'a>(
    alloc: &'a Allocator,
    source: &Program<'a>,
    kind: BundleKind,
    mut statements: AVec<'a, Statement<'a>>,
) -> Option<(Module<'a>, Vec<String>)> {
    let names = declared_names(&statements);
    let exports = format!("module.exports = {{ {} }};", names.join(", "));
    statements.extend(synthetic_statements(alloc, exports)?);
    let module = module_of(alloc, source.source_text, kind, statements)?;
    Some((module, names))
}

pub fn module_of<'a>(
    alloc: &'a Allocator,
    source_text: &'a str,
    kind: BundleKind,
    statements: AVec<'a, Statement<'a>>,
) -> Option<Module<'a>> {
    let builder = AstBuilder::new(alloc);
    let program = Program::new(
        oxc_span::SPAN,
        SourceType::cjs(),
        source_text,
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        statements,
        &builder,
    );
    let mut module = Module::new(runtime_id(kind), program, false);
    module.path = PosixPath::parse(&runtime_path(kind))?;
    Some(module)
}

pub fn runtime_binding_import<'a>(
    alloc: &'a Allocator,
    from: &PosixPath,
    kind: BundleKind,
    binding: &str,
) -> Option<Statement<'a>> {
    let specifier = relative_path(from.as_str(), &runtime_path(kind));
    let text = format!("const {binding} = require({});", js_string(&specifier));
    synthetic_statements(alloc, text)?.pop()
}

pub fn runtime_import<'a>(
    alloc: &'a Allocator,
    from: &PosixPath,
    kind: BundleKind,
    names: &[String],
) -> Option<Statement<'a>> {
    let specifier = relative_path(from.as_str(), &runtime_path(kind));
    let text = format!(
        "const {{ {} }} = require({});",
        names.join(", "),
        js_string(&specifier)
    );
    synthetic_statements(alloc, text)?.pop()
}
