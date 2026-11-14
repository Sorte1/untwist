use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy)]
pub enum Source<'a> {
    Expr(&'a Expression<'a>),
    Undefined,
    String,
    Top,
    Compound(BinaryOperator, SymbolId, &'a Expression<'a>),
    Update(SymbolId),
    Defaulted(Option<&'a Expression<'a>>, &'a Expression<'a>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    Binding { constant: bool, in_loop_head: bool },
    Param { function: usize, position: usize },
}

pub struct Slot<'a> {
    pub kind: SlotKind,
    pub at: Span,
    pub declarator: Option<&'a VariableDeclarator<'a>>,
    pub sources: Vec<Source<'a>>,
}

#[derive(Clone, Copy)]
pub enum FnRef<'a> {
    Function(&'a Function<'a>),
    Arrow(&'a ArrowFunctionExpression<'a>),
}

impl<'a> FnRef<'a> {
    pub fn params(self) -> &'a FormalParameters<'a> {
        match self {
            FnRef::Function(function) => &function.params,
            FnRef::Arrow(arrow) => &arrow.params,
        }
    }

    pub fn is_async(self) -> bool {
        match self {
            FnRef::Function(function) => function.r#async,
            FnRef::Arrow(arrow) => arrow.r#async,
        }
    }

    pub fn span(self) -> Span {
        match self {
            FnRef::Function(function) => function.span,
            FnRef::Arrow(arrow) => arrow.span,
        }
    }

    pub fn statements(self) -> &'a [Statement<'a>] {
        match self {
            FnRef::Function(function) => function
                .body
                .as_ref()
                .map_or(&[][..], |body| body.statements.as_slice()),
            FnRef::Arrow(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => body.statements.as_slice(),
                _ => &[],
            },
        }
    }

    pub fn expression_body(self) -> Option<&'a Expression<'a>> {
        match self {
            FnRef::Arrow(arrow) => arrow.body.as_expression(),
            FnRef::Function(_) => None,
        }
    }
}

pub struct FnFact<'a> {
    pub func: FnRef<'a>,
    pub node: NodeId,
    pub closed: bool,
    pub calls: Vec<&'a CallExpression<'a>>,
    pub params: Vec<Option<SymbolId>>,
    pub params_open: bool,
    pub returns_typed: bool,
    pub returns: Vec<Option<&'a Expression<'a>>>,
    pub falls_through: bool,
    pub symbol: Option<SymbolId>,
}

pub struct Facts<'a> {
    pub slots: FxHashMap<SymbolId, Slot<'a>>,
    pub functions: Vec<FnFact<'a>>,
    pub by_node: FxHashMap<NodeId, usize>,
    pub by_symbol: FxHashMap<SymbolId, usize>,
}

fn new_fact<'a>(func: FnRef<'a>, node: NodeId) -> FnFact<'a> {
    FnFact {
        func,
        node,
        closed: false,
        calls: Vec::new(),
        params: Vec::new(),
        params_open: false,
        returns_typed: false,
        returns: Vec::new(),
        falls_through: falls_through(func),
        symbol: None,
    }
}

fn falls_through(func: FnRef<'_>) -> bool {
    if func.expression_body().is_some() {
        return false;
    }
    !matches!(
        func.statements().last(),
        Some(Statement::ReturnStatement(_) | Statement::ThrowStatement(_))
    )
}

fn enclosing_function(nodes: &AstNodes<'_>, node: NodeId) -> Option<NodeId> {
    nodes.ancestor_ids(node).skip(1).find(|id| {
        matches!(
            nodes.kind(*id),
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
        )
    })
}

fn in_loop_head(nodes: &AstNodes<'_>, declarator: NodeId) -> bool {
    let declaration = nodes.parent_id(declarator);
    matches!(
        nodes.parent_kind(declaration),
        AstKind::ForInStatement(_) | AstKind::ForOfStatement(_)
    )
}

pub fn declaration_kind(
    nodes: &AstNodes<'_>,
    declarator: &VariableDeclarator<'_>,
) -> VariableDeclarationKind {
    match nodes.parent_kind(declarator.node_id.get()) {
        AstKind::VariableDeclaration(declaration) => declaration.kind,
        _ => VariableDeclarationKind::Var,
    }
}

pub struct Ctx<'s, 'a> {
    pub program: &'a Program<'a>,
    pub scoping: &'s Scoping,
    pub nodes: &'s AstNodes<'a>,
    pub module: bool,
}

impl<'a> Ctx<'_, 'a> {
    pub fn is_root(&self, symbol: SymbolId) -> bool {
        self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
    }

    pub fn reference_ident(&self, node: NodeId) -> Option<&'a IdentifierReference<'a>> {
        match self.nodes.kind(node) {
            AstKind::IdentifierReference(ident) => Some(ident),
            _ => None,
        }
    }

    fn direct_call(&self, node: NodeId, span: Span) -> Option<&'a CallExpression<'a>> {
        match self.nodes.parent_kind(node) {
            AstKind::CallExpression(call) if call.callee.without_parentheses().span() == span => {
                Some(call)
            }
            _ => None,
        }
    }

    fn every_reference_is_a_call(
        &self,
        symbol: SymbolId,
        calls: &mut Vec<&'a CallExpression<'a>>,
    ) -> bool {
        for reference in self.scoping.get_resolved_references(symbol) {
            if reference.flags().is_write() {
                return false;
            }
            let node = reference.node_id();
            let Some(ident) = self.reference_ident(node) else {
                return false;
            };
            match self.direct_call(node, ident.span) {
                Some(call) => calls.push(call),
                None => return false,
            }
        }
        true
    }

    fn single_declaration(&self, symbol: SymbolId) -> bool {
        self.scoping.symbol_redeclarations(symbol).is_empty()
    }

    fn open_root(&self, symbol: SymbolId) -> bool {
        self.is_root(symbol) && !self.module
    }
}

pub fn collect<'a>(cx: &Ctx<'_, 'a>) -> Facts<'a> {
    let mut facts = Facts {
        slots: FxHashMap::default(),
        functions: Vec::new(),
        by_node: FxHashMap::default(),
        by_symbol: FxHashMap::default(),
    };
    let mut declarators = Vec::new();
    let mut returns = Vec::new();
    for node in cx.nodes.iter() {
        match node.kind() {
            AstKind::Function(function) => {
                facts.by_node.insert(node.id(), facts.functions.len());
                facts
                    .functions
                    .push(new_fact(FnRef::Function(function), node.id()));
            }
            AstKind::ArrowFunctionExpression(arrow) => {
                facts.by_node.insert(node.id(), facts.functions.len());
                facts
                    .functions
                    .push(new_fact(FnRef::Arrow(arrow), node.id()));
            }
            AstKind::VariableDeclarator(declarator) => {
                declarators.push((declarator, in_loop_head(cx.nodes, node.id())));
            }
            AstKind::ReturnStatement(statement) => returns.push((node.id(), statement)),
            _ => {}
        }
    }
    for (node, statement) in returns {
        let owner = enclosing_function(cx.nodes, node).and_then(|id| facts.by_node.get(&id));
        if let Some(&owner) = owner {
            facts.functions[owner]
                .returns
                .push(statement.argument.as_ref());
        }
    }
    for index in 0..facts.functions.len() {
        settle_function(cx, &mut facts, index);
    }
    let early = early_reads(cx, &declarators);
    for (declarator, in_head) in declarators {
        binding_slot(cx, &mut facts, declarator, in_head, &early);
    }
    for index in 0..facts.functions.len() {
        param_slots(cx, &mut facts, index);
    }
    facts
}

fn early_reads<'a>(
    cx: &Ctx<'_, 'a>,
    declarators: &[(&'a VariableDeclarator<'a>, bool)],
) -> rustc_hash::FxHashSet<SymbolId> {
    let candidates: Vec<(SymbolId, Span)> = declarators
        .iter()
        .filter(|(declarator, _)| {
            declaration_kind(cx.nodes, declarator) == VariableDeclarationKind::Var
        })
        .filter_map(|(declarator, _)| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => {
                id.symbol_id.get().map(|s| (s, declarator.span))
            }
            _ => None,
        })
        .collect();
    untwist_core::semantic::bindings_possibly_used_before(cx.program, cx.scoping, &candidates)
}

fn function_symbol<'a>(cx: &Ctx<'_, 'a>, fact: &FnFact<'a>) -> Option<(SymbolId, bool)> {
    let parent = cx.nodes.parent_kind(fact.node);
    if let FnRef::Function(function) = fact.func {
        if function.is_declaration() {
            let placed = matches!(parent, AstKind::Program(_) | AstKind::FunctionBody(_));
            return function
                .id
                .as_ref()
                .and_then(|id| id.symbol_id.get())
                .map(|symbol| (symbol, placed));
        }
    }
    match parent {
        AstKind::VariableDeclarator(declarator) => match &declarator.id {
            BindingPattern::BindingIdentifier(id) => {
                let constant = declaration_kind(cx.nodes, declarator)
                    != VariableDeclarationKind::Var
                    || cx
                        .scoping
                        .get_resolved_references(id.symbol_id.get()?)
                        .all(|r| !r.is_write());
                id.symbol_id.get().map(|symbol| (symbol, constant))
            }
            _ => None,
        },
        _ => None,
    }
}

fn self_name_closed<'a>(
    cx: &Ctx<'_, 'a>,
    fact: &FnFact<'a>,
    calls: &mut Vec<&'a CallExpression<'a>>,
) -> bool {
    let FnRef::Function(function) = fact.func else {
        return true;
    };
    if function.is_declaration() {
        return true;
    }
    match function.id.as_ref().and_then(|id| id.symbol_id.get()) {
        Some(symbol) => cx.every_reference_is_a_call(symbol, calls),
        None => true,
    }
}

fn settle_function<'a>(cx: &Ctx<'_, 'a>, facts: &mut Facts<'a>, index: usize) {
    let fact = &facts.functions[index];
    let mut calls = Vec::new();
    let mut symbol = None;
    let closed = match iife_call(cx, fact) {
        Some(call) => {
            calls.push(call);
            self_name_closed(cx, fact, &mut calls)
        }
        None => match function_symbol(cx, fact) {
            Some((found, placed)) => {
                symbol = Some(found);
                placed
                    && cx.single_declaration(found)
                    && !cx.open_root(found)
                    && cx.every_reference_is_a_call(found, &mut calls)
                    && self_name_closed(cx, fact, &mut calls)
            }
            None => false,
        },
    };
    let constructible = constructible(cx, fact);
    let returns_typed = !is_generator(fact.func)
        && !is_accessor(cx, fact)
        && (closed || !constructible || fact.func.is_async());
    let reads_arguments = reads_arguments(fact.func);
    let params = fact
        .func
        .params()
        .items
        .iter()
        .map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect::<Vec<_>>();
    let spread = calls
        .iter()
        .any(|call| call.arguments.iter().any(Argument::is_spread));
    let params_open = !closed
        || reads_arguments
        || spread
        || fact.func.params().rest.is_some()
        || duplicate_params(cx, &params);
    let fact = &mut facts.functions[index];
    fact.closed = closed;
    fact.calls = calls;
    fact.params = params;
    fact.params_open = params_open;
    fact.returns_typed = returns_typed;
    fact.symbol = symbol;
    if closed {
        if let Some(symbol) = symbol {
            facts.by_symbol.insert(symbol, index);
        }
    }
}

fn duplicate_params(cx: &Ctx<'_, '_>, params: &[Option<SymbolId>]) -> bool {
    let mut names: Vec<&str> = params
        .iter()
        .flatten()
        .map(|symbol| cx.scoping.symbol_name(*symbol))
        .collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    names.len() != total
}

fn iife_call<'a>(cx: &Ctx<'_, 'a>, fact: &FnFact<'a>) -> Option<&'a CallExpression<'a>> {
    if let FnRef::Function(function) = fact.func {
        if function.is_declaration() {
            return None;
        }
    }
    cx.direct_call(fact.node, fact.func.span())
}

fn is_generator(func: FnRef<'_>) -> bool {
    matches!(func, FnRef::Function(function) if function.generator)
}

fn is_accessor(cx: &Ctx<'_, '_>, fact: &FnFact<'_>) -> bool {
    match cx.nodes.parent_kind(fact.node) {
        AstKind::MethodDefinition(method) => method.kind != MethodDefinitionKind::Method,
        AstKind::ObjectProperty(property) => property.kind != PropertyKind::Init,
        _ => false,
    }
}

fn constructible(cx: &Ctx<'_, '_>, fact: &FnFact<'_>) -> bool {
    let FnRef::Function(function) = fact.func else {
        return false;
    };
    if function.r#async || function.generator {
        return false;
    }
    match cx.nodes.parent_kind(fact.node) {
        AstKind::MethodDefinition(_) => false,
        AstKind::ObjectProperty(property) => !property.method,
        _ => true,
    }
}

struct ArgumentsScan {
    found: bool,
}

impl<'a> Visit<'a> for ArgumentsScan {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.found |= it.name == "arguments";
    }
}

fn reads_arguments(func: FnRef<'_>) -> bool {
    let mut scan = ArgumentsScan { found: false };
    scan.visit_formal_parameters(func.params());
    for statement in func.statements() {
        scan.visit_statement(statement);
    }
    if let Some(expression) = func.expression_body() {
        scan.visit_expression(expression);
    }
    scan.found
}

fn write_source<'a>(cx: &Ctx<'_, 'a>, symbol: SymbolId, node: NodeId) -> Source<'a> {
    match cx.nodes.parent_kind(node) {
        AstKind::AssignmentExpression(assign) => {
            let target = assign.left.get_identifier_name();
            if target.is_none() {
                return Source::Top;
            }
            match assign.operator {
                AssignmentOperator::Assign => Source::Expr(&assign.right),
                AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish => Source::Expr(&assign.right),
                operator => match operator.to_binary_operator() {
                    Some(binary) => Source::Compound(binary, symbol, &assign.right),
                    None => Source::Top,
                },
            }
        }
        AstKind::UpdateExpression(_) => Source::Update(symbol),
        AstKind::ForInStatement(_) => Source::String,
        _ => Source::Top,
    }
}

fn binding_slot<'a>(
    cx: &Ctx<'_, 'a>,
    facts: &mut Facts<'a>,
    declarator: &'a VariableDeclarator<'a>,
    in_head: bool,
    early: &rustc_hash::FxHashSet<SymbolId>,
) {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return;
    };
    let Some(symbol) = id.symbol_id.get() else {
        return;
    };
    let constant = declaration_kind(cx.nodes, declarator).is_const();
    let mut sources = Vec::new();
    let usable = cx.single_declaration(symbol)
        && (constant || !cx.open_root(symbol))
        && !cx
            .scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::Function);
    if !usable {
        sources.push(Source::Top);
    }
    match (&declarator.init, in_head) {
        (Some(init), _) => sources.push(Source::Expr(init)),
        (None, true) => sources.push(loop_head_source(cx, declarator)),
        (None, false) => sources.push(Source::Undefined),
    }
    if early.contains(&symbol) {
        sources.push(Source::Undefined);
    }
    for reference in cx.scoping.get_resolved_references(symbol) {
        if reference.flags().is_write() {
            sources.push(write_source(cx, symbol, reference.node_id()));
        }
    }
    facts.slots.insert(
        symbol,
        Slot {
            kind: SlotKind::Binding {
                constant,
                in_loop_head: in_head,
            },
            at: id.span,
            declarator: Some(declarator),
            sources,
        },
    );
}

fn loop_head_source<'a>(cx: &Ctx<'_, 'a>, declarator: &'a VariableDeclarator<'a>) -> Source<'a> {
    let declaration = cx.nodes.parent_id(declarator.node_id.get());
    match cx.nodes.parent_kind(declaration) {
        AstKind::ForInStatement(_) => Source::String,
        _ => Source::Top,
    }
}

fn param_slots<'a>(cx: &Ctx<'_, 'a>, facts: &mut Facts<'a>, index: usize) {
    let fact = &facts.functions[index];
    let items = &fact.func.params().items;
    let mut slots = Vec::new();
    for (position, param) in items.iter().enumerate() {
        let BindingPattern::BindingIdentifier(id) = &param.pattern else {
            continue;
        };
        let Some(symbol) = id.symbol_id.get() else {
            continue;
        };
        let mut sources = Vec::new();
        if fact.params_open {
            sources.push(Source::Top);
        }
        for call in &fact.calls {
            let argument = call
                .arguments
                .get(position)
                .and_then(Argument::as_expression);
            sources.push(match (&param.initializer, argument) {
                (Some(default), argument) => Source::Defaulted(argument, default),
                (None, Some(argument)) => Source::Expr(argument),
                (None, None) => Source::Undefined,
            });
        }
        for reference in cx.scoping.get_resolved_references(symbol) {
            if reference.flags().is_write() {
                sources.push(write_source(cx, symbol, reference.node_id()));
            }
        }
        slots.push((
            symbol,
            Slot {
                kind: SlotKind::Param {
                    function: index,
                    position,
                },
                at: id.span,
                declarator: None,
                sources,
            },
        ));
    }
    facts.slots.extend(slots);
}
