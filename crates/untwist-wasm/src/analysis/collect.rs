use std::cell::Cell;

use oxc_ast::ast::{
    Argument, AssignmentExpression, AssignmentTarget, AssignmentTargetPropertyIdentifier,
    BindingIdentifier, BindingPattern, BindingProperty, Class, ComputedMemberExpression,
    Declaration, ExportDeclaration, ExportDefaultDeclaration, ExportDefaultDeclarationKind,
    ExportNamedDeclaration, Expression, FormalParameters, Function, IdentifierReference,
    ImportDeclaration, ImportDeclarationSpecifier, ImportSpecifier, JSXElementName,
    ModuleExportName, ObjectProperty, StaticMemberExpression, VariableDeclarator,
};
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::{ScopeFlags, ScopeId, Scoping, SymbolFlags, SymbolId};
use oxc_span::{GetSpan, Span};

use rustc_hash::FxHashMap;

use super::index::{Detail, Form, Kind, Link, Member, Names, Occurrence, Shape, Symbol, NONE};
use super::insight::{
    class_init, constant_detail, constructor_params, function_init, object_init, object_keys,
    params_of,
};
use super::offsets::Utf16Map;

pub struct Collector<'s> {
    pub scoping: &'s Scoping,
    pub map: &'s Utf16Map,
    pub text: &'s str,
    pub details: FxHashMap<u32, Detail>,
    pub properties: Vec<(u32, u32, u32)>,
    pub names: Names,
    pub occurrences: Vec<Occurrence>,
    pub symbols: Vec<Symbol>,
    pub scope_spans: Vec<[u32; 2]>,
    pub members: Vec<Member>,
    pub exports: Vec<(Box<str>, [u32; 2])>,
    node_spans: Vec<Span>,
    scope_stack: Vec<u32>,
    pending: Option<(u32, Shape)>,
    skip_binding: Option<u32>,
}

fn index_of(id: usize) -> u32 {
    u32::try_from(id).unwrap_or(NONE)
}

fn kind_of(flags: SymbolFlags) -> Kind {
    if flags.contains(SymbolFlags::Import) {
        Kind::Import
    } else if flags.contains(SymbolFlags::Class) {
        Kind::Class
    } else if flags.contains(SymbolFlags::Function) {
        Kind::Function
    } else if flags.contains(SymbolFlags::ConstVariable) {
        Kind::Const
    } else if flags.contains(SymbolFlags::BlockScopedVariable) {
        Kind::Let
    } else if flags.contains(SymbolFlags::FunctionScopedVariable) {
        Kind::Var
    } else {
        Kind::Other
    }
}

fn require_source<'b>(init: Option<&'b Expression<'_>>) -> Option<&'b str> {
    let Some(Expression::CallExpression(call)) = init else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    match call.arguments.as_slice() {
        [Argument::StringLiteral(source)] if callee.name == "require" => {
            Some(source.value.as_str())
        }
        _ => None,
    }
}

fn is_exports_object(object: &Expression<'_>) -> bool {
    match object {
        Expression::Identifier(id) => id.name == "exports",
        Expression::StaticMemberExpression(member) => {
            member.property.name == "exports"
                && matches!(&member.object, Expression::Identifier(id) if id.name == "module")
        }
        _ => false,
    }
}

impl<'s> Collector<'s> {
    pub fn new(scoping: &'s Scoping, map: &'s Utf16Map, text: &'s str) -> Self {
        let mut names = Names::default();
        let symbols = scoping
            .symbol_ids()
            .map(|id| Symbol {
                name: names.intern(scoping.symbol_name(id)),
                decl: {
                    let span = scoping.symbol_span(id);
                    map.span(span.start, span.end)
                },
                scope: index_of(scoping.symbol_scope_id(id).index()),
                kind: kind_of(scoping.symbol_flags(id)),
                exported: false,
                link: None,
            })
            .collect();
        Self {
            scoping,
            map,
            text,
            details: FxHashMap::default(),
            properties: Vec::new(),
            names,
            occurrences: Vec::new(),
            symbols,
            scope_spans: vec![[0, 0]; scoping.scopes_len()],
            members: Vec::new(),
            exports: Vec::new(),
            node_spans: Vec::new(),
            scope_stack: Vec::new(),
            pending: None,
            skip_binding: None,
        }
    }

    fn shape_at(&mut self, start: u32) -> Shape {
        match self.pending.take() {
            Some((at, shape)) if at == start => shape,
            other => {
                self.pending = other;
                Shape::Plain
            }
        }
    }

    fn push(&mut self, span: Span, name: &str, symbol: u32, scope: u32) {
        let shape = self.shape_at(span.start);
        let name = self.names.intern(name);
        let [start, end] = self.map.span(span.start, span.end);
        self.occurrences.push(Occurrence {
            start,
            end,
            symbol,
            scope,
            name,
            shape,
        });
    }

    fn current_scope(&self) -> u32 {
        self.scope_stack.last().copied().unwrap_or(0)
    }

    fn record_binding(&mut self, id: &BindingIdentifier<'_>) {
        if let Some(symbol) = id.symbol_id.get() {
            self.push(
                id.span,
                &id.name,
                index_of(symbol.index()),
                self.current_scope(),
            );
        }
    }

    fn symbol_of_reference(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn symbol_mut(&mut self, symbol: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(symbol.index())
    }

    fn link(&mut self, symbol: Option<SymbolId>, module: &str, name: &str) {
        if let Some(entry) = symbol.and_then(|symbol| self.symbol_mut(symbol)) {
            entry.link = Some(Link {
                module: module.into(),
                name: name.into(),
            });
        }
    }

    fn export(&mut self, name: &str, symbol: Option<SymbolId>, fallback: Span) {
        let span = match symbol.and_then(|symbol| self.symbol_mut(symbol)) {
            Some(entry) => {
                entry.exported = true;
                entry.decl
            }
            None => self.map.span(fallback.start, fallback.end),
        };
        self.exports.push((name.into(), span));
    }

    fn export_binding(&mut self, id: &BindingIdentifier<'_>) {
        self.export(&id.name, id.symbol_id.get(), id.span);
    }

    fn export_declaration(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::VariableDeclaration(variables) => {
                for declarator in &variables.declarations {
                    for id in declarator.id.get_binding_identifiers() {
                        self.export_binding(id);
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.export_binding(id);
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.export_binding(id);
                }
            }
            _ => {}
        }
    }

    fn callable(
        &mut self,
        id: Option<&BindingIdentifier<'_>>,
        form: Form,
        params: Box<[Box<str>]>,
    ) {
        if let Some(symbol) = id.and_then(|id| id.symbol_id.get()) {
            self.details.insert(
                index_of(symbol.index()),
                Detail {
                    form,
                    params: Some(params),
                    ..Detail::default()
                },
            );
        }
    }

    fn record_init(&mut self, id: &BindingIdentifier<'_>, init: &Expression<'_>) {
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        if let Some(params) = function_init(init) {
            self.callable(Some(id), Form::Function, params_of(params, self.text));
        } else if let Some(class) = class_init(init) {
            self.callable(Some(id), Form::Class, constructor_params(class, self.text));
        } else if !self.scoping.symbol_is_mutated(symbol) {
            if let Some(detail) = constant_detail(init, self.text, &mut self.names) {
                self.details.insert(index_of(symbol.index()), detail);
            }
        }
        if let Some(object) = object_init(init) {
            for key in object_keys(object) {
                let name = self.names.intern(&key);
                self.properties.push((index_of(symbol.index()), NONE, name));
            }
        }
    }

    fn property_seen(&mut self, object: &Expression<'_>, property: &str) {
        let Expression::Identifier(object) = object else {
            return;
        };
        let key = match self.symbol_of_reference(object) {
            Some(symbol) => (index_of(symbol.index()), NONE),
            None => (NONE, self.names.intern(&object.name)),
        };
        let name = self.names.intern(property);
        self.properties.push((key.0, key.1, name));
    }

    fn mark_params(&mut self, pattern: &BindingPattern<'_>) {
        for id in pattern.get_binding_identifiers() {
            if let Some(symbol) = id
                .symbol_id
                .get()
                .and_then(|symbol| self.symbol_mut(symbol))
            {
                symbol.kind = Kind::Param;
            }
        }
    }

    fn named_binding(&mut self, id: Option<&BindingIdentifier<'_>>) {
        if let Some(id) = id {
            self.record_binding(id);
            self.skip_binding = Some(id.span.start);
        }
    }
}

fn shorthand_binding<'b>(pattern: &'b BindingPattern<'_>) -> Option<&'b BindingIdentifier<'b>> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id),
        BindingPattern::AssignmentPattern(assign) => match &assign.left {
            BindingPattern::BindingIdentifier(id) => Some(id),
            _ => None,
        },
        _ => None,
    }
}

impl<'a> VisitJs<'a> for Collector<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.node_spans.push(kind.span());
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.node_spans.pop();
    }

    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        let id = scope_id.get().map_or(NONE, |id| index_of(id.index()));
        let span = self.node_spans.last().copied().unwrap_or_default();
        let mapped = self.map.span(span.start, span.end);
        if let Some(slot) = self.scope_spans.get_mut(id as usize) {
            *slot = mapped;
        }
        self.scope_stack.push(id);
    }

    fn leave_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        let symbol = reference
            .symbol_id()
            .map_or(NONE, |id| index_of(id.index()));
        let scope = index_of(reference.scope_id().index());
        self.push(it.span, &it.name, symbol, scope);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.skip_binding == Some(it.span.start) {
            self.skip_binding = None;
            return;
        }
        self.record_binding(it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.callable(
            it.id.as_ref(),
            Form::Function,
            params_of(&it.params, self.text),
        );
        self.named_binding(it.id.as_ref());
        walk_js::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.callable(
            it.id.as_ref(),
            Form::Class,
            constructor_params(it, self.text),
        );
        self.named_binding(it.id.as_ref());
        walk_js::walk_class(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if let (true, Expression::Identifier(id)) = (it.shorthand, &it.value) {
            self.pending = Some((id.span.start, Shape::Property));
        }
        walk_js::walk_object_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
        if let Some(id) = shorthand_binding(&it.value).filter(|_| it.shorthand) {
            self.pending = Some((id.span.start, Shape::Property));
        }
        walk_js::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.pending = Some((it.binding.span.start, Shape::Property));
        walk_js::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        for specifier in it.specifiers.iter().flatten() {
            let (local, name) = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(s) => (&s.local, s.imported.name()),
                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                    (&s.local, "default".into())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => (&s.local, "*".into()),
            };
            self.link(
                local.symbol_id.get(),
                it.source.value.as_str(),
                name.as_str(),
            );
        }
        walk_js::walk_import_declaration(self, it);
    }

    fn visit_jsx_element_name(&mut self, it: &JSXElementName<'a>) {
        if let JSXElementName::IdentifierReference(id) = it {
            self.pending = Some((id.span.start, Shape::JsxTag));
        }
        walk_js::walk_jsx_element_name(self, it);
    }

    fn visit_import_specifier(&mut self, it: &ImportSpecifier<'a>) {
        if it.imported.span() == it.local.span {
            self.pending = Some((it.local.span.start, Shape::Import));
        }
        walk_js::walk_import_specifier(self, it);
    }

    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        self.export_declaration(&it.declaration);
        walk_js::walk_export_declaration(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        for specifier in &it.specifiers {
            if let ModuleExportName::IdentifierReference(local) = &specifier.local {
                let symbol = self.symbol_of_reference(local);
                self.export(specifier.exported.name().as_str(), symbol, local.span);
            }
        }
        walk_js::walk_export_named_declaration(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        let span = match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => f.id.as_ref().map(|id| id.span),
            ExportDefaultDeclarationKind::ClassDeclaration(c) => c.id.as_ref().map(|id| id.span),
            _ => None,
        }
        .unwrap_or_else(|| it.declaration.span());
        self.exports
            .push(("default".into(), self.map.span(span.start, span.end)));
        walk_js::walk_export_default_declaration(self, it);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        for item in &it.items {
            self.mark_params(&item.pattern);
        }
        if let Some(rest) = &it.rest {
            self.mark_params(&rest.rest.argument);
        }
        walk_js::walk_formal_parameters(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(property) = &it.expression {
            self.property_seen(&it.object, &property.value);
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(source)) =
            (&it.id, require_source(it.init.as_ref()))
        {
            self.link(id.symbol_id.get(), source, "*");
        }
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            self.record_init(id, init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::StaticMemberExpression(member) = &it.left {
            if is_exports_object(&member.object) {
                let span = member.property.span;
                self.exports.push((
                    member.property.name.as_str().into(),
                    self.map.span(span.start, span.end),
                ));
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.property_seen(&it.object, &it.property.name);
        if let Expression::Identifier(object) = &it.object {
            let symbol = self.symbol_of_reference(object);
            let linked = symbol
                .and_then(|symbol| self.symbols.get(symbol.index()))
                .is_some_and(|entry| entry.link.is_some());
            if let (true, Some(symbol)) = (linked, symbol) {
                let span = it.property.span;
                let [start, end] = self.map.span(span.start, span.end);
                self.members.push(Member {
                    start,
                    end,
                    object: index_of(symbol.index()),
                    name: it.property.name.as_str().into(),
                });
            }
        }
        walk_js::walk_static_member_expression(self, it);
    }
}
