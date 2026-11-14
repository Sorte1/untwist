use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

use crate::ty::Ty;

const UNKNOWN: &str = "unknown";

struct Writer<'s> {
    scoping: &'s Scoping,
    lookup: &'s dyn Fn(SymbolId) -> Option<Ty>,
    lines: Vec<String>,
    exported: Vec<(String, String)>,
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn quoted(name: &str) -> Option<String> {
    if is_identifier(name) {
        return Some(name.to_string());
    }
    let printable = name
        .chars()
        .all(|c| (' '..='~').contains(&c) && c != '"' && c != '\\');
    printable.then(|| format!("\"{name}\""))
}

fn export_name(name: &ModuleExportName<'_>) -> Option<String> {
    match name {
        ModuleExportName::IdentifierName(id) => quoted(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => quoted(id.name.as_str()),
        ModuleExportName::StringLiteral(text) => text.value.as_str().and_then(quoted),
    }
}

impl Writer<'_> {
    fn proven(&self, symbol: Option<SymbolId>) -> Option<Ty> {
        symbol
            .and_then(|symbol| (self.lookup)(symbol))
            .filter(Ty::promise_free)
    }

    fn type_of(&self, symbol: Option<SymbolId>) -> String {
        self.proven(symbol)
            .map_or_else(|| UNKNOWN.to_string(), |ty| ty.ts())
    }

    fn declare(&mut self, exported: Option<String>, ty: String) {
        let Some(exported) = exported else {
            return;
        };
        let local = format!("__untwist_export_{}", self.exported.len());
        self.lines.push(format!("declare const {local}: {ty};"));
        self.exported.push((local, exported));
    }

    fn reference_symbol(&self, name: &ModuleExportName<'_>) -> Option<SymbolId> {
        let ModuleExportName::IdentifierReference(id) = name else {
            return None;
        };
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn declaration(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::VariableDeclaration(variables) => {
                for declarator in &variables.declarations {
                    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                        for id in declarator.id.get_binding_identifiers() {
                            self.declare(quoted(id.name.as_str()), UNKNOWN.to_string());
                        }
                        continue;
                    };
                    let hoisted = variables.kind == VariableDeclarationKind::Var;
                    let ty = self
                        .proven(id.symbol_id.get())
                        .map(|ty| {
                            if hoisted {
                                ty.join(&Ty::undefined())
                            } else {
                                ty
                            }
                        })
                        .map_or_else(|| UNKNOWN.to_string(), |ty| ty.ts());
                    self.declare(quoted(id.name.as_str()), ty);
                }
            }
            Declaration::FunctionDeclaration(function) => {
                let name = function.id.as_ref().and_then(|id| quoted(id.name.as_str()));
                self.declare(name, UNKNOWN.to_string());
            }
            Declaration::ClassDeclaration(class) => {
                let name = class.id.as_ref().and_then(|id| quoted(id.name.as_str()));
                self.declare(name, UNKNOWN.to_string());
            }
            _ => {}
        }
    }

    fn statement(&mut self, statement: &Statement<'_>, source: &str) {
        match statement {
            Statement::ExportDeclaration(export) => self.declaration(&export.declaration),
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    let ty = self.type_of(self.reference_symbol(&specifier.local));
                    self.declare(export_name(&specifier.exported), ty);
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let ty = match &export.declaration {
                    ExportDefaultDeclarationKind::Identifier(id) => {
                        let symbol = id.reference_id.get().and_then(|reference| {
                            self.scoping.get_reference(reference).symbol_id()
                        });
                        self.type_of(symbol)
                    }
                    _ => UNKNOWN.to_string(),
                };
                self.declare(Some("default".to_string()), ty);
            }
            Statement::ExportFromDeclaration(export) => {
                let span = export.span;
                self.lines
                    .push(source[span.start as usize..span.end as usize].to_string());
            }
            Statement::ExportAllDeclaration(export) => {
                let span = export.span;
                self.lines
                    .push(source[span.start as usize..span.end as usize].to_string());
            }
            _ => {}
        }
    }
}

pub(crate) fn render(
    program: &Program<'_>,
    scoping: &Scoping,
    source: &str,
    lookup: &dyn Fn(SymbolId) -> Option<Ty>,
) -> Option<String> {
    if !untwist_core::semantic::has_module_syntax(program) {
        return None;
    }
    let mut writer = Writer {
        scoping,
        lookup,
        lines: Vec::new(),
        exported: Vec::new(),
    };
    for statement in &program.body {
        writer.statement(statement, source);
    }
    let names: Vec<String> = writer
        .exported
        .iter()
        .map(|(local, exported)| format!("{local} as {exported}"))
        .collect();
    let mut lines = writer.lines;
    lines.push(match names.is_empty() {
        true => "export {};".to_string(),
        false => format!("export {{ {} }};", names.join(", ")),
    });
    let mut text = lines.join("\n");
    text.push('\n');
    Some(text)
}
