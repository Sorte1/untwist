use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, CatchClause, Class, Function, Program, StaticBlock,
    VariableDeclaration, VariableDeclarationKind,
};
use std::cell::Cell;

use oxc_ast_visit::{walk_js, VisitJs};
use oxc_diagnostics::OxcDiagnostic;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

struct VarName {
    name: String,
    span: Span,
    symbol: Option<SymbolId>,
    static_block: Option<ScopeId>,
}

struct Declared {
    name: String,
    scope: ScopeId,
    span: Span,
}

pub struct Hoisting {
    var_level_functions: Vec<Span>,
    var_declarators: Vec<Span>,
    static_vars: Vec<Declared>,
    static_top_lexical: Vec<Declared>,
    static_nested_lexical: Vec<(Declared, ScopeId)>,
    catch_param_clashes: Vec<Span>,
    missed: Vec<OxcDiagnostic>,
    v8_reads: Vec<OxcDiagnostic>,
}

struct Walk<'s> {
    scoping: &'s Scoping,
    module: bool,
    var_level_functions: Vec<Span>,
    lexicals: Vec<Declared>,
    var_declarators: Vec<Span>,
    var_names: Vec<VarName>,
    simple_catch_names: Vec<(Span, ScopeId)>,
    frames: Vec<Option<ScopeId>>,
    static_blocks: Vec<ScopeId>,
    static_vars: Vec<Declared>,
    static_top_lexical: Vec<Declared>,
    static_nested_lexical: Vec<(Declared, ScopeId)>,
    catch_bodies: Vec<(ScopeId, Vec<String>)>,
    catch_param_clashes: Vec<Span>,
    scopes: Vec<Option<ScopeId>>,
}

impl Walk<'_> {
    fn simple_catch(&self, symbol: SymbolId) -> Option<ScopeId> {
        let span = self.scoping.symbol_span(symbol);
        self.simple_catch_names
            .iter()
            .find(|(name, _)| *name == span)
            .map(|(_, clause)| *clause)
    }

    fn var_level(&self, scope: ScopeId) -> bool {
        let flags = self.scoping.scope_flags(scope);
        flags.is_var() && !(self.module && flags.is_top())
    }

    fn static_block(&self) -> Option<ScopeId> {
        self.frames.last().copied().flatten()
    }

    fn blocked_in(&self, scope: ScopeId, name: &str) -> bool {
        self.lexicals.iter().any(|lexical| {
            lexical.scope == scope
                && lexical.name == name
                && !self
                    .simple_catch_names
                    .iter()
                    .any(|(span, _)| *span == lexical.span)
        })
    }

    fn hoisted_past(&self, var: &VarName) -> Option<ScopeId> {
        let symbol = var.symbol?;
        if !self
            .scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::CatchVariable)
        {
            return None;
        }
        let clause = self.simple_catch(symbol)?;
        for scope in self.scoping.scope_ancestors(clause).skip(1) {
            if self.blocked_in(scope, &var.name) {
                return Some(scope);
            }
            if self.scoping.scope_flags(scope).is_var() {
                return None;
            }
        }
        None
    }

    fn lexical(&mut self, name: &str, span: Span, scope: Option<ScopeId>) {
        let Some(scope) = scope else {
            return;
        };
        let declared = Declared {
            name: name.to_string(),
            scope,
            span,
        };
        self.lexicals.push(Declared {
            name: name.to_string(),
            scope,
            span,
        });
        match self.static_block() {
            Some(block) if block == scope => self.static_top_lexical.push(declared),
            Some(block) => self.static_nested_lexical.push((declared, block)),
            None => {}
        }
        let clashes = self
            .catch_bodies
            .iter()
            .any(|(body, names)| *body == scope && names.iter().any(|caught| caught == name));
        if clashes {
            self.catch_param_clashes.push(span);
        }
    }

    fn current_scope(&self) -> Option<ScopeId> {
        self.scopes.last().copied().flatten()
    }
}

fn names(pattern: &BindingPattern<'_>) -> Vec<VarName> {
    pattern
        .get_binding_identifiers()
        .into_iter()
        .map(|ident| VarName {
            name: ident.name.to_string(),
            span: ident.span,
            symbol: ident.symbol_id.get(),
            static_block: None,
        })
        .collect()
}

impl<'a> VisitJs<'a> for Walk<'_> {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(scope_id.get());
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let parent = it
            .scope_id
            .get()
            .and_then(|own| self.scoping.scope_parent_id(own));
        if let (true, Some(id), Some(parent)) = (it.is_declaration(), &it.id, parent) {
            if self.var_level(parent) {
                self.var_level_functions.push(id.span);
            } else {
                self.lexical(id.name.as_str(), id.span, Some(parent));
            }
        }
        self.frames.push(None);
        walk_js::walk_function(self, it, flags);
        self.frames.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.frames.push(None);
        walk_js::walk_arrow_function_expression(self, it);
        self.frames.pop();
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        let scope = it.scope_id.get();
        self.static_blocks.extend(scope);
        self.frames.push(scope);
        walk_js::walk_static_block(self, it);
        self.frames.pop();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let (true, Some(id)) = (it.is_declaration(), &it.id) {
            self.lexical(id.name.as_str(), id.span, self.current_scope());
        }
        walk_js::walk_class(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        let static_block = self.static_block();
        for declarator in &it.declarations {
            let mut declared = names(&declarator.id);
            if it.kind != VariableDeclarationKind::Var {
                let scope = self.current_scope();
                for name in &declared {
                    self.lexical(&name.name, name.span, scope);
                }
                continue;
            }
            self.var_declarators.push(declarator.span);
            if let Some(scope) = static_block {
                for (span, name) in declared
                    .iter()
                    .map(|name| (name.span, &name.name))
                    .chain(declared.first().map(|name| (declarator.span, &name.name)))
                {
                    self.static_vars.push(Declared {
                        name: name.clone(),
                        scope,
                        span,
                    });
                }
            }
            for name in &mut declared {
                name.static_block = static_block;
            }
            self.var_names.extend(declared);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        let simple = it.param.as_ref().map(|param| &param.pattern);
        if let (Some(BindingPattern::BindingIdentifier(ident)), Some(scope)) =
            (simple, it.scope_id.get())
        {
            self.simple_catch_names.push((ident.span, scope));
        }
        let caught: Vec<String> = it
            .param
            .iter()
            .flat_map(|param| param.pattern.get_binding_identifiers())
            .map(|ident| ident.name.to_string())
            .collect();
        if let Some(clause) = it.scope_id.get() {
            for ident in it
                .param
                .iter()
                .flat_map(|param| param.pattern.get_binding_identifiers())
            {
                self.lexical(ident.name.as_str(), ident.span, Some(clause));
            }
        }
        if let Some(body) = it.body.scope_id.get() {
            self.catch_bodies.push((body, caught));
        }
        walk_js::walk_catch_clause(self, it);
    }
}

fn top_lexical(declared: &[Declared], name: &str, scope: ScopeId) -> bool {
    declared
        .iter()
        .any(|lexical| lexical.scope == scope && lexical.name == name)
}

fn labelled(diagnostic: &OxcDiagnostic) -> Option<Vec<Span>> {
    let redeclared = diagnostic.message.starts_with("Identifier `")
        && diagnostic.message.ends_with("has already been declared");
    redeclared.then(|| {
        diagnostic
            .labels
            .iter()
            .map(|label| Span::sized(label.offset(), label.len()))
            .collect()
    })
}

impl Hoisting {
    pub fn scan(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut walk = Walk {
            scoping,
            module: program.source_type.is_module(),
            var_level_functions: Vec::new(),
            lexicals: Vec::new(),
            var_declarators: Vec::new(),
            var_names: Vec::new(),
            simple_catch_names: Vec::new(),
            frames: Vec::new(),
            static_blocks: Vec::new(),
            static_vars: Vec::new(),
            static_top_lexical: Vec::new(),
            static_nested_lexical: Vec::new(),
            catch_bodies: Vec::new(),
            catch_param_clashes: Vec::new(),
            scopes: Vec::new(),
        };
        walk.visit_program(program);
        let mut missed = Vec::new();
        let mut v8_reads = Vec::new();
        for var in &walk.var_names {
            let Some(blocked) = walk.hoisted_past(var) else {
                continue;
            };
            let diagnostic = OxcDiagnostic::error(format!(
                "Identifier `{}` has already been declared",
                var.name
            ))
            .with_label(var.span);
            let unseen = var.static_block.is_some_and(|block| {
                block != blocked && !top_lexical(&walk.static_top_lexical, &var.name, block)
            });
            match unseen {
                true => v8_reads.push(diagnostic),
                false => missed.push(diagnostic),
            }
        }
        Hoisting {
            var_level_functions: walk.var_level_functions,
            var_declarators: walk.var_declarators,
            static_vars: walk.static_vars,
            static_top_lexical: walk.static_top_lexical,
            static_nested_lexical: walk.static_nested_lexical,
            catch_param_clashes: walk.catch_param_clashes,
            missed,
            v8_reads,
        }
    }

    pub fn take_found(&mut self) -> (Vec<OxcDiagnostic>, Vec<OxcDiagnostic>) {
        (
            std::mem::take(&mut self.missed),
            std::mem::take(&mut self.v8_reads),
        )
    }

    pub fn read_by_v8(&self, diagnostic: &OxcDiagnostic) -> bool {
        let Some(spans) = labelled(diagnostic) else {
            return false;
        };
        if spans
            .iter()
            .any(|span| self.catch_param_clashes.contains(span))
        {
            return false;
        }
        let var = self
            .static_vars
            .iter()
            .find(|var| spans.contains(&var.span));
        if let Some(var) = var {
            return !top_lexical(&self.static_top_lexical, &var.name, var.scope);
        }
        self.nested_pair(&spans)
    }

    fn nested_pair(&self, spans: &[Span]) -> bool {
        let found: Vec<&(Declared, ScopeId)> = spans
            .iter()
            .filter_map(|span| {
                self.static_nested_lexical
                    .iter()
                    .find(|(lexical, _)| lexical.span == *span)
            })
            .collect();
        let [(first, block), (second, other)] = found.as_slice() else {
            return false;
        };
        let name = first.name.as_str();
        block == other
            && first.scope != second.scope
            && second.name == name
            && !top_lexical(&self.static_top_lexical, name, *block)
            && self
                .static_vars
                .iter()
                .any(|var| var.scope == *block && var.name == name)
    }

    pub fn explains(&self, diagnostic: &OxcDiagnostic) -> bool {
        let Some(spans) = labelled(diagnostic) else {
            return false;
        };
        match spans.as_slice() {
            [first, second] => {
                self.var_level_functions.contains(first) && self.var_declarators.contains(second)
            }
            _ => false,
        }
    }
}
