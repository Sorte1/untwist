use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::rename::{taken_names, UNIQUE_NAME_SUFFIX_CAP};
use untwist_core::pass::{Cx, Interest, Locality, Pass};
use untwist_core::semantic::scope_hazard;

#[derive(Default)]
struct Names {
    taken: Option<FxHashSet<String>>,
}

impl Names {
    fn fresh(&mut self, base: &str, scoping: &Scoping) -> Option<String> {
        let taken = self.taken.get_or_insert_with(|| {
            taken_names(scoping)
                .into_iter()
                .map(str::to_string)
                .collect()
        });
        let stem = base.strip_suffix("__").unwrap_or(base);
        let name = (2..=UNIQUE_NAME_SUFFIX_CAP)
            .map(|suffix| format!("{stem}_{suffix}__"))
            .find(|candidate| !taken.contains(candidate))?;
        taken.insert(name.clone());
        Some(name)
    }
}

#[derive(Default)]
pub struct SplitMergedVars {
    hazard: bool,
    names: Names,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(SplitMergedVars::default())
}

#[derive(Clone, Copy)]
enum Boundary {
    Assigned(ReferenceId),
    Declared(usize),
}

struct Site {
    at: usize,
    boundary: Boundary,
}

struct Shape<'k, 'a> {
    params: &'k FormalParameters<'a>,
    param_symbols: FxHashSet<SymbolId>,
    statements: &'k [Statement<'a>],
    scope: ScopeId,
}

fn target_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<(SymbolId, ReferenceId)> {
    let Expression::AssignmentExpression(assign) = expr else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
        return None;
    };
    if assign.operator != AssignmentOperator::Assign {
        return None;
    }
    let reference = id.reference_id.get()?;
    Some((scoping.get_reference(reference).symbol_id()?, reference))
}

fn boundary_of(stmt: &Statement<'_>, scoping: &Scoping) -> Option<(SymbolId, Boundary)> {
    match stmt {
        Statement::ExpressionStatement(node) => {
            let (symbol, reference) = target_symbol(&node.expression, scoping)?;
            Some((symbol, Boundary::Assigned(reference)))
        }
        Statement::VariableDeclaration(node) if node.kind == VariableDeclarationKind::Var => {
            let [declarator] = node.declarations.as_slice() else {
                return None;
            };
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            declarator.init.as_ref()?;
            Some((id.symbol_id.get()?, Boundary::Declared(address(id))))
        }
        _ => None,
    }
}

fn address(id: &BindingIdentifier<'_>) -> usize {
    std::ptr::from_ref(id).addr()
}

#[derive(Default)]
struct Params {
    symbols: FxHashSet<SymbolId>,
}

impl<'a> VisitJs<'a> for Params {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(symbol) = it.symbol_id.get() {
            self.symbols.insert(symbol);
        }
    }
}

fn param_symbols(params: &FormalParameters<'_>) -> FxHashSet<SymbolId> {
    let mut found = Params::default();
    found.visit_formal_parameters(params);
    found.symbols
}

fn qualifies(symbol: SymbolId, shape: &Shape<'_, '_>, scoping: &Scoping) -> bool {
    let flags = scoping.symbol_flags(symbol);
    let plain = flags.contains(SymbolFlags::FunctionScopedVariable)
        && !flags.intersects(SymbolFlags::Function | SymbolFlags::BlockScopedVariable);
    plain
        && scoping.symbol_scope_id(symbol) == shape.scope
        && (shape.param_symbols.contains(&symbol)
            || !scoping.symbol_redeclarations(symbol).is_empty())
}

fn candidates(shape: &Shape<'_, '_>, scoping: &Scoping) -> FxHashMap<SymbolId, Vec<Site>> {
    let mut found: FxHashMap<SymbolId, Vec<Site>> = FxHashMap::default();
    for (at, stmt) in shape.statements.iter().enumerate() {
        let Some((symbol, boundary)) = boundary_of(stmt, scoping) else {
            continue;
        };
        let read_enough = scoping.get_resolved_reference_ids(symbol).len() >= 2;
        if read_enough && qualifies(symbol, shape, scoping) {
            found.entry(symbol).or_default().push(Site { at, boundary });
        }
    }
    found
}

struct Use {
    at: Option<usize>,
    reference: Option<ReferenceId>,
    binding: Option<usize>,
    write: bool,
}

struct Gather<'s> {
    scoping: &'s Scoping,
    wanted: &'s FxHashMap<SymbolId, Vec<Site>>,
    uses: FxHashMap<SymbolId, Vec<Use>>,
    captured: FxHashMap<SymbolId, bool>,
    depth: u32,
    arguments: bool,
    at: Option<usize>,
    shadows: FxHashSet<String>,
    fresh: FxHashSet<String>,
}

impl Gather<'_> {
    fn record(&mut self, symbol: SymbolId, used: Use) {
        if self.depth > 0 {
            self.captured.insert(symbol, true);
        }
        self.uses.entry(symbol).or_default().push(used);
    }
}

impl<'a> VisitJs<'a> for Gather<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (true, Some(id)) = (it.is_declaration(), it.id.as_ref()) {
            self.shadows.insert(id.name.to_string());
        }
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.depth += 1;
        walk_js::walk_class(self, it);
        self.depth -= 1;
    }

    fn visit_catch_parameter(&mut self, it: &CatchParameter<'a>) {
        for id in it.pattern.get_binding_identifiers() {
            self.shadows.insert(id.name.to_string());
        }
        walk_js::walk_catch_parameter(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "arguments" {
            self.arguments = true;
        }
        let Some(reference) = it.reference_id.get() else {
            self.fresh.insert(it.name.to_string());
            return;
        };
        let reference_data = self.scoping.get_reference(reference);
        let Some(symbol) = reference_data.symbol_id() else {
            return;
        };
        if self.wanted.contains_key(&symbol) {
            let write = reference_data.is_write();
            self.record(
                symbol,
                Use {
                    at: self.at,
                    reference: Some(reference),
                    binding: None,
                    write,
                },
            );
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let Some(symbol) = it.symbol_id.get() else {
            self.fresh.insert(it.name.to_string());
            return;
        };
        if self.wanted.contains_key(&symbol) {
            self.record(
                symbol,
                Use {
                    at: self.at,
                    reference: None,
                    binding: Some(address(it)),
                    write: true,
                },
            );
        }
    }
}

fn is_boundary_use(site: &Site, used: &Use) -> bool {
    match site.boundary {
        Boundary::Assigned(reference) => used.reference == Some(reference),
        Boundary::Declared(binding) => used.binding == Some(binding),
    }
}

fn segment_of(sites: &[&Site], used: &Use) -> usize {
    let Some(at) = used.at else {
        return 0;
    };
    sites
        .iter()
        .filter(|site| site.at < at || (site.at == at && is_boundary_use(site, used)))
        .count()
}

fn reads_in(sites: &[&Site], segment: usize, uses: &[Use]) -> bool {
    uses.iter()
        .any(|used| !used.write && segment_of(sites, used) == segment)
}

fn kept_sites<'s>(sites: &'s [Site], uses: &[Use]) -> Vec<&'s Site> {
    let mut kept: Vec<&Site> = Vec::new();
    for site in sites {
        let ended = kept.len();
        kept.push(site);
        let useful = reads_in(&kept, ended, uses) && reads_in(&kept, ended + 1, uses);
        if !useful {
            kept.pop();
        }
    }
    kept
}

struct Pending {
    base: String,
    versions: usize,
    uses: Vec<(Option<ReferenceId>, Option<usize>, usize)>,
    assigned: Vec<(usize, bool)>,
}

#[derive(Default)]
struct Plan<'a> {
    pending: Vec<Pending>,
    references: FxHashMap<ReferenceId, Ident<'a>>,
    bindings: FxHashMap<usize, Ident<'a>>,
    assigned: FxHashMap<usize, Ident<'a>>,
}

impl<'a> Plan<'a> {
    fn name(&mut self, fresh: &mut Names, cx: &mut Cx<'a, '_>) -> Option<()> {
        for pending in std::mem::take(&mut self.pending) {
            let mut names = Vec::with_capacity(pending.versions);
            for _ in 0..pending.versions {
                let name = fresh.fresh(&pending.base, cx.scoping())?;
                names.push(Ident::from_str_in(&name, &*cx));
            }
            for (reference, binding, segment) in pending.uses {
                let Some(name) = segment.checked_sub(1).map(|at| names[at]) else {
                    continue;
                };
                if let Some(reference) = reference {
                    self.references.insert(reference, name);
                }
                if let Some(binding) = binding {
                    self.bindings.insert(binding, name);
                }
            }
            for ((at, assigned), name) in pending.assigned.iter().zip(&names) {
                if *assigned {
                    self.assigned.insert(*at, *name);
                }
            }
        }
        Some(())
    }
}

impl<'a> VisitMut<'a> for Plan<'a> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if let Some(name) = it.reference_id.get().and_then(|r| self.references.get(&r)) {
            it.name = *name;
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(name) = self.bindings.get(&address(it)) {
            it.name = *name;
        }
    }
}

fn simple_params(params: &FormalParameters<'_>) -> bool {
    params.rest.is_none()
        && params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        })
}

fn plan<'a>(shape: &Shape<'_, 'a>, names: &mut Names, cx: &mut Cx<'a, '_>) -> Option<Plan<'a>> {
    let scoping = cx.scoping();
    let wanted = candidates(shape, scoping);
    if wanted.is_empty() {
        return None;
    }
    let mut gather = Gather {
        scoping,
        wanted: &wanted,
        uses: FxHashMap::default(),
        captured: FxHashMap::default(),
        depth: 0,
        arguments: false,
        at: None,
        shadows: FxHashSet::default(),
        fresh: FxHashSet::default(),
    };
    gather.visit_formal_parameters(shape.params);
    for (at, stmt) in shape.statements.iter().enumerate() {
        gather.at = Some(at);
        gather.visit_statement(stmt);
    }
    let mapped = !scoping.scope_flags(shape.scope).is_strict_mode()
        && simple_params(shape.params)
        && gather.arguments;
    let Gather {
        uses,
        captured,
        shadows,
        fresh,
        ..
    } = gather;
    let mut out = Plan::default();
    let mut symbols: Vec<&SymbolId> = wanted.keys().collect();
    symbols.sort_unstable();
    for symbol in symbols {
        let name = scoping.symbol_name(*symbol);
        let shadowed = shadows.contains(name) || fresh.contains(name);
        if shadowed
            || captured.contains_key(symbol)
            || (mapped && shape.param_symbols.contains(symbol))
        {
            continue;
        }
        let Some(list) = uses.get(symbol) else {
            continue;
        };
        let kept = kept_sites(&wanted[symbol], list);
        if kept.is_empty() {
            continue;
        }
        let base = scoping.symbol_name(*symbol).to_string();
        let segments: Vec<usize> = list.iter().map(|used| segment_of(&kept, used)).collect();
        let assigned: Vec<(usize, bool)> = kept
            .iter()
            .map(|site| (site.at, matches!(site.boundary, Boundary::Assigned(_))))
            .collect();
        out.pending.push(Pending {
            base,
            versions: kept.len(),
            uses: list
                .iter()
                .zip(segments)
                .map(|(used, segment)| (used.reference, used.binding, segment))
                .collect(),
            assigned,
        });
    }
    if out.pending.is_empty() {
        return None;
    }
    out.name(names, cx)?;
    Some(out)
}

fn declare<'a>(stmt: &mut Statement<'a>, name: Ident<'a>, cx: &Cx<'a, '_>) {
    let Statement::ExpressionStatement(node) = stmt else {
        return;
    };
    let span = node.span;
    let Expression::AssignmentExpression(assign) = &mut node.expression else {
        return;
    };
    let init = std::mem::replace(&mut assign.right, Expression::new_null_literal(span, cx));
    let id = BindingPattern::new_binding_identifier(span, name, cx);
    let declarator = VariableDeclarator::new(span, id, None, Some(init), false, cx);
    let declarations = AVec::from_iter_in([declarator], cx);
    *stmt = Statement::new_variable_declaration(
        span,
        VariableDeclarationKind::Var,
        declarations,
        false,
        cx,
    );
}

fn split<'a>(
    names: &mut Names,
    params: &mut FormalParameters<'a>,
    body: &mut FunctionBody<'a>,
    scope: ScopeId,
    cx: &mut Cx<'a, '_>,
) {
    let shape = Shape {
        param_symbols: param_symbols(params),
        params,
        statements: &body.statements,
        scope,
    };
    let Some(mut plan) = plan(&shape, names, cx) else {
        return;
    };
    plan.visit_formal_parameters(params);
    for stmt in body.statements.iter_mut() {
        plan.visit_statement(stmt);
    }
    for (at, name) in &plan.assigned {
        if let Some(stmt) = body.statements.get_mut(*at) {
            declare(stmt, *name, &*cx);
        }
    }
    cx.changed();
}

impl<'a> Pass<'a> for SplitMergedVars {
    fn name(&self) -> &'static str {
        "split-merged-vars"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_FUNCTION | Interest::EXIT_ARROW_FUNCTION_EXPRESSION
    }

    fn enter_program(&mut self, _program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.names = Names::default();
        self.hazard = scope_hazard(cx.scoping()).is_some();
    }

    fn exit_function(&mut self, node: &mut Function<'a>, cx: &mut Cx<'a, '_>) {
        if self.hazard {
            return;
        }
        let (Some(scope), Some(body)) = (node.scope_id.get(), node.body.as_mut()) else {
            return;
        };
        split(&mut self.names, &mut node.params, body, scope, cx);
    }

    fn exit_arrow_function_expression(
        &mut self,
        node: &mut ArrowFunctionExpression<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        if self.hazard {
            return;
        }
        let Some(scope) = node.scope_id.get() else {
            return;
        };
        let ArrowFunctionBody::FunctionBody(body) = &mut node.body else {
            return;
        };
        split(&mut self.names, &mut node.params, body, scope, cx);
    }
}
