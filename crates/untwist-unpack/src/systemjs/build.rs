use oxc_allocator::{Allocator, CloneIn, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_str::{Ident, Str};
use oxc_syntax::operator::AssignmentOperator;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::naming_slot::{text_observes_names, unnamed};

use super::exports::{context_import, declarator_export, is_export_call, value, Value};
use super::layout::{Bind, Declare, Layout};
use super::DYNAMIC;
use crate::model::{module_from_body_with, Bodies, Module};
use crate::runtime::{js_string, synthetic_statements};

pub struct Shared<'n, 'a> {
    pub names: &'n FxHashSet<&'a str>,
    pub prefix: Option<String>,
    pub text: &'a str,
    pub names_read: Option<bool>,
}

impl Shared<'_, '_> {
    fn names_read(&mut self) -> bool {
        let text = self.text;
        *self
            .names_read
            .get_or_insert_with(|| text_observes_names(text))
    }
}

const UNBINDABLE: [&str; 51] = [
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "let",
    "static",
    "implements",
    "interface",
    "package",
    "private",
    "protected",
    "public",
    "await",
    "eval",
    "arguments",
    "undefined",
    "NaN",
    "Infinity",
];

fn fresh_prefix(text: &str) -> String {
    (0..)
        .map(|index| "$".repeat(index + 1))
        .find(|prefix| !text.contains(prefix.as_str()))
        .unwrap_or_default()
}

struct Rewrite<'a, 'r, 'n> {
    builder: AstBuilder<'a>,
    export: Option<&'a str>,
    context: Option<&'a str>,
    shared: &'r mut Shared<'n, 'a>,
    seen: FxHashSet<String>,
    exports: Vec<(String, String)>,
    fresh: Vec<String>,
    dynamic: Vec<String>,
    targets: &'r FxHashMap<String, String>,
    idents: FxHashSet<&'a str>,
    created: Vec<(Span, String)>,
    renames: &'r FxHashMap<Span, String>,
}

impl<'a> Rewrite<'a, '_, '_> {
    fn relink(&self, it: &mut ImportExpression<'a>) {
        let Expression::StringLiteral(literal) = &mut it.source else {
            return;
        };
        let path = literal
            .value
            .as_str()
            .and_then(|source| self.targets.get(source));
        if let Some(path) = path {
            literal.value = Str::from_str_in(path, &self.builder).into();
            literal.raw = None;
        }
    }

    fn add(&mut self, name: &str, local: &str) {
        if self.seen.insert(name.to_string()) {
            self.exports.push((name.to_string(), local.to_string()));
        }
    }

    fn single(&mut self, name: &str, mut expression: Expression<'a>) -> Expression<'a> {
        self.visit_expression(&mut expression);
        let local = match value(&expression) {
            Value::Write(id) | Value::Read(id) => id.name.to_string(),
            Value::Fresh | Value::Invalid => {
                if !self.seen.contains(name) {
                    self.fresh.push(name.to_string());
                }
                let local = name.to_string();
                self.add(name, &local);
                let span = expression.span();
                self.created.push((span, local.clone()));
                let target = AssignmentTarget::new_assignment_target_identifier(
                    span,
                    Ident::from_str_in(&local, &self.builder),
                    &self.builder,
                );
                return Expression::new_assignment_expression(
                    span,
                    AssignmentOperator::Assign,
                    target,
                    expression,
                    &self.builder,
                );
            }
        };
        self.add(name, &local);
        expression
    }

    fn exported(&mut self, call: &mut CallExpression<'a>) -> Option<Expression<'a>> {
        let span = call.span;
        match call.arguments.as_mut_slice() {
            [Argument::StringLiteral(name), argument] => {
                let name = name.value.as_str()?;
                let expression = argument.take_in(&self.builder).into_expression();
                let expression = match expression.is_anonymous_function_definition()
                    && self.shared.names_read()
                {
                    true => unnamed(expression, &self.builder),
                    false => expression,
                };
                Some(self.single(name, expression))
            }
            [Argument::ObjectExpression(object)] => {
                let mut values = AVec::new_in(&self.builder);
                for property in object.properties.iter_mut() {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        return None;
                    };
                    let name = property
                        .key
                        .static_name()
                        .and_then(|name| name.into_cow_str())?
                        .to_string();
                    let expression = property.value.take_in(&self.builder);
                    values.push(self.single(&name, expression));
                }
                Some(match values.len() {
                    1 => values.pop()?,
                    _ => Expression::new_sequence_expression(span, values, &self.builder),
                })
            }
            _ => None,
        }
    }

    fn imported(&mut self, call: &mut CallExpression<'a>) -> Option<Expression<'a>> {
        let specifier = context_import(call, self.context)??;
        if self.shared.names.contains(specifier) {
            self.dynamic.push(specifier.to_string());
        }
        let source = call
            .arguments
            .first_mut()?
            .take_in(&self.builder)
            .into_expression();
        Some(Expression::new_import_expression(
            call.span,
            source,
            None,
            None,
            &self.builder,
        ))
    }

    fn replacement(&mut self, it: &mut Expression<'a>) -> Option<Expression<'a>> {
        let Expression::CallExpression(call) = it else {
            return None;
        };
        if is_export_call(call, self.export) {
            return self.exported(call);
        }
        self.imported(call)
    }
}

impl<'a> VisitMut<'a> for Rewrite<'a, '_, '_> {
    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        if let Some((id, name)) = declarator_export(it, self.export) {
            let local = id.name.as_str();
            self.add(name, local);
            let value = match &mut it.init {
                Some(Expression::CallExpression(call)) => call
                    .arguments
                    .get_mut(1)
                    .map(|argument| argument.take_in(&self.builder).into_expression()),
                _ => None,
            };
            if value.is_some() {
                it.init = value;
            }
        }
        walk_mut::walk_variable_declarator(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if let Some(name) = self.renames.get(&it.span) {
            it.name = Ident::from_str_in(name, &self.builder);
        } else {
            self.idents.insert(it.name.as_str());
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        self.idents.insert(it.name.as_str());
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(replacement) = self.replacement(it) {
            *it = replacement;
            return;
        }
        if let Expression::ImportExpression(import) = it {
            self.relink(import);
        }
        walk_mut::walk_expression(self, it);
    }
}

fn specifier_list(pairs: &[(&str, &str)]) -> String {
    let list: Vec<String> = pairs
        .iter()
        .map(|(name, local)| match name == local {
            true => (*name).to_string(),
            false => format!("{name} as {local}"),
        })
        .collect();
    list.join(", ")
}

fn dep_text(dep: &str, binds: &[Bind<'_, '_>]) -> String {
    let source = js_string(dep);
    let mut named = Vec::new();
    let mut re_exports = Vec::new();
    let mut text = String::new();
    for bind in binds {
        match bind {
            Bind::Import(name, local) => named.push((*name, local.name.as_str())),
            Bind::Namespace(local) => {
                text.push_str(&format!("import * as {} from {source};\n", local.name))
            }
            Bind::ReExport(name, exported) => re_exports.push((*name, *exported)),
        }
    }
    if !named.is_empty() {
        text.insert_str(
            0,
            &format!("import {{ {} }} from {source};\n", specifier_list(&named)),
        );
    }
    if !re_exports.is_empty() {
        text.push_str(&format!(
            "export {{ {} }} from {source};\n",
            specifier_list(&re_exports)
        ));
    }
    if text.is_empty() {
        text = format!("import {source};\n");
    }
    text
}

fn prologue_text(layout: &Layout<'_, '_>, at: usize, fresh: &[String]) -> String {
    let declare = &layout.registers[at].declare;
    let mut text = String::new();
    for (index, dep) in layout.registers[at].deps.iter().enumerate() {
        let binds = declare.setters.get(index).map_or(&[][..], Vec::as_slice);
        text.push_str(&dep_text(dep, binds));
    }
    let bound: FxHashSet<&str> = declare
        .setters
        .iter()
        .flatten()
        .filter_map(|bind| match bind {
            Bind::Import(_, local) | Bind::Namespace(local) => Some(local.name.as_str()),
            Bind::ReExport(..) => None,
        })
        .collect();
    let mut vars: Vec<&str> = declare
        .vars
        .iter()
        .map(|id| id.name.as_str())
        .filter(|name| !bound.contains(name))
        .collect();
    vars.extend(fresh.iter().map(String::as_str));
    if !vars.is_empty() {
        text.push_str(&format!("var {};\n", vars.join(", ")));
    }
    text
}

fn export_text(exports: &[(String, String)]) -> String {
    let pairs: Vec<(&str, &str)> = exports
        .iter()
        .map(|(name, local)| (local.as_str(), name.as_str()))
        .collect();
    format!("export {{ {} }};", specifier_list(&pairs))
}

fn dependencies(
    layout: &Layout<'_, '_>,
    at: usize,
    names: &FxHashSet<&str>,
    dynamic: Vec<String>,
) -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = layout.registers[at]
        .deps
        .iter()
        .filter(|dep| names.contains(*dep))
        .map(|dep| ((*dep).to_string(), (*dep).to_string()))
        .collect();
    list.extend(dynamic.into_iter().map(|id| (DYNAMIC.to_string(), id)));
    list
}

fn assemble<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    first: AVec<'a, Statement<'a>>,
    last: AVec<'a, Statement<'a>>,
) {
    let body = std::mem::replace(&mut program.body, AVec::new_in(&alloc));
    let mut statements = AVec::with_capacity_in(first.len() + body.len() + last.len(), &alloc);
    statements.extend(first);
    statements.extend(body);
    statements.extend(last);
    program.body = statements;
}

type Rewritten = (Vec<(String, String)>, Vec<String>, Vec<String>);

struct Maps<'r> {
    targets: &'r FxHashMap<String, String>,
    renames: &'r FxHashMap<Span, String>,
}

fn rewriter<'a, 'r, 'n>(
    alloc: &'a Allocator,
    (export, context): (Option<&'a str>, Option<&'a str>),
    shared: &'r mut Shared<'n, 'a>,
    maps: Maps<'r>,
) -> Rewrite<'a, 'r, 'n> {
    Rewrite {
        builder: AstBuilder::new(alloc),
        export,
        context,
        shared,
        seen: FxHashSet::default(),
        exports: Vec::new(),
        fresh: Vec::new(),
        dynamic: Vec::new(),
        targets: maps.targets,
        idents: FxHashSet::default(),
        created: Vec::new(),
        renames: maps.renames,
    }
}

fn collides(name: &str, idents: &FxHashSet<&str>, declare: &Declare<'_, '_>) -> bool {
    idents.contains(name)
        || UNBINDABLE.contains(&name)
        || declare
            .params
            .iter()
            .chain(&declare.vars)
            .any(|id| id.name == name)
}

fn rewritten<'a>(
    alloc: &'a Allocator,
    (source, declare): (&Program<'a>, &Declare<'_, 'a>),
    program: &mut Program<'a>,
    shared: &mut Shared<'_, 'a>,
) -> Rewritten {
    let (targets, renames) = (FxHashMap::default(), FxHashMap::default());
    let names = (declare.export_name(), declare.context_name());
    let maps = Maps {
        targets: &targets,
        renames: &renames,
    };
    let mut rewrite = rewriter(alloc, names, shared, maps);
    rewrite.visit_program(program);
    let Rewrite {
        mut exports,
        mut fresh,
        dynamic,
        idents,
        created,
        ..
    } = rewrite;
    let collided: FxHashSet<String> = fresh
        .iter()
        .filter(|name| collides(name, &idents, declare))
        .cloned()
        .collect();
    if collided.is_empty() {
        return (exports, fresh, dynamic);
    }
    let text = source.source_text;
    let prefix = shared
        .prefix
        .get_or_insert_with(|| fresh_prefix(text))
        .clone();
    let renames: FxHashMap<Span, String> = created
        .into_iter()
        .filter(|(_, name)| collided.contains(name))
        .map(|(span, name)| (span, format!("{prefix}{name}")))
        .collect();
    let maps = Maps {
        targets: &targets,
        renames: &renames,
    };
    rewriter(alloc, (None, None), shared, maps).visit_program(program);
    for local in fresh
        .iter_mut()
        .chain(exports.iter_mut().map(|(_, local)| local))
    {
        if collided.contains(local.as_str()) {
            *local = format!("{prefix}{local}");
        }
    }
    (exports, fresh, dynamic)
}

pub fn module<'a>(
    alloc: &'a Allocator,
    (source, layout): (&Program<'a>, &Layout<'_, 'a>),
    at: usize,
    bodies: &mut Bodies<'a>,
    shared: &mut Shared<'_, 'a>,
) -> Option<Module<'a>> {
    let register = &layout.registers[at];
    let names = shared.names;
    let mut program = module_from_body_with(alloc, source, register.declare.execute, bodies);
    let id = layout.id(at).to_string();
    let is_entry = at == layout.entry;
    if matches!(bodies, Bodies::Planned(_)) {
        return Some(Module::new(id, program, is_entry));
    }
    program.directives = AVec::from_iter_in(
        layout
            .strict_directives(register)
            .iter()
            .map(|directive| directive.clone_in(alloc)),
        &alloc,
    );
    let (exports, fresh, dynamic) =
        rewritten(alloc, (source, &register.declare), &mut program, shared);
    let prologue = prologue_text(layout, at, &fresh);
    let first = match prologue.is_empty() {
        true => AVec::new_in(&alloc),
        false => synthetic_statements(alloc, prologue)?,
    };
    let last = match exports.is_empty() {
        true => AVec::new_in(&alloc),
        false => synthetic_statements(alloc, export_text(&exports))?,
    };
    assemble(alloc, &mut program, first, last);
    if program.body.iter().any(Statement::is_module_declaration) {
        program.source_type = SourceType::mjs();
    }
    let mut module = Module::new(id, program, is_entry);
    module.dependencies = dependencies(layout, at, names, dynamic);
    Some(module)
}

pub fn relink_dynamic<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    targets: &FxHashMap<String, String>,
) {
    let names = FxHashSet::default();
    let mut shared = Shared {
        names: &names,
        prefix: None,
        text: "",
        names_read: Some(true),
    };
    let renames = FxHashMap::default();
    let maps = Maps {
        targets,
        renames: &renames,
    };
    rewriter(alloc, (None, None), &mut shared, maps).visit_program(program);
}
