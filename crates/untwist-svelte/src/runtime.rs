use oxc_ast::ast::{Expression, ImportDeclarationSpecifier, Program, Statement};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rt {
    FromHtml,
    FromSvg,
    FromMathml,
    Comment,
    Text,
    FirstChild,
    Child,
    OnlyChild,
    Sibling,
    Next,
    Reset,
    RemoveInputDefaults,
    Append,
    Push,
    Pop,
    AppendStyles,
    TemplateEffect,
    SetText,
    SetAttribute,
    SetClass,
    SetStyle,
    SetValue,
    SetChecked,
    Clsx,
    Delegated,
    Event,
    Delegate,
    BindValue,
    BindChecked,
    BindGroup,
    BindThis,
    If,
    Each,
    Index,
    Key,
    Await,
    Html,
    Snippet,
    Noop,
    State,
    Proxy,
    Derived,
    Get,
    Set,
    Update,
    UpdatePre,
    UserEffect,
    UserPreEffect,
    Prop,
    Snapshot,
    UpdateProp,
    UpdatePreProp,
    RemoveTextareaChild,
    Window,
    Document,
    AttributeEffect,
    Transition,
    Animation,
    Element,
    Boundary,
    RestProps,
    SpreadProps,
    Fallback,
    DerivedSafeEqual,
    ToArray,
    SetAttributes,
    ToggleClass,
    Export(u8),
}

pub const EXPORTS: &[(&str, &str)] = &[
    ("fade", "svelte/transition"),
    ("fly", "svelte/transition"),
    ("slide", "svelte/transition"),
    ("scale", "svelte/transition"),
    ("blur", "svelte/transition"),
    ("draw", "svelte/transition"),
    ("flip", "svelte/animate"),
];

const NAMES: &[(&str, &str, Rt)] = &[
    ("from_html", "from_html", Rt::FromHtml),
    ("template", "template", Rt::FromHtml),
    ("from_svg", "from_svg", Rt::FromSvg),
    ("from_mathml", "from_mathml", Rt::FromMathml),
    ("comment", "comment", Rt::Comment),
    ("text", "text", Rt::Text),
    ("first_child", "first_child", Rt::FirstChild),
    ("child", "child", Rt::Child),
    ("only_child", "only_child", Rt::OnlyChild),
    ("sibling", "sibling", Rt::Sibling),
    ("next", "next", Rt::Next),
    ("reset", "reset", Rt::Reset),
    (
        "remove_input_defaults",
        "remove_input_defaults",
        Rt::RemoveInputDefaults,
    ),
    ("append", "append", Rt::Append),
    ("push", "push", Rt::Push),
    ("pop", "pop", Rt::Pop),
    ("append_styles", "append_styles", Rt::AppendStyles),
    ("template_effect", "template_effect", Rt::TemplateEffect),
    ("set_text", "set_text", Rt::SetText),
    ("set_attribute", "set_attribute", Rt::SetAttribute),
    ("set_class", "set_class", Rt::SetClass),
    ("set_style", "set_style", Rt::SetStyle),
    ("set_value", "set_value", Rt::SetValue),
    ("set_checked", "set_checked", Rt::SetChecked),
    ("clsx", "clsx", Rt::Clsx),
    ("delegated", "delegated", Rt::Delegated),
    ("event", "event", Rt::Event),
    ("delegate", "delegate", Rt::Delegate),
    ("bind_value", "bind_value", Rt::BindValue),
    ("bind_checked", "bind_checked", Rt::BindChecked),
    ("bind_group", "bind_group", Rt::BindGroup),
    ("bind_this", "bind_this", Rt::BindThis),
    ("if", "if_block", Rt::If),
    ("each", "each", Rt::Each),
    ("index", "index", Rt::Index),
    ("key", "key", Rt::Key),
    ("await", "await_block", Rt::Await),
    ("html", "html", Rt::Html),
    ("snippet", "snippet", Rt::Snippet),
    ("noop", "noop", Rt::Noop),
    ("state", "state", Rt::State),
    ("proxy", "proxy", Rt::Proxy),
    ("derived", "user_derived", Rt::Derived),
    ("get", "get", Rt::Get),
    ("set", "set", Rt::Set),
    ("update", "update", Rt::Update),
    ("update_pre", "update_pre", Rt::UpdatePre),
    ("user_effect", "user_effect", Rt::UserEffect),
    ("user_pre_effect", "user_pre_effect", Rt::UserPreEffect),
    ("prop", "prop", Rt::Prop),
    ("snapshot", "snapshot", Rt::Snapshot),
    ("update_prop", "update_prop", Rt::UpdateProp),
    ("update_pre_prop", "update_pre_prop", Rt::UpdatePreProp),
    (
        "remove_textarea_child",
        "remove_textarea_child",
        Rt::RemoveTextareaChild,
    ),
    ("window", "$window", Rt::Window),
    ("document", "$document", Rt::Document),
    ("attribute_effect", "attribute_effect", Rt::AttributeEffect),
    ("transition", "transition", Rt::Transition),
    ("animation", "animation", Rt::Animation),
    ("element", "element", Rt::Element),
    ("boundary", "boundary", Rt::Boundary),
    ("rest_props", "rest_props", Rt::RestProps),
    ("spread_props", "spread_props", Rt::SpreadProps),
    ("fallback", "fallback", Rt::Fallback),
    (
        "derived_safe_equal",
        "derived_safe_equal",
        Rt::DerivedSafeEqual,
    ),
    ("to_array", "to_array", Rt::ToArray),
    ("set_attributes", "set_attributes", Rt::SetAttributes),
    ("toggle_class", "toggle_class", Rt::ToggleClass),
];

const ARITY: &[(&str, usize)] = &[("clsx", 1)];

fn exported(name: &str) -> Option<Rt> {
    NAMES
        .iter()
        .find(|(export, _, _)| *export == name)
        .map(|row| row.2)
}

fn sourced(name: &str) -> Option<Rt> {
    NAMES
        .iter()
        .find(|(_, source, _)| *source == name)
        .map(|row| row.2)
}

fn export_named(name: &str) -> Option<Rt> {
    EXPORTS
        .iter()
        .position(|(export, _)| *export == name)
        .map(|at| Rt::Export(at as u8))
}

fn bare(name: &str) -> &str {
    match name.rfind('$') {
        Some(at)
            if at > 0
                && name[at + 1..].bytes().all(|b| b.is_ascii_digit())
                && at + 1 < name.len() =>
        {
            &name[..at]
        }
        _ => name,
    }
}

pub const CLIENT: &str = "svelte/internal/client";

fn names_svelte(source: &str) -> bool {
    source == CLIENT
        || source.starts_with("svelte/internal")
        || source.contains("/svelte/src/internal/")
}

#[derive(Default)]
pub struct Runtime {
    pub namespaces: Vec<SymbolId>,
    pub symbols: FxHashMap<SymbolId, Rt>,
    pub ambiguous: Vec<SymbolId>,
    pub imports_svelte: bool,
    pub inline: FxHashMap<u32, Rt>,
    pub exports: FxHashMap<SymbolId, u8>,
}

impl Runtime {
    pub fn of(program: &Program<'_>, scoping: &Scoping, nodes: &AstNodes<'_>) -> Self {
        let source = program.source_text;
        let mut runtime = Runtime::default();
        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            if !names_svelte(import.source.value.as_str().unwrap_or_default()) {
                continue;
            }
            runtime.imports_svelte = true;
            for specifier in import.specifiers.iter().flatten() {
                match specifier {
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                        runtime.namespaces.push(namespace.local.symbol_id());
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(named) => {
                        let name = named.imported.name();
                        if let Some(rt) = exported(name.as_str()).or_else(|| sourced(name.as_str()))
                        {
                            runtime.symbols.insert(named.local.symbol_id(), rt);
                        }
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {}
                }
            }
        }
        if runtime.namespaces.is_empty() && runtime.symbols.is_empty() {
            runtime.hoisted(source, scoping, nodes);
        }
        if runtime.imports_svelte {
            return runtime;
        }
        let marks = crate::fingerprint::Marks::of(nodes, scoping);
        let prints = crate::fingerprint::bound_functions(source, &marks, scoping, nodes);
        if runtime.is_empty() && runtime.ambiguous.is_empty() {
            runtime.fingerprinted(source, &marks, &prints, scoping, nodes);
        }
        if !runtime.is_empty() {
            runtime.exported(&prints);
        }
        runtime
    }

    fn exported(&mut self, prints: &[(SymbolId, crate::fingerprint::Print)]) {
        let mut found: FxHashMap<u8, Vec<SymbolId>> = FxHashMap::default();
        for (symbol, print) in prints {
            if let Some(Rt::Export(at)) = crate::fingerprints::lookup(print.hash) {
                found.entry(at).or_default().push(*symbol);
            }
        }
        for (at, symbols) in found {
            if let [only] = symbols.as_slice() {
                self.exports.insert(*only, at);
            }
        }
    }

    fn fingerprinted(
        &mut self,
        source: &str,
        marks: &crate::fingerprint::Marks,
        prints: &[(SymbolId, crate::fingerprint::Print)],
        scoping: &Scoping,
        nodes: &AstNodes<'_>,
    ) {
        let mut found: FxHashMap<Rt, Vec<SymbolId>> = FxHashMap::default();
        for (symbol, print) in prints {
            match crate::fingerprints::lookup(print.hash) {
                Some(Rt::Export(_)) | None => {}
                Some(rt) => found.entry(rt).or_default().push(*symbol),
            }
        }
        let Some(templates) = found.get(&Rt::FromHtml).cloned() else {
            return;
        };
        let end = region_end(source, &templates, scoping, nodes);
        for node in nodes.iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
                continue;
            };
            if let Some(rt) = crate::fingerprint::function(source, marks, function, scoping)
                .and_then(|print| crate::fingerprints::lookup(print.hash))
            {
                self.inline.insert(function.span.start, rt);
            }
        }
        for (rt, global) in [(Rt::Window, "window"), (Rt::Document, "document")] {
            found.insert(rt, global_aliases(scoping, nodes, global));
        }
        for (rt, symbols) in found {
            match symbols.as_slice() {
                [only] if in_region(rt, *only, end, scoping) => {
                    self.symbols.insert(*only, rt);
                }
                [_] => {}
                _ => self.ambiguous.extend(symbols),
            }
        }
    }

    fn hoisted(&mut self, source: &str, scoping: &Scoping, nodes: &AstNodes<'_>) {
        let mut candidates: FxHashMap<&str, Vec<SymbolId>> = FxHashMap::default();
        for symbol in scoping.symbol_ids() {
            let name = bare(scoping.symbol_name(symbol));
            if sourced(name).is_some_and(|rt| matches!(rt, Rt::Window | Rt::Document))
                || (sourced(name).is_some() && is_function_binding(scoping, nodes, symbol))
            {
                candidates.entry(name).or_default().push(symbol);
            }
        }
        if !candidates.contains_key("from_html") && !candidates.contains_key("template") {
            return;
        }
        let templates: Vec<SymbolId> = ["from_html", "template"]
            .iter()
            .filter_map(|name| candidates.get(name))
            .flatten()
            .copied()
            .collect();
        let end = region_end(source, &templates, scoping, nodes);
        for (name, mut symbols) in candidates {
            let mut picked = false;
            if let (true, Some((_, arity))) = (
                symbols.len() > 1,
                ARITY.iter().find(|(known, _)| *known == name),
            ) {
                let matching: Vec<SymbolId> = symbols
                    .iter()
                    .copied()
                    .filter(|symbol| param_count(scoping, nodes, *symbol) == Some(*arity))
                    .collect();
                if matching.len() == 1 {
                    self.ambiguous.extend(
                        symbols
                            .iter()
                            .copied()
                            .filter(|symbol| !matching.contains(symbol)),
                    );
                    symbols = matching;
                    picked = true;
                }
            }
            let ([only], Some(rt)) = (symbols.as_slice(), sourced(name)) else {
                self.ambiguous.extend(symbols);
                continue;
            };
            let scope = scoping.symbol_scope_id(*only);
            let twins = scoping
                .get_bindings(scope)
                .keys()
                .filter(|bound| bare(bound.as_str()) == name)
                .count();
            match twins == 1 || picked {
                true if in_region(rt, *only, end, scoping) => {
                    self.symbols.insert(*only, rt);
                }
                true => {}
                false => self.ambiguous.push(*only),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.namespaces.is_empty() && self.symbols.is_empty()
    }

    pub fn resolve(&self, expr: &Expression<'_>, scoping: &Scoping) -> Option<Rt> {
        match expr.without_parentheses() {
            Expression::StaticMemberExpression(member) => {
                let Expression::Identifier(object) = &member.object else {
                    return None;
                };
                let symbol = symbol_of(object, scoping)?;
                self.namespaces
                    .contains(&symbol)
                    .then(|| exported(member.property.name.as_str()))
                    .flatten()
            }
            Expression::Identifier(ident) => self.symbols.get(&symbol_of(ident, scoping)?).copied(),
            Expression::FunctionExpression(function) => {
                self.inline.get(&function.span.start).copied()
            }
            _ => None,
        }
    }
}

fn statements_of<'a>(
    scoping: &Scoping,
    nodes: &AstNodes<'a>,
    scope: ScopeId,
) -> Option<&'a [Statement<'a>]> {
    match nodes.kind(scoping.get_node_id(scope)) {
        AstKind::Program(program) => Some(program.body.as_slice()),
        AstKind::Function(function) => function
            .body
            .as_ref()
            .map(|body| body.statements.as_slice()),
        AstKind::ArrowFunctionExpression(arrow) => arrow
            .get_function_body()
            .map(|body| body.statements.as_slice()),
        _ => None,
    }
}

fn runs_at_top(scoping: &Scoping, from: ScopeId, home: ScopeId) -> bool {
    for scope in scoping.scope_ancestors(from) {
        if scope == home {
            return true;
        }
        if scoping.scope_flags(scope).is_function() {
            return false;
        }
    }
    false
}

fn is_disclosure(source: &str, statement: &Statement<'_>) -> bool {
    if matches!(
        statement,
        Statement::FunctionDeclaration(_) | Statement::ClassDeclaration(_)
    ) {
        return false;
    }
    let span = oxc_span::GetSpan::span(statement);
    let text = &source[span.start as usize..span.end as usize];
    text.contains("__svelte") && text.contains(".v") && text.contains("new Set")
}

fn is_component_shape(statement: &Statement<'_>) -> bool {
    let Statement::FunctionDeclaration(function) = statement else {
        return false;
    };
    function.params.items.first().is_some_and(|param| {
        matches!(&param.pattern, oxc_ast::ast::BindingPattern::BindingIdentifier(ident) if ident.name == "$$anchor")
    })
}

fn in_region(rt: Rt, symbol: SymbolId, end: u32, scoping: &Scoping) -> bool {
    matches!(rt, Rt::Window | Rt::Document) || scoping.symbol_span(symbol).start < end
}

pub fn region_end(
    source: &str,
    templates: &[SymbolId],
    scoping: &Scoping,
    nodes: &AstNodes<'_>,
) -> u32 {
    let Some(first) = templates.first() else {
        return u32::MAX;
    };
    let home = scoping.symbol_scope_id(*first);
    let declared = templates
        .iter()
        .map(|symbol| scoping.symbol_span(*symbol).start)
        .min()
        .unwrap_or(0);
    let mut end = u32::MAX;
    for symbol in templates {
        for reference in scoping.get_resolved_references(*symbol) {
            let node = nodes.get_node(reference.node_id());
            if runs_at_top(scoping, node.scope_id(), home) {
                end = end.min(oxc_span::GetSpan::span(&node.kind()).start);
            }
        }
    }
    for statement in statements_of(scoping, nodes, home).unwrap_or_default() {
        let start = oxc_span::GetSpan::span(statement).start;
        if is_component_shape(statement) || (start > declared && is_disclosure(source, statement)) {
            end = end.min(start);
            break;
        }
    }
    end
}

fn global_aliases(scoping: &Scoping, nodes: &AstNodes<'_>, global: &str) -> Vec<SymbolId> {
    scoping
        .symbol_ids()
        .filter(|symbol| {
            scoping
                .scope_ancestors(scoping.symbol_scope_id(*symbol))
                .count()
                <= 2
        })
        .filter(
            |symbol| match nodes.kind(scoping.symbol_declaration(*symbol)) {
                AstKind::VariableDeclarator(declarator) => declarator.init.is_none(),
                _ => false,
            },
        )
        .filter(|symbol| {
            let mut writes = scoping
                .get_resolved_references(*symbol)
                .filter(|reference| reference.is_write());
            let only = writes.next();
            writes.next().is_none()
                && only.is_some_and(|reference| {
                    matches!(
                        nodes.parent_kind(reference.node_id()),
                        AstKind::AssignmentExpression(assign)
                            if assign.operator.as_str() == "="
                                && matches!(
                                    assign.right.without_parentheses(),
                                    Expression::Identifier(ident)
                                        if ident.name == global
                                            && ident.reference_id.get().is_some_and(|id| {
                                                scoping.get_reference(id).symbol_id().is_none()
                                            })
                                )
                    )
                })
        })
        .collect()
}

pub fn sourced_name(name: &str) -> Option<Rt> {
    sourced(bare(name)).or_else(|| export_named(bare(name)))
}

pub fn symbol_of(
    ident: &oxc_ast::ast::IdentifierReference<'_>,
    scoping: &Scoping,
) -> Option<SymbolId> {
    let reference = ident.reference_id.get()?;
    scoping.get_reference(reference).symbol_id()
}

fn is_function_binding(scoping: &Scoping, nodes: &AstNodes<'_>, symbol: SymbolId) -> bool {
    if scoping.symbol_flags(symbol).is_function() {
        return true;
    }
    match nodes.kind(scoping.symbol_declaration(symbol)) {
        AstKind::VariableDeclarator(declarator) => matches!(
            declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses),
            Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
        ),
        _ => false,
    }
}

fn param_count(scoping: &Scoping, nodes: &AstNodes<'_>, symbol: SymbolId) -> Option<usize> {
    let count = |params: &oxc_ast::ast::FormalParameters<'_>| {
        params.items.len() + usize::from(params.rest.is_some())
    };
    match nodes.kind(scoping.symbol_declaration(symbol)) {
        AstKind::Function(function) => Some(count(&function.params)),
        AstKind::VariableDeclarator(declarator) => match declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
        {
            Some(Expression::ArrowFunctionExpression(arrow)) => Some(count(&arrow.params)),
            Some(Expression::FunctionExpression(function)) => Some(count(&function.params)),
            _ => None,
        },
        _ => None,
    }
}
