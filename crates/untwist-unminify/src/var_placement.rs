use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::pass::Cx;

#[derive(Clone, Copy)]
struct Frame {
    list: usize,
    index: usize,
    scope: Option<ScopeId>,
    looped: bool,
}

#[derive(Clone, Copy)]
struct Anchor {
    depth: usize,
    list: usize,
    index: usize,
    reference: ReferenceId,
    scope: ScopeId,
    looped: bool,
}

#[derive(Default)]
struct Fact {
    seen: usize,
    before_declaration: Option<usize>,
    order: usize,
    anchor: Option<Anchor>,
    anchored: bool,
}

pub(crate) struct Conversion {
    pub(crate) symbol: SymbolId,
    pub(crate) reference: ReferenceId,
    pub(crate) scope: ScopeId,
    pub(crate) kind: VariableDeclarationKind,
}

#[derive(Default)]
pub(crate) struct Plan {
    pub(crate) converted: Vec<Conversion>,
    pub(crate) hoisted: Vec<SymbolId>,
}

impl Plan {
    pub(crate) fn is_empty(&self) -> bool {
        self.converted.is_empty() && self.hoisted.is_empty()
    }
}

struct Scan<'s> {
    scoping: &'s Scoping,
    var_scope: ScopeId,
    frames: Vec<Frame>,
    next_scope: Option<ScopeId>,
    target: Option<ReferenceId>,
    facts: FxHashMap<SymbolId, Fact>,
    declared: usize,
    loops: usize,
    closures: usize,
    hoisted_closure: bool,
    descend: bool,
}

fn list_key(stmts: &AVec<'_, Statement<'_>>) -> usize {
    std::ptr::from_ref(stmts) as usize
}

fn assignment_target(stmt: &Statement<'_>) -> Option<ReferenceId> {
    let Statement::ExpressionStatement(node) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &node.expression else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    match &assign.left {
        AssignmentTarget::AssignmentTargetIdentifier(id) => id.reference_id.get(),
        _ => None,
    }
}

fn bare_symbols<'d>(
    declaration: &'d VariableDeclaration<'_>,
) -> impl Iterator<Item = SymbolId> + 'd {
    declaration
        .declarations
        .iter()
        .filter(|declarator| declarator.init.is_none())
        .filter_map(|declarator| match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
}

impl Scan<'_> {
    fn tracked(&self, symbol: SymbolId) -> bool {
        self.scoping.symbol_scope_id(symbol) == self.var_scope
            && self.scoping.symbol_flags(symbol) == SymbolFlags::FunctionScopedVariable
    }

    fn note_declarations(&mut self, stmt: &Statement<'_>) {
        let Statement::VariableDeclaration(declaration) = stmt else {
            return;
        };
        if declaration.kind != VariableDeclarationKind::Var || declaration.declare {
            return;
        }
        for symbol in bare_symbols(declaration) {
            if !self.tracked(symbol) {
                continue;
            }
            let order = self.declared;
            self.declared += 1;
            let fact = self.facts.entry(symbol).or_default();
            fact.before_declaration = Some(fact.seen);
            fact.order = order;
        }
    }

    fn reference(&mut self, reference: ReferenceId) {
        let Some(symbol) = self.scoping.get_reference(reference).symbol_id() else {
            return;
        };
        if !self.tracked(symbol) {
            return;
        }
        let top = self.frames.last().copied();
        let depth = self.frames.len().saturating_sub(1);
        let direct = self.target == Some(reference) && self.closures == 0;
        let captured = self.closures > 0;
        let hoisted = self.hoisted_closure;
        let frames = &self.frames;
        let fact = self.facts.entry(symbol).or_default();
        fact.seen += 1;
        if fact.seen == 1 {
            fact.anchor = top.filter(|_| direct).and_then(|frame| {
                Some(Anchor {
                    depth,
                    list: frame.list,
                    index: frame.index,
                    reference,
                    scope: frame.scope?,
                    looped: frame.looped,
                })
            });
            fact.anchored = fact.anchor.is_some();
            return;
        }
        if let Some(anchor) = fact.anchor.filter(|_| fact.anchored) {
            let after = frames
                .get(anchor.depth)
                .is_some_and(|frame| frame.list == anchor.list && frame.index > anchor.index);
            let shared = !captured || (!anchor.looped && !hoisted);
            fact.anchored = after && shared;
        }
    }

    fn looping(&mut self, walk: impl FnOnce(&mut Self)) {
        self.loops += 1;
        walk(self);
        self.loops -= 1;
    }

    fn closure(&mut self, hoisted: bool, walk: impl FnOnce(&mut Self)) {
        if !self.descend {
            return;
        }
        let (loops, outer) = (self.loops, self.hoisted_closure);
        if self.closures == 0 {
            self.hoisted_closure = hoisted;
        }
        self.closures += 1;
        self.loops = 0;
        walk(self);
        self.closures -= 1;
        self.loops = loops;
        self.hoisted_closure = outer;
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_statements(&mut self, it: &AVec<'a, Statement<'a>>) {
        let scope = self.next_scope.take();
        self.frames.push(Frame {
            list: list_key(it),
            index: 0,
            scope,
            looped: self.loops > 0,
        });
        for (index, stmt) in it.iter().enumerate() {
            if let Some(frame) = self.frames.last_mut() {
                frame.index = index;
            }
            self.target = assignment_target(stmt);
            self.note_declarations(stmt);
            self.visit_statement(stmt);
        }
        self.frames.pop();
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.next_scope = it.scope_id.get();
        walk_js::walk_block_statement(self, it);
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        self.next_scope = None;
        walk_js::walk_switch_case(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(reference) = it.reference_id.get() {
            self.reference(reference);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.closure(it.is_declaration(), |scan| {
            walk_js::walk_function(scan, it, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.closure(false, |scan| {
            walk_js::walk_arrow_function_expression(scan, it);
        });
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.closure(false, |scan| walk_js::walk_class(scan, it));
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.looping(|scan| walk_js::walk_for_statement(scan, it));
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.looping(|scan| walk_js::walk_for_in_statement(scan, it));
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.looping(|scan| walk_js::walk_for_of_statement(scan, it));
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.looping(|scan| walk_js::walk_while_statement(scan, it));
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.looping(|scan| walk_js::walk_do_while_statement(scan, it));
    }
}

fn bindings_clear(scoping: &Scoping, symbol: SymbolId, name: Ident<'_>, scope: ScopeId) -> bool {
    let owner = scoping.symbol_scope_id(symbol);
    for ancestor in scoping.scope_ancestors(scope) {
        if scoping
            .get_binding(ancestor, name)
            .is_some_and(|other| other != symbol)
        {
            return false;
        }
        if ancestor == owner {
            return true;
        }
    }
    false
}

fn only_one_write(scoping: &Scoping, symbol: SymbolId, anchor: ReferenceId) -> bool {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .all(|id| *id == anchor || !scoping.get_reference(*id).is_write())
}

fn conversion(scoping: &Scoping, symbol: SymbolId, fact: &Fact) -> Option<Conversion> {
    let anchor = fact.anchor.filter(|_| fact.anchored)?;
    let total = scoping.get_resolved_reference_ids(symbol).len();
    let name = scoping.symbol_ident(symbol);
    if total != fact.seen || !bindings_clear(scoping, symbol, name, anchor.scope) {
        return None;
    }
    let kind = if only_one_write(scoping, symbol, anchor.reference) {
        VariableDeclarationKind::Const
    } else {
        VariableDeclarationKind::Let
    };
    Some(Conversion {
        symbol,
        reference: anchor.reference,
        scope: anchor.scope,
        kind,
    })
}

fn scan(
    scoping: &Scoping,
    stmts: &AVec<'_, Statement<'_>>,
    var_scope: ScopeId,
    descend: bool,
) -> FxHashMap<SymbolId, Fact> {
    let mut scan = Scan {
        scoping,
        var_scope,
        frames: Vec::new(),
        next_scope: Some(var_scope),
        target: None,
        facts: FxHashMap::default(),
        declared: 0,
        loops: 0,
        closures: 0,
        hoisted_closure: false,
        descend,
    };
    scan.visit_statements(stmts);
    scan.facts
}

fn is_candidate(
    scoping: &Scoping,
    symbol: SymbolId,
    fact: &Fact,
    refused: &impl Fn(&str) -> bool,
) -> bool {
    fact.before_declaration.is_some()
        && scoping.symbol_redeclarations(symbol).is_empty()
        && !refused(scoping.symbol_name(symbol))
}

fn captured(scoping: &Scoping, symbol: SymbolId, fact: &Fact) -> bool {
    fact.seen < scoping.get_resolved_reference_ids(symbol).len()
}

pub(crate) fn plan(
    scoping: &Scoping,
    stmts: &AVec<'_, Statement<'_>>,
    var_scope: ScopeId,
    refused: impl Fn(&str) -> bool,
) -> Plan {
    let mut facts = scan(scoping, stmts, var_scope, false);
    facts.retain(|symbol, fact| is_candidate(scoping, *symbol, fact, &refused));
    if facts
        .iter()
        .any(|(symbol, fact)| captured(scoping, *symbol, fact))
    {
        let mut deep = scan(scoping, stmts, var_scope, true);
        deep.retain(|symbol, _| facts.contains_key(symbol));
        facts = deep;
    }
    let mut misplaced: Vec<(SymbolId, Fact)> = facts
        .into_iter()
        .filter(|(_, fact)| fact.before_declaration.is_some_and(|seen| seen > 0))
        .collect();
    misplaced.sort_by_key(|(_, fact)| fact.order);
    let mut plan = Plan::default();
    for (symbol, fact) in misplaced {
        match conversion(scoping, symbol, &fact) {
            Some(converted) => plan.converted.push(converted),
            None => plan.hoisted.push(symbol),
        }
    }
    plan
}

struct Apply<'p, 'a, 'c, 'x> {
    removed: FxHashSet<SymbolId>,
    hoisted: &'p [SymbolId],
    anchors: FxHashMap<ReferenceId, (SymbolId, VariableDeclarationKind)>,
    taken: Vec<(SymbolId, VariableDeclarator<'a>)>,
    cx: &'x mut Cx<'a, 'c>,
}

impl<'a> Apply<'_, 'a, '_, '_> {
    fn declared_at(&mut self, stmt: Statement<'a>, out: &mut AVec<'a, Statement<'a>>) {
        let Statement::VariableDeclaration(mut declaration) = stmt else {
            out.push(stmt);
            return;
        };
        let declarators = declaration.declarations.take_in(&*self.cx);
        let mut kept = AVec::with_capacity_in(declarators.len(), &*self.cx);
        for declarator in declarators {
            let symbol = match &declarator.id {
                BindingPattern::BindingIdentifier(id) if declarator.init.is_none() => {
                    id.symbol_id.get()
                }
                _ => None,
            };
            match symbol.filter(|symbol| self.removed.contains(symbol)) {
                Some(symbol) if self.hoisted.contains(&symbol) => {
                    self.taken.push((symbol, declarator));
                }
                Some(_) => {}
                None => kept.push(declarator),
            }
        }
        if kept.is_empty() {
            return;
        }
        declaration.declarations = kept;
        out.push(Statement::VariableDeclaration(declaration));
    }

    fn converted(&mut self, stmt: Statement<'a>) -> Result<Statement<'a>, Statement<'a>> {
        let Some(&(symbol, kind)) = assignment_target(&stmt).and_then(|r| self.anchors.get(&r))
        else {
            return Err(stmt);
        };
        let span = stmt.span();
        let mut node = match stmt {
            Statement::ExpressionStatement(node) => node,
            other => return Err(other),
        };
        let Expression::AssignmentExpression(assign) = &mut node.expression else {
            return Err(Statement::ExpressionStatement(node));
        };
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
            return Err(Statement::ExpressionStatement(node));
        };
        let (name, target_span) = (target.name, target.span);
        let init = assign.right.take_in(&*self.cx);
        let cx = &*self.cx;
        let pattern =
            BindingPattern::new_binding_identifier_with_symbol_id(target_span, name, symbol, cx);
        let declarator = VariableDeclarator::new(target_span, pattern, None, Some(init), false, cx);
        let declarators = AVec::from_iter_in([declarator], cx);
        Ok(Statement::new_variable_declaration(
            span,
            kind,
            declarators,
            false,
            cx,
        ))
    }
}

impl<'a> VisitMut<'a> for Apply<'_, 'a, '_, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        walk_mut::walk_statements(self, it);
        let old = it.take_in(&*self.cx);
        let mut out = AVec::with_capacity_in(old.len(), &*self.cx);
        for stmt in old {
            match self.converted(stmt) {
                Ok(declaration) => out.push(declaration),
                Err(stmt) => self.declared_at(stmt, &mut out),
            }
        }
        *it = out;
    }

    fn visit_function(&mut self, _it: &mut Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &mut ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &mut Class<'a>) {}
}

fn flags_of(kind: VariableDeclarationKind) -> SymbolFlags {
    if kind == VariableDeclarationKind::Const {
        SymbolFlags::BlockScopedVariable | SymbolFlags::ConstVariable
    } else {
        SymbolFlags::BlockScopedVariable
    }
}

fn rebind(scoping: &mut Scoping, conversion: &Conversion) -> bool {
    let symbol = conversion.symbol;
    let owner = scoping.symbol_scope_id(symbol);
    scoping.delete_resolved_reference(symbol, conversion.reference);
    *scoping.symbol_flags_mut(symbol) = flags_of(conversion.kind);
    if owner == conversion.scope {
        return false;
    }
    scoping.move_binding_by_symbol_id(owner, conversion.scope, symbol);
    scoping.set_symbol_scope_id(symbol, conversion.scope);
    true
}

pub(crate) fn apply<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    plan: &Plan,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let removed = plan
        .converted
        .iter()
        .map(|c| c.symbol)
        .chain(plan.hoisted.iter().copied())
        .collect();
    let anchors = plan
        .converted
        .iter()
        .map(|c| (c.reference, (c.symbol, c.kind)))
        .collect();
    let mut taken = {
        let mut apply = Apply {
            removed,
            hoisted: &plan.hoisted,
            anchors,
            taken: Vec::new(),
            cx: &mut *cx,
        };
        apply.visit_statements(stmts);
        apply.taken
    };
    taken.sort_by_key(|(symbol, _)| plan.hoisted.iter().position(|s| s == symbol));
    let moved = taken.len();
    let old = stmts.take_in(&*cx);
    let mut out = AVec::with_capacity_in(old.len() + moved, &*cx);
    for (_, declarator) in taken {
        let declarators = AVec::from_iter_in([declarator], &*cx);
        out.push(Statement::new_variable_declaration(
            declarator_span(&declarators),
            VariableDeclarationKind::Var,
            declarators,
            false,
            &*cx,
        ));
    }
    out.extend(old);
    *stmts = out;
    let mut bindings_moved = false;
    for conversion in &plan.converted {
        bindings_moved |= rebind(cx.scoping_mut(), conversion);
    }
    if bindings_moved {
        cx.binding_moved();
    }
    plan.converted.len() + moved
}

fn declarator_span(declarators: &AVec<'_, VariableDeclarator<'_>>) -> oxc_span::Span {
    declarators.first().map(|d| d.span).unwrap_or_default()
}
