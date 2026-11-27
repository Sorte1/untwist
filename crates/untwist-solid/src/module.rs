use std::collections::BTreeMap;

use oxc_ast::ast::{Expression, ImportDeclarationSpecifier};
use oxc_ast::AstKind;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::ast::expr_symbol;
use crate::convert::{fail, Cx, R};
use crate::jsx::indent_rest;
use crate::runtime::Rt;
use crate::{Component, Restore};

pub struct Found {
    pub symbol: Option<SymbolId>,
    pub name: String,
    pub span: Span,
    pub node: NodeId,
    pub arrow: bool,
    pub inline: bool,
}

fn unique(taken: &mut Vec<String>, base: &str) -> String {
    let mut chosen = base.to_string();
    let mut counter = 1;
    while taken.contains(&chosen) {
        counter += 1;
        chosen = format!("{base}{counter}");
    }
    taken.push(chosen.clone());
    chosen
}

fn captures(cx: &Cx<'_, '_>, span: Span, home: ScopeId) -> bool {
    cx.referenced_at(span).any(|(_, symbol)| {
        let scope = cx.scoping.symbol_scope_id(symbol);
        scope != home && !inside(span, cx.scoping.symbol_span(symbol))
    })
}

fn inline_components(
    cx: &Cx<'_, '_>,
    home: ScopeId,
    jsx_calls: &[Span],
    taken: &mut Vec<String>,
) -> Vec<Found> {
    let mut out = Vec::new();
    for node in cx.nodes.iter() {
        let (span, arrow) = match node.kind() {
            AstKind::Function(function) if function.id.is_none() => (function.span, false),
            AstKind::ArrowFunctionExpression(arrow) => (arrow.span, true),
            _ => continue,
        };
        if !jsx_calls.iter().any(|call| inside(span, *call)) {
            continue;
        }
        let parent = cx.nodes.parent_id(node.id());
        let placed = match cx.nodes.kind(parent) {
            AstKind::CallExpression(call) => {
                cx.is(&call.callee, Rt::CreateComponent)
                    && crate::ast::arg(call, 0)
                        .is_some_and(|first| first.without_parentheses().span() == span)
            }
            AstKind::ArrayExpression(_) | AstKind::ObjectProperty(_) => {
                cx.runtime.minified && cx.nodes.get_node(parent).scope_id() == home
            }
            _ => false,
        };
        if !placed || captures(cx, span, home) {
            continue;
        }
        let name = unique(taken, "Component");
        out.push(Found {
            symbol: None,
            name,
            span,
            node: node.id(),
            arrow,
            inline: true,
        });
    }
    out
}

fn function_span(cx: &Cx<'_, '_>, symbol: SymbolId) -> Option<(Span, NodeId, bool)> {
    let declaration = cx.scoping.symbol_declaration(symbol);
    match cx.nodes.kind(declaration) {
        AstKind::Function(function) => Some((function.span, declaration, false)),
        AstKind::VariableDeclarator(declarator) => match declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
        {
            Some(
                init @ (Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)),
            ) => Some((init.span(), declaration, true)),
            _ => None,
        },
        _ => None,
    }
}

pub fn inside(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

pub fn components(cx: &Cx<'_, '_>, home: ScopeId) -> Vec<Found> {
    let mut jsx_calls: Vec<Span> = Vec::new();
    let mut targets: FxHashSet<SymbolId> = FxHashSet::default();
    for node in cx.nodes.iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        if cx.is_template_call(call) {
            jsx_calls.push(call.span);
        }
        if cx.is(&call.callee, Rt::CreateComponent) {
            jsx_calls.push(call.span);
            if let Some(symbol) = crate::ast::arg(call, 0).and_then(|a| expr_symbol(a, cx.scoping))
            {
                targets.insert(symbol);
            }
        }
    }
    let mut symbols: Vec<SymbolId> = cx.scoping.get_bindings(home).values().copied().collect();
    symbols.sort_by_key(|symbol| cx.scoping.symbol_span(*symbol).start);
    let mut out = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    for symbol in symbols {
        if cx.runtime.symbols.contains_key(&symbol) || cx.runtime.ambiguous.contains(&symbol) {
            continue;
        }
        let Some((span, node, arrow)) = function_span(cx, symbol) else {
            continue;
        };
        let name = cx.scoping.symbol_name(symbol);
        let capital = name.starts_with(|c: char| c.is_ascii_uppercase())
            && name.len() >= 3
            && name.chars().any(|c| c.is_ascii_lowercase());
        let has_jsx = jsx_calls.iter().any(|call| inside(span, *call));
        let target = targets.contains(&symbol);
        if !(target || (has_jsx && (capital || cx.runtime.minified))) {
            continue;
        }
        let base = match capital {
            true => name.to_string(),
            false => "Component".to_string(),
        };
        let chosen = unique(&mut taken, &base);
        out.push(Found {
            symbol: Some(symbol),
            name: chosen,
            span,
            node,
            arrow,
            inline: false,
        });
    }
    out.extend(inline_components(cx, home, &jsx_calls, &mut taken));
    out
}

struct Import {
    start: u32,
    source: String,
    default: Option<String>,
    namespace: Option<String>,
    named: Vec<(u32, String)>,
}

#[derive(Default)]
struct Deps {
    api: BTreeMap<&'static str, Vec<&'static str>>,
    imports: Vec<Import>,
    children: Vec<String>,
}

impl Deps {
    fn import_line(import: &Import) -> String {
        let mut parts = Vec::new();
        if let Some(default) = &import.default {
            parts.push(default.clone());
        }
        if let Some(namespace) = &import.namespace {
            parts.push(format!("* as {namespace}"));
        }
        if !import.named.is_empty() {
            let mut named = import.named.clone();
            named.sort();
            let named: Vec<String> = named.into_iter().map(|(_, text)| text).collect();
            parts.push(format!("{{ {} }}", named.join(", ")));
        }
        format!(
            "import {} from {};",
            parts.join(", "),
            crate::jsx::js_string(&import.source)
        )
    }
}

pub fn helper_owner(cx: &Cx<'_, '_>, symbol: SymbolId) -> Option<(Span, NodeId)> {
    let declaration = cx.scoping.symbol_declaration(symbol);
    match cx.nodes.kind(declaration) {
        AstKind::Function(function) => Some((function.span, declaration)),
        AstKind::Class(class) => Some((class.span, declaration)),
        AstKind::VariableDeclarator(_) => {
            let parent = cx.nodes.parent_id(declaration);
            match cx.nodes.kind(parent) {
                AstKind::VariableDeclaration(declaration) => Some((declaration.span, parent)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_function_value(cx: &Cx<'_, '_>, symbol: SymbolId) -> bool {
    match cx.nodes.kind(cx.scoping.symbol_declaration(symbol)) {
        AstKind::Function(_) => true,
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

fn is_stateful_function(cx: &Cx<'_, '_>, symbol: SymbolId) -> bool {
    cx.scoping.get_resolved_references(symbol).any(|reference| {
        let at = cx.nodes.kind(reference.node_id()).span();
        !matches!(
            cx.nodes.parent_kind(reference.node_id()),
            AstKind::CallExpression(call) if call.callee.without_parentheses().span() == at
        )
    })
}

pub fn siblings(cx: &Cx<'_, '_>, home: ScopeId, symbol: SymbolId) -> Vec<SymbolId> {
    let Some((owner, _)) = helper_owner(cx, symbol) else {
        return vec![symbol];
    };
    if !matches!(
        cx.nodes.kind(cx.scoping.symbol_declaration(symbol)),
        AstKind::VariableDeclarator(_)
    ) {
        return vec![symbol];
    }
    cx.scoping
        .get_bindings(home)
        .values()
        .copied()
        .filter(|other| inside(owner, cx.scoping.symbol_span(*other)))
        .collect()
}

fn import_of(cx: &Cx<'_, '_>, symbol: SymbolId, imports: &mut Vec<Import>) -> bool {
    let declaration = cx.scoping.symbol_declaration(symbol);
    let local = cx.scoping.symbol_name(symbol).to_string();
    let kind = cx.nodes.kind(declaration);
    let parent = cx.nodes.parent_id(declaration);
    let AstKind::ImportDeclaration(import) = cx.nodes.kind(parent) else {
        return false;
    };
    let source = import.source.value.as_str().unwrap_or_default().to_string();
    let start = import.span.start;
    let at = match imports.iter().position(|found| found.start == start) {
        Some(at) => at,
        None => {
            imports.push(Import {
                start,
                source,
                default: None,
                namespace: None,
                named: Vec::new(),
            });
            imports.len() - 1
        }
    };
    let entry = &mut imports[at];
    match kind {
        AstKind::ImportDefaultSpecifier(_) => entry.default = Some(local),
        AstKind::ImportNamespaceSpecifier(_) => entry.namespace = Some(local),
        AstKind::ImportSpecifier(named) => {
            let imported = named.imported.name().to_string();
            let text = match imported == local {
                true => local,
                false => format!("{imported} as {local}"),
            };
            entry.named.push((named.span.start, text));
        }
        _ => return false,
    }
    let _ = ImportDeclarationSpecifier::ImportDefaultSpecifier;
    true
}

pub const SHARED: &str = "shared";

pub struct Unit {
    head: Vec<String>,
    helpers: Vec<String>,
    bodies: Vec<String>,
    children: Vec<String>,
    outside: Vec<SymbolId>,
    copied: Vec<SymbolId>,
    pub reads: FxHashSet<SymbolId>,
    pub spans: Vec<Span>,
    shares: bool,
}

fn render_inline(cx: &Cx<'_, '_>, found: &Found) -> R<String> {
    match cx.nodes.kind(found.node) {
        AstKind::Function(function) => {
            let Some(body) = &function.body else {
                return fail("an inline component has no body");
            };
            let head = match (function.r#async, function.generator) {
                (false, false) => "function",
                _ => return fail("an inline component is async or a generator"),
            };
            Ok(format!(
                "{head} {}{} {}",
                found.name,
                cx.rw.render(function.params.span)?,
                cx.rw.render(body.span)?
            ))
        }
        AstKind::ArrowFunctionExpression(arrow) => {
            let params = cx.rw.render(arrow.params.span)?;
            let params = match params.starts_with('(') {
                true => params,
                false => format!("({params})"),
            };
            Ok(format!("{params} => {}", cx.rw.render(arrow.body.span())?))
        }
        _ => fail("an inline component has an unknown shape"),
    }
}

fn unit(
    cx: &Cx<'_, '_>,
    home: ScopeId,
    roots: &[(Span, NodeId)],
    all: &[Found],
    shared: &FxHashSet<SymbolId>,
    into_shared: bool,
) -> R<Unit> {
    let mut bodies = Vec::new();
    for (span, node) in roots {
        cx.collect_node(*node)?;
        let text = match all.iter().find(|found| found.inline && found.span == *span) {
            Some(found) => render_inline(cx, found)?,
            None => cx.rw.render(*span)?,
        };
        bodies.push(indent_rest(&text, ""));
    }
    let excluded: Vec<Span> = all
        .iter()
        .filter(|found| found.inline && !roots.iter().any(|(span, _)| *span == found.span))
        .map(|found| found.span)
        .collect();
    let mut deps = Deps::default();
    let mut included: Vec<Span> = roots.iter().map(|(span, _)| *span).collect();
    let mut handled: FxHashSet<SymbolId> = FxHashSet::default();
    let mut helper_texts: Vec<(u32, String)> = Vec::new();
    let mut state: Vec<SymbolId> = Vec::new();
    let mut shared_names: Vec<String> = Vec::new();
    let mut scanned: Vec<Span> = Vec::new();
    loop {
        let mut seen: Vec<SymbolId> = Vec::new();
        for span in included.iter().filter(|span| !scanned.contains(span)) {
            seen.extend(
                cx.referenced_at(*span)
                    .filter(|(at, _)| !excluded.iter().any(|skip| inside(*skip, *at)))
                    .map(|(_, symbol)| symbol),
            );
        }
        scanned = included.clone();
        let mut fresh = Vec::new();
        for symbol in seen {
            if !handled.insert(symbol) || cx.scoping.symbol_scope_id(symbol) != home {
                continue;
            }
            if cx.templates.contains_key(&symbol) || cx.runtime.ambiguous.contains(&symbol) {
                continue;
            }
            let declared = cx.scoping.symbol_span(symbol);
            if included.iter().any(|span| inside(*span, declared)) {
                continue;
            }
            if let Some(rt) = cx.runtime.symbols.get(&symbol) {
                let (Some((name, module)), false) = (rt.spelling(), cx.runtime.esm) else {
                    continue;
                };
                let names = deps.api.entry(module).or_default();
                if !names.contains(&name) {
                    names.push(name);
                }
                continue;
            }
            if let Some(child) = all.iter().find(|other| other.symbol == Some(symbol)) {
                if !deps.children.contains(&child.name) {
                    deps.children.push(child.name.clone());
                }
                continue;
            }
            if cx.scoping.symbol_flags(symbol).is_import() {
                if import_of(cx, symbol, &mut deps.imports) {
                    continue;
                }
                return fail(format!(
                    "the import `{}` has an unknown shape",
                    cx.scoping.symbol_name(symbol)
                ));
            }
            if shared.contains(&symbol) && !into_shared {
                let name = cx.scoping.symbol_name(symbol).to_string();
                if !shared_names.contains(&name) {
                    shared_names.push(name);
                }
                continue;
            }
            if cx.library.borrow().contains(&symbol) {
                return fail(format!(
                    "`{}` belongs to the bundled library, not to the component",
                    cx.scoping.symbol_name(symbol)
                ));
            }
            let Some((span, node)) = helper_owner(cx, symbol) else {
                return fail(format!(
                    "`{}` is declared outside the component in a way JSX cannot carry",
                    cx.scoping.symbol_name(symbol)
                ));
            };
            if !is_function_value(cx, symbol) || is_stateful_function(cx, symbol) {
                state.push(symbol);
            }
            if included.contains(&span) {
                continue;
            }
            included.push(span);
            fresh.push((span, node));
        }
        if fresh.is_empty() {
            break;
        }
        for (span, node) in fresh {
            cx.collect_node(node)?;
            let text = cx.rw.render(span)?;
            helper_texts.push((span.start, indent_rest(&text, "")));
        }
    }
    for found in all.iter().filter(|found| excluded.contains(&found.span)) {
        if roots.iter().any(|(span, _)| inside(*span, found.span))
            && !deps.children.contains(&found.name)
        {
            deps.children.push(found.name.clone());
        }
    }
    let copied = state.clone();
    let outside: Vec<SymbolId> = state
        .into_iter()
        .filter(|symbol| {
            siblings(cx, home, *symbol).into_iter().any(|sibling| {
                cx.scoping
                    .get_resolved_references(sibling)
                    .any(|reference| {
                        let span = cx.nodes.kind(reference.node_id()).span();
                        !included.iter().any(|outer| inside(*outer, span))
                    })
            })
        })
        .collect();
    helper_texts.sort_by_key(|(start, _)| *start);
    deps.imports.sort_by_key(|import| import.start);
    let mut head: Vec<String> = Vec::new();
    let written: Vec<&str> = bodies
        .iter()
        .chain(helper_texts.iter().map(|(_, text)| text))
        .map(String::as_str)
        .collect();
    for (module, names) in &deps.api {
        let names: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| written.iter().any(|text| mentions(text, name)))
            .collect();
        if names.is_empty() {
            continue;
        }
        head.push(format!(
            "import {{ {} }} from {};",
            names.join(", "),
            crate::jsx::js_string(module)
        ));
    }
    for import in &deps.imports {
        head.push(Deps::import_line(import));
    }
    if !shared_names.is_empty() {
        head.push(format!(
            "import {{ {} }} from \"./{SHARED}.jsx\";",
            shared_names.join(", ")
        ));
    }
    for child in &deps.children {
        head.push(format!("import {child} from \"./{child}.jsx\";"));
    }
    Ok(Unit {
        head,
        helpers: helper_texts.into_iter().map(|(_, text)| text).collect(),
        bodies,
        children: deps.children,
        outside,
        copied,
        reads: handled,
        spans: included,
        shares: !shared_names.is_empty(),
    })
}

fn mentions(text: &str, name: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    text.match_indices(name).any(|(at, _)| {
        !text[..at].ends_with(ident)
            && !text[..at].ends_with('.')
            && !text[at + name.len()..].starts_with(ident)
    })
}

fn assemble(head: &[String], helpers: Vec<String>, main: Vec<String>) -> String {
    let mut sections: Vec<String> = Vec::new();
    if !head.is_empty() {
        sections.push(head.join("\n"));
    }
    sections.extend(helpers);
    sections.extend(main);
    let mut text = sections.join("\n\n");
    text.push('\n');
    text
}

fn main_text(found: &Found, body: &str) -> String {
    match found.arrow || (found.inline && !body.starts_with("function")) {
        true => format!(
            "const {} = {body};\n\nexport default {};",
            found.name, found.name
        ),
        false => format!("export default {body}"),
    }
}

fn written_outside(cx: &Cx<'_, '_>, symbol: SymbolId) -> bool {
    cx.scoping
        .get_resolved_references(symbol)
        .any(|reference| reference.is_write())
}

fn shared_unit(
    cx: &Cx<'_, '_>,
    home: ScopeId,
    shared: &FxHashSet<SymbolId>,
    all: &[Found],
) -> R<(String, Vec<String>)> {
    let mut roots: Vec<(Span, NodeId)> = Vec::new();
    let mut names: Vec<(u32, String)> = Vec::new();
    for symbol in shared {
        if written_outside(cx, *symbol) {
            return fail(format!(
                "the module binding `{}` is shared and assigned",
                cx.scoping.symbol_name(*symbol)
            ));
        }
        let Some(owner) = helper_owner(cx, *symbol) else {
            return fail("a shared binding has an unknown shape");
        };
        if !roots.contains(&owner) {
            roots.push(owner);
        }
        names.push((
            cx.scoping.symbol_span(*symbol).start,
            cx.scoping.symbol_name(*symbol).to_string(),
        ));
    }
    roots.sort_by_key(|(span, _)| span.start);
    names.sort();
    let built = unit(cx, home, &roots, all, shared, true)?;
    let mut main = built.bodies;
    main.push(format!(
        "export {{ {} }};",
        names
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
            .join(", ")
    ));
    Ok((assemble(&built.head, built.helpers, main), built.children))
}

fn is_constant(cx: &Cx<'_, '_>, symbol: SymbolId) -> bool {
    let declaration = cx.scoping.symbol_declaration(symbol);
    let AstKind::VariableDeclarator(declarator) = cx.nodes.kind(declaration) else {
        return false;
    };
    let constant = matches!(cx.nodes.parent_kind(declaration), AstKind::VariableDeclaration(list) if list.kind == oxc_ast::ast::VariableDeclarationKind::Const);
    constant
        && declarator.init.as_ref().is_some_and(|init| {
            matches!(
                init.without_parentheses(),
                Expression::StringLiteral(_)
                    | Expression::NumericLiteral(_)
                    | Expression::BooleanLiteral(_)
                    | Expression::NullLiteral(_)
                    | Expression::BigIntLiteral(_)
            ) || matches!(init.without_parentheses(), Expression::TemplateLiteral(template) if template.expressions.is_empty())
        })
}

fn library(cx: &Cx<'_, '_>, home: ScopeId) -> FxHashSet<SymbolId> {
    let mut reached: FxHashSet<SymbolId> = FxHashSet::default();
    let mut queue: Vec<SymbolId> = cx.runtime.symbols.keys().copied().collect();
    while let Some(symbol) = queue.pop() {
        let Some((span, _)) = helper_owner(cx, symbol) else {
            continue;
        };
        for (_, used) in cx.referenced_at(span) {
            if cx.scoping.symbol_scope_id(used) == home && reached.insert(used) {
                queue.push(used);
            }
        }
    }
    reached
}

struct Done {
    name: String,
    text: String,
    children: Vec<String>,
    reads: FxHashSet<SymbolId>,
}

fn cascade(done: &mut Vec<Done>, kept: &mut Vec<(String, String)>) {
    loop {
        let names: Vec<String> = kept.iter().map(|(name, _)| name.clone()).collect();
        let Some(at) = done
            .iter()
            .position(|entry| entry.children.iter().any(|child| names.contains(child)))
        else {
            return;
        };
        let entry = done.remove(at);
        let child = entry
            .children
            .into_iter()
            .find(|child| names.contains(child))
            .unwrap_or_default();
        if entry.name != SHARED {
            kept.push((
                entry.name,
                format!("its child component {child} is kept as JS"),
            ));
        }
    }
}

pub fn restore_all(cx: &Cx<'_, '_>, home: ScopeId, found: &[Found]) -> Restore {
    if !cx.runtime.esm {
        *cx.library.borrow_mut() = library(cx, home);
    }
    for component in found {
        match component.symbol {
            Some(symbol) if cx.scoping.symbol_name(symbol) != component.name => {
                cx.rw.rename(symbol, component.name.clone());
            }
            Some(_) => {}
            None => cx.name_inline(component.span, &component.name),
        }
    }
    let none = FxHashSet::default();
    let mut shared: FxHashSet<SymbolId> = FxHashSet::default();
    let mut copies: rustc_hash::FxHashMap<u32, usize> = Default::default();
    let mut copied: Vec<SymbolId> = Vec::new();
    for component in found {
        if let Ok(built) = unit(
            cx,
            home,
            &[(component.span, component.node)],
            found,
            &none,
            false,
        ) {
            shared.extend(built.outside);
            let mut owners: Vec<u32> = built
                .copied
                .iter()
                .filter_map(|symbol| helper_owner(cx, *symbol).map(|(owner, _)| owner.start))
                .collect();
            owners.sort_unstable();
            owners.dedup();
            for owner in owners {
                *copies.entry(owner).or_default() += 1;
            }
            copied.extend(built.copied);
        }
    }
    shared.extend(copied.into_iter().filter(|symbol| {
        !is_constant(cx, *symbol)
            && helper_owner(cx, *symbol)
                .is_some_and(|(owner, _)| copies.get(&owner.start).is_some_and(|count| *count > 1))
    }));
    let effects = crate::effects::module_effects(cx, home, found);
    let mut out = Restore::default();
    let mut done: Vec<Done> = Vec::new();
    for component in found {
        let built = unit(
            cx,
            home,
            &[(component.span, component.node)],
            found,
            &shared,
            false,
        )
        .and_then(|built| match built.outside.first() {
            Some(symbol) => fail(format!(
                "it shares the module binding `{}` with code outside it",
                cx.scoping.symbol_name(*symbol)
            )),
            None => match effects
                .iter()
                .find(|effect| effect.touches(cx, home, &built, &shared))
            {
                Some(effect) => fail(format!(
                    "the module statement `{}` changes what it reads",
                    effect.line
                )),
                None => Ok(built),
            },
        });
        match built {
            Ok(built) => {
                let mut children = built.children;
                if built.shares {
                    children.push(SHARED.to_string());
                }
                let main = built
                    .bodies
                    .first()
                    .map(|body| main_text(component, body))
                    .into_iter()
                    .collect();
                let reads = built.reads.intersection(&shared).copied().collect();
                done.push(Done {
                    name: component.name.clone(),
                    text: assemble(&built.head, built.helpers, main),
                    children,
                    reads,
                });
            }
            Err(reason) => out.kept.push((component.name.clone(), reason)),
        }
    }
    cascade(&mut done, &mut out.kept);
    let used: FxHashSet<SymbolId> = done
        .iter()
        .flat_map(|entry| entry.reads.iter().copied())
        .collect();
    if !used.is_empty() {
        match shared_unit(cx, home, &used, found) {
            Ok((text, children)) => done.push(Done {
                name: SHARED.to_string(),
                text,
                children,
                reads: FxHashSet::default(),
            }),
            Err(reason) => {
                let (sharing, rest): (Vec<Done>, Vec<Done>) =
                    done.into_iter().partition(|entry| !entry.reads.is_empty());
                done = rest;
                out.kept.extend(
                    sharing
                        .into_iter()
                        .map(|entry| (entry.name, reason.clone())),
                );
                cascade(&mut done, &mut out.kept);
            }
        }
    }
    out.components = done
        .into_iter()
        .map(|entry| Component {
            name: entry.name,
            text: entry.text,
        })
        .collect();
    out
}
