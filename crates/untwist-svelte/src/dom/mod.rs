use std::cell::RefCell;

use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentTarget, CallExpression, Expression, FormalParameters,
    IfStatement, ObjectPropertyKind, PropertyKey, PropertyKind, Statement, VariableDeclarator,
};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::html::{self, Node};
use crate::markup::{braces, escape_text, M};
use crate::runtime::{symbol_of, Rt, Runtime};
use crate::script::{
    arg, arrow_statements, arrow_value, binding, number, param_symbols, string, thunk_body, Class,
    Facts,
};
use crate::text::{Reason, Rewriter};

mod attrs;
mod blocks;
pub mod pattern;
mod snippet;
mod special;

pub type R<T> = Result<T, Reason>;

pub struct Template {
    pub nodes: Vec<Node>,
    pub fragment: bool,
    pub svg: bool,
}

pub struct Cx<'a, 's> {
    pub scoping: &'a Scoping,
    pub nodes: &'a AstNodes<'a>,
    pub runtime: &'a Runtime,
    pub facts: &'a Facts,
    pub rw: &'a Rewriter<'s>,
    pub hash: Option<String>,
    pub snippets: RefCell<Vec<SymbolId>>,
    pub children: RefCell<Vec<SymbolId>>,
    pub specials: RefCell<Vec<(&'static str, String)>>,
    pub components: Vec<(Span, String)>,
    pub consumed: RefCell<FxHashSet<SymbolId>>,
}

enum DKind {
    Fragment,
    Element { name: String, svg: bool },
    Text(String),
    Comment(String),
}

enum Fill {
    Text(String),
    Children(Vec<M>),
    Nodes(Vec<M>),
}

struct DNode {
    kind: DKind,
    parent: Option<usize>,
    children: Vec<usize>,
    attrs: Vec<String>,
    fill: Option<Fill>,
}

pub struct Body {
    anchor: SymbolId,
    dom: Vec<DNode>,
    refs: FxHashMap<SymbolId, usize>,
    result: Option<usize>,
    deferred: Vec<SymbolId>,
    used: FxHashSet<SymbolId>,
    component: bool,
    popped: bool,
    consts: Vec<String>,
}

const PROPERTIES: &[(&str, &str)] = &[
    ("disabled", "disabled"),
    ("checked", "checked"),
    ("value", "value"),
    ("hidden", "hidden"),
    ("selected", "selected"),
    ("readOnly", "readonly"),
    ("multiple", "multiple"),
    ("muted", "muted"),
    ("autofocus", "autofocus"),
    ("open", "open"),
    ("required", "required"),
    ("defaultValue", "value"),
    ("defaultChecked", "checked"),
];

fn fail<T>(message: impl Into<String>) -> R<T> {
    Err(message.into())
}

pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

pub fn attr(name: &str, code: &str) -> String {
    match code == name && is_identifier(name) {
        true => format!("{{{name}}}"),
        false => format!("{name}={{{code}}}"),
    }
}

fn directive(kind: &str, name: &str, code: &str) -> String {
    match code == name && is_identifier(name) {
        true => format!("{kind}:{name}"),
        false => format!("{kind}:{name}={{{code}}}"),
    }
}

fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        _ => None,
    }
}

fn nullish_empty<'e, 'a>(expr: &'e Expression<'a>) -> &'e Expression<'a> {
    if let Expression::LogicalExpression(logical) = expr.without_parentheses() {
        if logical.operator.as_str() == "??" && string(&logical.right) == Some("") {
            return &logical.left;
        }
    }
    expr
}

fn is_nullish(expr: Option<&Expression<'_>>) -> bool {
    match expr.map(Expression::without_parentheses) {
        None => true,
        Some(Expression::NullLiteral(_)) => true,
        Some(Expression::Identifier(ident)) => ident.name == "undefined",
        Some(Expression::UnaryExpression(unary)) => unary.operator.as_str() == "void",
        _ => false,
    }
}

fn boolean(expr: Option<&Expression<'_>>) -> Option<bool> {
    match expr.map(Expression::without_parentheses) {
        None => Some(false),
        Some(Expression::BooleanLiteral(literal)) => Some(literal.value),
        Some(Expression::NumericLiteral(literal)) => Some(literal.value != 0.0),
        Some(Expression::UnaryExpression(unary)) if unary.operator.as_str() == "!" => {
            number(&unary.argument).map(|value| value == 0)
        }
        _ => is_nullish(expr).then_some(false),
    }
}

impl<'a, 's> Cx<'a, 's> {
    pub fn rt(&self, expr: &Expression<'_>) -> Option<Rt> {
        self.runtime.resolve(expr, self.scoping)
    }

    fn symbol(&self, expr: &Expression<'_>) -> Option<SymbolId> {
        match expr.without_parentheses() {
            Expression::Identifier(ident) => symbol_of(ident, self.scoping),
            _ => None,
        }
    }

    pub fn declarator(&self, symbol: SymbolId) -> Option<&'a VariableDeclarator<'a>> {
        match self.nodes.kind(self.scoping.symbol_declaration(symbol)) {
            AstKind::VariableDeclarator(declarator) => Some(declarator),
            _ => None,
        }
    }

    pub fn component_name(&self, symbol: SymbolId) -> Option<&str> {
        let AstKind::Function(function) = self.nodes.kind(self.scoping.symbol_declaration(symbol))
        else {
            return None;
        };
        self.components
            .iter()
            .find(|(span, _)| *span == function.span)
            .map(|(_, name)| name.as_str())
    }

    pub fn arrow(&self, symbol: SymbolId) -> Option<&'a ArrowFunctionExpression<'a>> {
        match self
            .declarator(symbol)?
            .init
            .as_ref()?
            .without_parentheses()
        {
            Expression::ArrowFunctionExpression(arrow) => Some(arrow),
            _ => None,
        }
    }

    fn template(&self, symbol: SymbolId) -> Option<Template> {
        let Expression::CallExpression(call) = self
            .declarator(symbol)?
            .init
            .as_ref()?
            .without_parentheses()
        else {
            return None;
        };
        let svg = match self.rt(&call.callee)? {
            Rt::FromHtml => false,
            Rt::FromSvg => true,
            _ => return None,
        };
        let flags = match arg(call, 1) {
            Some(flags) => number(flags)?,
            None => 0,
        };
        let nodes = html::parse(string(arg(call, 0)?)?)?;
        Some(Template {
            nodes,
            fragment: flags & 1 != 0,
            svg,
        })
    }

    pub fn scratch(&self) -> Body {
        Body {
            anchor: SymbolId::new(0),
            dom: Vec::new(),
            refs: FxHashMap::default(),
            result: None,
            deferred: Vec::new(),
            used: FxHashSet::default(),
            component: false,
            popped: false,
            consts: Vec::new(),
        }
    }

    pub fn render(&self, span: Span) -> R<String> {
        self.rw.render(span)
    }

    fn code(&self, expr: &Expression<'_>) -> R<String> {
        self.render(expr.span())
    }

    fn thunk(&self, expr: &Expression<'_>) -> R<String> {
        if let Some(body) = thunk_body(expr) {
            return self.render(body);
        }
        if let Expression::Identifier(ident) = expr.without_parentheses() {
            if self
                .symbol(expr)
                .and_then(|symbol| self.facts.classes.get(&symbol))
                == Some(&Class::Getter)
            {
                return Ok(ident.name.to_string());
            }
        }
        fail(format!(
            "`{}` is not a getter of a template expression",
            self.rw.text(expr.span())
        ))
    }

    fn text_value(&self, expr: &Expression<'_>) -> R<String> {
        if let Some(value) = string(expr) {
            return Ok(escape_text(value));
        }
        if let Expression::TemplateLiteral(template) = expr.without_parentheses() {
            let mut out = String::new();
            for (index, quasi) in template.quasis.iter().enumerate() {
                let cooked = quasi.value.cooked.and_then(|cooked| cooked.as_str());
                out.push_str(&escape_text(
                    cooked.ok_or("a template text with an invalid escape")?,
                ));
                if let Some(expr) = template.expressions.get(index) {
                    out.push_str(&format!("{{{}}}", self.code(nullish_empty(expr))?));
                }
            }
            return Ok(out);
        }
        Ok(format!("{{{}}}", self.code(nullish_empty(expr))?))
    }

    fn forbid_params(&self, params: &FormalParameters<'_>) -> R<Vec<SymbolId>> {
        let symbols =
            param_symbols(params).ok_or("a render callback takes a pattern or rest parameter")?;
        for (index, symbol) in symbols.iter().enumerate() {
            if index == 0 || self.scoping.symbol_name(*symbol).starts_with("$$") {
                self.rw.forbid(*symbol);
            }
        }
        Ok(symbols)
    }

    fn callback(
        &self,
        expr: &'a Expression<'a>,
        body: &mut Body,
    ) -> R<(&'a FormalParameters<'a>, Code<'a>)> {
        match expr.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => Ok((&arrow.params, Code::of(arrow))),
            Expression::FunctionExpression(function) => {
                let block = function
                    .body
                    .as_ref()
                    .ok_or("a render callback without a body")?;
                Ok((&function.params, Code::Stmts(&block.statements)))
            }
            Expression::Identifier(_) => {
                let symbol = self
                    .symbol(expr)
                    .ok_or("a render callback that is not declared here")?;
                let arrow = self
                    .arrow(symbol)
                    .ok_or("a render callback that is not a local arrow function")?;
                body.used.insert(symbol);
                Ok((&arrow.params, Code::of(arrow)))
            }
            _ => fail("a render callback of an unknown shape"),
        }
    }

    pub fn render_fn(
        &self,
        expr: &'a Expression<'a>,
        outer: &mut Body,
    ) -> R<(Vec<M>, Vec<SymbolId>)> {
        let (params, statements) = self.callback(expr, outer)?;
        let symbols = self.forbid_params(params)?;
        let anchor = *symbols
            .first()
            .ok_or("a render callback without an anchor parameter")?;
        let nodes = self.body(statements, anchor, false)?.0;
        Ok((nodes, symbols))
    }

    pub fn body(&self, code: Code<'a>, anchor: SymbolId, component: bool) -> R<(Vec<M>, bool)> {
        self.body_seeded(code, anchor, component, None)
            .map(|(nodes, popped, _)| (nodes, popped))
    }

    pub fn body_seeded(
        &self,
        code: Code<'a>,
        anchor: SymbolId,
        component: bool,
        seed: Option<SymbolId>,
    ) -> R<(Vec<M>, bool, Vec<String>)> {
        self.rw.forbid(anchor);
        let mut body = Body {
            anchor,
            dom: Vec::new(),
            refs: FxHashMap::default(),
            result: None,
            deferred: Vec::new(),
            used: FxHashSet::default(),
            component,
            popped: false,
            consts: Vec::new(),
        };
        if let Some(seed) = seed {
            self.rw.forbid(seed);
            body.dom.push(DNode {
                kind: DKind::Element {
                    name: "svelte:element".to_string(),
                    svg: false,
                },
                parent: None,
                children: Vec::new(),
                attrs: Vec::new(),
                fill: None,
            });
            body.refs.insert(seed, 0);
        }
        match code {
            Code::Expr(expr) => self.expression(&mut body, expr)?,
            Code::Stmts(statements) => {
                self.forbid_declared(statements);
                for statement in statements {
                    if body.popped {
                        return fail("a statement follows pop()");
                    }
                    self.statement(&mut body, statement)?;
                }
            }
        }
        if let Some(unused) = body
            .deferred
            .iter()
            .find(|symbol| !body.used.contains(symbol))
        {
            return fail(format!(
                "the render callback `{}` is never used by a block",
                self.scoping.symbol_name(*unused)
            ));
        }
        let mut nodes: Vec<M> = body
            .consts
            .iter()
            .flat_map(|text| [M::Tag(text.clone()), M::Text(" ".to_string())])
            .collect();
        match body.result {
            Some(root) => nodes.extend(self.emit_root(&body, root)?),
            None if body.dom.len() == usize::from(seed.is_some()) => {}
            None => return fail("the template is never appended to the anchor"),
        }
        let attrs = match seed {
            Some(_) => {
                if body.dom[0].fill.is_some() || !body.dom[0].children.is_empty() {
                    return fail("svelte:element content written through the element");
                }
                body.dom[0].attrs.clone()
            }
            None => Vec::new(),
        };
        Ok((nodes, body.popped, attrs))
    }

    fn forbid_declared(&self, statements: &[Statement<'_>]) {
        for statement in statements {
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    for declarator in &declaration.declarations {
                        if let Some(symbol) = binding(&declarator.id) {
                            self.rw.forbid(symbol);
                        }
                    }
                }
                Statement::BlockStatement(block) => self.forbid_declared(&block.body),
                _ => {}
            }
        }
    }

    fn statement(&self, body: &mut Body, statement: &'a Statement<'a>) -> R<()> {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    let symbol = binding(&declarator.id)
                        .ok_or("a destructuring declaration in the template code")?;
                    if self.consumed.borrow().contains(&symbol) {
                        continue;
                    }
                    match declarator
                        .init
                        .as_ref()
                        .map(Expression::without_parentheses)
                    {
                        None => {}
                        Some(Expression::ArrowFunctionExpression(_))
                            if self.facts.classes.get(&symbol) == Some(&Class::Getter) => {}
                        Some(Expression::CallExpression(call))
                            if self.rt(&call.callee) == Some(Rt::Derived) =>
                        {
                            if body.component {
                                return fail("a derived value declared inside the template code");
                            }
                            let value = arg(call, 0)
                                .and_then(thunk_body)
                                .ok_or("a {@const} of an unknown shape")?;
                            body.consts.push(format!(
                                "{{@const {} = {}}}",
                                self.scoping.symbol_name(symbol),
                                self.render(value)?
                            ));
                        }
                        Some(
                            Expression::ArrowFunctionExpression(_)
                            | Expression::FunctionExpression(_),
                        ) => body.deferred.push(symbol),
                        Some(Expression::ArrayExpression(array)) if array.elements.is_empty() => {}
                        Some(init) => {
                            let node = self.node_expr(body, init)?;
                            body.refs.insert(symbol, node);
                        }
                    }
                }
                Ok(())
            }
            Statement::BlockStatement(block) => {
                for inner in &block.body {
                    self.statement(body, inner)?;
                }
                Ok(())
            }
            Statement::ExpressionStatement(stmt) => self.expression(body, &stmt.expression),
            Statement::IfStatement(stmt) => self.radio_value(body, stmt),
            _ => fail(format!(
                "the statement `{}` has no template meaning",
                first_line(self.rw.text(statement.span()))
            )),
        }
    }

    fn add(&self, body: &mut Body, node: &Node, parent: Option<usize>, svg: bool) -> usize {
        let id = body.dom.len();
        let (kind, attrs, children) = match node {
            Node::Element(element) => {
                let svg = svg || element.name == "svg";
                let attrs = element
                    .attrs
                    .iter()
                    .filter_map(|(name, value)| self.static_attr(name, value.as_deref()))
                    .collect();
                (
                    DKind::Element {
                        name: element.name.clone(),
                        svg,
                    },
                    attrs,
                    Some((&element.children, svg)),
                )
            }
            Node::Text(text) => (DKind::Text(text.clone()), Vec::new(), None),
            Node::Comment(text) => (DKind::Comment(text.clone()), Vec::new(), None),
        };
        body.dom.push(DNode {
            kind,
            parent,
            children: Vec::new(),
            attrs,
            fill: None,
        });
        if let Some((children, svg)) = children {
            let ids: Vec<usize> = children
                .iter()
                .map(|child| self.add(body, child, Some(id), svg))
                .collect();
            body.dom[id].children = ids;
        }
        id
    }

    fn static_attr(&self, name: &str, value: Option<&str>) -> Option<String> {
        let Some(value) = value else {
            return Some(name.to_string());
        };
        let value = match (name, self.hash.as_deref()) {
            ("class", Some(hash)) => {
                let kept: Vec<&str> = value
                    .split_whitespace()
                    .filter(|class| *class != hash)
                    .collect();
                if kept.is_empty() {
                    return None;
                }
                kept.join(" ")
            }
            _ => value.to_string(),
        };
        Some(format!("{name}=\"{}\"", braces(&value)))
    }

    fn fragment(&self, body: &mut Body, nodes: &[Node], svg: bool) -> usize {
        let id = body.dom.len();
        body.dom.push(DNode {
            kind: DKind::Fragment,
            parent: None,
            children: Vec::new(),
            attrs: Vec::new(),
            fill: None,
        });
        let ids: Vec<usize> = nodes
            .iter()
            .map(|node| self.add(body, node, Some(id), svg))
            .collect();
        body.dom[id].children = ids;
        id
    }

    fn instantiate(&self, body: &mut Body, template: &Template) -> R<usize> {
        let root = self.fragment(body, &template.nodes, template.svg);
        if template.fragment {
            return Ok(root);
        }
        match body.dom[root].children.as_slice() {
            [only] => {
                let only = *only;
                body.dom[only].parent = None;
                Ok(only)
            }
            _ => fail("a single-node template holds several nodes"),
        }
    }

    fn node_expr(&self, body: &mut Body, expr: &'a Expression<'a>) -> R<usize> {
        match expr.without_parentheses() {
            Expression::Identifier(ident) => {
                let symbol = symbol_of(ident, self.scoping).ok_or("an unresolved template node")?;
                body.refs
                    .get(&symbol)
                    .copied()
                    .ok_or_else(|| format!("`{}` is not a template node", ident.name))
            }
            Expression::CallExpression(call) => self.node_call(body, call),
            _ => fail(format!(
                "`{}` is not a template node",
                self.rw.text(expr.span())
            )),
        }
    }

    fn node_call(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<usize> {
        if let Some(template) = self
            .symbol(&call.callee)
            .and_then(|symbol| self.template(symbol))
        {
            if !call.arguments.is_empty() {
                return fail("a template is instantiated with arguments");
            }
            return self.instantiate(body, &template);
        }
        match self.rt(&call.callee) {
            Some(Rt::Comment) => Ok(self.fragment(body, &[Node::Comment(String::new())], false)),
            Some(Rt::Text) if call.arguments.is_empty() => {
                let root = self.fragment(body, &[Node::Text(String::new())], false);
                Ok(body.dom[root].children[0])
            }
            Some(Rt::Text) if call.arguments.len() == 1 => {
                let text = arg(call, 0)
                    .and_then(string)
                    .filter(|text| !text.contains(['<', '&']))
                    .ok_or("a text node with computed or markup content")?;
                let root = self.fragment(body, &[Node::Text(text.to_string())], false);
                Ok(body.dom[root].children[0])
            }
            Some(Rt::FirstChild | Rt::Child | Rt::OnlyChild) => {
                let parent =
                    self.node_expr(body, arg(call, 0).ok_or("a child() without a node")?)?;
                body.dom[parent]
                    .children
                    .first()
                    .copied()
                    .ok_or_else(|| "the walk reads the child of an empty node".to_string())
            }
            Some(Rt::Sibling) => {
                let from =
                    self.node_expr(body, arg(call, 0).ok_or("a sibling() without a node")?)?;
                let count = match arg(call, 1) {
                    Some(count) => {
                        number(count).ok_or("a sibling() with a computed count")? as usize
                    }
                    None => 1,
                };
                let parent = body.dom[from]
                    .parent
                    .ok_or("the walk leaves the template")?;
                let at = body.dom[parent]
                    .children
                    .iter()
                    .position(|child| *child == from)
                    .ok_or("a detached template node")?;
                body.dom[parent]
                    .children
                    .get(at + count)
                    .copied()
                    .ok_or_else(|| "the walk runs past the last sibling".to_string())
            }
            _ => fail(format!(
                "`{}` is not a template walk",
                first_line(self.rw.text(call.span))
            )),
        }
    }

    fn element(&self, body: &mut Body, expr: Option<&'a Expression<'a>>) -> R<usize> {
        let node = self.node_expr(body, expr.ok_or("a missing element argument")?)?;
        match body.dom[node].kind {
            DKind::Element { .. } => Ok(node),
            _ => fail("an element update targets a node that is not an element"),
        }
    }

    fn fill(&self, body: &mut Body, node: usize, fill: Fill) -> R<()> {
        if body.dom[node].fill.is_some() {
            return fail("a template node is filled twice");
        }
        body.dom[node].fill = Some(fill);
        Ok(())
    }

    fn fill_anchor(&self, body: &mut Body, node: usize, nodes: Vec<M>) -> R<()> {
        match body.dom[node].kind {
            DKind::Comment(ref text) if text.is_empty() => {
                self.fill(body, node, Fill::Nodes(nodes))
            }
            DKind::Text(_) => self.fill(body, node, Fill::Nodes(nodes)),
            _ => fail("a block is anchored on a node that is not an anchor"),
        }
    }

    fn expression(&self, body: &mut Body, expr: &'a Expression<'a>) -> R<()> {
        let call = match expr.without_parentheses() {
            Expression::CallExpression(call) => call,
            Expression::SequenceExpression(sequence) => {
                for inner in &sequence.expressions {
                    self.expression(body, inner)?;
                }
                return Ok(());
            }
            _ => return self.update(body, expr),
        };
        let Some(rt) = self.rt(&call.callee) else {
            return self.user_call(body, call);
        };
        match rt {
            Rt::Append => {
                if self.symbol(arg(call, 0).ok_or("append() without an anchor")?)
                    != Some(body.anchor)
                {
                    return fail("append() targets a node other than the anchor");
                }
                let root = self.node_expr(body, arg(call, 1).ok_or("append() without a node")?)?;
                if body.result.replace(root).is_some() {
                    return fail("the anchor is appended to twice");
                }
                Ok(())
            }
            Rt::Next
            | Rt::Reset
            | Rt::RemoveInputDefaults
            | Rt::RemoveTextareaChild
            | Rt::Delegate => Ok(()),
            Rt::Pop if body.component && call.arguments.is_empty() => {
                body.popped = true;
                Ok(())
            }
            Rt::TemplateEffect => self.effect(body, call),
            Rt::SetText
            | Rt::SetAttribute
            | Rt::SetValue
            | Rt::SetChecked
            | Rt::SetClass
            | Rt::SetStyle => self.update(body, expr),
            Rt::Delegated | Rt::Event => self.event(body, call, rt),
            Rt::BindValue | Rt::BindChecked | Rt::BindGroup | Rt::BindThis => {
                self.bind(body, call, rt)
            }
            Rt::If => self.if_block(body, call),
            Rt::Each => self.each_block(body, call),
            Rt::Key => self.key_block(body, call),
            Rt::Await => self.await_block(body, call),
            Rt::Html => self.html_tag(body, call),
            Rt::Snippet => self.dynamic_render(body, call),
            Rt::AttributeEffect => self.spread_attrs(body, call),
            Rt::Transition => self.transition(body, call),
            Rt::Animation => self.animation(body, call),
            Rt::Element => self.dynamic_element(body, call),
            Rt::Boundary => self.boundary(body, call),
            other => fail(format!(
                "the runtime call {other:?} has no template meaning here"
            )),
        }
    }

    fn effect(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        if call.arguments.len() > 2 {
            return fail("an async template effect");
        }
        let callback = arg(call, 0).ok_or("an effect without a callback")?;
        let statements = match callback.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) => match arrow.get_expression() {
                Some(expr) => return self.update(body, expr),
                None => arrow_statements(arrow),
            },
            Expression::FunctionExpression(function) => {
                &function
                    .body
                    .as_ref()
                    .ok_or("an effect without a body")?
                    .statements
            }
            _ => return fail("an effect callback of an unknown shape"),
        };
        for statement in statements {
            match statement {
                Statement::ExpressionStatement(stmt) => self.update(body, &stmt.expression)?,
                Statement::IfStatement(stmt) => self.radio_value(body, stmt)?,
                _ => {
                    return fail(format!(
                        "the effect statement `{}` has no template meaning",
                        first_line(self.rw.text(statement.span()))
                    ))
                }
            }
        }
        Ok(())
    }

    fn emit_root(&self, body: &Body, root: usize) -> R<Vec<M>> {
        match body.dom[root].kind {
            DKind::Fragment => self.emit_children(body, root),
            _ => self.emit(body, root),
        }
    }

    fn emit_children(&self, body: &Body, id: usize) -> R<Vec<M>> {
        let mut out = Vec::new();
        for child in &body.dom[id].children {
            out.extend(self.emit(body, *child)?);
        }
        Ok(out)
    }

    fn emit(&self, body: &Body, id: usize) -> R<Vec<M>> {
        let node = &body.dom[id];
        match (&node.kind, &node.fill) {
            (DKind::Fragment, _) => self.emit_children(body, id),
            (DKind::Element { name, svg }, fill) => {
                let children = match fill {
                    Some(Fill::Children(children)) => children.clone(),
                    None => self.emit_children(body, id)?,
                    Some(_) => return fail("an element filled as text"),
                };
                Ok(vec![M::Element {
                    name: name.clone(),
                    attrs: node.attrs.clone(),
                    children,
                    svg: *svg,
                }])
            }
            (DKind::Text(_), Some(Fill::Text(text))) => Ok(vec![M::Text(text.clone())]),
            (DKind::Text(text), None) => Ok(vec![M::Text(braces(text))]),
            (DKind::Text(_) | DKind::Comment(_), Some(Fill::Nodes(nodes))) => Ok(nodes.clone()),
            (DKind::Comment(_), None) => fail("an anchor in the template holds no block"),
            _ => fail("a template node filled with the wrong kind of content"),
        }
    }
}

fn thunk_body_any<'e, 'a>(arrow: &'e ArrowFunctionExpression<'a>) -> Option<&'e Expression<'a>> {
    match arrow_statements(arrow) {
        [Statement::ExpressionStatement(stmt)] => Some(&stmt.expression),
        _ => arrow_value(arrow),
    }
}

#[derive(Clone, Copy)]
pub enum Code<'a> {
    Stmts(&'a [Statement<'a>]),
    Expr(&'a Expression<'a>),
}

impl<'a> Code<'a> {
    fn of(arrow: &'a ArrowFunctionExpression<'a>) -> Self {
        match arrow.get_expression() {
            Some(expr) => Code::Expr(expr),
            None => Code::Stmts(arrow_statements(arrow)),
        }
    }
}

pub fn first_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default();
    match line.len() > 80 {
        true => format!(
            "{}...",
            &line[..line.char_indices().nth(77).map_or(line.len(), |(at, _)| at)]
        ),
        false => line.to_string(),
    }
}
