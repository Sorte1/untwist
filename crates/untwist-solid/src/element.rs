use std::cell::Cell;

use oxc_ast::ast::{
    BinaryOperator, BindingPattern, CallExpression, Expression, LogicalOperator, Statement,
};
use oxc_span::{GetSpan, Span};
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
    seq: usize,
    attr: String,
    handler: String,
    data: Option<String>,
}

struct Entry {
    seq: usize,
    name: String,
    value: String,
    fixed: bool,
    marked: bool,
}

#[derive(Default)]
struct Ops {
    attrs: Vec<(usize, String, String)>,
    classes: Vec<Entry>,
    styles: Vec<Entry>,
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
    seq: usize,
    fixed: bool,
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
    pub fn element_start(cx: &Cx<'_, '_>, statements: &[Statement<'_>]) -> Option<usize> {
        let Some(Statement::ReturnStatement(ret)) = statements.last() else {
            return None;
        };
        let root = expr_symbol(ret.argument.as_ref()?, cx.scoping)?;
        let found = statements
            .iter()
            .position(|statement| statement_template(cx, statement, root).is_some())?;
        let tail = Span::new(statements[found].span().start, ret.span.end);
        let mut start = found;
        while start > 0 {
            let Statement::VariableDeclaration(declaration) = &statements[start - 1] else {
                break;
            };
            let assigned = declaration.declarations.iter().all(|declarator| {
                declarator.init.is_none()
                    && binding(&declarator.id).is_some_and(|symbol| {
                        cx.scoping.get_resolved_references(symbol).any(|reference| {
                            let at = cx.nodes.kind(reference.node_id()).span();
                            reference.is_write() && tail.start <= at.start && at.end <= tail.end
                        })
                    })
            });
            if !assigned {
                break;
            }
            start -= 1;
        }
        Some(start)
    }

    pub fn starts_element(cx: &Cx<'_, '_>, statements: &[Statement<'_>]) -> bool {
        Self::element_start(cx, statements) == Some(0)
    }

    pub fn static_element(cx: &'c Cx<'a, 's>, template: &'a Template) -> R<String> {
        let block = Block {
            cx,
            template,
            paths: FxHashMap::default(),
            ops: FxHashMap::default(),
            root: None,
            dropped: Vec::new(),
            seq: 0,
            fixed: false,
        };
        block.build()
    }

    fn rooted(cx: &'c Cx<'a, 's>, root: SymbolId, template: &'a Template) -> Self {
        let mut paths = FxHashMap::default();
        paths.insert(root, Vec::new());
        Block {
            cx,
            template,
            paths,
            ops: FxHashMap::default(),
            root: Some(root),
            dropped: Vec::new(),
            seq: 0,
            fixed: false,
        }
    }

    pub fn element(cx: &'c Cx<'a, 's>, statements: &[Statement<'_>]) -> R<String> {
        let Some(Statement::ReturnStatement(ret)) = statements.last() else {
            return fail("an element block does not return its element");
        };
        let Some(root) = ret
            .argument
            .as_ref()
            .and_then(|value| expr_symbol(value, cx.scoping))
        else {
            return fail("an element block does not return its element");
        };
        let Some(template) = statements
            .iter()
            .find_map(|statement| statement_template(cx, statement, root))
        else {
            return fail("an element block does not start with its template");
        };
        let mut block = Block::rooted(cx, root, template);
        block.statements(statements)?;
        block.build()
    }

    pub fn sequence(
        cx: &'c Cx<'a, 's>,
        items: &[Expression<'_>],
    ) -> R<Option<(String, Vec<SymbolId>)>> {
        let Some((last, ops)) = items.split_last() else {
            return Ok(None);
        };
        let Some(root) = expr_symbol(last, cx.scoping) else {
            return Ok(None);
        };
        let Some(template) = ops.iter().find_map(|item| template_assign(cx, item, root)) else {
            return Ok(None);
        };
        let mut block = Block::rooted(cx, root, template);
        for item in ops {
            block.prebind(item)?;
            block.op(item)?;
        }
        let bound = block.paths.keys().copied().collect();
        Ok(Some((block.build()?, bound)))
    }

    fn prebind(&mut self, expr: &Expression<'_>) -> R<()> {
        match expr.without_parentheses() {
            Expression::AssignmentExpression(assign) => {
                if let Some((object, _)) = member_target(&assign.left) {
                    self.prebind(object)?;
                }
                self.prebind(&assign.right)?;
                let Some(symbol) = crate::convert::target_symbol(&assign.left, self.cx.scoping)
                else {
                    return Ok(());
                };
                if Some(symbol) == self.root && self.cx.template_of(&assign.right).is_some() {
                    return Ok(());
                }
                if let Some(path) = self.walk(&assign.right) {
                    self.paths.insert(symbol, path);
                }
                Ok(())
            }
            Expression::CallExpression(call) => {
                self.prebind(&call.callee)?;
                for arg in args(call).unwrap_or_default() {
                    self.prebind(arg)?;
                }
                Ok(())
            }
            Expression::StaticMemberExpression(member) => self.prebind(&member.object),
            Expression::SequenceExpression(sequence) => {
                for item in &sequence.expressions {
                    self.prebind(item)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn node(&self, expr: &Expression<'_>) -> Option<Path> {
        if let Expression::AssignmentExpression(assign) = expr.without_parentheses() {
            let symbol = crate::convert::target_symbol(&assign.left, self.cx.scoping)?;
            return self.paths.get(&symbol).cloned();
        }
        self.paths
            .get(&expr_symbol(expr, self.cx.scoping)?)
            .cloned()
    }

    fn target(&self, expr: &Expression<'_>) -> Option<Path> {
        self.node(expr).or_else(|| self.walk(expr))
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
        let from = self
            .node(&member.object)
            .or_else(|| self.walk(&member.object))?;
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
        let (BindingPattern::ObjectPattern(object), Some(from)) = (pattern, self.target(init))
        else {
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
        let mut pending: Vec<SymbolId> = Vec::new();
        let last = statements.len() - 1;
        for (index, statement) in statements.iter().enumerate() {
            if self.cx.is_deleted(statement) {
                continue;
            }
            if let Some(at) = holders.iter().position(|(holder, init)| {
                self.element_ref(statement, *holder, init).unwrap_or(false)
            }) {
                holders.remove(at);
                continue;
            }
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    for declarator in &declaration.declarations {
                        let Some(init) = &declarator.init else {
                            match binding(&declarator.id) {
                                Some(symbol) => pending.push(symbol),
                                None => return fail("an element block declares a pattern"),
                            }
                            continue;
                        };
                        self.prebind(init)?;
                        if binding(&declarator.id) == self.root
                            && self.cx.template_of(init).is_some()
                        {
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
                Statement::ExpressionStatement(statement) => {
                    self.prebind(&statement.expression)?;
                    self.op(&statement.expression)?
                }
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
        if pending
            .iter()
            .any(|symbol| !self.paths.contains_key(symbol))
        {
            return fail("an element block declares a variable it never binds to a node");
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

    fn tick(&mut self) -> usize {
        self.seq += 1;
        self.seq
    }

    fn push_attr(&mut self, path: Path, name: &str, text: String) {
        let seq = self.tick();
        self.ops_at(path).attrs.push((seq, name.to_string(), text));
    }

    fn push_event(&mut self, path: Path, attr: String, handler: String) {
        let seq = self.tick();
        self.ops_at(path).events.push(Event {
            seq,
            attr,
            handler,
            data: None,
        });
    }

    fn entry(&mut self, name: &str, value: &Expression<'_>, text: String) -> Entry {
        Entry {
            seq: self.tick(),
            name: name.to_string(),
            value: text,
            fixed: self.fixed,
            marked: self.fixed && crate::ast::is_dynamic(value),
        }
    }

    fn marked(&self, value: &Expression<'_>, text: String) -> String {
        match self.fixed {
            true => crate::ast::once(value, text),
            false => text,
        }
    }

    fn fixed_setter(&mut self, expr: &Expression<'_>) -> R<()> {
        self.fixed = true;
        let done = self.setter(expr, None, &FxHashMap::default());
        self.fixed = false;
        done
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
                Some(_) => self.fixed_setter(expr),
                None => {
                    if self.native_listener(call)? {
                        return Ok(());
                    }
                    self.fixed_setter(expr)
                }
            },
            Expression::AssignmentExpression(assign) => {
                if let Some(symbol) = crate::convert::target_symbol(&assign.left, cx.scoping) {
                    let is_root =
                        Some(symbol) == self.root && cx.template_of(&assign.right).is_some();
                    let is_walk =
                        self.paths.contains_key(&symbol) && self.walk(&assign.right).is_some();
                    if is_root || is_walk {
                        return Ok(());
                    }
                }
                if let Some((object, property)) = member_target(&assign.left) {
                    if let Some(path) = self.target(object) {
                        let custom = matches!(self.at(&path), Some(Node::Element(element)) if element.name.contains('-'));
                        if property == "_$owner" && custom && is_get_owner(cx, &assign.right) {
                            cx.ignored.borrow_mut().push(assign.right.span());
                            return Ok(());
                        }
                        if let Some(event) = property.strip_prefix("$$") {
                            return self.delegated(path, event, &assign.right);
                        }
                    }
                }
                self.fixed_setter(expr)
            }
            Expression::StaticMemberExpression(_) if self.walk(expr).is_some() => Ok(()),
            Expression::SequenceExpression(sequence) => {
                for item in &sequence.expressions {
                    self.op(item)?;
                }
                Ok(())
            }
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
        self.push_event(path, event_attr(event), text);
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
                    other => match self.target(other) {
                        Some(path) => Marker::Before(path),
                        None => return fail("insert has a marker outside the template"),
                    },
                };
                (parent, value, marker)
            }
            _ => return fail("insert has an unknown arity"),
        };
        let Some(parent) = self.target(parent) else {
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
        let children = self.cx.child(value, Ctx::Insert, true)?;
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
        let (Some(path), Some(name)) = (self.target(node), string(name)) else {
            return fail("addEventListener has an unknown target");
        };
        let handler = self.cx.render(handler)?;
        let attr = match delegate {
            true if name.chars().all(|c| !c.is_ascii_uppercase()) => event_attr(name),
            true => return fail(format!("the delegated event `{name}` has capitals")),
            false => format!("on:{name}"),
        };
        self.push_event(path, attr, handler);
        Ok(())
    }

    fn native_listener(&mut self, call: &CallExpression<'_>) -> R<bool> {
        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return Ok(false);
        };
        if member.property.name != "addEventListener" {
            return Ok(false);
        }
        let Some(path) = self.target(&member.object) else {
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
        self.push_event(path, attr, handler);
        Ok(true)
    }

    fn use_call(&mut self, call: &CallExpression<'_>) -> R<()> {
        let parts = args(call).unwrap_or_default();
        match parts.as_slice() {
            [function, node] => {
                let Some(path) = self.target(node) else {
                    return fail("use has a target outside the template");
                };
                let text = self.cx.render(function)?;
                self.push_attr(path, "ref", format!("ref={{{text}}}"));
                Ok(())
            }
            [function, node, value] => {
                let (Some(path), Expression::Identifier(ident)) =
                    (self.target(node), function.without_parentheses())
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
        let Some(path) = self.target(node) else {
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
        if !blank_state(arg(call, 1)) {
            return fail("an effect starts from a state other than undefined");
        }
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
                    let inner = match &branch.consequent {
                        Statement::ExpressionStatement(statement) => &statement.expression,
                        Statement::BlockStatement(block) => match block.body.as_slice() {
                            [Statement::ExpressionStatement(statement)] => &statement.expression,
                            _ => return fail("an effect guard holds several statements"),
                        },
                        _ => return fail("an effect guard holds a statement"),
                    };
                    if !self.guards(&branch.test, inner, param, &values) {
                        return fail("an effect has an unknown guard");
                    }
                    self.setter(inner, param, &values)?;
                }
                Statement::ExpressionStatement(statement) => {
                    match statement.expression.without_parentheses() {
                        Expression::LogicalExpression(logical)
                            if logical.operator == LogicalOperator::And
                                && self.guard(&logical.left, param, &values).is_some() =>
                        {
                            if !self.guards(&logical.left, &logical.right, param, &values) {
                                return fail("an effect has an unknown guard");
                            }
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
    ) -> Option<(SymbolId, String)> {
        let Expression::BinaryExpression(binary) = test.without_parentheses() else {
            return None;
        };
        if binary.operator != BinaryOperator::StrictInequality {
            return None;
        }
        let symbol = expr_symbol(&binary.left, self.cx.scoping)
            .filter(|symbol| values.contains_key(symbol))?;
        Some((symbol, self.state_key(&binary.right, param)?))
    }

    fn guards(
        &self,
        test: &Expression<'_>,
        inner: &Expression<'_>,
        param: Option<SymbolId>,
        values: &FxHashMap<SymbolId, &Expression<'_>>,
    ) -> bool {
        let Some(guarded) = self.guard(test, param, values) else {
            return false;
        };
        self.stored(inner, param) == Some(guarded)
    }

    fn stored(&self, expr: &Expression<'_>, param: Option<SymbolId>) -> Option<(SymbolId, String)> {
        match expr.without_parentheses() {
            Expression::AssignmentExpression(assign) => {
                if let Some((object, property)) = member_target(&assign.left) {
                    if param.is_some() && expr_symbol(object, self.cx.scoping) == param {
                        let symbol = expr_symbol(&assign.right, self.cx.scoping)?;
                        return Some((symbol, property.to_string()));
                    }
                }
                self.stored(&assign.right, param)
            }
            Expression::CallExpression(call) => {
                let mut found = call
                    .arguments
                    .iter()
                    .filter_map(|argument| argument.as_expression())
                    .filter_map(|argument| self.stored(argument, param));
                let first = found.next();
                found.next().is_none().then_some(first).flatten()
            }
            _ => None,
        }
    }

    fn state_key(&self, expr: &Expression<'_>, param: Option<SymbolId>) -> Option<String> {
        let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
            return None;
        };
        (param.is_some() && expr_symbol(&member.object, self.cx.scoping) == param)
            .then(|| member.property.name.to_string())
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
                let Some(path) = self.target(object) else {
                    return fail("an element block assigns to a value outside the template");
                };
                let value = self.value(&assign.right, param, values);
                let text = self.marked(value, cx.render(value)?);
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
                    let Some(path) = parts.first().and_then(|node| self.target(node)) else {
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
                            let entry = self.entry(name, value, text);
                            self.ops_at(path).styles.push(entry);
                            return Ok(());
                        }
                        _ => return fail("a setter call has no JSX spelling"),
                    };
                    let Some(value) = value else {
                        return fail("a setter has no value");
                    };
                    let text = self.marked(value, cx.render(value)?);
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
                let (Some(path), [name, value]) = (self.target(&list.object), parts.as_slice())
                else {
                    return fail("classList.toggle targets a value outside the template");
                };
                let Some(name) = string(name) else {
                    return fail("classList.toggle has a computed name");
                };
                let value = strip_bang(self.value(value, param, values));
                let text = cx.render(value)?;
                let entry = self.entry(name, value, text);
                self.ops_at(path).classes.push(entry);
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
        let mut fixed_styles: Vec<(String, String)> = Vec::new();
        for (name, value) in &element.attrs {
            if name == "style" && !ops.styles.is_empty() {
                let Some(entries) = value.as_deref().and_then(static_style) else {
                    return fail("a static style does not parse");
                };
                fixed_styles.extend(entries);
                continue;
            }
            names.push(name.clone());
            attrs.push(match value {
                Some(value) => format!(
                    "{name}=\"{}\"",
                    value
                        .replace('"', "&quot;")
                        .replace('\n', &untwist_restore::text::LINE.to_string())
                ),
                None => name.clone(),
            });
        }
        let mut placed: Vec<(usize, String, String)> = Vec::new();
        for (seq, name, text) in &ops.attrs {
            placed.push((*seq, name.clone(), text.clone()));
        }
        if let Some(text) = grouped("classList", &[], &ops.classes)? {
            placed.push((first_seq(&ops.classes), "classList".to_string(), text));
        }
        if let Some(text) = grouped("style", &fixed_styles, &ops.styles)? {
            placed.push((first_seq(&ops.styles), "style".to_string(), text));
        }
        placed.sort_by_key(|(seq, _, _)| *seq);
        for event in &ops.events {
            placed.push((
                event.seq,
                event.attr.clone(),
                match &event.data {
                    Some(data) => format!("{}={{[{}, {}]}}", event.attr, event.handler, data),
                    None => format!("{}={{{}}}", event.attr, event.handler),
                },
            ));
        }
        for (_, name, text) in placed {
            if !name.starts_with("...") {
                names.push(name);
            }
            attrs.push(text);
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

fn blank_state(state: Option<&Expression<'_>>) -> bool {
    let blank = |value: &Expression<'_>| match value.without_parentheses() {
        Expression::Identifier(ident) => ident.name == "undefined",
        Expression::UnaryExpression(unary) => {
            unary.operator == oxc_ast::ast::UnaryOperator::Void
                && matches!(
                    unary.argument.without_parentheses(),
                    Expression::NumericLiteral(_)
                )
        }
        _ => false,
    };
    match state.map(Expression::without_parentheses) {
        None => true,
        Some(Expression::ObjectExpression(object)) => object.properties.iter().all(|property| {
            matches!(property, oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) if !property.computed && property.kind == oxc_ast::ast::PropertyKind::Init && blank(&property.value))
        }),
        Some(other) => blank(other),
    }
}

fn first_seq(entries: &[Entry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.seq)
        .min()
        .unwrap_or(usize::MAX)
}

fn grouped(name: &str, fixed: &[(String, String)], entries: &[Entry]) -> R<Option<String>> {
    if entries.is_empty() {
        return Ok(None);
    }
    let marked = entries.iter().any(|entry| entry.marked);
    if marked && entries.iter().any(|entry| !entry.fixed) {
        return fail(format!(
            "`{name}` mixes values set once with values kept up to date"
        ));
    }
    let items: Vec<String> = fixed
        .iter()
        .map(|(key, value)| format!("{}: {value}", object_key(key)))
        .chain(
            entries
                .iter()
                .map(|entry| format!("{}: {}", object_key(&entry.name), entry.value)),
        )
        .collect();
    let mark = match marked {
        true => "/*@once*/ ",
        false => "",
    };
    Ok(Some(format!("{name}={{{mark}{{ {} }}}}", items.join(", "))))
}

fn template_assign<'a>(
    cx: &Cx<'a, '_>,
    expr: &Expression<'_>,
    root: SymbolId,
) -> Option<&'a Template> {
    match expr.without_parentheses() {
        Expression::AssignmentExpression(assign) => {
            if crate::convert::target_symbol(&assign.left, cx.scoping) == Some(root) {
                if let Some(template) = cx.template_of(&assign.right) {
                    return Some(template);
                }
            }
            template_assign(cx, &assign.right, root)
        }
        Expression::CallExpression(call) => template_assign(cx, &call.callee, root).or_else(|| {
            args(call)
                .unwrap_or_default()
                .into_iter()
                .find_map(|arg| template_assign(cx, arg, root))
        }),
        Expression::StaticMemberExpression(member) => template_assign(cx, &member.object, root),
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .iter()
            .find_map(|item| template_assign(cx, item, root)),
        _ => None,
    }
}

fn statement_template<'a>(
    cx: &Cx<'a, '_>,
    statement: &Statement<'_>,
    root: SymbolId,
) -> Option<&'a Template> {
    match statement {
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().find_map(|declarator| {
                let init = declarator.init.as_ref()?;
                match binding(&declarator.id) == Some(root) {
                    true => cx.template_of(init),
                    false => template_assign(cx, init, root),
                }
            })
        }
        Statement::ExpressionStatement(statement) => {
            template_assign(cx, &statement.expression, root)
        }
        _ => None,
    }
}

fn is_get_owner(cx: &Cx<'_, '_>, expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::CallExpression(call) if call.arguments.is_empty() && (cx.is(&call.callee, Rt::GetOwner) || cx.rt(&call.callee).is_none()))
}
