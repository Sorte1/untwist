use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::baseline::Baseline;
use super::collect::{DeclKind, Facts};
use super::compat::{Compat, Verdict};

const WELL_KNOWN: &str = "internals/well-known-symbol";
const GET_BUILT_IN: &str = "internals/get-built-in";
const HAS_OWN: &str = "internals/has-own-property";
const UNCURRY: &str = "internals/function-uncurry-this";
const IS_CALLABLE: &str = "internals/is-callable";

const SYMBOLS: [(&str, &str); 13] = [
    ("asyncIterator", "async-iterator"),
    ("hasInstance", "has-instance"),
    ("isConcatSpreadable", "is-concat-spreadable"),
    ("iterator", "iterator"),
    ("match", "match"),
    ("matchAll", "match-all"),
    ("replace", "replace"),
    ("search", "search"),
    ("species", "species"),
    ("split", "split"),
    ("toPrimitive", "to-primitive"),
    ("toStringTag", "to-string-tag"),
    ("unscopables", "unscopables"),
];

const CALLABLE_ALWAYS: [&str; 15] = [
    "Object",
    "Function",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Date",
    "RegExp",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "EvalError",
    "URIError",
];

const OBJECTS_ALWAYS: [&str; 2] = ["Math", "JSON"];

const CALLABLE_WITH_MODULE: [(&str, &str); 11] = [
    ("Symbol", "es.symbol"),
    ("Promise", "es.promise"),
    ("Map", "es.map"),
    ("Set", "es.set"),
    ("WeakMap", "es.weak-map"),
    ("WeakSet", "es.weak-set"),
    ("ArrayBuffer", "es.array-buffer.constructor"),
    ("DataView", "es.data-view"),
    ("URL", "web.url"),
    ("URLSearchParams", "web.url-search-params"),
    ("AggregateError", "es.aggregate-error"),
];

const OBJECTS_WITH_MODULE: [(&str, &str); 1] = [("Reflect", "es.reflect.own-keys")];

const METHODS: [(&str, &str, &[&str]); 4] = [
    (
        "String",
        "string",
        &[
            "charAt",
            "charCodeAt",
            "codePointAt",
            "concat",
            "endsWith",
            "includes",
            "indexOf",
            "lastIndexOf",
            "localeCompare",
            "match",
            "normalize",
            "repeat",
            "replace",
            "search",
            "slice",
            "split",
            "startsWith",
            "substr",
            "substring",
            "toLowerCase",
            "toUpperCase",
            "toLocaleLowerCase",
            "toLocaleUpperCase",
            "toString",
            "trim",
            "valueOf",
        ],
    ),
    (
        "Array",
        "array",
        &[
            "concat",
            "copyWithin",
            "entries",
            "every",
            "fill",
            "filter",
            "find",
            "findIndex",
            "forEach",
            "indexOf",
            "join",
            "keys",
            "lastIndexOf",
            "map",
            "pop",
            "push",
            "reduce",
            "reduceRight",
            "reverse",
            "shift",
            "slice",
            "some",
            "sort",
            "splice",
            "toString",
            "unshift",
            "values",
        ],
    ),
    (
        "Object",
        "object",
        &[
            "hasOwnProperty",
            "isPrototypeOf",
            "propertyIsEnumerable",
            "toString",
            "valueOf",
        ],
    ),
    (
        "Number",
        "number",
        &[
            "toExponential",
            "toFixed",
            "toPrecision",
            "toString",
            "valueOf",
        ],
    ),
];

const SERVER_ENGINES: [&str; 5] = ["node", "deno", "bun", "hermes", "react-native"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Site {
    Symbol(String),
    Global(String),
    Member {
        object: String,
        property: String,
        unbound: bool,
    },
    HasOwn,
    Method {
        object: String,
        method: String,
    },
    Typeof,
}

impl Site {
    pub fn kind(&self) -> &'static str {
        match self {
            Site::Symbol(_) => "wellKnownSymbol(name) -> Symbol.name",
            Site::Global(_) | Site::Member { .. } => "getBuiltIn(...) -> the global",
            Site::HasOwn => "hasOwn(it, key) -> Object.hasOwn(it, key)",
            Site::Method { .. } => "uncurryThis(method)(that, ...) -> Proto.method.call(that, ...)",
            Site::Typeof => "isCallable(it) -> typeof it == \"function\"",
        }
    }
}

pub struct Inputs<'i> {
    pub facts: &'i Facts,
    pub scoping: &'i Scoping,
    pub helpers: &'i FxHashMap<SymbolId, (Vec<String>, (String, String))>,
    pub compat: &'i Compat,
    pub baseline: &'i Baseline,
    pub pinned: &'i FxHashSet<SymbolId>,
}

type Versions = (String, String);

enum Alias {
    Module(&'static str, Versions),
    Uncurried { object: String, method: String },
}

pub fn plan(program: &Program<'_>, inputs: &Inputs<'_>) -> FxHashMap<Span, Site> {
    let written: FxHashSet<SymbolId> = inputs
        .facts
        .refs
        .iter()
        .filter(|site| site.write)
        .map(|site| site.symbol)
        .collect();
    let mut planner = Planner {
        inputs,
        scopes: vec![inputs.scoping.root_scope_id()],
        written,
        aliases: FxHashMap::default(),
        sites: FxHashMap::default(),
        callees: FxHashSet::default(),
        deleted: FxHashSet::default(),
        dominating: FxHashMap::default(),
        budget: inputs
            .facts
            .refs
            .len()
            .saturating_mul(4)
            .saturating_add(PREFIX_BUDGET),
    };
    planner.visit_program(program);
    let callees = std::mem::take(&mut planner.callees);
    let mut sites = planner.sites;
    for (span, site) in &mut sites {
        if let Site::Member { unbound, .. } = site {
            *unbound = callees.contains(span);
        }
    }
    sites
}

struct Planner<'p, 'i> {
    inputs: &'p Inputs<'i>,
    scopes: Vec<oxc_syntax::scope::ScopeId>,
    written: FxHashSet<SymbolId>,
    aliases: FxHashMap<SymbolId, (Alias, Set)>,
    sites: FxHashMap<Span, Site>,
    callees: FxHashSet<Span>,
    deleted: FxHashSet<Span>,
    dominating: FxHashMap<Span, Set>,
    budget: usize,
}

const PREFIX_BUDGET: usize = 100_000;

fn alias_candidate(statement: &Statement<'_>) -> bool {
    let Statement::VariableDeclaration(declaration) = statement else {
        return false;
    };
    declaration.declarations.iter().any(|declarator| {
        matches!(
            declarator.init.as_ref().map(Expression::without_parentheses),
            Some(Expression::CallExpression(call)) if matches!(call.callee.without_parentheses(), Expression::Identifier(_))
        )
    })
}

#[derive(Clone, Copy)]
struct Set {
    scope: ScopeId,
    end: u32,
}

fn string_argument<'a>(argument: &'a Argument<'_>) -> Option<&'a str> {
    match argument.as_expression()?.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn identifier_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return None;
    };
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

fn helper_module(modules: &[String]) -> Option<&'static str> {
    let [module] = modules else {
        return None;
    };
    [WELL_KNOWN, GET_BUILT_IN, HAS_OWN, UNCURRY, IS_CALLABLE]
        .into_iter()
        .find(|known| *known == module)
}

impl Planner<'_, '_> {
    fn verdict(&self, module: &str, versions: &Versions) -> Verdict {
        let range = (versions.0.as_str(), versions.1.as_str());
        self.inputs
            .compat
            .verdict(module, range, self.inputs.baseline)
    }

    fn native(&self, module: &str, versions: &Versions) -> bool {
        self.verdict(module, versions) == Verdict::Native
    }

    fn free(&self, name: &str) -> bool {
        let scope = self
            .scopes
            .last()
            .copied()
            .unwrap_or(self.inputs.scoping.root_scope_id());
        self.inputs
            .scoping
            .find_binding(scope, oxc_str::Ident::from(name))
            .is_none()
    }

    fn usable(&self, symbol: SymbolId) -> bool {
        !self.written.contains(&symbol)
            && !self.inputs.pinned.contains(&symbol)
            && self.inputs.scoping.symbol_redeclarations(symbol).is_empty()
    }

    fn var_scope(&self) -> ScopeId {
        let scoping = self.inputs.scoping;
        self.scopes
            .iter()
            .rev()
            .copied()
            .find(|scope| scoping.scope_flags(*scope).is_var())
            .unwrap_or(scoping.root_scope_id())
    }

    fn initialised(&self, set: Set, call: &CallExpression<'_>) -> bool {
        set.scope != self.var_scope() || call.span.start >= set.end
    }

    fn receiver(&self, expr: &Expression<'_>, versions: &Versions) -> Option<(String, String)> {
        let Expression::StaticMemberExpression(member) = expr.without_parentheses() else {
            return None;
        };
        let method = member.property.name.as_str();
        let object = match member.object.without_parentheses() {
            Expression::StringLiteral(_) => "String",
            Expression::ArrayExpression(array) if array.elements.is_empty() => "Array",
            Expression::ObjectExpression(object) if object.properties.is_empty() => "Object",
            Expression::NumericLiteral(_) => "Number",
            Expression::StaticMemberExpression(proto) if proto.property.name == "prototype" => {
                let Expression::Identifier(global) = proto.object.without_parentheses() else {
                    return None;
                };
                if global
                    .reference_id
                    .get()
                    .and_then(|id| self.inputs.scoping.get_reference(id).symbol_id())
                    .is_some()
                {
                    return None;
                }
                global.name.as_str()
            }
            _ => return None,
        };
        let (_, short, methods) = METHODS.iter().find(|(name, _, _)| *name == object)?;
        if !methods.contains(&method) || !self.free(object) {
            return None;
        }
        let module = format!("es.{short}.{}", kebab(method));
        let replaced = self.verdict(&module, versions);
        if !matches!(replaced, Verdict::Native | Verdict::Unlisted) {
            return None;
        }
        Some((object.to_string(), method.to_string()))
    }

    fn note_alias(&mut self, declarator: &VariableDeclarator<'_>) {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        if !self.usable(symbol) {
            return;
        }
        let Some(set) = self.dominating.get(&declarator.span).copied() else {
            return;
        };
        let Some(decl) = self.inputs.facts.decls.get(&symbol) else {
            return;
        };
        if let DeclKind::Var {
            call_of: Some(callee),
            ..
        } = decl.kind
        {
            if let Some((modules, versions)) = self.inputs.helpers.get(&callee) {
                if let Some(module) = helper_module(modules) {
                    self.aliases
                        .insert(symbol, (Alias::Module(module, versions.clone()), set));
                }
            }
            return;
        }
        let Some(Expression::CallExpression(call)) = declarator
            .init
            .as_ref()
            .map(Expression::without_parentheses)
        else {
            return;
        };
        let versions = match identifier_symbol(&call.callee, self.inputs.scoping)
            .and_then(|callee| self.aliases.get(&callee))
        {
            Some((Alias::Module(UNCURRY, versions), _)) => versions.clone(),
            _ => return,
        };
        let [argument] = call.arguments.as_slice() else {
            return;
        };
        if call.optional {
            return;
        }
        if let Some((object, method)) = argument
            .as_expression()
            .and_then(|expr| self.receiver(expr, &versions))
        {
            self.aliases
                .insert(symbol, (Alias::Uncurried { object, method }, set));
        }
    }

    fn site(&self, call: &CallExpression<'_>) -> Option<Site> {
        if call.optional || self.deleted.contains(&call.span) {
            return None;
        }
        let symbol = identifier_symbol(&call.callee, self.inputs.scoping)?;
        let (alias, set) = self.aliases.get(&symbol)?;
        if !self.initialised(*set, call) {
            return None;
        }
        match alias {
            Alias::Module(WELL_KNOWN, versions) => self.well_known(call, versions),
            Alias::Module(GET_BUILT_IN, versions) => self.built_in(call, versions),
            Alias::Module(HAS_OWN, versions) => {
                let native = self.native("es.object.has-own", versions);
                (native && self.free("Object")).then_some(Site::HasOwn)
            }
            Alias::Module(IS_CALLABLE, _) => self.is_callable(call),
            Alias::Module(..) => None,
            Alias::Uncurried { object, method } => self.free(object).then(|| Site::Method {
                object: object.clone(),
                method: method.clone(),
            }),
        }
    }

    fn well_known(&self, call: &CallExpression<'_>, versions: &Versions) -> Option<Site> {
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        let name = string_argument(argument)?;
        let (_, module) = SYMBOLS.iter().find(|(symbol, _)| *symbol == name)?;
        let native = self.native("es.symbol", versions)
            && self.native(&format!("es.symbol.{module}"), versions);
        (native && self.free("Symbol")).then(|| Site::Symbol(name.to_string()))
    }

    fn exists(&self, name: &str, callable: bool, versions: &Versions) -> bool {
        if CALLABLE_ALWAYS.contains(&name) {
            return true;
        }
        if !callable && OBJECTS_ALWAYS.contains(&name) {
            return true;
        }
        let with_module = CALLABLE_WITH_MODULE
            .iter()
            .chain(if callable {
                [].iter()
            } else {
                OBJECTS_WITH_MODULE.iter()
            })
            .find(|(global, _)| *global == name);
        with_module.is_some_and(|(_, module)| self.native(module, versions))
    }

    fn built_in(&self, call: &CallExpression<'_>, versions: &Versions) -> Option<Site> {
        match call.arguments.as_slice() {
            [namespace] => {
                let name = string_argument(namespace)?;
                (self.exists(name, true, versions) && self.free(name))
                    .then(|| Site::Global(name.to_string()))
            }
            [namespace, method] => {
                let (name, property) = (string_argument(namespace)?, string_argument(method)?);
                let identifier = !property.is_empty()
                    && property
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
                    && !property.starts_with(|c: char| c.is_ascii_digit());
                (identifier && self.exists(name, false, versions) && self.free(name)).then(|| {
                    Site::Member {
                        object: name.to_string(),
                        property: property.to_string(),
                        unbound: false,
                    }
                })
            }
            _ => None,
        }
    }

    fn is_callable(&self, call: &CallExpression<'_>) -> Option<Site> {
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        let argument = argument.as_expression()?;
        if let Expression::Identifier(id) = argument.without_parentheses() {
            let unresolved = id
                .reference_id
                .get()
                .and_then(|reference| self.inputs.scoping.get_reference(reference).symbol_id())
                .is_none();
            if unresolved {
                return None;
            }
        }
        let servers = self
            .inputs
            .baseline
            .engines()
            .iter()
            .all(|(engine, _)| SERVER_ENGINES.contains(&engine.as_str()));
        servers.then_some(Site::Typeof)
    }
}

fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

impl<'a> Visit<'a> for Planner<'_, '_> {
    fn enter_scope(
        &mut self,
        _flags: oxc_syntax::scope::ScopeFlags,
        scope_id: &std::cell::Cell<Option<oxc_syntax::scope::ScopeId>>,
    ) {
        if let Some(scope) = scope_id.get() {
            self.scopes.push(scope);
        }
    }

    fn leave_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        let scope = self.var_scope();
        let mut prefix = super::inert::Prefix::new(self.inputs.scoping, it.span, self.budget);
        let last = it.statements.iter().rposition(alias_candidate);
        for statement in it.statements.iter().take(last.map_or(0, |last| last + 1)) {
            let Statement::VariableDeclaration(declaration) = statement else {
                prefix.statement(statement);
                continue;
            };
            for declarator in &declaration.declarations {
                let symbol = match &declarator.id {
                    BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                    _ => None,
                };
                if symbol.is_some_and(|symbol| prefix.set_before(symbol)) {
                    let end = declarator.span.end;
                    self.dominating.insert(declarator.span, Set { scope, end });
                }
                prefix.declarator(declarator);
            }
        }
        self.budget = prefix.budget;
        walk::walk_function_body(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == oxc_syntax::operator::UnaryOperator::Delete {
            if let Expression::CallExpression(call) = it.argument.without_parentheses() {
                self.deleted.insert(call.span);
            }
        }
        walk::walk_unary_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.note_alias(it);
        walk::walk_variable_declarator(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::CallExpression(inner) = it.callee.without_parentheses() {
            self.callees.insert(inner.span);
        }
        if let Some(site) = self.site(it) {
            self.sites.insert(it.span, site);
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if let Expression::CallExpression(inner) = it.tag.without_parentheses() {
            self.callees.insert(inner.span);
        }
        walk::walk_tagged_template_expression(self, it);
    }
}
