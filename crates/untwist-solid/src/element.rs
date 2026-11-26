use std::cell::Cell;

use oxc_ast::ast::{
    BinaryOperator, BindingPattern, CallExpression, Expression, LogicalOperator, Statement,
};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use untwist_restore::html::Node;

use crate::ast::{arg, args, binding, expr_symbol, is_identifier, string, thunk};
use crate::convert::{
    assigns_to, fail, member_target, ref_branch, strip_bang, Ctx, Cx, Template, R,
};
use crate::jsx::{self, js_string, Child};
use crate::runtime::Rt;

const PROPERTIES: &[&str] = &[
    "adauctionheaders",
    "adAuctionHeaders",
    "allowfullscreen",
    "allowFullscreen",
    "alpha",
    "async",
    "autofocus",
    "autoplay",
    "browsingtopics",
    "browsingTopics",
    "checked",
    "children",
    "className",
    "controls",
    "credentialless",
    "default",
    "defaultchecked",
    "defaultChecked",
    "defaultmuted",
    "defaultMuted",
    "defaultselected",
    "defaultSelected",
    "defer",
    "disabled",
    "disablepictureinpicture",
    "disablePictureInPicture",
    "disableremoteplayback",
    "disableRemotePlayback",
    "formnovalidate",
    "formNoValidate",
    "hidden",
    "indeterminate",
    "inert",
    "innerHTML",
    "innerText",
    "ismap",
    "isMap",
    "loop",
    "multiple",
    "muted",
    "nomodule",
    "noModule",
    "novalidate",
    "noValidate",
    "open",
    "playsinline",
    "playsInline",
    "preservespitch",
    "preservesPitch",
    "readonly",
    "readOnly",
    "required",
    "reversed",
    "seamless",
    "selected",
    "shadowrootclonable",
    "shadowRootClonable",
    "shadowrootcustomelementregistry",
    "shadowRootCustomElementRegistry",
    "shadowrootdelegatesfocus",
    "shadowRootDelegatesFocus",
    "shadowrootserializable",
    "shadowRootSerializable",
    "sharedstoragewritable",
    "sharedStorageWritable",
    "textContent",
    "value",
];

const DELEGATED: &[&str] = &[
    "beforeinput",
    "click",
    "dblclick",
    "contextmenu",
    "focusin",
    "focusout",
    "input",
    "keydown",
    "keyup",
    "mousedown",
    "mousemove",
    "mouseout",
    "mouseover",
    "mouseup",
    "pointerdown",
    "pointermove",
    "pointerout",
    "pointerover",
    "pointerup",
    "touchend",
    "touchmove",
    "touchstart",
];

type Path = Vec<usize>;

#[derive(Clone, PartialEq, Eq)]
enum Marker {
    Only,
    End,
    Before(Path),
}

struct Event {
    attr: String,
    handler: String,
    data: Option<String>,
}

#[derive(Default)]
struct Ops {
    attrs: Vec<(String, String)>,
    classes: Vec<(String, String)>,
    styles: Vec<(String, String)>,
    events: Vec<Event>,
    inserts: Vec<(Marker, Vec<Child>)>,
}

pub struct Block<'c, 'a, 's> {
    cx: &'c Cx<'a, 's>,
    template: &'a Template,
    paths: FxHashMap<SymbolId, Path>,
    ops: FxHashMap<Path, Ops>,
    root: Option<SymbolId>,
    dropped: Vec<Path>,
}

const EVENT_WORDS: &[(&str, &str)] = &[
    ("beforeinput", "BeforeInput"),
    ("dblclick", "DblClick"),
    ("contextmenu", "ContextMenu"),
    ("focusin", "FocusIn"),
    ("focusout", "FocusOut"),
    ("keydown", "KeyDown"),
    ("keyup", "KeyUp"),
    ("keypress", "KeyPress"),
    ("mousedown", "MouseDown"),
    ("mousemove", "MouseMove"),
    ("mouseout", "MouseOut"),
    ("mouseover", "MouseOver"),
    ("mouseup", "MouseUp"),
    ("mouseenter", "MouseEnter"),
    ("mouseleave", "MouseLeave"),
    ("pointerdown", "PointerDown"),
    ("pointermove", "PointerMove"),
    ("pointerout", "PointerOut"),
    ("pointerover", "PointerOver"),
    ("pointerup", "PointerUp"),
    ("pointerenter", "PointerEnter"),
    ("pointerleave", "PointerLeave"),
    ("touchend", "TouchEnd"),
    ("touchmove", "TouchMove"),
    ("touchstart", "TouchStart"),
    ("dragstart", "DragStart"),
    ("dragend", "DragEnd"),
    ("dragover", "DragOver"),
    ("dragenter", "DragEnter"),
    ("dragleave", "DragLeave"),
    ("animationend", "AnimationEnd"),
    ("transitionend", "TransitionEnd"),
];

fn event_attr(name: &str) -> String {
    match EVENT_WORDS.iter().find(|(word, _)| *word == name) {
        Some((_, camel)) => format!("on{camel}"),
        None => format!("on{}", capitalised(name)),
    }
}

fn capitalised(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn object_key(name: &str) -> String {
    match is_identifier(name) {
        true => name.to_string(),
        false => js_string(name),
    }
}

fn static_style(text: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    for part in text.split(';') {
        if part.trim().is_empty() {
            continue;
        }
        let (key, value) = part.split_once(':')?;
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() || value.contains(['(', '"', '\'', '&']) {
            return None;
        }
        out.push((key.to_string(), js_string(value)));
    }
    Some(out)
}

impl<'c, 'a, 's> Block<'c, 'a, 's> {
    pub fn starts_element(cx: &Cx<'_, '_>, statements: &[Statement<'_>]) -> bool {
        let Some(Statement::VariableDeclaration(declaration)) = statements.first() else {
            return false;
        };
        declaration
            .declarations
            .first()
            .and_then(|declarator| declarator.init.as_ref())
            .is_some_and(|init| cx.template_of(init).is_some())
    }

    pub fn static_element(cx: &'c Cx<'a, 's>, template: &'a Template) -> R<String> {
        let block = Block {
            cx,
            template,
            paths: FxHashMap::default(),
            ops: FxHashMap::default(),
            root: None,
            dropped: Vec::new(),
        };
        block.build()
    }

    pub fn element(cx: &'c Cx<'a, 's>, statements: &[Statement<'_>]) -> R<String> {
        let Some(Statement::VariableDeclaration(declaration)) = statements.first() else {
            return fail("an element block does not start with its template");
        };
        let Some(template) = declaration
            .declarations
            .first()
            .and_then(|declarator| declarator.init.as_ref())
            .and_then(|init| cx.template_of(init))
        else {
            return fail("an element block does not start with its template");
        };
        let mut block = Block {
            cx,
            template,
            paths: FxHashMap::default(),
            ops: FxHashMap::default(),
            root: None,
            dropped: Vec::new(),
        };
        block.statements(statements)?;
        block.build()
    }

    fn node(&self, expr: &Expression<'_>) -> Option<Path> {
        self.paths
            .get(&expr_symbol(expr, self.cx.scoping)?)
            .cloned()
    }

    fn at(&self, path: &[usize]) -> Option<&'a Node> {
        let mut node = self.template.nodes.first()?;
        for index in path {
            let Node::Element(element) = node else {
                return None;
            };
            node = element.children.get(*index)?;
        }
        Some(node)
    }

    fn step(&self, from: &Path, property: &str) -> Option<Path> {
        let mut path = from.clone();
        match property {
            "firstChild" => path.push(0),
            "nextSibling" => *path.last_mut()? += 1,
            _ => return None,
        }
        self.at(&path).map(|_| path)
    }

    fn walk(&self, expr: &Expression<'_>) -> Option<Path> {
        let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
            return None;
        };
        let from = self.node(&member.object)?;
        self.step(&from, member.property.name.as_str())
    }

    fn bind(&mut self, pattern: &BindingPattern<'_>, init: &Expression<'_>) -> R<bool> {
        if let Some(path) = self.walk(init) {
            let Some(symbol) = binding(pattern) else {
                return fail("a walk binds a pattern");
            };
            self.paths.insert(symbol, path);
            return Ok(true);
        }
        let (BindingPattern::ObjectPattern(object), Some(from)) = (pattern, self.node(init)) else {
            return Ok(false);
        };
        if object.rest.is_some() {
            return fail("a walk binds a rest pattern");
        }
        for property in &object.properties {
            let key = property
                .key
                .static_name()
                .and_then(|key| key.as_str().map(str::to_string));
            let (Some(key), Some(symbol)) = (key, binding(&property.value)) else {
                return fail("a walk binds a pattern");
            };
            let Some(path) = self.step(&from, &key) else {
                return fail("a walk leaves the template");
            };
            self.paths.insert(symbol, path);
        }
        Ok(true)
    }

    fn statements(&mut self, statements: &[Statement<'_>]) -> R<()> {
        let mut holders: Vec<(SymbolId, &Expression<'_>)> = Vec::new();
        let last = statements.len() - 1;
        for (index, statement) in statements.iter().enumerate() {
            if let Some(at) = holders.iter().position(|(holder, init)| {
                self.element_ref(statement, *holder, init).unwrap_or(false)
            }) {
                holders.remove(at);
                continue;
            }
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    for (at, declarator) in declaration.declarations.iter().enumerate() {
                        let Some(init) = &declarator.init else {
                            return fail("an element block declares a variable with no value");
                        };
                        if index == 0 && at == 0 {
                            let Some(symbol) = binding(&declarator.id) else {
                                return fail("an element block binds its template to a pattern");
                            };
                            self.root = Some(symbol);
                            self.paths.insert(symbol, Vec::new());
                            continue;
                        }
                        if self.bind(&declarator.id, init)? {
                            continue;
                        }
                        let Some(symbol) = binding(&declarator.id) else {
                            return fail("an element block binds a pattern");
                        };
                        holders.push((symbol, init));
                    }
                }
                Statement::ExpressionStatement(statement) => self.op(&statement.expression)?,
                Statement::ReturnStatement(ret) if index == last => {
                    let returns_root = ret
                        .argument
                        .as_ref()
                        .and_then(|value| expr_symbol(value, self.cx.scoping))
                        .is_some_and(|symbol| Some(symbol) == self.root);
                    if !returns_root {
                        return fail("an element block returns something other than its element");
                    }
                }
                _ => return fail("an element block holds a statement with no JSX spelling"),
            }
        }
        if !matches!(statements.last(), Some(Statement::ReturnStatement(_))) {
            return fail("an element block does not return its element");
        }
        if !holders.is_empty() {
            return fail("an element block declares a value it never uses as a ref");
        }
        Ok(())
    }

    fn element_ref(
        &mut self,
        statement: &Statement<'_>,
        holder: SymbolId,
        init: &Expression<'_>,
    ) -> R<bool> {
        let cx = self.cx;
        let found: Cell<Option<SymbolId>> = Cell::new(None);
        let yes = |expr: &Expression<'_>| -> bool {
            let Expression::CallExpression(call) = expr.without_parentheses() else {
                return false;
            };
            if !cx.is(&call.callee, Rt::Use) || call.arguments.len() != 2 {
                return false;
            }
            if arg(call, 0).and_then(|a| expr_symbol(a, cx.scoping)) != Some(holder) {
                return false;
            }
            found.set(arg(call, 1).and_then(|a| expr_symbol(a, cx.scoping)));
            found.get().is_some()
        };
        let target = cx.rw.text(init.span()).to_string();
        let no = |expr: &Expression<'_>| -> bool {
            found
                .get()
                .is_some_and(|node| assigns_to(cx, expr, &target, node))
        };
        if !ref_branch(cx, statement, holder, &yes, &no) {
            return Ok(false);
        }
        let Some(path) = found
            .get()
            .and_then(|symbol| self.paths.get(&symbol).cloned())
        else {
            return Ok(false);
        };
        let text = cx.render(init)?;
        self.push_attr(path, "ref", format!("ref={{{text}}}"));
        Ok(true)
    }

    fn ops_at(&mut self, path: Path) -> &mut Ops {
        self.ops.entry(path).or_default()
    }

    fn push_attr(&mut self, path: Path, name: &str, text: String) {
        self.ops_at(path).attrs.push((name.to_string(), text));
    }

    fn op(&mut self, expr: &Expression<'_>) -> R<()> {
        let cx = self.cx;
        match expr.without_parentheses() {
            Expression::CallExpression(call) => match cx.rt(&call.callee) {
                Some(Rt::Insert) => self.insert(call),
                Some(Rt::Effect) => self.effect(call),
                Some(Rt::AddEventListener) => self.listener(call),
                Some(Rt::Use) => self.use_call(call),
                Some(Rt::Spread) => self.spread(call),
                Some(_) => self.setter(expr, None, &FxHashMap::default()),
                None => {
                    if self.native_listener(call)? {
                        return Ok(());
                    }
                    self.setter(expr, None, &FxHashMap::default())
                }
            },
            Expression::AssignmentExpression(assign) => {
                if let Some((object, property)) = member_target(&assign.left) {
                    if let Some(path) = self.node(object) {
                        if property == "_$owner" && is_get_owner(cx, &assign.right) {
                            return Ok(());
                        }
                        if let Some(event) = property.strip_prefix("$$") {
                            return self.delegated(path, event, &assign.right);
                        }
                    }
                }
                self.setter(expr, None, &FxHashMap::default())
            }
            Expression::StaticMemberExpression(_) if self.walk(expr).is_some() => Ok(()),
            _ => fail("an element block holds an expression with no JSX spelling"),
        }
    }

    fn delegated(&mut self, path: Path, event: &str, value: &Expression<'_>) -> R<()> {
        let text = self.cx.render(value)?;
        if let Some(name) = event.strip_suffix("Data") {
            let ops = self.ops_at(path.clone());
            if let Some(found) = ops
                .events
                .iter_mut()
                .rev()
                .find(|found| found.attr == event_attr(name))
            {
                if found.data.is_none() {
                    found.data = Some(text);
                    return Ok(());
                }
            }
            if !DELEGATED.contains(&name) {
                return fail(format!("the event data `$${event}` has no handler"));
            }
        }
        if !DELEGATED.contains(&event) {
            return fail(format!("the event `{event}` is not delegated"));
        }
        self.ops_at(path).events.push(Event {
            attr: event_attr(event),
            handler: text,
            data: None,
        });
        Ok(())
    }

    fn insert(&mut self, call: &CallExpression<'_>) -> R<()> {
        let Some(parts) = args(call) else {
            return fail("insert has a spread argument");
        };
        let (parent, value, marker) = match parts.as_slice() {
            [parent, value] => (parent, value, Marker::Only),
            [parent, value, marker] => {
                let marker = match marker.without_parentheses() {
                    Expression::NullLiteral(_) => Marker::End,
                    other => match self.node(other) {
                        Some(path) => Marker::Before(path),
                        None => return fail("insert has a marker outside the template"),
                    },
                };
                (parent, value, marker)
            }
            _ => return fail("insert has an unknown arity"),
        };
        let Some(parent) = self.node(parent) else {
            return fail("insert has a parent outside the template");
        };
        if let Marker::Before(path) = &marker {
            if path.len() != parent.len() + 1 || !path.starts_with(&parent) {
                return fail("insert has a marker that is not a child of its parent");
            }
        }
        if !matches!(self.at(&parent), Some(Node::Element(_))) {
            return fail("insert has a parent that is not an element");
        }
        let children = self.cx.child(value, Ctx::Insert)?;
        self.ops_at(parent).inserts.push((marker, children));
        Ok(())
    }

    fn listener(&mut self, call: &CallExpression<'_>) -> R<()> {
        let parts = args(call).unwrap_or_default();
        let (node, name, handler, delegate) = match parts.as_slice() {
            [node, name, handler] => (node, name, handler, false),
            [node, name, handler, flag] => (
                node,
                name,
                handler,
                matches!(flag.without_parentheses(), Expression::BooleanLiteral(b) if b.value),
            ),
            _ => return fail("addEventListener has an unknown arity"),
        };
        let (Some(path), Some(name)) = (self.node(node), string(name)) else {
            return fail("addEventListener has an unknown target");
        };
        let handler = self.cx.render(handler)?;
        let attr = match delegate {
            true if name.chars().all(|c| !c.is_ascii_uppercase()) => event_attr(name),
            true => return fail(format!("the delegated event `{name}` has capitals")),
            false => format!("on:{name}"),
        };
        self.ops_at(path).events.push(Event {
            attr,
            handler,
            data: None,
        });
        Ok(())
    }

    fn native_listener(&mut self, call: &CallExpression<'_>) -> R<bool> {
        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return Ok(false);
        };
        if member.property.name != "addEventListener" {
            return Ok(false);
        }
        let Some(path) = self.node(&member.object) else {
            return Ok(false);
        };
        let parts = args(call).unwrap_or_default();
        let (name, handler, capture) = match parts.as_slice() {
            [name, handler] => (name, handler, false),
            [name, handler, flag] => match flag.without_parentheses() {
                Expression::BooleanLiteral(b) if b.value => (name, handler, true),
                _ => return fail("a listener has options"),
            },
            _ => return fail("a listener has an unknown arity"),
        };
        let Some(name) = string(name) else {
            return fail("a listener has a computed event name");
        };
        let attr = match capture {
            true => format!("oncapture:{name}"),
            false if DELEGATED.contains(&name) || name.chars().any(|c| c.is_ascii_uppercase()) => {
                return fail(format!("the listener `{name}` has no JSX spelling"))
            }
            false => event_attr(name),
        };
        let handler = self.cx.render(handler)?;
        self.ops_at(path).events.push(Event {
            attr,
            handler,
            data: None,
        });
        Ok(true)
    }

    fn use_call(&mut self, call: &CallExpression<'_>) -> R<()> {
        let parts = args(call).unwrap_or_default();
        match parts.as_slice() {
            [function, node] => {
                let Some(path) = self.node(node) else {
                    return fail("use has a target outside the template");
                };
                let text = self.cx.render(function)?;
                self.push_attr(path, "ref", format!("ref={{{text}}}"));
                Ok(())
            }
            [function, node, value] => {
                let (Some(path), Expression::Identifier(ident)) =
                    (self.node(node), function.without_parentheses())
                else {
                    return fail("a directive is not a plain name");
                };
                let Some(value) = thunk(value) else {
                    return fail("a directive value is not a thunk");
                };
                let name = self.cx.render(function)?;
                let _ = ident;
                let text = self.cx.render(value)?;
                self.push_attr(
                    path,
                    &format!("use:{name}"),
                    format!("use:{name}={{{text}}}"),
                );
                Ok(())
            }
            _ => fail("use has an unknown arity"),
        }
    }

    fn spread(&mut self, call: &CallExpression<'_>) -> R<()> {
        let parts = args(call).unwrap_or_default();
        let (Some(node), Some(props)) = (parts.first(), parts.get(1)) else {
            return fail("spread has no props");
        };
        let Some(path) = self.node(node) else {
            return fail("spread has a target outside the template");
        };
        let mut attrs = Vec::new();
        let mut children = Vec::new();
        match props.without_parentheses() {
            Expression::CallExpression(merge) if self.cx.is(&merge.callee, Rt::MergeProps) => {
                for part in args(merge).unwrap_or_default() {
                    match part.without_parentheses() {
                        Expression::ObjectExpression(object) => {
                            self.cx.props_object(object, &mut attrs, &mut children)?
                        }
                        other => attrs.push(format!("{{...{}}}", self.cx.render(other)?)),
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                self.cx.props_object(object, &mut attrs, &mut children)?
            }
            other => attrs.push(format!("{{...{}}}", self.cx.render(other)?)),
        }
        if !children.is_empty() {
            return fail("a spread carries children");
        }
        for (index, attr) in attrs.into_iter().enumerate() {
            self.push_attr(path.clone(), &format!("...{index}"), attr);
        }
        Ok(())
    }

    fn effect(&mut self, call: &CallExpression<'_>) -> R<()> {
        let Some(Expression::ArrowFunctionExpression(arrow)) =
            arg(call, 0).map(Expression::without_parentheses)
        else {
            return fail("an effect has no callback");
        };
        let param = match arrow.params.items.as_slice() {
            [] => None,
            [param] => match binding(&param.pattern) {
                Some(symbol) => Some(symbol),
                None => return fail("an effect binds a pattern"),
            },
            _ => return fail("an effect takes several parameters"),
        };
        if let Some(body) = arrow.get_expression() {
            return self.setter(body, param, &FxHashMap::default());
        }
        let statements = crate::ast::arrow_statements(arrow);
        let mut values: FxHashMap<SymbolId, &Expression<'_>> = FxHashMap::default();
        for statement in statements {
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    for declarator in &declaration.declarations {
                        let (Some(symbol), Some(init)) =
                            (binding(&declarator.id), &declarator.init)
                        else {
                            return fail("an effect declares a pattern");
                        };
                        values.insert(symbol, init);
                    }
                }
                Statement::IfStatement(branch) if branch.alternate.is_none() => {
                    if !self.guard(&branch.test, param, &values) {
                        return fail("an effect has an unknown guard");
                    }
                    let inner = match &branch.consequent {
                        Statement::ExpressionStatement(statement) => &statement.expression,
                        Statement::BlockStatement(block) => match block.body.as_slice() {
                            [Statement::ExpressionStatement(statement)] => &statement.expression,
                            _ => return fail("an effect guard holds several statements"),
                        },
                        _ => return fail("an effect guard holds a statement"),
                    };
                    self.setter(inner, param, &values)?;
                }
                Statement::ExpressionStatement(statement) => {
                    match statement.expression.without_parentheses() {
                        Expression::LogicalExpression(logical)
                            if logical.operator == LogicalOperator::And
                                && self.guard(&logical.left, param, &values) =>
                        {
                            self.setter(&logical.right, param, &values)?
                        }
                        other => self.setter(other, param, &values)?,
                    }
                }
                Statement::ReturnStatement(ret) => {
                    let returns_param = ret
                        .argument
                        .as_ref()
                        .and_then(|value| expr_symbol(value, self.cx.scoping));
                    if returns_param.is_none() || returns_param != param {
                        return fail("an effect returns something other than its state");
                    }
                }
                _ => return fail("an effect holds a statement with no JSX spelling"),
            }
        }
        Ok(())
    }

    fn guard(
        &self,
        test: &Expression<'_>,
        param: Option<SymbolId>,
        values: &FxHashMap<SymbolId, &Expression<'_>>,
    ) -> bool {
        let Expression::BinaryExpression(binary) = test.without_parentheses() else {
            return false;
        };
        binary.operator == BinaryOperator::StrictInequality
            && expr_symbol(&binary.left, self.cx.scoping)
                .is_some_and(|symbol| values.contains_key(&symbol))
            && self.is_state(&binary.right, param)
    }

    fn is_state(&self, expr: &Expression<'_>, param: Option<SymbolId>) -> bool {
        let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
            return false;
        };
        param.is_some() && expr_symbol(&member.object, self.cx.scoping) == param
    }

    fn value<'e>(
        &self,
        expr: &'e Expression<'e>,
        param: Option<SymbolId>,
        values: &FxHashMap<SymbolId, &'e Expression<'e>>,
    ) -> &'e Expression<'e> {
        let expr = expr.without_parentheses();
        if let Expression::AssignmentExpression(assign) = expr {
            if let Some((object, _)) = member_target(&assign.left) {
                if param.is_some() && expr_symbol(object, self.cx.scoping) == param {
                    return self.value(&assign.right, param, values);
                }
            }
        }
        if let Some(found) =
            expr_symbol(expr, self.cx.scoping).and_then(|symbol| values.get(&symbol))
        {
            return found;
        }
        expr
    }

    fn setter<'e>(
        &mut self,
        expr: &'e Expression<'e>,
        param: Option<SymbolId>,
        values: &FxHashMap<SymbolId, &'e Expression<'e>>,
    ) -> R<()> {
        let cx = self.cx;
        let expr = expr.without_parentheses();
        match expr {
            Expression::AssignmentExpression(assign) => {
                let Some((object, property)) = member_target(&assign.left) else {
                    return fail("an element block assigns to a plain name");
                };
                if param.is_some() && expr_symbol(object, cx.scoping) == param {
                    return self.setter(&assign.right, param, values);
                }
                let Some(path) = self.node(object) else {
                    return fail("an element block assigns to a value outside the template");
                };
                let value = self.value(&assign.right, param, values);
                let text = cx.render(value)?;
                if let Some(Node::Text(_)) = self.at(&path) {
                    let parent = path[..path.len().saturating_sub(1)].to_vec();
                    let lone = matches!(self.at(&parent), Some(Node::Element(element)) if element.children.len() == 1);
                    if property != "data" || path.is_empty() || !lone {
                        return fail("an element block sets a text node in a way JSX cannot spell");
                    }
                    self.dropped.push(path);
                    self.push_attr(parent, "textContent", format!("textContent={{{text}}}"));
                    return Ok(());
                }
                let name = match PROPERTIES.contains(&property) {
                    true => property.to_string(),
                    false => format!("prop:{property}"),
                };
                self.push_attr(path, &name, format!("{name}={{{text}}}"));
                Ok(())
            }
            Expression::CallExpression(call) => {
                let parts = args(call).unwrap_or_default();
                if let Some(rt) = cx.rt(&call.callee) {
                    let Some(path) = parts.first().and_then(|node| self.node(node)) else {
                        return fail("a setter targets a value outside the template");
                    };
                    let value_at = |index: usize| {
                        parts
                            .get(index)
                            .map(|value| self.value(value, param, values))
                    };
                    let (name, value) = match (rt, parts.len()) {
                        (Rt::SetAttribute, 3) => {
                            let Some(name) = parts.get(1).and_then(|name| string(name)) else {
                                return fail("setAttribute has a computed name");
                            };
                            let custom = matches!(self.at(&path), Some(Node::Element(element)) if element.name.contains('-'));
                            let name = match PROPERTIES.contains(&name) || custom {
                                true => format!("attr:{name}"),
                                false => name.to_string(),
                            };
                            (name, value_at(2))
                        }
                        (Rt::SetAttributeNs, 4) => match parts.get(2).and_then(|name| string(name))
                        {
                            Some(name) => (name.to_string(), value_at(3)),
                            None => return fail("setAttributeNS has a computed name"),
                        },
                        (Rt::SetBoolAttribute, 3) => {
                            match parts.get(1).and_then(|name| string(name)) {
                                Some(name) => (format!("bool:{name}"), value_at(2)),
                                None => return fail("setBoolAttribute has a computed name"),
                            }
                        }
                        (Rt::ClassName, 2) => ("class".to_string(), value_at(1)),
                        (Rt::ClassList, 2 | 3) => ("classList".to_string(), value_at(1)),
                        (Rt::Style, 2 | 3) => ("style".to_string(), value_at(1)),
                        (Rt::SetStyleProperty, 3) => {
                            let (Some(name), Some(value)) =
                                (parts.get(1).and_then(|name| string(name)), value_at(2))
                            else {
                                return fail("setStyleProperty has a computed name");
                            };
                            let text = cx.render(value)?;
                            self.ops_at(path).styles.push((name.to_string(), text));
                            return Ok(());
                        }
                        _ => return fail("a setter call has no JSX spelling"),
                    };
                    let Some(value) = value else {
                        return fail("a setter has no value");
                    };
                    let text = cx.render(value)?;
                    self.push_attr(path, &name, format!("{name}={{{text}}}"));
                    return Ok(());
                }
                let Expression::StaticMemberExpression(member) = call.callee.without_parentheses()
                else {
                    return fail("an element block calls a function with no JSX spelling");
                };
                let Expression::StaticMemberExpression(list) = member.object.without_parentheses()
                else {
                    return fail("an element block calls a method with no JSX spelling");
                };
                if member.property.name != "toggle" || list.property.name != "classList" {
                    return fail("an element block calls a method with no JSX spelling");
                }
                let (Some(path), [name, value]) = (self.node(&list.object), parts.as_slice())
                else {
                    return fail("classList.toggle targets a value outside the template");
                };
                let Some(name) = string(name) else {
                    return fail("classList.toggle has a computed name");
                };
                let value = strip_bang(self.value(value, param, values));
                let text = cx.render(value)?;
                self.ops_at(path).classes.push((name.to_string(), text));
                Ok(())
            }
            _ => fail("an effect holds an expression with no JSX spelling"),
        }
    }

    fn build(&self) -> R<String> {
        if self.template.nodes.len() != 1 {
            return fail("a template holds several roots");
        }
        if self
            .ops
            .keys()
            .any(|path| !matches!(self.at(path), Some(Node::Element(_))))
        {
            return fail("an element block sets a value on a text node");
        }
        match self.build_node(&self.template.nodes[0], Vec::new())? {
            Some(Child::Jsx(text)) => Ok(text),
            _ => fail("a template root is not an element"),
        }
    }

    fn build_node(&self, node: &Node, path: Path) -> R<Option<Child>> {
        let element = match node {
            Node::Text(text) => return Ok(Some(Child::Html(text.clone()))),
            Node::Comment(_) => return Ok(None),
            Node::Element(element) => element,
        };
        let empty = Ops::default();
        let ops = self.ops.get(&path).unwrap_or(&empty);
        let mut names: Vec<String> = Vec::new();
        let mut attrs: Vec<String> = Vec::new();
        let mut styles: Vec<(String, String)> = Vec::new();
        for (name, value) in &element.attrs {
            if name == "style" && !ops.styles.is_empty() {
                let Some(entries) = value.as_deref().and_then(static_style) else {
                    return fail("a static style does not parse");
                };
                styles.extend(entries);
                continue;
            }
            names.push(name.clone());
            attrs.push(match value {
                Some(value) => format!("{name}=\"{}\"", value.replace('"', "&quot;")),
                None => name.clone(),
            });
        }
        for (name, text) in &ops.attrs {
            if !name.starts_with("...") {
                names.push(name.clone());
            }
            attrs.push(text.clone());
        }
        for event in &ops.events {
            names.push(event.attr.clone());
            attrs.push(match &event.data {
                Some(data) => format!("{}={{[{}, {}]}}", event.attr, event.handler, data),
                None => format!("{}={{{}}}", event.attr, event.handler),
            });
        }
        if !ops.classes.is_empty() {
            names.push("classList".to_string());
            let entries: Vec<String> = ops
                .classes
                .iter()
                .map(|(name, value)| format!("{}: {value}", object_key(name)))
                .collect();
            attrs.push(format!("classList={{{{ {} }}}}", entries.join(", ")));
        }
        styles.extend(ops.styles.iter().cloned());
        if !styles.is_empty() {
            names.push("style".to_string());
            let entries: Vec<String> = styles
                .iter()
                .map(|(name, value)| format!("{}: {value}", object_key(name)))
                .collect();
            attrs.push(format!("style={{{{ {} }}}}", entries.join(", ")));
        }
        let mut seen: Vec<&String> = Vec::new();
        for name in &names {
            if seen.contains(&name) {
                return fail(format!("the attribute `{name}` is set twice"));
            }
            seen.push(name);
        }
        let mut children = Vec::new();
        let only: Vec<&Vec<Child>> = ops
            .inserts
            .iter()
            .filter(|(marker, _)| *marker == Marker::Only)
            .map(|(_, children)| children)
            .collect();
        if !only.is_empty() {
            if only.len() != 1 || ops.inserts.len() != 1 || !element.children.is_empty() {
                return fail("an element both holds content and is replaced by an insert");
            }
            children.extend(only[0].iter().cloned());
        } else {
            for (index, child) in element.children.iter().enumerate() {
                let mut at = path.clone();
                at.push(index);
                for (marker, inserted) in &ops.inserts {
                    if *marker == Marker::Before(at.clone()) {
                        children.extend(inserted.iter().cloned());
                    }
                }
                if self.dropped.contains(&at) {
                    continue;
                }
                if let Some(child) = self.build_node(child, at)? {
                    children.push(child);
                }
            }
            for (marker, inserted) in &ops.inserts {
                if *marker == Marker::End {
                    children.extend(inserted.iter().cloned());
                }
            }
        }
        Ok(Some(Child::Jsx(jsx::element(
            &element.name,
            &attrs,
            &children,
        ))))
    }
}

fn is_get_owner(cx: &Cx<'_, '_>, expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::CallExpression(call) if cx.is(&call.callee, Rt::GetOwner) && call.arguments.is_empty())
}
