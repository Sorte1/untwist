use oxc_ast::ast::{
    AssignmentOperator, AssignmentTarget, Expression, ObjectExpression, PropertyKey,
};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::keyword::is_reserved_keyword_or_global_object;
use oxc_syntax::node::NodeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::library::known_function;
use super::module::{ExportKind, WasmImport, WasmSummary};
use super::rename::{Claimed, Renamer};

const MIN_GLUE_MODULE_LEN: usize = 3;
const MIN_NAME_LEN: usize = 3;
const MAX_NAME_LEN: usize = 64;
const EXPORTS_HOLDER: &str = "wasmExports";
const IMPORTS_OBJECT: &str = "wasmImports";
const MEMORY_BINDING: &str = "wasmMemory";
const TABLE_BINDING: &str = "wasmTable";
const EXPORTS_PROPERTY: &str = "asm";
const MISSING_SHOWN: usize = 3;
const NOT_NAMES: &[&str] = &[
    "arguments",
    "eval",
    "let",
    "static",
    "yield",
    "await",
    "async",
];

#[derive(Debug, Default)]
pub struct Naming {
    pub edits: Vec<(Span, String)>,
    pub renamed: usize,
    pub mismatch: Option<String>,
}

struct Provided {
    modules: Vec<String>,
    field: String,
    value: Option<SymbolId>,
}

struct Glue<'s, 'm> {
    semantic: &'s Semantic<'s>,
    modules: FxHashSet<&'m str>,
    holders: FxHashSet<SymbolId>,
    exports_owners: FxHashSet<SymbolId>,
    provided: Vec<Provided>,
    import_objects: FxHashSet<SymbolId>,
    reads: Vec<String>,
    bindings: Vec<(SymbolId, String)>,
    key_exports: FxHashMap<String, FxHashSet<String>>,
    symbol_keys: FxHashMap<SymbolId, FxHashSet<String>>,
}

enum Link<'e> {
    Symbol(SymbolId),
    Key(&'e str),
}

fn static_key<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(text) => text.value.as_str(),
        _ => None,
    }
}

fn unchained<'a>(mut expression: &'a Expression<'a>) -> &'a Expression<'a> {
    loop {
        expression = expression.without_parentheses();
        match expression {
            Expression::AssignmentExpression(assign)
                if assign.operator == AssignmentOperator::Assign =>
            {
                expression = &assign.right;
            }
            _ => return expression,
        }
    }
}

fn is_instance_exports(expression: &Expression<'_>) -> bool {
    let Some(member) = expression.without_parentheses().as_member_expression() else {
        return false;
    };
    if member.static_property_name().and_then(|name| name.as_str()) != Some("exports") {
        return false;
    }
    match member.object().without_parentheses() {
        Expression::Identifier(id) => id.name != "module" && id.name != "exports",
        object => object.as_member_expression().is_some_and(|inner| {
            inner.static_property_name().and_then(|name| name.as_str()) == Some("instance")
        }),
    }
}

fn is_name(name: &str) -> bool {
    (MIN_NAME_LEN..=MAX_NAME_LEN).contains(&name.len())
        && is_identifier_name(name)
        && !is_reserved_keyword_or_global_object(name)
        && !NOT_NAMES.contains(&name)
}

fn sanitized(name: &str) -> Option<String> {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            true => out.push(ch),
            false if !out.ends_with('_') => out.push('_'),
            false => {}
        }
    }
    let out = out.trim_matches('_').to_string();
    let out = match out.starts_with(|c: char| c.is_ascii_digit()) {
        true => format!("_{out}"),
        false => out,
    };
    is_name(&out).then_some(out)
}

pub fn chosen_name(wasm_name: &str, function_name: Option<&String>) -> Option<String> {
    if is_name(wasm_name) {
        return Some(wasm_name.to_string());
    }
    function_name.and_then(|name| sanitized(name))
}

impl<'s> Glue<'s, '_> {
    fn nodes(&self) -> &'s AstNodes<'s> {
        self.semantic.nodes()
    }

    fn scoping(&self) -> &'s Scoping {
        self.semantic.scoping()
    }

    fn symbol_of(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = expression.without_parentheses() else {
            return None;
        };
        self.scoping().get_reference(id.reference_id()).symbol_id()
    }

    fn object_fields(&self, object: &ObjectExpression<'_>) -> Vec<(String, Option<SymbolId>)> {
        object
            .properties
            .iter()
            .filter_map(|property| {
                let property = property.as_property()?;
                let key = static_key(&property.key)?;
                let value = (!property.shorthand)
                    .then(|| self.symbol_of(&property.value))
                    .flatten();
                Some((key.to_string(), value))
            })
            .collect()
    }

    fn object_of(&self, expression: &'s Expression<'s>) -> Option<&'s ObjectExpression<'s>> {
        match expression.without_parentheses() {
            Expression::ObjectExpression(object) => Some(object),
            other => {
                let symbol = self.symbol_of(other)?;
                let declaration = self.scoping().symbol_declaration(symbol);
                let AstKind::VariableDeclarator(declarator) = self.nodes().kind(declaration) else {
                    return None;
                };
                match declarator.init.as_ref()?.without_parentheses() {
                    Expression::ObjectExpression(object) => Some(object),
                    _ => None,
                }
            }
        }
    }

    fn collect_holders(&mut self) {
        for node in self.nodes().iter() {
            match node.kind() {
                AstKind::VariableDeclarator(declarator) => {
                    let (Some(id), Some(init)) =
                        (declarator.id.get_binding_identifier(), &declarator.init)
                    else {
                        continue;
                    };
                    if is_instance_exports(unchained(init)) {
                        self.holders.insert(id.symbol_id());
                    }
                }
                AstKind::AssignmentExpression(assign) => {
                    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
                        continue;
                    };
                    let symbol = self.scoping().get_reference(id.reference_id()).symbol_id();
                    if let (Some(symbol), true) =
                        (symbol, is_instance_exports(unchained(&assign.right)))
                    {
                        self.holders.insert(symbol);
                    }
                }
                _ => {}
            }
        }
        self.collect_exports_owners();
    }

    fn collect_exports_owners(&mut self) {
        for node in self.nodes().iter() {
            let AstKind::AssignmentExpression(assign) = node.kind() else {
                continue;
            };
            let Some(target) = assign.left.as_member_expression() else {
                continue;
            };
            if target.static_property_name().and_then(|name| name.as_str())
                != Some(EXPORTS_PROPERTY)
            {
                continue;
            }
            let value = unchained(&assign.right);
            let holds_exports = is_instance_exports(value)
                || self
                    .symbol_of(value)
                    .is_some_and(|symbol| self.holders.contains(&symbol));
            if let (Some(owner), true) = (self.symbol_of(target.object()), holds_exports) {
                self.exports_owners.insert(owner);
            }
        }
    }

    fn holds_exports(&self, object: &Expression<'_>) -> bool {
        let object = object.without_parentheses();
        if is_instance_exports(object)
            || self
                .symbol_of(object)
                .is_some_and(|symbol| self.holders.contains(&symbol))
        {
            return true;
        }
        object.as_member_expression().is_some_and(|member| {
            member.static_property_name().and_then(|name| name.as_str()) == Some(EXPORTS_PROPERTY)
                && self
                    .symbol_of(member.object())
                    .is_some_and(|owner| self.exports_owners.contains(&owner))
        })
    }

    fn export_read<'e>(&self, expression: &'e Expression<'e>) -> Option<&'e str> {
        let member = expression.without_parentheses().as_member_expression()?;
        let name = member
            .static_property_name()
            .and_then(|name| name.as_str())?;
        self.holds_exports(member.object()).then_some(name)
    }

    fn is_write_target(&self, id: NodeId, span: Span) -> bool {
        match self.nodes().parent_kind(id) {
            AstKind::AssignmentExpression(assign) => assign.left.span() == span,
            _ => false,
        }
    }

    fn collect_reads(&mut self) {
        let nodes = self.nodes();
        for node in nodes.iter() {
            let (span, read) = match node.kind() {
                AstKind::StaticMemberExpression(member) => (
                    member.span,
                    self.member_read(&member.object, Some(member.property.name.as_str())),
                ),
                AstKind::ComputedMemberExpression(member) => (
                    member.span,
                    self.member_read(
                        &member.object,
                        member.static_property_name().and_then(|name| name.as_str()),
                    ),
                ),
                _ => continue,
            };
            if self.is_write_target(node.id(), span) {
                continue;
            }
            if let Some(read) = read {
                self.reads.push(read.to_string());
            }
        }
        self.reads.sort_unstable();
        self.reads.dedup();
    }

    fn member_read<'e>(
        &self,
        object: &'e Expression<'e>,
        name: Option<&'e str>,
    ) -> Option<&'e str> {
        self.holds_exports(object).then_some(name?)
    }

    fn collect_bindings(&mut self) {
        let nodes = self.nodes();
        for node in nodes.iter() {
            let (symbol, value) = match node.kind() {
                AstKind::VariableDeclarator(declarator) => {
                    let (Some(id), Some(init)) =
                        (declarator.id.get_binding_identifier(), &declarator.init)
                    else {
                        continue;
                    };
                    (id.symbol_id(), init)
                }
                AstKind::AssignmentExpression(assign) => {
                    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
                        continue;
                    };
                    let Some(symbol) = self.scoping().get_reference(id.reference_id()).symbol_id()
                    else {
                        continue;
                    };
                    (symbol, &assign.right)
                }
                _ => continue,
            };
            if let Some(name) = self.export_read(unchained(value)) {
                self.bindings.push((symbol, name.to_string()));
            }
        }
    }

    fn push_target<'e>(&self, links: &mut Vec<Link<'e>>, target: &'e AssignmentTarget<'e>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
            if let Some(symbol) = self.scoping().get_reference(id.reference_id()).symbol_id() {
                links.push(Link::Symbol(symbol));
            }
            return;
        }
        let Some(member) = target.as_member_expression() else {
            return;
        };
        if !matches!(
            member.object().without_parentheses(),
            Expression::Identifier(_)
        ) {
            return;
        }
        if let Some(key) = member.static_property_name().and_then(|name| name.as_str()) {
            links.push(Link::Key(key));
        }
    }

    fn chain<'e>(
        &self,
        links: &mut Vec<Link<'e>>,
        mut expression: &'e Expression<'e>,
    ) -> &'e Expression<'e> {
        loop {
            expression = expression.without_parentheses();
            match expression {
                Expression::AssignmentExpression(assign)
                    if assign.operator == AssignmentOperator::Assign =>
                {
                    self.push_target(links, &assign.left);
                    expression = &assign.right;
                }
                _ => return expression,
            }
        }
    }

    fn collect_wrappers(&mut self) {
        let nodes = self.nodes();
        for node in nodes.iter() {
            let mut links = Vec::new();
            let value = match node.kind() {
                AstKind::VariableDeclarator(declarator) => {
                    let (Some(id), Some(init)) =
                        (declarator.id.get_binding_identifier(), &declarator.init)
                    else {
                        continue;
                    };
                    links.push(Link::Symbol(id.symbol_id()));
                    self.chain(&mut links, init)
                }
                AstKind::AssignmentExpression(assign)
                    if assign.operator == AssignmentOperator::Assign =>
                {
                    self.push_target(&mut links, &assign.left);
                    self.chain(&mut links, &assign.right)
                }
                _ => continue,
            };
            self.record_wrapper(&links, value);
        }
    }

    fn record_wrapper(&mut self, links: &[Link<'_>], value: &Expression<'_>) {
        if let Some(read) = self.export_read(value) {
            for link in links {
                if let Link::Key(key) = link {
                    self.key_exports
                        .entry((*key).to_string())
                        .or_default()
                        .insert(read.to_string());
                }
            }
        }
        for link in links {
            let Link::Symbol(symbol) = link else { continue };
            for other in links {
                if let Link::Key(key) = other {
                    self.symbol_keys
                        .entry(*symbol)
                        .or_default()
                        .insert((*key).to_string());
                }
            }
        }
    }

    fn collect_provided(&mut self) {
        let nodes = self.nodes();
        let mut bound: Vec<(String, Span, &'s ObjectExpression<'s>)> = Vec::new();
        for node in nodes.iter() {
            match node.kind() {
                AstKind::ObjectProperty(property) => {
                    let Some(key) = static_key(&property.key) else {
                        continue;
                    };
                    if !self.modules.contains(key) {
                        continue;
                    }
                    if let Some(object) = self.object_of(&property.value) {
                        bound.push((key.to_string(), object.span, object));
                        if let Some(symbol) = self.symbol_of(&property.value) {
                            self.import_objects.insert(symbol);
                        }
                    }
                }
                AstKind::AssignmentExpression(assign) => self.assigned_import(assign, &mut bound),
                _ => {}
            }
        }
        let mut by_object: FxHashMap<Span, (Vec<String>, &'s ObjectExpression<'s>)> =
            FxHashMap::default();
        for (module, span, object) in bound {
            by_object
                .entry(span)
                .or_insert_with(|| (Vec::new(), object))
                .0
                .push(module);
        }
        let mut groups: Vec<_> = by_object.into_values().collect();
        groups.sort_by_key(|(_, object)| object.span.start);
        for (modules, object) in groups {
            for (field, value) in self.object_fields(object) {
                self.provided.push(Provided {
                    modules: modules.clone(),
                    field,
                    value,
                });
            }
        }
    }

    fn assigned_import(
        &mut self,
        assign: &'s oxc_ast::ast::AssignmentExpression<'s>,
        bound: &mut Vec<(String, Span, &'s ObjectExpression<'s>)>,
    ) {
        let Some(target) = assign.left.as_member_expression() else {
            return;
        };
        let Some(field) = target.static_property_name().and_then(|name| name.as_str()) else {
            return;
        };
        let glue_module =
            |name: &str| self.modules.contains(name) && name.len() >= MIN_GLUE_MODULE_LEN;
        if glue_module(field) {
            if let Expression::ObjectExpression(object) = assign.right.without_parentheses() {
                bound.push((field.to_string(), object.span, object));
            }
            return;
        }
        let Some(module) = target
            .object()
            .without_parentheses()
            .as_member_expression()
            .and_then(|inner| inner.static_property_name().and_then(|name| name.as_str()))
        else {
            return;
        };
        if glue_module(module) {
            self.provided.push(Provided {
                modules: vec![module.to_string()],
                field: field.to_string(),
                value: self.symbol_of(&assign.right),
            });
        }
    }

    fn provides(&self, import: &WasmImport) -> bool {
        self.provided.iter().any(|provided| {
            provided.field == import.name && provided.modules.contains(&import.module)
        })
    }

    fn unprovided(&self, summary: &WasmSummary) -> Vec<String> {
        if summary.has_function_names() {
            return Vec::new();
        }
        let bound: FxHashSet<&str> = self
            .provided
            .iter()
            .flat_map(|provided| provided.modules.iter().map(String::as_str))
            .collect();
        summary
            .imports
            .iter()
            .filter(|import| bound.contains(import.module.as_str()) && !self.provides(import))
            .map(|import| format!("{}.{}", import.module, import.name))
            .collect()
    }

    fn import_of<'m>(
        &self,
        provided: &Provided,
        summary: &'m WasmSummary,
    ) -> Option<&'m WasmImport> {
        provided
            .modules
            .iter()
            .find_map(|module| summary.import(module, &provided.field))
    }

    fn library_name(
        &self,
        provided: &Provided,
        import: &WasmImport,
    ) -> Option<Result<String, String>> {
        if chosen_name(&import.name, import.function_name.as_ref()).is_some() {
            return None;
        }
        let known = known_function(self.semantic, provided.value?)?;
        Some(
            match import
                .signature
                .as_ref()
                .is_some_and(|signature| known.fits(signature))
            {
                true => Ok(known.name.to_string()),
                false => Err(format!(
                    "{}.{} ({})",
                    import.module, import.name, known.name
                )),
            },
        )
    }

    fn unfit(&self, summary: &WasmSummary) -> Vec<String> {
        self.provided
            .iter()
            .filter_map(|provided| {
                let import = self.import_of(provided, summary)?;
                self.library_name(provided, import)?.err()
            })
            .collect()
    }

    fn problems(&self, summary: &WasmSummary) -> Vec<String> {
        let mut problems = Vec::new();
        let mut push = |what: &str, list: Vec<String>| {
            if list.is_empty() {
                return;
            }
            let shown: Vec<&str> = list
                .iter()
                .take(MISSING_SHOWN)
                .map(String::as_str)
                .collect();
            problems.push(format!("{} {what}, e.g. {}", list.len(), shown.join(", ")));
        };
        push(
            "glue name(s) missing from the module",
            self.missing(summary),
        );
        push(
            "module import(s) the glue does not provide",
            self.unprovided(summary),
        );
        push(
            "glue function(s) whose module import has another signature",
            self.unfit(summary),
        );
        problems
    }

    fn missing(&self, summary: &WasmSummary) -> Vec<String> {
        let imports = self.provided.iter().filter_map(|provided| {
            let covered = provided
                .modules
                .iter()
                .any(|module| summary.import(module, &provided.field).is_some());
            (!covered).then(|| format!("import {}.{}", provided.modules.join("|"), provided.field))
        });
        let exports = self
            .reads
            .iter()
            .filter(|read| summary.export(read).is_none())
            .map(|read| format!("export {read}"));
        imports.chain(exports).collect()
    }

    fn wanted(&self, summary: &WasmSummary) -> Vec<(SymbolId, String)> {
        let mut wanted = Vec::new();
        for provided in &self.provided {
            let Some(symbol) = provided.value else {
                continue;
            };
            let Some(import) = self.import_of(provided, summary) else {
                continue;
            };
            let name = chosen_name(&import.name, import.function_name.as_ref())
                .or_else(|| self.library_name(provided, import)?.ok());
            if let Some(name) = name {
                wanted.push((symbol, name));
            }
        }
        for (symbol, read) in &self.bindings {
            let export = summary.export(read);
            if let Some(name) =
                export.and_then(|export| chosen_name(&export.name, export.function_name.as_ref()))
            {
                wanted.push((*symbol, name));
            }
        }
        if let [holder] = self.holders.iter().copied().collect::<Vec<_>>()[..] {
            wanted.push((holder, EXPORTS_HOLDER.to_string()));
        }
        wanted.extend(self.wrapper_names(summary));
        wanted.extend(self.kind_names(summary, ExportKind::Memory, MEMORY_BINDING));
        wanted.extend(self.kind_names(summary, ExportKind::Table, TABLE_BINDING));
        if let [object] = self.import_objects.iter().copied().collect::<Vec<_>>()[..] {
            wanted.push((object, IMPORTS_OBJECT.to_string()));
        }
        wanted
    }

    fn wrapper_names(&self, summary: &WasmSummary) -> Vec<(SymbolId, String)> {
        let mut named: Vec<(SymbolId, String)> = self
            .symbol_keys
            .iter()
            .filter_map(|(symbol, keys)| {
                let [key] = keys.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                let [read] = self.key_exports.get(key)?.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                let export = summary.export(read)?;
                (export.kind == ExportKind::Function && is_name(key))
                    .then(|| (*symbol, key.clone()))
            })
            .collect();
        named.sort_by(|a, b| a.1.cmp(&b.1));
        named
    }

    fn kind_names(
        &self,
        summary: &WasmSummary,
        kind: ExportKind,
        name: &str,
    ) -> Option<(SymbolId, String)> {
        let [_] = summary
            .exports
            .iter()
            .filter(|export| export.kind == kind)
            .collect::<Vec<_>>()[..]
        else {
            return None;
        };
        let mut bound: Vec<SymbolId> = self
            .bindings
            .iter()
            .filter(|(_, read)| {
                summary
                    .export(read)
                    .is_some_and(|export| export.kind == kind)
            })
            .map(|(symbol, _)| *symbol)
            .collect();
        bound.sort_unstable();
        bound.dedup();
        match bound[..] {
            [only] => Some((only, name.to_string())),
            _ => None,
        }
    }

    fn naming(&self, summary: &WasmSummary, claimed: &mut Claimed) -> Naming {
        let problems = self.problems(summary);
        if !problems.is_empty() {
            return Naming {
                mismatch: Some(problems.join("; ")),
                ..Naming::default()
            };
        }
        let (edits, applied) = Renamer::new(self.semantic).apply(self.wanted(summary), claimed);
        Naming {
            edits,
            renamed: applied.len(),
            mismatch: None,
        }
    }
}

pub(super) fn name_glue<'s>(
    semantic: &'s Semantic<'s>,
    summary: &WasmSummary,
    claimed: &mut Claimed,
) -> Naming {
    let mut glue = Glue {
        semantic,
        modules: summary.import_modules().into_iter().collect(),
        holders: FxHashSet::default(),
        exports_owners: FxHashSet::default(),
        provided: Vec::new(),
        import_objects: FxHashSet::default(),
        reads: Vec::new(),
        bindings: Vec::new(),
        key_exports: FxHashMap::default(),
        symbol_keys: FxHashMap::default(),
    };
    glue.collect_holders();
    glue.collect_reads();
    glue.collect_bindings();
    glue.collect_wrappers();
    glue.collect_provided();
    glue.naming(summary, claimed)
}
