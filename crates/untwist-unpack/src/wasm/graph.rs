use oxc_ast::ast::{
    Argument, ArrowFunctionBody, CallExpression, Expression, Function, ObjectExpression,
    PropertyKey, Statement,
};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;

use super::holders::holders_of;

const REQUIRE_NAMES: &[&str] = &["require", "__webpack_require__"];
const HASHED_SUFFIX: &str = ".module.wasm";
const MAX_NUMERIC_ID: f64 = 9_007_199_254_740_992.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportValue {
    Required { local: String, export: String },
    Cached { id: String, export: String },
    Local(String),
    Opaque,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlueImport {
    pub module: String,
    pub field: String,
    pub value: ImportValue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModuleFacts {
    pub requires: Vec<(String, String)>,
    pub reads: Vec<(String, String)>,
    pub bindings: Vec<(String, String, String)>,
    pub exports: Vec<(String, String)>,
    pub async_wrapper: Option<(String, Vec<GlueImport>)>,
    pub sync_wrapper: bool,
    pub import_objects: Vec<(String, Vec<GlueImport>)>,
    pub id_hashes: Vec<(String, String)>,
    pub setters: Vec<(String, String)>,
    pub handoffs: Vec<(String, String, String)>,
    pub loose: Vec<(String, String)>,
    pub calls: Vec<(String, String, String)>,
}

fn numeric_name(value: f64) -> Option<String> {
    (value.fract() == 0.0 && (0.0..MAX_NUMERIC_ID).contains(&value)).then(|| format!("{value}"))
}

fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(text) => text.value.as_str().map(str::to_string),
        PropertyKey::NumericLiteral(number) => numeric_name(number.value),
        _ => None,
    }
}

fn string_of<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(text) => text.value.as_str(),
        _ => None,
    }
}

pub(super) fn required_spec<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.without_parentheses() else {
        return None;
    };
    if !REQUIRE_NAMES.contains(&callee.name.as_str()) || call.arguments.len() != 1 {
        return None;
    }
    string_of(call.arguments[0].as_expression()?)
}

fn member_of<'a>(expression: &'a Expression<'a>) -> Option<(&'a Expression<'a>, &'a str)> {
    let member = expression.without_parentheses().as_member_expression()?;
    Some((
        member.object().without_parentheses(),
        member
            .static_property_name()
            .and_then(|name| name.as_str())?,
    ))
}

fn identifier_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

pub(super) fn unchained<'a>(mut expression: &'a Expression<'a>) -> &'a Expression<'a> {
    while let Expression::AssignmentExpression(assign) = expression.without_parentheses() {
        expression = &assign.right;
    }
    expression.without_parentheses()
}

fn returned_identifier<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    let statements = match expression.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &body.statements,
            body => return body.as_expression().and_then(identifier_name),
        },
        Expression::FunctionExpression(function) => &function.body.as_ref()?.statements,
        _ => return None,
    };
    match statements.as_slice() {
        [Statement::ReturnStatement(ret)] => identifier_name(ret.argument.as_ref()?),
        _ => None,
    }
}

fn returned_object<'a>(function: &'a Function<'a>) -> Option<&'a ObjectExpression<'a>> {
    match function.body.as_ref()?.statements.as_slice() {
        [Statement::ReturnStatement(ret)] => match ret.argument.as_ref()?.without_parentheses() {
            Expression::ObjectExpression(object) => Some(object),
            _ => None,
        },
        _ => None,
    }
}

fn cache_id(expression: &Expression<'_>) -> Option<String> {
    let (object, property) = member_of(expression)?;
    if property != "exports" {
        return None;
    }
    let Expression::ComputedMemberExpression(entry) = object else {
        return None;
    };
    match entry.expression.without_parentheses() {
        Expression::NumericLiteral(number) => numeric_name(number.value),
        _ => entry
            .static_property_name()
            .and_then(|name| name.as_str())
            .map(str::to_string),
    }
}

struct Collector<'s> {
    nodes: &'s AstNodes<'s>,
    facts: ModuleFacts,
}

impl<'s> Collector<'s> {
    fn within(&self, span: Span) -> impl Iterator<Item = AstKind<'s>> + '_ {
        self.nodes
            .iter()
            .map(|node| node.kind())
            .filter(move |kind| {
                let inner = kind.span();
                inner.start >= span.start && inner.end <= span.end
            })
    }

    fn cached_call(&self, function: Span) -> ImportValue {
        let mut aliases: Vec<(&str, String)> = Vec::new();
        for kind in self.within(function) {
            if let AstKind::AssignmentExpression(assign) = kind {
                let target = match &assign.left {
                    oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id) => {
                        id.name.as_str()
                    }
                    _ => continue,
                };
                if let Some(id) = cache_id(&assign.right) {
                    aliases.push((target, id));
                }
            }
        }
        for kind in self.within(function) {
            let AstKind::CallExpression(call) = kind else {
                continue;
            };
            let Some((object, export)) = member_of(&call.callee) else {
                continue;
            };
            let id = cache_id(object).or_else(|| {
                let name = identifier_name(object)?;
                aliases
                    .iter()
                    .find(|(alias, _)| *alias == name)
                    .map(|(_, id)| id.clone())
            });
            if let Some(id) = id {
                return ImportValue::Cached {
                    id,
                    export: export.to_string(),
                };
            }
        }
        ImportValue::Opaque
    }

    fn import_value(&self, value: &'s Expression<'s>) -> ImportValue {
        if let Some(name) = identifier_name(value) {
            return ImportValue::Local(name.to_string());
        }
        if let Some((object, export)) = member_of(value) {
            if let Some(local) = identifier_name(object) {
                return ImportValue::Required {
                    local: local.to_string(),
                    export: export.to_string(),
                };
            }
        }
        match value.without_parentheses() {
            Expression::FunctionExpression(function) => self.cached_call(function.span),
            Expression::ArrowFunctionExpression(arrow) => self.cached_call(arrow.span),
            _ => ImportValue::Opaque,
        }
    }

    fn glue_imports(&self, object: &'s ObjectExpression<'s>) -> Vec<GlueImport> {
        let mut imports = Vec::new();
        for property in object.properties.iter().filter_map(|p| p.as_property()) {
            let Some(module) = key_name(&property.key) else {
                continue;
            };
            let Expression::ObjectExpression(fields) = property.value.without_parentheses() else {
                continue;
            };
            for field in fields.properties.iter().filter_map(|p| p.as_property()) {
                let Some(name) = key_name(&field.key) else {
                    continue;
                };
                imports.push(GlueImport {
                    module: module.clone(),
                    field: name,
                    value: self.import_value(&field.value),
                });
            }
        }
        imports
    }

    fn call(&mut self, call: &'s CallExpression<'s>) {
        let Some((_, property)) = member_of(&call.callee) else {
            return;
        };
        let arguments: Vec<Option<&'s Expression<'s>>> =
            call.arguments.iter().map(Argument::as_expression).collect();
        match (property, arguments.as_slice()) {
            ("v", [_, _, Some(hash), Some(Expression::ObjectExpression(imports)), ..]) => {
                if let Some(hash) = string_of(hash) {
                    let imports = self.glue_imports(imports);
                    self.facts.async_wrapper = Some((hash.to_string(), imports));
                }
            }
            ("d", [_, Some(Expression::ObjectExpression(definitions))]) => {
                for property in definitions
                    .properties
                    .iter()
                    .filter_map(|p| p.as_property())
                {
                    if let (Some(export), Some(local)) = (
                        key_name(&property.key),
                        returned_identifier(&property.value),
                    ) {
                        self.facts.exports.push((export, local.to_string()));
                    }
                }
            }
            ("d", [_, Some(export), Some(getter)]) => {
                if let (Some(export), Some(local)) =
                    (string_of(export), returned_identifier(getter))
                {
                    self.facts
                        .exports
                        .push((export.to_string(), local.to_string()));
                }
            }
            _ => {}
        }
    }

    fn hashed(&mut self, left: &'s Expression<'s>) {
        let mut operand = left.without_parentheses();
        if let Expression::BinaryExpression(inner) = operand {
            operand = inner.right.without_parentheses();
        }
        let mut object = operand;
        while let Expression::ComputedMemberExpression(member) = object {
            object = member.object.without_parentheses();
        }
        let Expression::ObjectExpression(map) = object else {
            return;
        };
        let mut pending = vec![map];
        while let Some(map) = pending.pop() {
            for property in map.properties.iter().filter_map(|p| p.as_property()) {
                match (
                    key_name(&property.key),
                    property.value.without_parentheses(),
                ) {
                    (Some(id), Expression::StringLiteral(hash)) => {
                        if let Some(hash) = hash.value.as_str() {
                            self.facts.id_hashes.push((id, hash.to_string()));
                        }
                    }
                    (_, Expression::ObjectExpression(nested)) => pending.push(nested),
                    _ => {}
                }
            }
        }
    }

    fn visit(&mut self) {
        let nodes = self.nodes;
        for node in nodes.iter() {
            match node.kind() {
                AstKind::VariableDeclarator(declarator) => {
                    let (Some(id), Some(init)) =
                        (declarator.id.get_binding_identifier(), &declarator.init)
                    else {
                        continue;
                    };
                    if let Some(spec) = required_spec(init) {
                        self.facts
                            .requires
                            .push((id.name.to_string(), spec.to_string()));
                    }
                }
                AstKind::AssignmentExpression(assign) => {
                    if let (
                        oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id),
                        Some(spec),
                    ) = (&assign.left, required_spec(&assign.right))
                    {
                        self.facts
                            .requires
                            .push((id.name.to_string(), spec.to_string()));
                    }
                }
                AstKind::ComputedMemberExpression(member) => {
                    let instance = member_of(&member.expression)
                        .and_then(|(object, key)| identifier_name(object).map(|name| (name, key)));
                    if matches!(member_of(&member.object), Some((_, "w")))
                        && matches!(instance, Some(("module", "id" | "i")))
                    {
                        self.facts.sync_wrapper = true;
                    }
                }
                AstKind::CallExpression(call) => self.call(call),
                AstKind::ObjectProperty(property) => {
                    let (Some(id), Expression::FunctionExpression(function)) = (
                        key_name(&property.key),
                        property.value.without_parentheses(),
                    ) else {
                        continue;
                    };
                    if let Some(object) = returned_object(function) {
                        let imports = self.glue_imports(object);
                        if !imports.is_empty() {
                            self.facts.import_objects.push((id, imports));
                        }
                    }
                }
                AstKind::BinaryExpression(binary)
                    if binary.operator == BinaryOperator::Addition
                        && string_of(&binary.right) == Some(HASHED_SUFFIX) =>
                {
                    self.hashed(&binary.left);
                }
                AstKind::ExportSpecifier(specifier) => {
                    self.facts.exports.push((
                        specifier.exported.name().to_string(),
                        specifier.local.name().to_string(),
                    ));
                }
                _ => {}
            }
        }
    }
}

pub fn facts_of(semantic: &Semantic<'_>) -> ModuleFacts {
    let mut collector = Collector {
        nodes: semantic.nodes(),
        facts: ModuleFacts::default(),
    };
    collector.visit();
    let held = holders_of(semantic);
    ModuleFacts {
        reads: held.reads,
        bindings: held.bindings,
        setters: held.setters,
        handoffs: held.handoffs,
        loose: held.loose,
        calls: held.calls,
        ..collector.facts
    }
}
