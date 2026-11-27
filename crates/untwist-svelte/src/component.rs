use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, Expression, Function, ObjectPropertyKind, Program,
    Statement,
};
use oxc_ast::AstKind;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::css;
use crate::dom::{first_line, is_identifier, Code, Cx, R};
use crate::markup::{print, M};
use crate::runtime::Rt;
use crate::script::{arg, binding, component_params, string, thunk_body, Facts};
use crate::text::Rewriter;
use oxc_semantic::{AstNodes, Scoping};
use rustc_hash::FxHashMap;

pub struct Shared<'a, 's> {
    pub program: &'a Program<'a>,
    pub scoping: &'a Scoping,
    pub nodes: &'a AstNodes<'a>,
    pub runtime: &'a crate::runtime::Runtime,
    pub facts: &'a Facts,
    pub rw: &'a Rewriter<'s>,
    pub keys: &'a FxHashMap<SymbolId, Vec<String>>,
    pub components: Vec<(&'a Function<'a>, String)>,
}

fn fail<T>(message: impl Into<String>) -> R<T> {
    Err(message.into())
}

const DOM_CALLS: &[Rt] = &[
    Rt::Append,
    Rt::Next,
    Rt::Reset,
    Rt::RemoveInputDefaults,
    Rt::Pop,
    Rt::TemplateEffect,
    Rt::SetText,
    Rt::SetAttribute,
    Rt::SetClass,
    Rt::SetStyle,
    Rt::SetValue,
    Rt::SetChecked,
    Rt::Delegated,
    Rt::Event,
    Rt::BindValue,
    Rt::BindChecked,
    Rt::BindGroup,
    Rt::BindThis,
    Rt::If,
    Rt::Each,
    Rt::Key,
    Rt::Await,
    Rt::Html,
    Rt::Snippet,
];

const WALKS: &[Rt] = &[
    Rt::Comment,
    Rt::Text,
    Rt::FirstChild,
    Rt::Child,
    Rt::OnlyChild,
    Rt::Sibling,
];

struct Css {
    hash: String,
    code: String,
}

struct Prop {
    local: String,
    key: String,
    fallback: Option<String>,
    bindable: bool,
}

impl Prop {
    fn text(&self) -> String {
        let mut text = match (self.local == self.key, is_identifier(&self.key)) {
            (true, true) => self.key.clone(),
            (_, true) => format!("{}: {}", self.key, self.local),
            (_, false) => format!("'{}': {}", self.key.replace('\'', "\\'"), self.local),
        };
        match (&self.fallback, self.bindable) {
            (Some(fallback), true) => text.push_str(&format!(" = $bindable({fallback})")),
            (None, true) => text.push_str(" = $bindable()"),
            (Some(fallback), false) => text.push_str(&format!(" = {fallback}")),
            (None, false) => {}
        }
        text
    }
}

fn spaced(items: &[String]) -> String {
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            let wide = item.contains('\n') || items[index - 1].contains('\n');
            let after_imports = items[index - 1].trim_start().starts_with("import ")
                && !item.trim_start().starts_with("import ");
            out.push_str(if wide || after_imports { "\n\n" } else { "\n" });
        }
        out.push_str(item);
    }
    out
}

fn indented(source: &str, span: Span, text: &str) -> String {
    let start = span.start as usize;
    let column = source[..start].rsplit('\n').next().map_or(0, str::len);
    if text.contains('`') {
        return format!("\t{text}");
    }
    let mut lines = text.lines();
    let mut out = format!("\t{}", lines.next().unwrap_or_default());
    for line in lines {
        let cut = line.len() - line.trim_start().len();
        let line = &line[cut.min(column)..];
        let spaces = line.len() - line.trim_start_matches(' ').len();
        out.push_str(&format!(
            "\n\t{}{}",
            "\t".repeat(spaces / 2),
            &line[spaces - spaces % 2..]
        ));
    }
    out
}

impl<'a, 's> Shared<'a, 's> {
    pub fn cx(&self, hash: Option<String>) -> Cx<'a, 's> {
        Cx {
            scoping: self.scoping,
            nodes: self.nodes,
            runtime: self.runtime,
            facts: self.facts,
            rw: self.rw,
            hash,
            snippets: Default::default(),
            children: Default::default(),
            specials: Default::default(),
            components: self
                .components
                .iter()
                .map(|(function, name)| (function.span, name.clone()))
                .collect(),
            consumed: Default::default(),
            scoped: Default::default(),
        }
    }

    fn is_template_call(&self, cx: &Cx<'a, 's>, expr: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        if let Some(rt) = cx.rt(&call.callee) {
            return WALKS.contains(&rt);
        }
        let Expression::Identifier(ident) = &call.callee else {
            return false;
        };
        crate::runtime::symbol_of(ident, cx.scoping)
            .and_then(|symbol| cx.declarator(symbol))
            .and_then(|declarator| declarator.init.as_ref())
            .is_some_and(|init| matches!(init.without_parentheses(), Expression::CallExpression(inner) if matches!(cx.rt(&inner.callee), Some(Rt::FromHtml | Rt::FromSvg))))
    }

    fn is_dom(&self, cx: &Cx<'a, 's>, statement: &Statement<'_>) -> bool {
        match statement {
            Statement::VariableDeclaration(declaration) => declaration
                .declarations
                .first()
                .and_then(|declarator| declarator.init.as_ref())
                .is_some_and(|init| self.is_template_call(cx, init)),
            Statement::BlockStatement(_) => true,
            Statement::ExpressionStatement(stmt) => match stmt.expression.without_parentheses() {
                Expression::CallExpression(call) => cx
                    .rt(&call.callee)
                    .is_some_and(|rt| DOM_CALLS.contains(&rt)),
                _ => false,
            },
            _ => false,
        }
    }

    fn css(&self, cx: &Cx<'a, 's>, expr: Option<&Expression<'_>>) -> R<Css> {
        let symbol = match expr.map(Expression::without_parentheses) {
            Some(Expression::Identifier(ident)) => crate::runtime::symbol_of(ident, cx.scoping),
            _ => None,
        }
        .ok_or("append_styles() without a style object")?;
        let init = cx
            .declarator(symbol)
            .and_then(|declarator| declarator.init.as_ref())
            .ok_or("a style object that is not declared here")?;
        let Expression::ObjectExpression(object) = init.without_parentheses() else {
            return fail("a style object of an unknown shape");
        };
        let (mut hash, mut code) = (None, None);
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return fail("a style object of an unknown shape");
            };
            match property
                .key
                .static_name()
                .as_ref()
                .and_then(|name| name.as_str())
            {
                Some("hash") => hash = string(&property.value).map(str::to_string),
                Some("code") => code = string(&property.value).map(str::to_string),
                _ => return fail("a style object of an unknown shape"),
            }
        }
        match (hash, code) {
            (Some(hash), Some(code)) if hash.starts_with("svelte-") => Ok(Css { hash, code }),
            _ => fail("a style object of an unknown shape"),
        }
    }

    fn prop(&self, cx: &Cx<'a, 's>, symbol: SymbolId) -> R<Prop> {
        let decl = cx
            .facts
            .props
            .get(&symbol)
            .ok_or("a prop of an unknown shape")?;
        if decl.flags & !31 != 0 {
            return fail("a prop with unknown flags");
        }
        let fallback = match decl.fallback {
            None => None,
            Some(_) if decl.flags & 16 != 0 => {
                let call = cx
                    .declarator(symbol)
                    .and_then(|declarator| declarator.init.as_ref());
                let Some(Expression::CallExpression(call)) =
                    call.map(Expression::without_parentheses)
                else {
                    return fail("a lazy prop fallback of an unknown shape");
                };
                let thunk = arg(call, 3)
                    .and_then(thunk_body)
                    .ok_or("a lazy prop fallback of an unknown shape")?;
                Some(cx.render(thunk)?)
            }
            Some(span) => Some(cx.render(span)?),
        };
        Ok(Prop {
            local: cx.scoping.symbol_name(symbol).to_string(),
            key: decl.key.clone(),
            fallback,
            bindable: decl.flags & 8 != 0,
        })
    }

    fn clashes(&self, cx: &Cx<'a, 's>, scope: ScopeId, name: &str) -> bool {
        cx.scoping.symbol_ids().any(|symbol| {
            cx.scoping.symbol_name(symbol) == name
                && cx
                    .scoping
                    .scope_ancestors(cx.scoping.symbol_scope_id(symbol))
                    .any(|ancestor| ancestor == scope)
        })
    }

    fn snippets(&self, cx: &Cx<'a, 's>) -> R<Vec<M>> {
        let mut done = Vec::new();
        let mut out = Vec::new();
        loop {
            let pending: Vec<SymbolId> = cx
                .snippets
                .borrow()
                .iter()
                .filter(|symbol| !done.contains(*symbol))
                .copied()
                .collect();
            if pending.is_empty() {
                return Ok(out);
            }
            for symbol in pending {
                done.push(symbol);
                out.push(cx.snippet_block(symbol)?);
            }
        }
    }

    fn is_component_declaration(&self, statement: &Statement<'_>) -> bool {
        let function = match statement {
            Statement::FunctionDeclaration(function) => Some(&**function),
            Statement::ExportDefaultDeclaration(export) => match &export.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => Some(&**function),
                _ => None,
            },
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::FunctionDeclaration(function) => Some(&**function),
                _ => None,
            },
            _ => None,
        };
        function.is_some_and(|function| {
            self.components
                .iter()
                .any(|(known, _)| std::ptr::eq(*known, function))
        })
    }

    fn is_machinery(
        &self,
        cx: &Cx<'a, 's>,
        statement: &Statement<'_>,
        used: &FxHashSet<SymbolId>,
    ) -> bool {
        match statement {
            Statement::ImportDeclaration(import) => import
                .source
                .value
                .as_str()
                .unwrap_or_default()
                .starts_with("svelte/internal"),
            Statement::VariableDeclaration(declaration) => {
                declaration.declarations.iter().all(|declarator| {
                    let symbol = binding(&declarator.id);
                    symbol.is_some_and(|symbol| used.contains(&symbol))
                        || declarator.init.as_ref().is_some_and(|init| {
                            match init.without_parentheses() {
                                Expression::CallExpression(call) => matches!(
                                    cx.rt(&call.callee),
                                    Some(Rt::FromHtml | Rt::FromSvg | Rt::FromMathml)
                                ),
                                Expression::ObjectExpression(object) => {
                                    symbol.is_some_and(|symbol| {
                                        cx.scoping.symbol_name(symbol) == "$$css"
                                    }) && object.properties.len() == 2
                                }
                                _ => false,
                            }
                        })
                })
            }
            Statement::ExpressionStatement(stmt) => {
                matches!(stmt.expression.without_parentheses(), Expression::CallExpression(call) if cx.rt(&call.callee) == Some(Rt::Delegate))
            }
            _ => self.is_component_declaration(statement),
        }
    }

    fn module_parts(
        &self,
        cx: &Cx<'a, 's>,
        used: &FxHashSet<SymbolId>,
    ) -> R<(Vec<String>, Vec<String>)> {
        let source = self.program.source_text;
        let (mut imports, mut module) = (Vec::new(), Vec::new());
        for statement in &self.program.body {
            if self.is_machinery(cx, statement, used) {
                continue;
            }
            let text = cx.render(statement.span()).map_err(|reason| {
                format!(
                    "{reason} (in `{}`)",
                    first_line(cx.rw.text(statement.span()))
                )
            })?;
            match statement {
                Statement::ImportDeclaration(_) => {
                    imports.push(indented(source, statement.span(), &text))
                }
                _ => module.push(indented(source, statement.span(), &text)),
            }
        }
        Ok((imports, module))
    }

    fn child_imports(&self, cx: &Cx<'a, 's>) -> Vec<String> {
        let mut out = Vec::new();
        for symbol in cx.children.borrow().iter() {
            if let Some(name) = cx.component_name(*symbol) {
                out.push(format!("\timport {name} from './{name}.svelte';"));
            }
        }
        out
    }

    pub fn restore(&self, function: &'a Function<'a>, esm: bool) -> R<(String, Vec<String>)> {
        self.rw.seen.borrow_mut().clear();
        let base = self.cx(None);
        let (anchor, props) = component_params(function, self.runtime, self.scoping)
            .ok_or("not a component function")?;
        let statements = &function
            .body
            .as_ref()
            .ok_or("a component without a body")?
            .statements;
        let mut parts = Parts {
            anchor,
            props,
            css: None,
            prop_symbols: Vec::new(),
            rest: None,
            script: Vec::new(),
            pushed: false,
            dom_start: statements.len(),
        };
        for (index, statement) in statements.iter().enumerate() {
            if self.is_dom(&base, statement) {
                parts.dom_start = index;
                break;
            }
            if self.preamble(&base, statement, &mut parts)? {
                continue;
            }
            parts.script.push(statement);
        }
        let cx = self.cx(parts.css.as_ref().map(|css| css.hash.clone()));
        let text = self.finish(&cx, function, statements, parts, esm)?;
        if !esm {
            self.closed(&cx, function)?;
        }
        let children = cx
            .children
            .borrow()
            .iter()
            .filter_map(|symbol| cx.component_name(*symbol).map(str::to_string))
            .collect();
        Ok((text, children))
    }

    fn export_imports(&self, cx: &Cx<'a, 's>, function: &Function<'_>) -> R<Vec<String>> {
        let scope = function
            .scope_id
            .get()
            .ok_or("a component without a scope")?;
        let mut modules: Vec<(&str, Vec<&str>)> = Vec::new();
        let seen = self.rw.seen.borrow();
        let mut used: Vec<(&SymbolId, &u8)> = self
            .runtime
            .exports
            .iter()
            .filter(|(symbol, _)| seen.contains(symbol))
            .collect();
        used.sort_by_key(|(_, at)| **at);
        for (symbol, at) in used {
            let (name, module) = crate::runtime::EXPORTS[usize::from(*at)];
            if cx.scoping.symbol_name(*symbol) != name && self.clashes(cx, scope, name) {
                return fail(format!("the import `{name}` would clash with a binding"));
            }
            match modules.iter_mut().find(|(known, _)| *known == module) {
                Some((_, names)) => names.push(name),
                None => modules.push((module, vec![name])),
            }
        }
        Ok(modules
            .into_iter()
            .map(|(module, names)| format!("\timport {{ {} }} from '{module}';", names.join(", ")))
            .collect())
    }

    fn closed(&self, cx: &Cx<'a, 's>, function: &Function<'_>) -> R<()> {
        let scope = function
            .scope_id
            .get()
            .ok_or("a component without a scope")?;
        let enclosing: Vec<ScopeId> = cx.scoping.scope_ancestors(scope).skip(1).collect();
        let allowed: FxHashSet<SymbolId> = cx
            .snippets
            .borrow()
            .iter()
            .chain(cx.children.borrow().iter())
            .chain(cx.facts.handlers.iter())
            .chain(self.runtime.exports.keys())
            .copied()
            .collect();
        let seen = self.rw.seen.borrow();
        let outside = seen.iter().find(|symbol| {
            !allowed.contains(symbol)
                && enclosing.contains(&cx.scoping.symbol_scope_id(**symbol))
                && !self.components.iter().any(|(function, _)| {
                    function.id.as_ref().and_then(|id| id.symbol_id.get()) == Some(**symbol)
                })
        });
        match outside {
            Some(symbol) => fail(format!(
                "`{}` is declared outside the component in the bundle (`{}`)",
                cx.scoping.symbol_name(*symbol),
                first_line(
                    cx.rw
                        .text(cx.nodes.kind(cx.scoping.symbol_declaration(*symbol)).span())
                )
            )),
            None => Ok(()),
        }
    }

    fn preamble(
        &self,
        cx: &Cx<'a, 's>,
        statement: &'a Statement<'a>,
        parts: &mut Parts<'a>,
    ) -> R<bool> {
        if let Statement::ExpressionStatement(stmt) = statement {
            if let Expression::CallExpression(call) = stmt.expression.without_parentheses() {
                match cx.rt(&call.callee) {
                    Some(Rt::Push) => {
                        let runes = matches!(arg(call, 1).map(Expression::without_parentheses), Some(Expression::BooleanLiteral(literal)) if literal.value);
                        if !runes || parts.props.is_none() {
                            return fail("a legacy (non-runes) component");
                        }
                        parts.pushed = true;
                        return Ok(true);
                    }
                    Some(Rt::AppendStyles) => {
                        parts.css = Some(self.css(cx, arg(call, 1))?);
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        if let Statement::VariableDeclaration(declaration) = statement {
            if self.is_group_list(cx, declaration) {
                return Ok(true);
            }
            let mut props = Vec::new();
            let mut rest = None;
            for declarator in &declaration.declarations {
                let Some(symbol) = binding(&declarator.id) else {
                    return Ok(false);
                };
                if cx.facts.props.contains_key(&symbol) {
                    props.push(symbol);
                    continue;
                }
                let Some(Expression::CallExpression(call)) = declarator
                    .init
                    .as_ref()
                    .map(Expression::without_parentheses)
                else {
                    return Ok(false);
                };
                let props_arg = arg(call, 0).and_then(|props| match props {
                    Expression::Identifier(ident) => crate::runtime::symbol_of(ident, cx.scoping),
                    _ => None,
                });
                if cx.rt(&call.callee) != Some(Rt::RestProps)
                    || rest.is_some()
                    || parts.props.is_none()
                    || call.arguments.len() != 2
                    || props_arg != parts.props
                {
                    return Ok(false);
                }
                let excludes = arg(call, 1).ok_or("rest props without an exclusion list")?;
                let (list, holder) = match excludes.without_parentheses() {
                    Expression::Identifier(ident) => {
                        let holder = crate::runtime::symbol_of(ident, cx.scoping)
                            .ok_or("rest props with an exclusion list of an unknown shape")?;
                        (self.excluded(cx, holder)?, Some(holder))
                    }
                    Expression::ArrayExpression(array) => (strings(array)?, None),
                    _ => return fail("rest props with an exclusion list of an unknown shape"),
                };
                rest = Some((symbol, list, holder));
            }
            if rest.is_some() && parts.rest.is_some() {
                return fail("two rest props declarations");
            }
            cx.rw
                .forbid_some(rest.as_ref().and_then(|(_, _, holder)| *holder));
            if rest.is_some() {
                parts.rest = rest;
            }
            parts.prop_symbols.extend(props);
            return Ok(true);
        }
        Ok(false)
    }

    fn is_group_list(
        &self,
        cx: &Cx<'a, 's>,
        declaration: &oxc_ast::ast::VariableDeclaration<'_>,
    ) -> bool {
        let [declarator] = declaration.declarations.as_slice() else {
            return false;
        };
        let empty = matches!(declarator.init.as_ref().map(Expression::without_parentheses), Some(Expression::ArrayExpression(array)) if array.elements.is_empty());
        let Some(symbol) = binding(&declarator.id).filter(|_| empty) else {
            return false;
        };
        let mut references = cx.scoping.get_resolved_references(symbol).peekable();
        references.peek().is_some()
            && references.all(
                |reference| match cx.nodes.parent_kind(reference.node_id()) {
                    AstKind::CallExpression(call) => {
                        cx.rt(&call.callee) == Some(Rt::BindGroup)
                            && arg(call, 0).map(GetSpan::span)
                                == Some(cx.nodes.kind(reference.node_id()).span())
                    }
                    _ => false,
                },
            )
            && {
                cx.rw.forbid(symbol);
                true
            }
    }

    fn props_line(
        &self,
        cx: &Cx<'a, 's>,
        function: &Function<'_>,
        parts: &Parts<'a>,
    ) -> R<Vec<String>> {
        let mut line = Vec::new();
        for symbol in &parts.prop_symbols {
            line.push(self.prop(cx, *symbol)?.text());
        }
        let Some(props) = parts.props else {
            return Ok(line);
        };
        let scope = function
            .scope_id
            .get()
            .ok_or("a component without a scope")?;
        let mut seen: Vec<String> = parts
            .prop_symbols
            .iter()
            .filter_map(|symbol| cx.facts.props.get(symbol))
            .map(|prop| prop.key.clone())
            .collect();
        for key in self.keys.get(&props).into_iter().flatten() {
            if seen.contains(key) {
                continue;
            }
            if !is_identifier(key) || self.clashes(cx, scope, key) {
                return fail(format!(
                    "the prop `{key}` would clash with a binding of the same name"
                ));
            }
            line.push(key.clone());
            seen.push(key.clone());
        }
        if let Some((rest, excluded, _)) = &parts.rest {
            let rest = *rest;
            if let Some(key) = seen.iter().find(|key| !excluded.contains(key)) {
                return fail(format!(
                    "the prop `{key}` is read but also lands in the rest props"
                ));
            }
            for key in excluded.iter().filter(|key| !key.starts_with("$$")) {
                if seen.contains(key) {
                    continue;
                }
                if !is_identifier(key) || self.clashes(cx, scope, key) {
                    return fail(format!(
                        "the prop `{key}` would clash with a binding of the same name"
                    ));
                }
                line.push(key.clone());
            }
            line.push(format!("...{}", cx.scoping.symbol_name(rest)));
        }
        Ok(line)
    }

    fn excluded(&self, cx: &Cx<'a, 's>, excludes: SymbolId) -> R<Vec<String>> {
        let shape = "rest props with an exclusion list of an unknown shape";
        let init = cx
            .declarator(excludes)
            .and_then(|declarator| declarator.init.as_ref())
            .ok_or(shape)?;
        let Expression::NewExpression(new) = init.without_parentheses() else {
            return fail(shape);
        };
        if !matches!(&new.callee, Expression::Identifier(ident) if ident.name == "Set" && crate::runtime::symbol_of(ident, cx.scoping).is_none())
        {
            return fail(shape);
        }
        let [oxc_ast::ast::Argument::ArrayExpression(array)] = new.arguments.as_slice() else {
            return fail(shape);
        };
        strings(array)
    }

    fn finish(
        &self,
        cx: &Cx<'a, 's>,
        function: &'a Function<'a>,
        statements: &'a [Statement<'a>],
        parts: Parts<'a>,
        esm: bool,
    ) -> R<String> {
        let (markup, popped) = cx.body(
            Code::Stmts(&statements[parts.dom_start..]),
            parts.anchor,
            true,
        )?;
        if parts.pushed != popped {
            return fail("push() and pop() do not pair");
        }
        let snippets = self.snippets(cx)?;
        let mut used: FxHashSet<SymbolId> = cx.snippets.borrow().iter().copied().collect();
        used.extend(parts.rest.as_ref().and_then(|(_, _, holder)| *holder));
        used.extend(cx.facts.handlers.iter().copied());
        let props_line = self.props_line(cx, function, &parts)?;
        let source = self.program.source_text;
        let mut instance: Vec<String> = Vec::new();
        let mut module: Vec<String> = Vec::new();
        if esm {
            let (imports, rest) = self.module_parts(cx, &used)?;
            instance.extend(imports);
            module = rest;
        } else {
            instance.extend(self.child_imports(cx));
        }
        if !props_line.is_empty() {
            instance.push(format!("\tlet {{ {} }} = $props();", props_line.join(", ")));
        }
        for statement in &parts.script {
            if let Statement::VariableDeclaration(declaration) = statement {
                if declaration.declarations.iter().all(|declarator| {
                    binding(&declarator.id).is_some_and(|symbol| used.contains(&symbol))
                }) {
                    continue;
                }
            }
            let text = cx.render(statement.span()).map_err(|reason| {
                format!(
                    "{reason} (in `{}`)",
                    first_line(cx.rw.text(statement.span()))
                )
            })?;
            instance.push(indented(source, statement.span(), &text));
        }
        if !esm {
            instance.splice(0..0, self.export_imports(cx, function)?);
        }
        let mut out = String::new();
        if !module.is_empty() {
            out.push_str(&format!(
                "<script module>\n{}\n</script>\n\n",
                spaced(&module)
            ));
        }
        if !instance.is_empty() {
            out.push_str(&format!("<script>\n{}\n</script>\n\n", spaced(&instance)));
        }
        let mut nodes = Vec::new();
        for (special, attrs) in group_specials(&cx.specials.borrow()) {
            nodes.push(M::Element {
                name: special.to_string(),
                attrs,
                children: Vec::new(),
                svg: true,
            });
            nodes.push(M::Text(" ".to_string()));
        }
        for snippet in snippets {
            nodes.push(snippet);
            nodes.push(M::Text(" ".to_string()));
        }
        nodes.extend(markup);
        out.push_str(&print(&nodes));
        if let Some(css) = parts.css {
            let elements = crate::scope::elements(&nodes);
            let code = css::restore(&css.code, &css.hash, &elements, &cx.scoped.borrow())?;
            out.push_str(&format!("\n<style>\n{code}\n</style>\n"));
        }
        Ok(out)
    }
}

struct Parts<'a> {
    anchor: SymbolId,
    props: Option<SymbolId>,
    css: Option<Css>,
    prop_symbols: Vec<SymbolId>,
    rest: Option<(SymbolId, Vec<String>, Option<SymbolId>)>,
    script: Vec<&'a Statement<'a>>,
    pushed: bool,
    dom_start: usize,
}

fn group_specials(specials: &[(&'static str, String)]) -> Vec<(&'static str, Vec<String>)> {
    let mut out: Vec<(&'static str, Vec<String>)> = Vec::new();
    for (special, attr) in specials {
        match out.iter_mut().find(|(known, _)| known == special) {
            Some((_, attrs)) => attrs.push(attr.clone()),
            None => out.push((special, vec![attr.clone()])),
        }
    }
    out
}

fn strings(array: &oxc_ast::ast::ArrayExpression<'_>) -> R<Vec<String>> {
    array
        .elements
        .iter()
        .map(|element| {
            element
                .as_expression()
                .and_then(string)
                .map(str::to_string)
                .ok_or_else(|| "rest props with an exclusion list of an unknown shape".to_string())
        })
        .collect()
}
