use std::cell::RefCell;

use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentTarget, CallExpression, Expression, FormalParameters,
    IfStatement, ObjectPropertyKind, PropertyKey, PropertyKind, Statement, VariableDeclarator,
};
use oxc_ast::AstKind;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::{GetSpan, Span};
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
            None if body.dom.is_empty() => {}
            None => return fail("the template is never appended to the anchor"),
        }
        Ok((nodes, body.popped))
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
                            if !body.dom.is_empty() {
                                return fail("a {@const} after the template is built");
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
                Statement::IfStatement(stmt) => self.radio_value(body, &**stmt)?,
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

    fn push_attr(&self, body: &mut Body, node: usize, attr: String) {
        body.dom[node].attrs.push(attr);
    }

    fn update(&self, body: &mut Body, expr: &'a Expression<'a>) -> R<()> {
        match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => {
                for inner in &sequence.expressions {
                    self.update(body, inner)?;
                }
                Ok(())
            }
            Expression::CallExpression(call) => self.update_call(body, call),
            Expression::AssignmentExpression(assign) if assign.operator.as_str() == "=" => {
                match &assign.left {
                    AssignmentTarget::AssignmentTargetIdentifier(_) => {
                        match assign.right.without_parentheses() {
                            Expression::CallExpression(call)
                                if matches!(
                                    self.rt(&call.callee),
                                    Some(Rt::SetClass | Rt::SetStyle)
                                ) =>
                            {
                                self.update_call(body, call)
                            }
                            _ => fail("an assignment in the template code"),
                        }
                    }
                    AssignmentTarget::StaticMemberExpression(member) => {
                        let node = self.node_expr(body, &member.object)?;
                        self.property(body, node, member.property.name.as_str(), &assign.right)
                    }
                    _ => fail("an assignment in the template code"),
                }
            }
            _ => fail(format!(
                "`{}` has no template meaning",
                first_line(self.rw.text(expr.span()))
            )),
        }
    }

    fn property(
        &self,
        body: &mut Body,
        node: usize,
        name: &str,
        value: &'a Expression<'a>,
    ) -> R<()> {
        match (name, &body.dom[node].kind) {
            ("textContent", DKind::Element { .. }) => {
                let text = self.text_value(value)?;
                self.fill(body, node, Fill::Children(vec![M::Text(text)]))
            }
            ("nodeValue", DKind::Text(_)) => {
                let text = self.text_value(value)?;
                self.fill(body, node, Fill::Text(text))
            }
            (_, DKind::Element { .. }) => {
                let attribute = PROPERTIES
                    .iter()
                    .find(|(property, _)| *property == name)
                    .map(|row| row.1)
                    .ok_or_else(|| {
                        format!("the element property `{name}` has no attribute spelling")
                    })?;
                let code = self.code(value)?;
                self.push_attr(body, node, attr(attribute, &code));
                Ok(())
            }
            _ => fail(format!(
                "the property `{name}` is written on a node that is not an element"
            )),
        }
    }

    fn update_call(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        match self.rt(&call.callee) {
            Some(Rt::SetText) => {
                let node =
                    self.node_expr(body, arg(call, 0).ok_or("set_text() without a node")?)?;
                if !matches!(body.dom[node].kind, DKind::Text(_)) {
                    return fail("set_text() on a node that is not text");
                }
                let text = self.text_value(arg(call, 1).ok_or("set_text() without a value")?)?;
                self.fill(body, node, Fill::Text(text))
            }
            Some(Rt::SetAttribute) => {
                let node = self.element(body, arg(call, 0))?;
                let name = arg(call, 1)
                    .and_then(string)
                    .ok_or("set_attribute() with a computed name")?;
                if call.arguments.len() > 3 {
                    return fail("set_attribute() with extra arguments");
                }
                let text =
                    self.attr_value(name, arg(call, 2).ok_or("set_attribute() without a value")?)?;
                self.push_attr(body, node, text);
                Ok(())
            }
            Some(rt @ (Rt::SetValue | Rt::SetChecked)) => {
                let node = self.element(body, arg(call, 0))?;
                let code = self.code(arg(call, 1).ok_or("a property update without a value")?)?;
                self.push_attr(
                    body,
                    node,
                    attr(
                        if rt == Rt::SetValue {
                            "value"
                        } else {
                            "checked"
                        },
                        &code,
                    ),
                );
                Ok(())
            }
            Some(Rt::SetClass) => self.set_class(body, call),
            Some(Rt::SetStyle) => self.set_style(body, call),
            _ => fail(format!(
                "`{}` has no template meaning",
                first_line(self.rw.text(call.span))
            )),
        }
    }

    fn set_class(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.element(body, arg(call, 0))?;
        if call.arguments.len() > 6 {
            return fail("set_class() with extra arguments");
        }
        let value = arg(call, 2).ok_or("set_class() without a value")?;
        if let Some(text) = string(value) {
            if let Some(attr) = self.static_attr("class", Some(text)) {
                self.push_attr(body, node, attr);
            }
        } else if !is_nullish(Some(value)) {
            let inner = match value.without_parentheses() {
                Expression::CallExpression(clsx)
                    if self.rt(&clsx.callee) == Some(Rt::Clsx) && clsx.arguments.len() == 1 =>
                {
                    arg(clsx, 0).ok_or("clsx() without a value")?
                }
                _ => value,
            };
            let text = self.attr_value("class", inner)?;
            self.push_attr(body, node, text);
        }
        match arg(call, 5).map(Expression::without_parentheses) {
            None => Ok(()),
            Some(Expression::ObjectExpression(object)) => {
                for property in &object.properties {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        return fail("a spread in the class directives");
                    };
                    let name = key_name(&property.key)
                        .filter(|_| !property.computed)
                        .ok_or("a computed class directive")?;
                    let code = self.code(&property.value)?;
                    self.push_attr(body, node, directive("class", &name, &code));
                }
                Ok(())
            }
            Some(_) => fail("class directives that are not an object literal"),
        }
    }

    fn set_style(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.element(body, arg(call, 0))?;
        if call.arguments.len() > 4 {
            return fail("set_style() with extra arguments");
        }
        let value = arg(call, 1).ok_or("set_style() without a value")?;
        match string(value) {
            Some("") => {}
            Some(text) => self.push_attr(body, node, format!("style=\"{}\"", braces(text))),
            None if is_nullish(Some(value)) => {}
            None => {
                let text = self.attr_value("style", value)?;
                self.push_attr(body, node, text);
            }
        }
        match arg(call, 3).map(Expression::without_parentheses) {
            None => Ok(()),
            Some(Expression::ObjectExpression(object)) => {
                for property in &object.properties {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        return fail("a spread in the style directives");
                    };
                    let name = key_name(&property.key)
                        .filter(|_| !property.computed)
                        .ok_or("a computed style directive")?;
                    let directive_text = match string(&property.value) {
                        Some(text) => format!("style:{name}=\"{}\"", braces(text)),
                        None => directive("style", &name, &self.code(&property.value)?),
                    };
                    self.push_attr(body, node, directive_text);
                }
                Ok(())
            }
            Some(_) => fail("style directives that are not an object literal"),
        }
    }

    fn radio_value(&self, body: &mut Body, stmt: &'a IfStatement<'a>) -> R<()> {
        let shape = "an if statement in the template code";
        if stmt.alternate.is_some() {
            return fail(shape);
        }
        let Expression::BinaryExpression(test) = stmt.test.without_parentheses() else {
            return fail(shape);
        };
        let Expression::AssignmentExpression(cache) = test.right.without_parentheses() else {
            return fail(shape);
        };
        let consequent = match &stmt.consequent {
            Statement::BlockStatement(block) => match block.body.as_slice() {
                [only] => only,
                _ => return fail(shape),
            },
            other => other,
        };
        let Statement::ExpressionStatement(write) = consequent else {
            return fail(shape);
        };
        let Expression::AssignmentExpression(write) = write.expression.without_parentheses() else {
            return fail(shape);
        };
        let AssignmentTarget::StaticMemberExpression(member) = &write.left else {
            return fail(shape);
        };
        if test.operator.as_str() != "!==" || member.property.name != "value" {
            return fail(shape);
        }
        let node = self.node_expr(body, &member.object)?;
        let code = self.code(&cache.right)?;
        self.push_attr(body, node, attr("value", &code));
        Ok(())
    }

    fn event(&self, body: &mut Body, call: &'a CallExpression<'a>, rt: Rt) -> R<()> {
        let name = arg(call, 0)
            .and_then(string)
            .ok_or("an event with a computed name")?;
        let special = match arg(call, 1).and_then(|target| self.rt(target)) {
            Some(Rt::Window) => Some("svelte:window"),
            Some(Rt::Document) => Some("svelte:document"),
            _ => None,
        };
        if let Some(special) = special {
            if rt != Rt::Event || call.arguments.len() > 3 {
                return fail("a window or document event with flags");
            }
            let code = self.code(arg(call, 2).ok_or("an event without a handler")?)?;
            self.specials
                .borrow_mut()
                .push((special, attr(&format!("on{name}"), &code)));
            return Ok(());
        }
        let node = self.element(body, arg(call, 1))?;
        let handler = arg(call, 2).ok_or("an event without a handler")?;
        let capture = match rt {
            Rt::Event => boolean(arg(call, 3)).ok_or("an event with a computed capture flag")?,
            _ => false,
        };
        if rt == Rt::Delegated && call.arguments.len() > 3 {
            return fail("a delegated event with extra arguments");
        }
        let code = self.code(handler)?;
        let attribute = format!("on{name}{}", if capture { "capture" } else { "" });
        self.push_attr(body, node, attr(&attribute, &code));
        Ok(())
    }

    fn getter_code(&self, expr: &'a Expression<'a>) -> R<String> {
        if let Some(body) = thunk_body(expr) {
            return self.render(body);
        }
        let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
            return fail("a binding getter of an unknown shape");
        };
        if !arrow.params.items.is_empty() {
            return fail("a binding getter takes parameters");
        }
        let (last, reads) = arrow_statements(arrow)
            .split_last()
            .ok_or("an empty binding getter")?;
        for read in reads {
            let Statement::ExpressionStatement(read) = read else {
                return fail("a binding getter with statements");
            };
            if !matches!(read.expression.without_parentheses(), Expression::CallExpression(call) if self.rt(&call.callee) == Some(Rt::Get))
            {
                return fail("a binding getter with statements");
            }
        }
        match last {
            Statement::ReturnStatement(ret) => self.code(
                ret.argument
                    .as_ref()
                    .ok_or("a binding getter returns nothing")?,
            ),
            _ => fail("a binding getter of an unknown shape"),
        }
    }

    fn setter_target(&self, expr: &'a Expression<'a>) -> R<String> {
        let shape = "a binding setter of an unknown shape";
        let Expression::ArrowFunctionExpression(arrow) = expr.without_parentheses() else {
            return fail(shape);
        };
        let params = param_symbols(&arrow.params).ok_or(shape)?;
        let [param] = params.as_slice() else {
            return fail(shape);
        };
        self.rw.forbid(*param);
        self.assigned(thunk_body_any(arrow).ok_or(shape)?, *param, shape, 2)
    }

    fn setter_function(&self, expr: &'a Expression<'a>) -> R<String> {
        let shape = "a component prop setter of an unknown shape";
        let Expression::FunctionExpression(setter) = expr.without_parentheses() else {
            return fail(shape);
        };
        let params = param_symbols(&setter.params).ok_or(shape)?;
        let [param] = params.as_slice() else {
            return fail(shape);
        };
        let statements = &setter.body.as_ref().ok_or(shape)?.statements;
        let [Statement::ExpressionStatement(stmt)] = statements.as_slice() else {
            return fail(shape);
        };
        self.rw.forbid(*param);
        self.assigned(&stmt.expression, *param, shape, 3)
    }

    fn assigned(
        &self,
        body: &'a Expression<'a>,
        param: SymbolId,
        shape: &str,
        set_args: usize,
    ) -> R<String> {
        let is_param =
            |expr: Option<&Expression<'_>>| expr.and_then(|expr| self.symbol(expr)) == Some(param);
        match body.without_parentheses() {
            Expression::CallExpression(call)
                if self.rt(&call.callee) == Some(Rt::Set)
                    && is_param(arg(call, 1))
                    && call.arguments.len() <= set_args =>
            {
                let Some(Expression::Identifier(target)) = arg(call, 0) else {
                    return fail(shape);
                };
                Ok(target.name.to_string())
            }
            Expression::CallExpression(call)
                if is_param(arg(call, 0)) && call.arguments.len() == 1 =>
            {
                let Expression::Identifier(target) = &call.callee else {
                    return fail(shape);
                };
                match self
                    .symbol(&call.callee)
                    .and_then(|symbol| self.facts.classes.get(&symbol))
                {
                    Some(Class::Getter) => Ok(target.name.to_string()),
                    _ => fail(shape),
                }
            }
            Expression::AssignmentExpression(assign)
                if assign.operator.as_str() == "=" && is_param(Some(&assign.right)) =>
            {
                self.render(assign.left.span())
            }
            _ => fail(shape),
        }
    }

    fn attr_value(&self, name: &str, expr: &Expression<'_>) -> R<String> {
        let Expression::TemplateLiteral(template) = expr.without_parentheses() else {
            return Ok(attr(name, &self.code(expr)?));
        };
        if template.expressions.is_empty() {
            return Ok(attr(name, &self.code(expr)?));
        }
        let mut out = String::new();
        for (index, quasi) in template.quasis.iter().enumerate() {
            let cooked = quasi
                .value
                .cooked
                .and_then(|cooked| cooked.as_str())
                .ok_or("a template text with an invalid escape")?;
            out.push_str(&braces(
                &cooked.replace('&', "&amp;").replace('"', "&quot;"),
            ));
            if let Some(expr) = template.expressions.get(index) {
                out.push_str(&format!("{{{}}}", self.code(nullish_empty(expr))?));
            }
        }
        Ok(format!("{name}=\"{out}\""))
    }

    fn bind(&self, body: &mut Body, call: &'a CallExpression<'a>, rt: Rt) -> R<()> {
        let (node, getter, setter, name) = match rt {
            Rt::BindValue => (arg(call, 0), arg(call, 1), arg(call, 2), "value"),
            Rt::BindChecked => (arg(call, 0), arg(call, 1), arg(call, 2), "checked"),
            Rt::BindGroup => {
                if !matches!(arg(call, 1), Some(Expression::ArrayExpression(array)) if array.elements.is_empty())
                {
                    return fail("a group binding inside a keyed or nested each");
                }
                (arg(call, 2), arg(call, 3), arg(call, 4), "group")
            }
            _ => {
                if call.arguments.len() > 3 {
                    return fail("bind:this inside an each block");
                }
                (arg(call, 0), arg(call, 2), arg(call, 1), "this")
            }
        };
        let node = self.element(body, node)?;
        let getter = self.getter_code(getter.ok_or("a binding without a getter")?)?;
        let target = self.setter_target(setter.ok_or("a binding without a setter")?)?;
        if getter != target {
            return fail(format!(
                "bind:{name} reads `{getter}` but writes `{target}`"
            ));
        }
        let text = match getter == name {
            true => format!("bind:{name}"),
            false => format!("bind:{name}={{{getter}}}"),
        };
        self.push_attr(body, node, text);
        Ok(())
    }

    fn if_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        if call.arguments.len() != 2 {
            return fail("an if block with an else-if flag");
        }
        let node = self.node_expr(body, arg(call, 0).ok_or("an if block without an anchor")?)?;
        let Some(Expression::ArrowFunctionExpression(arrow)) =
            arg(call, 1).map(Expression::without_parentheses)
        else {
            return fail("an if block of an unknown shape");
        };
        let params = param_symbols(&arrow.params).ok_or("an if block of an unknown shape")?;
        let [render] = params.as_slice() else {
            return fail("an if block of an unknown shape");
        };
        let render = *render;
        self.rw.forbid(render);
        let mut branches: Vec<(Option<Span>, &'a Expression<'a>)> = Vec::new();
        match (arrow.get_expression(), arrow_statements(arrow)) {
            (None, [Statement::IfStatement(stmt)]) => {
                self.branches(&**stmt, render, &mut branches)?
            }
            (None, [Statement::ExpressionStatement(stmt)]) => {
                self.ternary(&stmt.expression, render, &mut branches)?
            }
            (Some(expr), _) => self.ternary(expr, render, &mut branches)?,
            _ => return fail("an if block of an unknown shape"),
        }
        let mut sections = Vec::new();
        for (index, (test, branch)) in branches.into_iter().enumerate() {
            let header = match (index, test) {
                (0, Some(test)) => format!("{{#if {}}}", self.render(test)?),
                (0, None) => return fail("an if block without a condition"),
                (_, Some(test)) => format!("{{:else if {}}}", self.render(test)?),
                (_, None) => "{:else}".to_string(),
            };
            let (nodes, _) = self.render_fn(branch, body)?;
            sections.push((header, nodes));
        }
        self.fill_anchor(
            body,
            node,
            vec![M::Block {
                sections,
                close: "{/if}".to_string(),
            }],
        )
    }

    fn render_call(&self, expr: &'a Expression<'a>, render: SymbolId) -> R<&'a Expression<'a>> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return fail("an if branch of an unknown shape");
        };
        if self.symbol(&call.callee) != Some(render)
            || call.arguments.is_empty()
            || call.arguments.len() > 2
        {
            return fail("an if branch of an unknown shape");
        }
        arg(call, 0).ok_or_else(|| "an if branch of an unknown shape".to_string())
    }

    fn branch_statement(
        &self,
        statement: &'a Statement<'a>,
        render: SymbolId,
    ) -> R<&'a Expression<'a>> {
        match statement {
            Statement::ExpressionStatement(stmt) => self.render_call(&stmt.expression, render),
            Statement::BlockStatement(block) => match block.body.as_slice() {
                [only] => self.branch_statement(only, render),
                _ => fail("an if branch of an unknown shape"),
            },
            _ => fail("an if branch of an unknown shape"),
        }
    }

    fn branches(
        &self,
        stmt: &'a IfStatement<'a>,
        render: SymbolId,
        out: &mut Vec<(Option<Span>, &'a Expression<'a>)>,
    ) -> R<()> {
        out.push((
            Some(stmt.test.span()),
            self.branch_statement(&stmt.consequent, render)?,
        ));
        match &stmt.alternate {
            None => Ok(()),
            Some(Statement::IfStatement(next)) => self.branches(next, render, out),
            Some(other) => {
                out.push((None, self.branch_statement(other, render)?));
                Ok(())
            }
        }
    }

    fn ternary(
        &self,
        expr: &'a Expression<'a>,
        render: SymbolId,
        out: &mut Vec<(Option<Span>, &'a Expression<'a>)>,
    ) -> R<()> {
        match expr.without_parentheses() {
            Expression::ConditionalExpression(conditional) => {
                out.push((
                    Some(conditional.test.span()),
                    self.render_call(&conditional.consequent, render)?,
                ));
                match conditional.alternate.without_parentheses() {
                    Expression::ConditionalExpression(_) => {
                        self.ternary(&conditional.alternate, render, out)
                    }
                    _ => {
                        out.push((None, self.render_call(&conditional.alternate, render)?));
                        Ok(())
                    }
                }
            }
            Expression::LogicalExpression(logical) if logical.operator.as_str() == "&&" => {
                out.push((
                    Some(logical.left.span()),
                    self.render_call(&logical.right, render)?,
                ));
                Ok(())
            }
            _ => fail("an if block of an unknown shape"),
        }
    }

    fn each_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let flags = arg(call, 1)
            .and_then(number)
            .ok_or("an each block with computed flags")?;
        if flags & 8 != 0 {
            return fail("an animated each block");
        }
        let node = self.node_expr(body, arg(call, 0).ok_or("an each block without an anchor")?)?;
        let collection = self.thunk(arg(call, 2).ok_or("an each block without a collection")?)?;
        let render = arg(call, 4).ok_or("an each block without a body")?;
        let (nodes, params) = self.render_fn(render, body)?;
        let mut item = params
            .get(1)
            .map(|symbol| self.scoping.symbol_name(*symbol).to_string());
        let index = params
            .get(2)
            .map(|symbol| self.scoping.symbol_name(*symbol).to_string())
            .filter(|name| name != "$$index");
        if item.as_deref() == Some("$$item") {
            item = Some(self.destructured(render)?);
        }
        let key = match arg(call, 3) {
            Some(key) if self.rt(key) == Some(Rt::Index) => None,
            Some(Expression::ArrowFunctionExpression(arrow))
                if item.as_deref().is_some_and(|item| item.starts_with('{')) =>
            {
                Some(
                    self.render(
                        thunk_body_any(arrow)
                            .ok_or("a key function of an unknown shape")?
                            .span(),
                    )?,
                )
            }
            Some(Expression::ArrowFunctionExpression(arrow)) => {
                let params =
                    param_symbols(&arrow.params).ok_or("a key function of an unknown shape")?;
                let [param] = params.as_slice() else {
                    return fail("a key function of an unknown shape");
                };
                if Some(self.scoping.symbol_name(*param)) != item.as_deref() {
                    return fail("a key function names its item differently");
                }
                Some(
                    self.render(
                        thunk_body_any(arrow)
                            .ok_or("a key function of an unknown shape")?
                            .span(),
                    )?,
                )
            }
            _ => return fail("an each block with a key of an unknown shape"),
        };
        let mut header = format!("{{#each {collection}");
        if let Some(item) = &item {
            header.push_str(&format!(" as {item}"));
        }
        if let Some(index) = &index {
            if item.is_none() {
                return fail("an each block with an index but no item");
            }
            header.push_str(&format!(", {index}"));
        }
        if let Some(key) = key {
            header.push_str(&format!(" ({key})"));
        }
        header.push('}');
        let mut sections = vec![(header, nodes)];
        if let Some(fallback) = arg(call, 5).filter(|fallback| !is_nullish(Some(fallback))) {
            let (nodes, _) = self.render_fn(fallback, body)?;
            sections.push(("{:else}".to_string(), nodes));
        }
        let block = M::Block {
            sections,
            close: "{/each}".to_string(),
        };
        match flags & 4 != 0 {
            true => {
                if !body.dom[node].children.is_empty()
                    || !matches!(body.dom[node].kind, DKind::Element { .. })
                {
                    return fail("a controlled each block on a node with children");
                }
                self.fill(body, node, Fill::Children(vec![block]))
            }
            false => self.fill_anchor(body, node, vec![block]),
        }
    }

    fn destructured(&self, render: &'a Expression<'a>) -> R<String> {
        let shape = "an each block that destructures its item in a way with no simple pattern";
        let Expression::ArrowFunctionExpression(arrow) = render.without_parentheses() else {
            return fail(shape);
        };
        let mut names = Vec::new();
        for symbol in crate::script::leading_getters(arrow_statements(arrow)) {
            let getter = self.arrow(symbol).ok_or(shape)?;
            let Some(Expression::StaticMemberExpression(member)) =
                getter.get_expression().map(Expression::without_parentheses)
            else {
                return fail(shape);
            };
            let Expression::CallExpression(read) = member.object.without_parentheses() else {
                return fail(shape);
            };
            let name = self.scoping.symbol_name(symbol);
            if self.rt(&read.callee) != Some(Rt::Get) || member.property.name != name {
                return fail(shape);
            }
            names.push(name.to_string());
        }
        if names.is_empty() {
            return fail(shape);
        }
        Ok(format!("{{ {} }}", names.join(", ")))
    }

    fn key_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("a key block without an anchor")?)?;
        let value = self.thunk(arg(call, 1).ok_or("a key block without a value")?)?;
        let (nodes, _) = self.render_fn(arg(call, 2).ok_or("a key block without a body")?, body)?;
        let block = M::Block {
            sections: vec![(format!("{{#key {value}}}"), nodes)],
            close: "{/key}".to_string(),
        };
        self.fill_anchor(body, node, vec![block])
    }

    fn await_block(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(
            body,
            arg(call, 0).ok_or("an await block without an anchor")?,
        )?;
        let value = self.thunk(arg(call, 1).ok_or("an await block without a value")?)?;
        let part = |index: usize, body: &mut Body| -> R<Option<(Vec<M>, Option<String>)>> {
            match arg(call, index).filter(|expr| !is_nullish(Some(expr))) {
                None => Ok(None),
                Some(callback) => {
                    let (nodes, params) = self.render_fn(callback, body)?;
                    Ok(Some((
                        nodes,
                        params
                            .get(1)
                            .map(|symbol| self.scoping.symbol_name(*symbol).to_string()),
                    )))
                }
            }
        };
        let pending = part(2, body)?;
        let then = part(3, body)?;
        let catch = part(4, body)?;
        let named = |word: &str, name: Option<String>| match name {
            Some(name) => format!("{word} {name}"),
            None => word.to_string(),
        };
        let mut sections = Vec::new();
        match (pending, then) {
            (Some((nodes, _)), then) => {
                sections.push((format!("{{#await {value}}}"), nodes));
                if let Some((nodes, name)) = then {
                    sections.push((format!("{{:{}}}", named("then", name)), nodes));
                }
            }
            (None, Some((nodes, name))) => {
                sections.push((format!("{{#await {value} {}}}", named("then", name)), nodes))
            }
            (None, None) => {}
        }
        if let Some((nodes, name)) = catch {
            match sections.is_empty() {
                true => sections.push((
                    format!("{{#await {value} {}}}", named("catch", name)),
                    nodes,
                )),
                false => sections.push((format!("{{:{}}}", named("catch", name)), nodes)),
            }
        }
        if sections.is_empty() {
            return fail("an await block without branches");
        }
        self.fill_anchor(
            body,
            node,
            vec![M::Block {
                sections,
                close: "{/await}".to_string(),
            }],
        )
    }

    fn html_tag(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("an html tag without an anchor")?)?;
        let value = self.thunk(arg(call, 1).ok_or("an html tag without a value")?)?;
        if call.arguments.len() > 5 {
            return fail("an html tag with extra arguments");
        }
        let controlled = boolean(arg(call, 2)).ok_or("an html tag with a computed flag")?;
        let tag = M::Tag(format!("{{@html {value}}}"));
        match controlled {
            true => {
                if !body.dom[node].children.is_empty() {
                    return fail("a controlled html tag on a node with children");
                }
                self.fill(body, node, Fill::Children(vec![tag]))
            }
            false => self.fill_anchor(body, node, vec![tag]),
        }
    }

    fn render_args(&self, call: &'a CallExpression<'a>, from: usize) -> R<String> {
        let mut args = Vec::new();
        for argument in call.arguments.iter().skip(from) {
            let expr = argument
                .as_expression()
                .ok_or("a spread argument to a snippet")?;
            args.push(self.thunk(expr)?);
        }
        Ok(args.join(", "))
    }

    fn dynamic_render(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let node = self.node_expr(body, arg(call, 0).ok_or("a render tag without an anchor")?)?;
        let getter = arg(call, 1).ok_or("a render tag without a snippet")?;
        let args = self.render_args(call, 2)?;
        let optional = match getter.without_parentheses() {
            Expression::ArrowFunctionExpression(arrow) if arrow.params.items.is_empty() => {
                match arrow_value(arrow).map(Expression::without_parentheses) {
                    Some(Expression::LogicalExpression(logical))
                        if logical.operator.as_str() == "??"
                            && self.rt(&logical.right) == Some(Rt::Noop) =>
                    {
                        Some(self.code(&logical.left)?)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let tag = match optional {
            Some(snippet) => format!("{{@render {snippet}?.({args})}}"),
            None => format!("{{@render {}({args})}}", self.thunk(getter)?),
        };
        self.fill_anchor(body, node, vec![M::Tag(tag)])
    }

    fn user_call(&self, body: &mut Body, call: &'a CallExpression<'a>) -> R<()> {
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return fail(format!(
                "the call `{}` has no template meaning",
                first_line(self.rw.text(call.span))
            ));
        };
        let symbol = symbol_of(callee, self.scoping).ok_or_else(|| {
            format!(
                "the call to the unbound `{}` has no template meaning",
                callee.name
            )
        })?;
        let node = self.node_expr(body, arg(call, 0).ok_or("a call in the template code")?)?;
        if self.arrow(symbol).is_some() && !body.deferred.contains(&symbol) {
            let args = self.render_args(call, 1)?;
            if !self.snippets.borrow().contains(&symbol) {
                self.snippets.borrow_mut().push(symbol);
            }
            return self.fill_anchor(
                body,
                node,
                vec![M::Tag(format!("{{@render {}({args})}}", callee.name))],
            );
        }
        let name = callee.name.as_str();
        if !name.starts_with(|c: char| c.is_ascii_uppercase()) || call.arguments.len() != 2 {
            return fail(format!(
                "the call to `{name}` is neither a snippet nor a component"
            ));
        }
        let Some(Expression::ObjectExpression(object)) =
            arg(call, 1).map(Expression::without_parentheses)
        else {
            return fail(format!(
                "the component `{name}` gets props that are not an object literal"
            ));
        };
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return fail(format!("the component `{name}` gets spread props"));
            };
            let key = key_name(&property.key)
                .filter(|_| !property.computed)
                .ok_or("a computed component prop")?;
            match (property.kind, key.as_str()) {
                (PropertyKind::Init, "$$slots") => {}
                (PropertyKind::Init, "children")
                    if matches!(
                        property.value.without_parentheses(),
                        Expression::ArrowFunctionExpression(_)
                    ) =>
                {
                    children = self.render_fn(&property.value, body)?.0;
                }
                (PropertyKind::Init, _) => match property.value.without_parentheses() {
                    Expression::StringLiteral(_) => attrs.push(format!(
                        "{key}=\"{}\"",
                        braces(
                            &string(&property.value)
                                .ok_or("a prop text with an invalid escape")?
                                .replace('&', "&amp;")
                                .replace('"', "&quot;")
                        )
                    )),
                    _ => attrs.push(attr(&key, &self.code(&property.value)?)),
                },
                (PropertyKind::Get, _) => {
                    let Expression::FunctionExpression(getter) = &property.value else {
                        return fail("a component prop getter of an unknown shape");
                    };
                    let statements = &getter
                        .body
                        .as_ref()
                        .ok_or("an empty prop getter")?
                        .statements;
                    let [Statement::ReturnStatement(ret)] = statements.as_slice() else {
                        return fail("a component prop getter of an unknown shape");
                    };
                    let code = self.code(
                        ret.argument
                            .as_ref()
                            .ok_or("a prop getter returns nothing")?,
                    )?;
                    attrs.push(attr(&key, &code));
                }
                (PropertyKind::Set, _) => {
                    let target = self.setter_function(&property.value)?;
                    let at = attrs
                        .iter()
                        .position(|text| *text == attr(&key, &target))
                        .ok_or_else(|| {
                            format!(
                                "the component prop `{key}` is written but not read the same way"
                            )
                        })?;
                    attrs[at] = match target == key {
                        true => format!("bind:{key}"),
                        false => format!("bind:{key}={{{target}}}"),
                    };
                }
            }
        }
        if !self.children.borrow().contains(&symbol) {
            self.children.borrow_mut().push(symbol);
        }
        let element = M::Element {
            name: name.to_string(),
            attrs,
            children,
            svg: true,
        };
        self.fill_anchor(body, node, vec![element])
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
