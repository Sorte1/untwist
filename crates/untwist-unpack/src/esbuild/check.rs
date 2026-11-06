use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_str::Ident;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::esbuild::layout::FactoryDecl;
use crate::scan::{is_direct_eval, Frame, Policy, Scan, Step};

const HOST_NAMES: [&str; 5] = ["require", "exports", "module", "__filename", "__dirname"];

pub struct Proof<'s> {
    pub scoping: &'s Scoping,
    pub requires: FxHashMap<SymbolId, Span>,
    pub shim: Option<SymbolId>,
    pub hosts: FxHashSet<SymbolId>,
    pub host_params: FxHashSet<SymbolId>,
    pub shim_init: Option<Span>,
    pub helpers: FxHashSet<SymbolId>,
}

enum Callee {
    Require(Span),
    Shim,
    Host,
    Other,
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

pub fn bare_specifier(call: &CallExpression<'_>) -> bool {
    match &call.arguments[..] {
        [Argument::StringLiteral(text)] => {
            !text.value.has_lone_surrogate()
                && !text.value.is_empty()
                && !text.value.starts_with('.')
                && !text.value.starts_with('/')
        }
        _ => false,
    }
}

impl Proof<'_> {
    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
    }

    fn is_write(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_write())
    }

    fn host_name<'n>(&self, id: &'n IdentifierReference<'_>) -> Option<&'n str> {
        let name = id.name.as_str();
        match self.symbol(id) {
            None => HOST_NAMES.contains(&name).then_some(name),
            Some(symbol) if self.hosts.contains(&symbol) || self.host_params.contains(&symbol) => {
                Some(name)
            }
            Some(_) => None,
        }
    }

    fn callee(&self, callee: &IdentifierReference<'_>) -> Callee {
        match self.symbol(callee) {
            Some(symbol) => match self.requires.get(&symbol) {
                Some(span) => Callee::Require(*span),
                None if self.shim == Some(symbol) => Callee::Shim,
                None if self.hosts.contains(&symbol) => Callee::Host,
                None => Callee::Other,
            },
            None => match callee.name.as_str() {
                "require" => Callee::Host,
                _ => Callee::Other,
            },
        }
    }

    pub fn requires_host_at(&self, scope: ScopeId) -> bool {
        self.scoping
            .find_binding(scope, Ident::from("require"))
            .is_none_or(|symbol| self.hosts.contains(&symbol))
    }

    fn site(&self, callee: &IdentifierReference<'_>) -> bool {
        callee.reference_id.get().is_some_and(|reference| {
            self.requires_host_at(self.scoping.get_reference(reference).scope_id())
        })
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Factory {
        inside: Span,
        module: Option<SymbolId>,
    },
    Entry,
}

struct Check<'p, 's> {
    proof: &'p Proof<'s>,
    mode: Mode,
    ok: bool,
}

impl Check<'_, '_> {
    fn is_module(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr else {
            return false;
        };
        match self.mode {
            Mode::Factory { module, .. } => module.is_some() && self.proof.symbol(id) == module,
            Mode::Entry => self.proof.host_name(id) == Some("module"),
        }
    }

    fn is_typeof_host(&self, expr: &Expression<'_>) -> bool {
        let (Mode::Entry, Expression::Identifier(id)) = (self.mode, expr) else {
            return false;
        };
        self.proof.host_name(id).is_some()
    }

    fn factory_reference(
        &self,
        (id, frame): (&IdentifierReference<'_>, Frame),
        inside: Span,
        module: Option<SymbolId>,
    ) -> bool {
        let Some(symbol) = self.proof.symbol(id) else {
            let name = id.name.as_str();
            return !HOST_NAMES.contains(&name) && !(name == "arguments" && !frame.receiver());
        };
        if contains(inside, self.proof.scoping.symbol_span(symbol)) {
            return Some(symbol) != module;
        }
        self.proof.helpers.contains(&symbol) && !self.proof.is_write(id)
    }

    fn entry_reference(&self, id: &IdentifierReference<'_>) -> bool {
        if let Some(name) = self.proof.host_name(id) {
            let in_shim = self
                .proof
                .shim_init
                .is_some_and(|init| contains(init, id.span));
            return name == "require" && in_shim;
        }
        let Some(symbol) = self.proof.symbol(id) else {
            return true;
        };
        if self.proof.shim == Some(symbol) {
            return false;
        }
        let required = self.proof.requires.contains_key(&symbol);
        let written_helper = self.proof.helpers.contains(&symbol) && self.proof.is_write(id);
        !required && !written_helper
    }

    fn call_fits(
        &self,
        (call, frame): (&CallExpression<'_>, Frame),
        callee: &IdentifierReference<'_>,
    ) -> Option<bool> {
        let factory = matches!(self.mode, Mode::Factory { .. });
        let fits = match self.proof.callee(callee) {
            Callee::Require(declared) => {
                let deferred = frame.functions + frame.arrows > 0;
                let after = factory || deferred || call.span.start >= declared.end;
                call.arguments.is_empty() && after && self.proof.site(callee)
            }
            Callee::Shim => bare_specifier(call) && self.proof.site(callee),
            Callee::Host => bare_specifier(call),
            Callee::Other => return None,
        };
        Some(fits)
    }
}

impl<'a> Policy<'a> for Check<'_, '_> {
    fn done(&self) -> bool {
        !self.ok
    }

    fn call(&mut self, frame: Frame, it: &CallExpression<'a>) -> Step {
        if is_direct_eval(self.proof.scoping, &it.callee) {
            self.ok = false;
            return Step::Skip;
        }
        if let (Expression::Identifier(callee), false) = (&it.callee, it.optional) {
            if let Some(fits) = self.call_fits((it, frame), callee) {
                self.ok &= fits;
                return Step::Skip;
            }
        }
        Step::Walk
    }

    fn static_member(&mut self, it: &StaticMemberExpression<'a>) -> Step {
        match it.property.name == "exports" && self.is_module(&it.object) {
            true => Step::Skip,
            false => Step::Walk,
        }
    }

    fn unary(&mut self, it: &UnaryExpression<'a>) -> Step {
        let typeof_module = it.operator == oxc_syntax::operator::UnaryOperator::Typeof
            && (self.is_module(&it.argument) || self.is_typeof_host(&it.argument));
        match typeof_module {
            true => Step::Skip,
            false => Step::Walk,
        }
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        self.ok &= match self.mode {
            Mode::Factory { inside, module } => self.factory_reference((it, frame), inside, module),
            Mode::Entry => self.entry_reference(it),
        };
    }

    fn this(&mut self, frame: Frame) {
        if matches!(self.mode, Mode::Factory { .. }) {
            self.ok &= frame.receiver();
        }
    }
}

fn param_symbol(params: &FormalParameters<'_>, index: usize) -> Option<SymbolId> {
    match &params.items.get(index)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

pub fn factory_closed(proof: &Proof<'_>, factory: &FactoryDecl<'_, '_>) -> bool {
    let function = factory.function;
    let mut check = Check {
        proof,
        mode: Mode::Factory {
            inside: Span::new(function.params.span.start, function.body.span.end),
            module: param_symbol(function.params, 1),
        },
        ok: true,
    };
    Scan::new(&mut check).visit_function_body(function.body);
    check.ok
}

pub fn entry_sound<'a>(
    proof: &Proof<'_>,
    statements: impl Iterator<Item = &'a Statement<'a>>,
    declarators: impl Iterator<Item = &'a VariableDeclarator<'a>>,
) -> bool {
    let mut check = Check {
        proof,
        mode: Mode::Entry,
        ok: true,
    };
    let mut scan = Scan::new(&mut check);
    for statement in statements {
        scan.visit_statement(statement);
    }
    for declarator in declarators {
        scan.visit_variable_declarator(declarator);
    }
    check.ok
}
