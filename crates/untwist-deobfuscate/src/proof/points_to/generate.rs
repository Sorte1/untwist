use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;

use super::object::{Builtin, ObjectId, SiteKind};
use super::solve::{Call, Field, FnInfo, Graph, Key, Node};
use crate::array_rotator::rotator_unit;
use crate::proof::aliased_flows::is_machinery;

pub(super) enum SiteKey {
    Name(String),
    Node(Node),
    Unknown,
}

pub(super) struct MemberSite {
    pub(super) span: Span,
    pub(super) base: Node,
    pub(super) key: SiteKey,
}

pub(super) struct Frame {
    pub(super) this: Node,
    pub(super) ret: Node,
    pub(super) arguments: Option<(ObjectId, Node)>,
    pub(super) function: Option<ObjectId>,
    pub(super) arrow: bool,
    pub(super) strict: bool,
    pub(super) home: Option<Node>,
    pub(super) heritage: Option<Node>,
    pub(super) supers: Option<Node>,
}

pub(super) struct Gen<'s> {
    pub(super) g: Graph,
    pub(super) scoping: &'s Scoping,
    pub(super) decoded: &'s FxHashMap<Span, String>,
    skip: &'s [Span],
    bindings: FxHashMap<SymbolId, Node>,
    global_reads: FxHashMap<String, Node>,
    pub(super) frames: Vec<Frame>,
    pub(super) global: Node,
    global_object: ObjectId,
    script: bool,
    pub(super) members: Vec<MemberSite>,
    pub(super) fallback: Option<&'static str>,
    pub(super) dump: Option<Vec<(Span, Node)>>,
}

fn has_use_strict(directives: &[Directive<'_>]) -> bool {
    directives
        .iter()
        .any(|directive| directive.directive.as_str() == "use strict")
}

#[derive(Default)]
struct Declared {
    found: Vec<SymbolId>,
}

impl<'a> VisitJs<'a> for Declared {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found.extend(it.symbol_id.get());
    }

    fn visit_expression(&mut self, _it: &Expression<'a>) {}

    fn visit_function_body(&mut self, _it: &FunctionBody<'a>) {}

    fn visit_formal_parameters(&mut self, _it: &FormalParameters<'a>) {}

    fn visit_class_body(&mut self, _it: &ClassBody<'a>) {}
}

impl<'s> Gen<'s> {
    pub(super) fn new(
        program: &Program<'_>,
        scoping: &'s Scoping,
        decoded: &'s FxHashMap<Span, String>,
        skip: &'s [Span],
    ) -> Self {
        let mut g = Graph::with_caps(program.source_text.len());
        let global_object = g.builtin(Builtin::Global);
        let global = g.singleton(global_object);
        Gen {
            g,
            scoping,
            decoded,
            skip,
            bindings: FxHashMap::default(),
            global_reads: FxHashMap::default(),
            frames: Vec::new(),
            global,
            global_object,
            script: !untwist_core::semantic::has_module_syntax(program),
            members: Vec::new(),
            fallback: None,
            dump: None,
        }
    }

    pub(super) fn run(&mut self, program: &Program<'_>) {
        let strict = !self.script || has_use_strict(&program.directives);
        let this = if self.script {
            self.global
        } else {
            self.g.node()
        };
        let ret = self.g.node();
        self.frames.push(Frame {
            this,
            ret,
            arguments: None,
            function: None,
            arrow: false,
            strict,
            home: None,
            heritage: None,
            supers: None,
        });
        for stmt in &program.body {
            self.stmt(stmt);
        }
        self.frames.pop();
    }

    pub(super) fn frame(&self) -> &Frame {
        self.frames.last().expect("a frame is always open")
    }

    pub(super) fn strict(&self) -> bool {
        self.frame().strict
    }

    pub(super) fn world(&self) -> Node {
        self.g.world
    }

    pub(super) fn prim(&mut self, builtin: Builtin) -> Node {
        self.g.builtin_node(builtin)
    }

    pub(super) fn string(&mut self, text: &str) -> Node {
        let object = self.g.objects.string(text);
        self.g.singleton(object)
    }

    pub(super) fn union(&mut self, left: Option<Node>, right: Option<Node>) -> Option<Node> {
        match (left, right) {
            (Some(left), Some(right)) if left != right => {
                let node = self.g.node();
                self.g.copy(left, node);
                self.g.copy(right, node);
                Some(node)
            }
            (Some(left), _) => Some(left),
            (None, right) => right,
        }
    }

    pub(super) fn flow(&mut self, from: Option<Node>, to: Node) {
        if let Some(from) = from {
            self.g.copy(from, to);
        }
    }

    pub(super) fn escape(&mut self, value: Option<Node>) {
        if let Some(value) = value {
            self.g.escape(value);
        }
    }

    pub(super) fn symbol_of(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    pub(super) fn binding(&mut self, symbol: SymbolId) -> Node {
        if let Some(node) = self.bindings.get(&symbol) {
            return *node;
        }
        let node = self.g.node();
        self.bindings.insert(symbol, node);
        let root = self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id();
        let flags = self.scoping.symbol_flags(symbol);
        let var_like =
            flags.intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function);
        if self.script && root && var_like {
            let name = self.scoping.symbol_name(symbol).to_string();
            let field = self.g.name(&name);
            self.g.global_declared.insert(field);
            let slot = self.g.field_node(self.global_object, field);
            self.g.copy(node, slot);
            self.g.copy(slot, node);
            self.g.add(slot, ObjectId::WORLD);
            self.g.escape(node);
        }
        node
    }

    pub(super) fn global_read(&mut self, name: &str) -> Node {
        if let Some(node) = self.global_reads.get(name) {
            return *node;
        }
        let node = self.g.node();
        let global = self.global;
        self.g.load_named(global, name, node);
        self.global_reads.insert(name.to_string(), node);
        node
    }

    pub(super) fn arguments_node(&mut self) -> Option<Node> {
        let index = self.frames.iter().rposition(|frame| !frame.arrow)?;
        let function = self.frames[index].function?;
        if let Some((object, _)) = self.frames[index].arguments {
            return Some(self.g.singleton(object));
        }
        let span = self.objects_span(function);
        let object = self
            .g
            .new_site(span, SiteKind::Arguments, &[Builtin::ObjectProto]);
        let fields = self.g.field_node(object, Field::INDEX);
        let node = self.g.singleton(object);
        let callee = self.g.singleton(function);
        self.g.store_named(node, "callee", callee);
        let number = self.prim(Builtin::PrimNumber);
        self.g.store_named(node, "length", number);
        self.frames[index].arguments = Some((object, fields));
        Some(node)
    }

    fn objects_span(&self, object: ObjectId) -> Span {
        match self.g.objects.get(object) {
            super::object::Obj::Site(span, _) => *span,
            _ => Span::default(),
        }
    }

    fn mark_machinery(&mut self, stmt: &Statement<'_>) {
        let mut declared = Declared::default();
        match stmt {
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    declared.found.extend(id.symbol_id.get());
                }
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    declared.visit_binding_pattern(&declarator.id);
                }
            }
            _ => {}
        }
        let machinery = self.prim(Builtin::Machinery);
        for symbol in declared.found {
            let node = self.binding(symbol);
            self.g.copy(machinery, node);
        }
    }

    pub(super) fn stmt(&mut self, stmt: &Statement<'_>) {
        if is_machinery(stmt) || rotator_unit(stmt).is_some() || self.skip.contains(&stmt.span()) {
            self.mark_machinery(stmt);
            return;
        }
        match stmt {
            Statement::ExpressionStatement(it) => {
                self.expr(&it.expression);
            }
            Statement::VariableDeclaration(it) => self.declaration(it),
            Statement::FunctionDeclaration(it) => {
                let node = self.function(it, false);
                self.bind_id(it.id.as_ref(), node);
                self.hoist_block_function(it.id.as_ref(), node);
            }
            Statement::ClassDeclaration(it) => {
                self.class(it);
            }
            Statement::ReturnStatement(it) => {
                let value = it.argument.as_ref().and_then(|value| self.expr(value));
                let ret = self.frame().ret;
                self.flow(value, ret);
            }
            Statement::ThrowStatement(it) => {
                let value = self.expr(&it.argument);
                self.escape(value);
            }
            Statement::BlockStatement(it) => self.stmts(&it.body),
            Statement::IfStatement(it) => {
                self.expr(&it.test);
                self.stmt(&it.consequent);
                if let Some(alternate) = &it.alternate {
                    self.stmt(alternate);
                }
            }
            Statement::WhileStatement(it) => {
                self.expr(&it.test);
                self.stmt(&it.body);
            }
            Statement::DoWhileStatement(it) => {
                self.stmt(&it.body);
                self.expr(&it.test);
            }
            Statement::ForStatement(it) => {
                match &it.init {
                    Some(ForStatementInit::VariableDeclaration(declaration)) => {
                        self.declaration(declaration)
                    }
                    Some(init) => {
                        if let Some(expr) = init.as_expression() {
                            self.expr(expr);
                        }
                    }
                    None => {}
                }
                if let Some(test) = &it.test {
                    self.expr(test);
                }
                if let Some(update) = &it.update {
                    self.expr(update);
                }
                self.stmt(&it.body);
            }
            Statement::ForInStatement(it) => {
                let source = self.expr(&it.right);
                let keys = self.g.node();
                if let Some(source) = source {
                    self.g.keys_of(source, keys);
                }
                self.for_left(&it.left, Some(keys));
                self.stmt(&it.body);
            }
            Statement::ForOfStatement(it) => {
                let source = self.expr(&it.right);
                let mut items = source.map(|source| self.g.iterate(source));
                if it.r#await {
                    let world = self.world();
                    items = self.union(items, Some(world));
                    self.escape(source);
                }
                self.for_left(&it.left, items);
                self.stmt(&it.body);
            }
            Statement::SwitchStatement(it) => {
                self.expr(&it.discriminant);
                for case in &it.cases {
                    if let Some(test) = &case.test {
                        self.expr(test);
                    }
                    self.stmts(&case.consequent);
                }
            }
            Statement::LabeledStatement(it) => self.stmt(&it.body),
            Statement::TryStatement(it) => {
                self.stmts(&it.block.body);
                if let Some(handler) = &it.handler {
                    if let Some(param) = &handler.param {
                        let world = self.world();
                        self.bind_pattern(&param.pattern, Some(world));
                    }
                    self.stmts(&handler.body.body);
                }
                if let Some(finalizer) = &it.finalizer {
                    self.stmts(&finalizer.body);
                }
            }
            Statement::WithStatement(it) => {
                self.fallback.get_or_insert("a with statement");
                let object = self.expr(&it.object);
                self.escape(object);
                self.stmt(&it.body);
            }
            Statement::ImportDeclaration(it) => {
                let mut declared = Declared::default();
                declared.visit_import_declaration(it);
                let world = self.world();
                for symbol in declared.found {
                    let node = self.binding(symbol);
                    self.g.copy(world, node);
                }
            }
            Statement::ExportNamedDeclaration(it) => self.export_named(it),
            Statement::ExportDeclaration(it) => self.export_declaration(it),
            Statement::ExportDefaultDeclaration(it) => self.export_default(it),
            Statement::BreakStatement(_)
            | Statement::ContinueStatement(_)
            | Statement::DebuggerStatement(_)
            | Statement::EmptyStatement(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportFromDeclaration(_) => {}
            other => walk_js::walk_statement(self, other),
        }
    }

    pub(super) fn stmts(&mut self, stmts: &[Statement<'_>]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn declaration(&mut self, declaration: &VariableDeclaration<'_>) {
        for declarator in &declaration.declarations {
            let value = declarator.init.as_ref().and_then(|init| self.expr(init));
            if declarator.init.is_some()
                || !matches!(declarator.id, BindingPattern::BindingIdentifier(_))
            {
                self.bind_pattern(&declarator.id, value);
            } else {
                let undefined = self.prim(Builtin::PrimUndefined);
                self.bind_pattern(&declarator.id, Some(undefined));
            }
        }
    }

    fn for_left(&mut self, left: &ForStatementLeft<'_>, value: Option<Node>) {
        match left {
            ForStatementLeft::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    self.bind_pattern(&declarator.id, value);
                }
            }
            other => {
                if let Some(target) = other.as_assignment_target() {
                    self.assign_target(target, value);
                }
            }
        }
    }

    fn export_declaration(&mut self, it: &ExportDeclaration<'_>) {
        let mut declared = Declared::default();
        declared.visit_declaration(&it.declaration);
        match &it.declaration {
            Declaration::VariableDeclaration(declaration) => self.declaration(declaration),
            Declaration::FunctionDeclaration(function) => {
                let node = self.function(function, false);
                self.bind_id(function.id.as_ref(), node);
            }
            Declaration::ClassDeclaration(class) => {
                self.class(class);
            }
            _ => {}
        }
        for symbol in declared.found {
            let node = self.binding(symbol);
            self.g.escape(node);
        }
    }

    fn export_named(&mut self, it: &ExportNamedDeclaration<'_>) {
        for specifier in &it.specifiers {
            if let ModuleExportName::IdentifierReference(id) = &specifier.local {
                if let Some(symbol) = self.symbol_of(id) {
                    let node = self.binding(symbol);
                    self.g.escape(node);
                }
            }
        }
    }

    fn export_default(&mut self, it: &ExportDefaultDeclaration<'_>) {
        let value = match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                let node = self.function(function, false);
                self.bind_id(function.id.as_ref(), node);
                Some(node)
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => Some(self.class(class)),
            other => other.as_expression().and_then(|expr| self.expr(expr)),
        };
        self.escape(value);
    }

    fn hoist_block_function(&mut self, id: Option<&BindingIdentifier<'_>>, value: Node) {
        let Some(id) = id else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        if self.frame().strict {
            return;
        }
        let mut scope = self.scoping.symbol_scope_id(symbol);
        if self.scoping.scope_flags(scope).is_var() {
            return;
        }
        while let Some(parent) = self.scoping.scope_parent_id(scope) {
            scope = parent;
            if self.scoping.scope_flags(scope).is_var() {
                break;
            }
        }
        match self.scoping.find_binding(scope, id.name) {
            Some(outer) if outer != symbol => {
                let node = self.binding(outer);
                self.g.copy(value, node);
            }
            _ => {
                let global = self.global;
                self.g.store_named(global, &id.name, value);
            }
        }
    }

    pub(super) fn bind_id(&mut self, id: Option<&BindingIdentifier<'_>>, value: Node) {
        if let Some(symbol) = id.and_then(|id| id.symbol_id.get()) {
            let node = self.binding(symbol);
            self.g.copy(value, node);
        }
    }

    pub(super) fn function_object(&mut self, span: Span, prototype: bool) -> (ObjectId, Node) {
        let object = self
            .g
            .new_site(span, SiteKind::Function, &[Builtin::FunctionProto]);
        let node = self.g.singleton(object);
        if prototype {
            let proto = self
                .g
                .new_site(span, SiteKind::FunctionPrototype, &[Builtin::ObjectProto]);
            let proto_node = self.g.singleton(proto);
            self.g.store_named(node, "prototype", proto_node);
            self.g.store_named(proto_node, "constructor", node);
        }
        (object, node)
    }

    pub(super) fn function(&mut self, it: &Function<'_>, method: bool) -> Node {
        let home = if method { self.frame().home } else { None };
        self.function_with(it, method, home)
    }

    pub(super) fn function_with(
        &mut self,
        it: &Function<'_>,
        method: bool,
        home: Option<Node>,
    ) -> Node {
        let prototype = !method && !it.r#async;
        let (object, node) = self.function_object(it.span, prototype);
        let strict = self.strict()
            || it
                .body
                .as_ref()
                .is_some_and(|body| has_use_strict(&body.directives));
        let this = self.g.node();
        let ret = self.g.node();
        if it.r#async || it.generator {
            let world = self.world();
            self.g.copy(world, ret);
            self.g.escape(ret);
        }
        let heritage = self.frame().heritage;
        self.frames.push(Frame {
            this,
            ret,
            arguments: None,
            function: Some(object),
            arrow: false,
            strict,
            home,
            heritage,
            supers: None,
        });
        let (params, rest) = self.params(&it.params);
        if let Some(body) = &it.body {
            self.stmts(&body.statements);
        }
        if !strict {
            self.arguments_node();
        }
        let frame = self.frames.pop().expect("pushed above");
        self.finish_function(object, (params, rest), frame);
        if it.r#type == FunctionType::FunctionExpression {
            self.bind_id(it.id.as_ref(), node);
        }
        node
    }

    fn finish_function(
        &mut self,
        object: ObjectId,
        (params, rest): (Vec<Node>, Option<Node>),
        frame: Frame,
    ) {
        let arguments = frame.arguments.map(|(arguments, fields)| {
            if !frame.strict {
                let node = self.g.singleton(arguments);
                for param in &params {
                    self.g.load(node, Key::Static(Field::ELEMENTS), *param);
                    self.g.copy(*param, fields);
                }
                self.g.add(fields, ObjectId::WORLD);
                self.g.escape(fields);
            }
            fields
        });
        for node in params.iter().chain([&frame.ret]) {
            self.g.add_builtin(*node, Builtin::PrimUndefined);
        }
        let info = FnInfo {
            params,
            rest,
            this: (!frame.arrow).then_some(frame.this),
            ret: frame.ret,
            arguments,
            sloppy: !frame.strict && !frame.arrow,
            supers: frame.supers.filter(|_| !frame.arrow),
        };
        self.g.functions.insert(object, info);
    }

    pub(super) fn arrow(&mut self, it: &ArrowFunctionExpression<'_>) -> Node {
        let (object, node) = self.function_object(it.span, false);
        let ret = self.g.node();
        if it.r#async {
            let world = self.world();
            self.g.copy(world, ret);
            self.g.escape(ret);
        }
        let parent = self.frame();
        let frame = Frame {
            this: parent.this,
            ret,
            arguments: None,
            function: Some(object),
            arrow: true,
            strict: parent.strict,
            home: parent.home,
            heritage: parent.heritage,
            supers: parent.supers,
        };
        self.frames.push(frame);
        let (params, rest) = self.params(&it.params);
        match &it.body {
            ArrowFunctionBody::FunctionBody(body) => self.stmts(&body.statements),
            other => {
                if let Some(body) = other.as_expression() {
                    let value = self.expr(body);
                    self.flow(value, ret);
                }
            }
        }
        let frame = self.frames.pop().expect("pushed above");
        self.finish_function(object, (params, rest), frame);
        node
    }

    fn params(&mut self, params: &FormalParameters<'_>) -> (Vec<Node>, Option<Node>) {
        let mut nodes = Vec::new();
        for param in &params.items {
            let node = match &param.pattern {
                BindingPattern::BindingIdentifier(id) => match id.symbol_id.get() {
                    Some(symbol) => self.binding(symbol),
                    None => self.g.node(),
                },
                _ => self.g.node(),
            };
            if let Some(init) = &param.initializer {
                let value = self.expr(init);
                self.flow(value, node);
            }
            if !matches!(param.pattern, BindingPattern::BindingIdentifier(_)) {
                self.bind_pattern(&param.pattern, Some(node));
            }
            nodes.push(node);
        }
        let rest = params.rest.as_ref().map(|rest| {
            let object = self
                .g
                .new_site(rest.span, SiteKind::Rest, &[Builtin::ArrayProto]);
            let fields = self.g.field_node(object, Field::INDEX);
            let node = self.g.singleton(object);
            self.bind_pattern(&rest.rest.argument, Some(node));
            fields
        });
        (nodes, rest)
    }

    pub(super) fn class(&mut self, it: &Class<'_>) -> Node {
        let (class, node) = self.function_object(it.span, false);
        let proto = self.g.new_site(
            it.span,
            SiteKind::FunctionPrototype,
            &[Builtin::ObjectProto],
        );
        let proto_node = self.g.singleton(proto);
        self.g.store_named(node, "prototype", proto_node);
        self.g.store_named(proto_node, "constructor", node);
        if let Some(id) = &it.id {
            self.bind_id(Some(id), node);
        }
        let heritage = it
            .heritage
            .as_ref()
            .and_then(|heritage| self.expr(&heritage.expression));
        let class_proto = self.g.proto_node(class);
        let instance_proto = self.g.proto_node(proto);
        if let Some(heritage) = heritage {
            self.g.copy(heritage, class_proto);
            let parent = self.g.node();
            self.g.load_named(heritage, "prototype", parent);
            self.g.copy(parent, instance_proto);
        }
        let this = self.g.node();
        let ret = self.g.node();
        let supers = heritage.map(|_| self.g.node());
        if let Some(supers) = supers {
            self.g.copy(supers, this);
        }
        let constructor = it.body.body.iter().find_map(|element| match element {
            ClassElement::MethodDefinition(method)
                if method.kind == MethodDefinitionKind::Constructor =>
            {
                Some(method)
            }
            _ => None,
        });
        self.frames.push(Frame {
            this,
            ret,
            arguments: None,
            function: Some(class),
            arrow: false,
            strict: true,
            home: Some(instance_proto),
            heritage,
            supers,
        });
        let (params, rest) = match constructor {
            Some(method) => {
                let params = self.params(&method.value.params);
                if let Some(body) = &method.value.body {
                    self.stmts(&body.statements);
                }
                params
            }
            None => self.default_constructor(heritage, this, supers),
        };
        let frame = self.frames.pop().expect("pushed above");
        self.finish_function(class, (params, rest), frame);
        for element in &it.body.body {
            self.class_element(
                element,
                (node, proto_node, this),
                (class_proto, instance_proto),
                heritage,
            );
        }
        node
    }

    fn default_constructor(
        &mut self,
        heritage: Option<Node>,
        this: Node,
        supers: Option<Node>,
    ) -> (Vec<Node>, Option<Node>) {
        let Some(heritage) = heritage else {
            return (Vec::new(), None);
        };
        let rest = self.g.node();
        self.g.call(Call {
            callee: heritage,
            this: Some(this),
            args: Vec::new(),
            spread: Some(rest),
            result: supers,
            construct: false,
            loose: false,
            firm: false,
            span: Span::default(),
        });
        (Vec::new(), Some(rest))
    }

    fn class_element(
        &mut self,
        element: &ClassElement<'_>,
        (class, proto, this): (Node, Node, Node),
        (class_proto, instance_proto): (Node, Node),
        heritage: Option<Node>,
    ) {
        match element {
            ClassElement::MethodDefinition(method) => {
                if method.kind == MethodDefinitionKind::Constructor {
                    return;
                }
                let (target, home) = if method.r#static {
                    (class, class_proto)
                } else {
                    (proto, instance_proto)
                };
                let key = self.property_key(&method.key, method.computed);
                let strict_frame = self.class_frame(target, home, heritage);
                self.frames.push(strict_frame);
                let function = self.function_with(&method.value, true, Some(home));
                self.frames.pop();
                let kind = match method.kind {
                    MethodDefinitionKind::Get => PropertyKind::Get,
                    MethodDefinitionKind::Set => PropertyKind::Set,
                    _ => PropertyKind::Init,
                };
                self.define_member(target, key, function, kind);
            }
            ClassElement::PropertyDefinition(property) => {
                let (target, home) = if property.r#static {
                    (class, class_proto)
                } else {
                    (this, instance_proto)
                };
                let key = self.property_key(&property.key, property.computed);
                let frame = self.class_frame(target, home, heritage);
                self.frames.push(frame);
                let value = property.value.as_ref().and_then(|value| self.expr(value));
                self.frames.pop();
                if let Some(value) = value {
                    self.g.store(target, key, value);
                }
            }
            ClassElement::AccessorProperty(property) => {
                let target = if property.r#static { class } else { this };
                let key = self.property_key(&property.key, property.computed);
                let frame = self.class_frame(target, instance_proto, heritage);
                self.frames.push(frame);
                let value = property.value.as_ref().and_then(|value| self.expr(value));
                self.frames.pop();
                if let Some(value) = value {
                    self.g.store(target, key, value);
                }
            }
            ClassElement::StaticBlock(block) => {
                let frame = self.class_frame(class, class_proto, heritage);
                self.frames.push(frame);
                self.stmts(&block.body);
                self.frames.pop();
            }
            ClassElement::TSIndexSignature(_) => {}
        }
    }

    fn class_frame(&mut self, this: Node, home: Node, heritage: Option<Node>) -> Frame {
        let ret = self.g.node();
        Frame {
            this,
            ret,
            arguments: None,
            function: None,
            arrow: true,
            strict: true,
            home: Some(home),
            heritage,
            supers: None,
        }
    }

    pub(super) fn define_member(
        &mut self,
        target: Node,
        key: Key,
        value: Node,
        kind: PropertyKind,
    ) {
        match kind {
            PropertyKind::Init => self.g.store(target, key, value),
            PropertyKind::Get => {
                let result = self.g.node();
                self.g.getter_call(target, value, result, Span::default());
                self.g.store(target, key, result);
                self.g.store(target, Key::Static(Field::ACCESSORS), value);
            }
            PropertyKind::Set => {
                self.g.setter_call(target, key, value, Span::default());
                self.g.store(target, Key::Static(Field::ACCESSORS), value);
            }
        }
    }
}

impl<'a> VisitJs<'a> for Gen<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        self.stmt(it);
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        let value = self.expr(it);
        self.escape(value);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let node = self.function(it, false);
        self.bind_id(it.id.as_ref(), node);
        self.g.escape(node);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let node = self.class(it);
        self.g.escape(node);
    }
}
