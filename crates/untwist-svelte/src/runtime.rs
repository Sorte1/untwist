use oxc_ast::ast::{Expression, ImportDeclarationSpecifier, Program, Statement};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
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
}

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
}

impl Runtime {
    pub fn of(program: &Program<'_>, scoping: &Scoping, nodes: &AstNodes<'_>) -> Self {
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
            runtime.hoisted(scoping, nodes);
        }
        runtime
    }

    fn hoisted(&mut self, scoping: &Scoping, nodes: &AstNodes<'_>) {
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
                true => {
                    self.symbols.insert(*only, rt);
                }
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
            _ => None,
        }
    }
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
