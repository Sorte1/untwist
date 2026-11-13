use std::cell::Cell;

use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::build_scoping;

use super::layout::{Def, DefFunction, Hoisted, Layout, Registry};
use super::links::{loader, module_name, require_name, ParcelView};
use super::slot::reaches_slot;
use super::{
    bare, candidates, is_builtin, moved_helpers, resolve, single_string, ENTRY_ID, HOISTED_NAMES,
    INHERITED, REGISTRY_NAMES, REGISTRY_PARAMS,
};
use crate::matcher::{expression_statement, param_names};
use crate::runtime::{first_unclosed_helper, helper_symbols};
use crate::scan::{is_direct_eval, Frame, Policy, Scan, Step};

const REQUIRE: &str = "require";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Factory,
    Entry,
    Prelude,
    Helper,
}

struct Rules<'s, 'l> {
    scoping: &'s Scoping,
    ids: &'l FxHashSet<&'l str>,
    global: &'l str,
    deps: &'l [(String, String)],
    inside: Span,
    load: Option<Span>,
    target: Option<Span>,
    require_param: Option<SymbolId>,
    module_param: Option<SymbolId>,
    require: Option<SymbolId>,
    register: Option<SymbolId>,
    names: [&'l str; 2],
    helpers: &'l FxHashMap<SymbolId, usize>,
    moved: &'l FxHashSet<SymbolId>,
    builtins: bool,
    slot: bool,
    mode: Mode,
    arrow: bool,
    legacy: bool,
    ok: bool,
}

impl Rules<'_, '_> {
    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn site(&self, id: &IdentifierReference<'_>) -> Option<ScopeId> {
        let reference = id.reference_id.get()?;
        Some(self.scoping.get_reference(reference).scope_id())
    }

    fn is_write(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_write())
    }

    fn inside(&self, symbol: SymbolId) -> bool {
        let span = self.scoping.symbol_span(symbol);
        self.inside.start <= span.start && span.end <= self.inside.end
    }

    fn require_unbound(&self, site: Option<ScopeId>, allowed: Option<SymbolId>) -> bool {
        let Some(site) = site else {
            return false;
        };
        self.scoping
            .find_binding(site, Ident::from(REQUIRE))
            .is_none_or(|symbol| Some(symbol) == allowed)
    }

    fn resolved(&self, symbol: SymbolId, write: bool) -> bool {
        match self.mode {
            Mode::Factory if self.inside(symbol) => {
                Some(symbol) != self.require_param && Some(symbol) != self.module_param
            }
            Mode::Factory => self.helpers.contains_key(&symbol) && !write,
            Mode::Prelude => !(write && self.moved.contains(&symbol)),
            Mode::Entry | Mode::Helper => {
                Some(symbol) != self.require
                    && Some(symbol) != self.register
                    && !(write && self.moved.contains(&symbol))
            }
        }
    }

    fn free(&self, name: &str, frame: Frame) -> bool {
        match name {
            REQUIRE | "__filename" | "__dirname" => false,
            "module" | "exports" => matches!(self.mode, Mode::Entry | Mode::Prelude),
            "arguments" => frame.receiver(),
            name => name != self.global,
        }
    }

    fn registry_require(&mut self, call: &CallExpression<'_>, site: Option<ScopeId>) -> bool {
        let Some(target) = single_string(call).map(|spec| resolve(self.deps, spec)) else {
            return false;
        };
        let builtin = !self.ids.contains(target);
        self.slot |= builtin;
        (!builtin || (self.builtins && is_builtin(target)))
            && self.require_unbound(site, self.require_param)
    }

    fn internal(&self, call: &CallExpression<'_>, site: Option<ScopeId>) -> bool {
        let target = self.mode == Mode::Factory && self.target == Some(call.span);
        single_string(call).is_some_and(|id| target || self.ids.contains(id))
            && self.require_unbound(site, self.require_param)
    }

    fn is_module(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Identifier(id) if self.module_param.is_some() && self.symbol(id) == self.module_param)
    }

    fn root_site(&self, callee: &Expression<'_>) -> Option<Option<ScopeId>> {
        let Expression::StaticMemberExpression(root) = callee else {
            return None;
        };
        let Expression::StaticMemberExpression(bundle) = &root.object else {
            return None;
        };
        let Expression::Identifier(module) = &bundle.object else {
            return None;
        };
        let fits = !root.optional
            && !bundle.optional
            && root.property.name == "root"
            && bundle.property.name == "bundle"
            && self.is_module(&bundle.object);
        fits.then(|| self.site(module))
    }

    fn loading(&self, call: &CallExpression<'_>) -> bool {
        match &call.callee {
            Expression::Identifier(id) if self.legacy && self.require_param.is_some() => {
                self.symbol(id) == self.require_param
                    && single_string(call).is_some_and(|spec| {
                        resolve(self.deps, spec) == spec && !self.ids.contains(spec)
                    })
            }
            Expression::Identifier(id) => id.name == REQUIRE && self.symbol(id).is_none(),
            Expression::StaticMemberExpression(member) => {
                member.property.name == REQUIRE && self.is_module(&member.object)
            }
            _ => false,
        }
    }

    fn call_fits(&mut self, call: &CallExpression<'_>) -> Option<bool> {
        if call.optional {
            return None;
        }
        if self.mode == Mode::Factory && self.load == Some(call.span) {
            return Some(self.loading(call));
        }
        if self.mode == Mode::Factory {
            if let Some(site) = self.root_site(&call.callee) {
                self.slot = true;
                return Some(self.internal(call, site));
            }
        }
        let Expression::Identifier(callee) = &call.callee else {
            return None;
        };
        let site = self.site(callee);
        match self.symbol(callee) {
            Some(symbol) if Some(symbol) == self.require_param => {
                Some(self.registry_require(call, site))
            }
            Some(symbol) if Some(symbol) == self.require => {
                Some(matches!(self.mode, Mode::Factory | Mode::Entry) && self.internal(call, site))
            }
            None if callee.name == REQUIRE => {
                Some(self.mode != Mode::Helper && single_string(call).is_some_and(bare))
            }
            _ => None,
        }
    }

    fn typeof_allowed(&self, argument: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = argument else {
            return false;
        };
        if self.mode == Mode::Factory && self.is_module(argument) {
            return true;
        }
        self.mode != Mode::Helper && id.name == REQUIRE && self.symbol(id).is_none()
    }
}

impl<'a> Policy<'a> for Rules<'_, '_> {
    fn done(&self) -> bool {
        !self.ok
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        self.ok &= match self.symbol(it) {
            Some(symbol) => self.resolved(symbol, self.is_write(it)),
            None => self.free(it.name.as_str(), frame),
        };
    }

    fn binding(&mut self, it: &BindingIdentifier<'a>) {
        let own = it.symbol_id.get();
        let shadows = (it.name == self.names[0] && own != self.require)
            || (it.name == self.names[1] && own != self.register);
        self.ok &= !shadows;
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        if is_direct_eval(self.scoping, &it.callee) {
            self.ok = false;
            return Step::Skip;
        }
        match self.call_fits(it) {
            Some(fits) => {
                self.ok &= fits;
                Step::Skip
            }
            None => Step::Walk,
        }
    }

    fn static_member(&mut self, it: &StaticMemberExpression<'a>) -> Step {
        let exports = self.mode == Mode::Factory
            && it.property.name == "exports"
            && self.is_module(&it.object);
        match exports {
            true => Step::Skip,
            false => Step::Walk,
        }
    }

    fn unary(&mut self, it: &UnaryExpression<'a>) -> Step {
        match it.operator == UnaryOperator::Typeof && self.typeof_allowed(&it.argument) {
            true => Step::Skip,
            false => Step::Walk,
        }
    }

    fn this(&mut self, frame: Frame) {
        let lexical = self.mode == Mode::Helper || self.arrow;
        self.ok &= !lexical || frame.receiver();
    }

    fn import_meta(&mut self) {
        self.ok = false;
    }

    fn new_target(&mut self, frame: Frame) {
        self.ok &= frame.receiver();
    }

    fn returns(&mut self, frame: Frame) {
        self.ok &= self.mode != Mode::Factory || frame.deferred();
    }

    fn with(&mut self) {
        self.ok = false;
    }
}

fn param_symbols(function: DefFunction<'_, '_>) -> Vec<Option<SymbolId>> {
    function
        .params
        .items
        .iter()
        .map(|item| match &item.pattern {
            BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn canonical(function: DefFunction<'_, '_>, names: &[&str]) -> bool {
    let params = param_names(function.params);
    let mut seen = FxHashSet::default();
    params.iter().enumerate().all(|(index, name)| {
        let placed = names.iter().position(|canon| canon == name);
        seen.insert(*name) && placed.is_none_or(|at| at == index)
    })
}

fn declared_once(scoping: &Scoping, symbols: impl IntoIterator<Item = Option<SymbolId>>) -> bool {
    symbols
        .into_iter()
        .all(|symbol| symbol.is_some_and(|s| scoping.symbol_redeclarations(s).is_empty()))
}

fn factory_span(function: DefFunction<'_, '_>) -> Span {
    Span::new(function.params.span.start, function.body.span.end)
}

struct Context<'s, 'l> {
    scoping: &'s Scoping,
    ids: FxHashSet<&'l str>,
    view: Option<&'l ParcelView>,
    global: &'l str,
    hoisted: Option<(SymbolId, SymbolId, [&'l str; 2])>,
    helpers: FxHashMap<SymbolId, usize>,
    moved: FxHashSet<SymbolId>,
    builtins: bool,
    legacy: bool,
    slot: Cell<bool>,
}

impl<'s, 'l> Context<'s, 'l> {
    fn rules(&self, mode: Mode) -> Rules<'s, '_> {
        let (require, register, names) = match self.hoisted {
            Some((require, register, names)) => (Some(require), Some(register), names),
            None => (None, None, ["", ""]),
        };
        Rules {
            scoping: self.scoping,
            ids: &self.ids,
            global: self.global,
            deps: &[],
            inside: Span::default(),
            load: None,
            target: None,
            require_param: None,
            module_param: None,
            require,
            register,
            names,
            helpers: &self.helpers,
            moved: &self.moved,
            builtins: self.builtins,
            slot: false,
            mode,
            arrow: false,
            legacy: self.legacy,
            ok: true,
        }
    }

    fn factory(&self, def: &Def<'_, '_>, names: &[&str]) -> bool {
        let symbols = param_symbols(def.function);
        if !canonical(def.function, names) || !declared_once(self.scoping, symbols.iter().copied())
        {
            return false;
        }
        let Some(body) = def.body() else {
            return false;
        };
        let offset = usize::from(self.hoisted.is_some());
        let mut rules = self.rules(Mode::Factory);
        rules.deps = &def.deps;
        rules.inside = factory_span(def.function);
        rules.arrow = def.function.arrow;
        (rules.load, rules.target) = self.load(def, body).unzip();
        rules.require_param = if offset == 0 {
            symbols.first().copied().flatten()
        } else {
            None
        };
        rules.module_param = symbols.get(1 - offset).copied().flatten();
        Scan::new(&mut rules).visit_function_body(body);
        self.slot.set(self.slot.get() || rules.slot);
        rules.ok && (offset == 1 || self.global_unshadowed(def, symbols.len()))
    }

    fn load(&self, def: &Def<'_, '_>, body: &FunctionBody<'_>) -> Option<(Span, Span)> {
        let view = self.view?;
        let require = self.hoisted.map(|(_, _, names)| names[0]);
        let module = module_name(def, self.hoisted.is_some());
        let plain = self.legacy.then(|| require_name(def));
        let found = loader(&body.statements, require, module, plain)?;
        view.loadable
            .contains(found.id)
            .then_some((found.load, found.target))
    }

    fn global_unshadowed(&self, def: &Def<'_, '_>, params: usize) -> bool {
        if params < REGISTRY_PARAMS {
            return true;
        }
        let unread = param_symbols(def.function)
            .get(REGISTRY_PARAMS - 1)
            .copied()
            .flatten()
            .is_some_and(|symbol| self.scoping.get_resolved_reference_ids(symbol).is_empty());
        if unread {
            return true;
        }
        let Some(scope) = def.function.scope.get() else {
            return false;
        };
        let inside = factory_span(def.function);
        self.scoping
            .find_binding(scope, Ident::from("globalThis"))
            .is_none_or(|symbol| {
                let span = self.scoping.symbol_span(symbol);
                !(inside.start <= span.start && span.end <= inside.end)
            })
    }
}

fn ids<'l>(defs: &'l [Def<'_, '_>], view: Option<&'l ParcelView>) -> Option<FxHashSet<&'l str>> {
    let ready = view.into_iter().flat_map(|view| view.ready.iter());
    let ids: FxHashSet<&str> = defs
        .iter()
        .map(|def| def.id.as_str())
        .chain(ready.map(String::as_str))
        .collect();
    (!ids.iter().any(|id| INHERITED.contains(id))).then_some(ids)
}

fn own_ids<'l>(defs: &'l [Def<'_, '_>]) -> FxHashSet<&'l str> {
    defs.iter().map(|def| def.id.as_str()).collect()
}

fn registry(scoping: &Scoping, layout: &Registry<'_, '_>, view: Option<&ParcelView>) -> bool {
    let Some(ids) = ids(&layout.defs, view) else {
        return false;
    };
    let own = own_ids(&layout.defs);
    let entries = layout.entries.iter().all(|id| own.contains(id.as_str()))
        && layout
            .main
            .as_ref()
            .is_none_or(|id| own.contains(id.as_str()));
    let synthetic = layout.entries.len() != 1 || layout.main.as_ref() != layout.entries.first();
    if !entries || (synthetic && ids.contains(ENTRY_ID)) {
        return false;
    }
    let context = Context {
        scoping,
        ids,
        view,
        global: &layout.global,
        hoisted: None,
        helpers: FxHashMap::default(),
        moved: FxHashSet::default(),
        builtins: layout.externals,
        legacy: layout.legacy,
        slot: Cell::new(false),
    };
    let factories = layout
        .defs
        .iter()
        .all(|def| context.factory(def, &REGISTRY_NAMES));
    factories && !(context.slot.get() && slot_reached(scoping, layout))
}

fn slot_reached(scoping: &Scoping, layout: &Registry<'_, '_>) -> bool {
    layout.defs.iter().any(|def| {
        let global = param_symbols(def.function)
            .get(REGISTRY_PARAMS - 1)
            .copied()
            .flatten();
        def.body()
            .is_none_or(|body| reaches_slot(scoping, body, &layout.global, global))
    })
}

fn entry_statement<'a>(
    scan: &mut Scan<'_, Rules<'_, '_>>,
    statement: &Statement<'a>,
    layout: &Hoisted<'_, 'a>,
    index: usize,
) {
    let registers: Vec<Option<usize>> = layout
        .defs_at(index)
        .iter()
        .map(|def| def.element)
        .collect();
    if registers.is_empty() {
        scan.visit_statement(statement);
        return;
    }
    let Some(Expression::SequenceExpression(sequence)) = expression_statement(statement) else {
        return;
    };
    for (at, expr) in sequence.expressions.iter().enumerate() {
        if registers.binary_search(&Some(at)).is_err() {
            scan.visit_expression(expr);
        }
    }
}

fn candidate_symbols(
    top: &[Statement<'_>],
    candidates: &FxHashMap<String, usize>,
) -> FxHashMap<SymbolId, usize> {
    candidates
        .values()
        .flat_map(|&index| {
            helper_symbols(top, &[index])
                .into_iter()
                .map(move |s| (s, index))
        })
        .collect()
}

fn hoisted(
    scoping: &Scoping,
    top: &[Statement<'_>],
    layout: &Hoisted<'_, '_>,
    view: Option<&ParcelView>,
) -> bool {
    let (Some(require), Some(register)) = (
        layout.require.symbol_id.get(),
        layout.register.symbol_id.get(),
    ) else {
        return false;
    };
    let Some(ids) = ids(&layout.defs, view) else {
        return false;
    };
    if !declared_once(scoping, [Some(require), Some(register)]) {
        return false;
    }
    let names = [layout.require.name.as_str(), layout.register.name.as_str()];
    let candidates = candidates(top, layout);
    let helpers = candidate_symbols(top, &candidates);
    let mut context = Context {
        scoping,
        ids,
        view,
        global: &layout.global,
        hoisted: Some((require, register, names)),
        helpers,
        moved: FxHashSet::default(),
        builtins: true,
        legacy: false,
        slot: Cell::new(false),
    };
    if !layout
        .defs
        .iter()
        .all(|def| context.factory(def, &HOISTED_NAMES))
    {
        return false;
    }
    let bodies = layout
        .defs
        .iter()
        .filter_map(Def::body)
        .flat_map(|body| body.statements.iter());
    let moved = moved_helpers(top, &candidates, bodies);
    context.moved = helper_symbols(top, &moved);
    if !declared_once(scoping, context.moved.iter().map(|&s| Some(s)))
        || first_unclosed_helper(scoping, top, &moved).is_some()
    {
        return false;
    }
    entry_sound(&context, top, layout, &moved)
}

fn entry_sound(
    context: &Context<'_, '_>,
    top: &[Statement<'_>],
    layout: &Hoisted<'_, '_>,
    moved: &[usize],
) -> bool {
    top.iter().enumerate().all(|(index, statement)| {
        let mode = if moved.binary_search(&index).is_ok() {
            Mode::Helper
        } else if layout.prelude.contains(&index) {
            Mode::Prelude
        } else {
            Mode::Entry
        };
        let mut rules = context.rules(mode);
        entry_statement(&mut Scan::new(&mut rules), statement, layout, index);
        rules.ok
    })
}

pub fn verify(source: &Program<'_>, layout: &Layout<'_, '_>, view: Option<&ParcelView>) -> bool {
    let scoping = build_scoping(source).scoping;
    match layout {
        Layout::Registry(registry_layout) => registry(&scoping, registry_layout, view),
        Layout::Hoisted(hoisted_layout) => hoisted(&scoping, &source.body, hoisted_layout, view),
    }
}
