use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::semantic::{closed_eval_names, is_closed_eval_at};

use crate::browserify::requests::leaves_the_bundle;
use crate::matcher::Factory;
use crate::scan::{is_direct_eval, is_free, Frame, Policy, Scan, Step, Suspend};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Captured {
    Binding(String),
    Parameter(String),
    Host(String),
    HostUse(String),
    Arguments,
    Eval,
    With,
    Outer(&'static str),
    Context(&'static str),
}

impl Captured {
    pub fn describe(&self) -> String {
        match self {
            Self::Binding(name) => format!("it reads {name}, a binding of the bundle's scope"),
            Self::Parameter(name) => format!("it reads {name}, a factory parameter no module maps"),
            Self::Host(name) => format!("it reads {name}, which every module has for itself"),
            Self::HostUse(what) => {
                format!("it uses the host's require ({what}), which a written file answers for its own path")
            }
            Self::Arguments => "it reads the factory's own arguments".to_string(),
            Self::Eval => "it calls eval directly".to_string(),
            Self::With => "it holds a with statement".to_string(),
            Self::Outer(what) => {
                format!("it uses {what}, which an arrow factory takes from the bundle's scope")
            }
            Self::Context(what) => {
                format!("it uses {what} of the bundle, which the runtime module does not share")
            }
        }
    }
}

pub struct Allowed<'s> {
    pub mapped_params: usize,
    pub helpers: &'s FxHashSet<SymbolId>,
}

const GLOBAL_FALLBACK: [&str; 3] = ["globalThis", "global", "window"];

fn typeof_guard<'k, 'a>(test: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return None;
    };
    let Expression::UnaryExpression(unary) = &binary.left else {
        return None;
    };
    let undefined =
        matches!(&binary.right, Expression::StringLiteral(text) if text.value == "undefined");
    let guard = binary.operator == BinaryOperator::StrictInequality
        && unary.operator == UnaryOperator::Typeof
        && undefined;
    match (&unary.argument, guard) {
        (Expression::Identifier(id), true) => Some(id),
        _ => None,
    }
}

fn global_fallback(scoping: &Scoping, it: &ConditionalExpression<'_>) -> bool {
    let mut level = it;
    for name in GLOBAL_FALLBACK {
        let Some(guarded) = typeof_guard(&level.test) else {
            return false;
        };
        let read = matches!(level.consequent.without_parentheses(),
            Expression::Identifier(id) if id.name == name && is_free(Some(scoping), id));
        if guarded.name != name || !is_free(Some(scoping), guarded) || !read {
            return false;
        }
        match level.alternate.without_parentheses() {
            Expression::ConditionalExpression(next) => level = next,
            Expression::ThisExpression(_) => return name == "window",
            _ => return false,
        }
    }
    false
}

pub const HOST_NAMES: [&str; 5] = ["require", "exports", "module", "__filename", "__dirname"];

const FILE_NAMES: [&str; 2] = ["__filename", "__dirname"];

const KNOWN_PARAMS: usize = 8;

#[derive(Clone, Copy)]
pub struct Region<'k, 'a> {
    inside: Span,
    params: Option<&'k FormalParameters<'a>>,
    symbols: [Option<SymbolId>; KNOWN_PARAMS],
    mapped_params: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    Local,
    Free,
    Param(usize),
    Helper(SymbolId),
    Binding(SymbolId),
    Unmapped(SymbolId),
    Arguments,
}

impl Reach {
    pub fn captured(self, scoping: &Scoping) -> Option<Captured> {
        let name = |symbol| scoping.symbol_name(symbol).to_string();
        match self {
            Self::Binding(symbol) => Some(Captured::Binding(name(symbol))),
            Self::Unmapped(symbol) => Some(Captured::Parameter(name(symbol))),
            Self::Arguments => Some(Captured::Arguments),
            Self::Local | Self::Free | Self::Param(_) | Self::Helper(_) => None,
        }
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

impl<'k, 'a> Region<'k, 'a> {
    pub fn factory(factory: Factory<'k, 'a>, mapped_params: usize) -> Self {
        let mut symbols = [None; KNOWN_PARAMS];
        for (slot, item) in symbols.iter_mut().zip(&factory.params.items) {
            if let BindingPattern::BindingIdentifier(id) = &item.pattern {
                *slot = id.symbol_id.get();
            }
        }
        Self {
            inside: factory.body.span,
            params: Some(factory.params),
            symbols,
            mapped_params,
        }
    }

    pub fn statement(statement: &Statement<'_>) -> Self {
        Self {
            inside: statement.span(),
            params: None,
            symbols: [None; KNOWN_PARAMS],
            mapped_params: 0,
        }
    }

    fn parameter(&self, index: usize, symbol: SymbolId) -> Reach {
        match index < self.mapped_params {
            true => Reach::Param(index),
            false => Reach::Unmapped(symbol),
        }
    }

    fn parameter_index(&self, declared: Span) -> Option<usize> {
        let params = self.params?;
        if !contains(params.span, declared) {
            return None;
        }
        let index = params
            .items
            .iter()
            .position(|item| contains(item.span, declared))
            .unwrap_or(params.items.len());
        Some(index)
    }

    #[inline(always)]
    fn resolved(
        &self,
        scoping: &Scoping,
        symbol: SymbolId,
        helpers: &FxHashSet<SymbolId>,
    ) -> Reach {
        if let Some(index) = self.symbols.iter().position(|s| *s == Some(symbol)) {
            return self.parameter(index, symbol);
        }
        let declared = scoping.symbol_span(symbol);
        if contains(self.inside, declared) {
            return Reach::Local;
        }
        if let Some(index) = self.parameter_index(declared) {
            return self.parameter(index, symbol);
        }
        match helpers.contains(&symbol) {
            true => Reach::Helper(symbol),
            false => Reach::Binding(symbol),
        }
    }

    #[inline(always)]
    pub fn reach(
        &self,
        (scoping, helpers): (&Scoping, &FxHashSet<SymbolId>),
        frame: Frame,
        it: &IdentifierReference<'_>,
    ) -> Reach {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| scoping.get_reference(id).symbol_id());
        match symbol {
            Some(symbol) => self.resolved(scoping, symbol, helpers),
            None if it.name == "arguments" && !frame.receiver() => Reach::Arguments,
            None => Reach::Free,
        }
    }
}

pub struct Check<'s, 'k, 'a> {
    scoping: &'s Scoping,
    region: Region<'k, 'a>,
    helpers: &'s FxHashSet<SymbolId>,
    hosts: bool,
    host_require: bool,
    arrow: bool,
    captured: Option<Captured>,
}

impl<'s, 'k, 'a> Check<'s, 'k, 'a> {
    pub fn factory(scoping: &'s Scoping, factory: Factory<'k, 'a>, allowed: &Allowed<'s>) -> Self {
        Self {
            scoping,
            region: Region::factory(factory, allowed.mapped_params),
            helpers: allowed.helpers,
            hosts: false,
            host_require: false,
            arrow: factory.arrow,
            captured: None,
        }
    }

    fn statement(
        scoping: &'s Scoping,
        statement: &Statement<'_>,
        helpers: &'s FxHashSet<SymbolId>,
        hosts: bool,
    ) -> Self {
        Self {
            scoping,
            region: Region::statement(statement),
            helpers,
            hosts,
            host_require: false,
            arrow: false,
            captured: None,
        }
    }

    pub fn into_captured(self) -> Option<Captured> {
        self.captured
    }

    fn host(&self, reach: Reach, name: &str) -> Option<Captured> {
        let named = FILE_NAMES.contains(&name) || (self.hosts && HOST_NAMES.contains(&name));
        (reach == Reach::Free && named).then(|| Captured::Host(name.to_string()))
    }

    pub fn refuse_every_eval(&mut self, call: &CallExpression<'_>) {
        if self.captured.is_none() && is_direct_eval(self.scoping, &call.callee) {
            self.captured = Some(Captured::Eval);
        }
    }

    fn eval_capture(&self, call: &CallExpression<'_>) -> Option<Captured> {
        if !is_closed_eval_at(self.scoping, call) {
            return Some(Captured::Eval);
        }
        closed_eval_names(call)?
            .into_iter()
            .filter(|name| !(self.host_require && name == SHIELDED_HOST))
            .find(|name| HOST_NAMES.contains(&name.as_str()))
            .map(Captured::Host)
    }

    fn outer(&mut self, frame: Frame, what: &'static str) {
        if self.arrow && !frame.receiver() {
            self.captured.get_or_insert(Captured::Outer(what));
        }
    }

    fn context(&mut self, outer: bool, what: &'static str) {
        if self.hosts && outer {
            self.captured.get_or_insert(Captured::Context(what));
        }
    }
}

impl<'a> Policy<'a> for Check<'_, '_, 'a> {
    fn done(&self) -> bool {
        self.captured.is_some()
    }

    fn reference(&mut self, frame: Frame, it: &IdentifierReference<'a>) {
        if self.captured.is_none() {
            let reach = self.region.reach((self.scoping, self.helpers), frame, it);
            self.captured = reach
                .captured(self.scoping)
                .or_else(|| self.host(reach, it.name.as_str()));
        }
    }

    fn call(&mut self, _frame: Frame, it: &CallExpression<'a>) -> Step {
        if self.captured.is_none() && is_direct_eval(self.scoping, &it.callee) {
            self.captured = self.eval_capture(it);
        }
        Step::Walk
    }

    fn with(&mut self) {
        self.captured.get_or_insert(Captured::With);
    }

    fn this(&mut self, frame: Frame) {
        self.outer(frame, "this");
        self.context(!frame.receiver(), "this");
    }

    fn super_(&mut self, frame: Frame) {
        self.outer(frame, "super");
        self.context(!frame.receiver(), "super");
    }

    fn new_target(&mut self, frame: Frame) {
        self.outer(frame, "new.target");
        self.context(!frame.receiver(), "new.target");
    }

    fn conditional(&mut self, frame: Frame, it: &ConditionalExpression<'a>) -> Step {
        let fallback = self.hosts && !frame.receiver() && global_fallback(self.scoping, it);
        match fallback {
            true => Step::Skip,
            false => Step::Walk,
        }
    }

    fn suspends(&mut self, frame: Frame, what: Suspend) {
        let what = match what {
            Suspend::Await => "await",
            Suspend::Yield => "yield",
        };
        self.context(!frame.deferred(), what);
    }

    fn import_meta(&mut self) {
        self.context(true, "import.meta");
    }

    fn dynamic_import(&mut self) {
        self.context(true, "import()");
    }
}

pub fn captured_by_factory(
    scoping: &Scoping,
    factory: Factory<'_, '_>,
    allowed: &Allowed<'_>,
) -> Option<Captured> {
    captured_with(scoping, factory, allowed, false)
}

fn captured_with(
    scoping: &Scoping,
    factory: Factory<'_, '_>,
    allowed: &Allowed<'_>,
    host_require: bool,
) -> Option<Captured> {
    let mut check = Check {
        host_require,
        ..Check::factory(scoping, factory, allowed)
    };
    let mut scan = Scan::new(&mut check);
    scan.visit_formal_parameters(factory.params);
    scan.visit_function_body(factory.body);
    check.captured
}

pub fn first_open<'k, 'a: 'k>(
    scoping: &Scoping,
    factories: impl IntoIterator<Item = (String, Factory<'k, 'a>)>,
    (mapped_params, host_require): (usize, bool),
) -> Option<(String, Captured)> {
    let helpers = FxHashSet::default();
    let allowed = Allowed {
        mapped_params,
        helpers: &helpers,
    };
    factories.into_iter().find_map(|(id, factory)| {
        captured_with(scoping, factory, &allowed, host_require).map(|captured| (id, captured))
    })
}

pub fn captured_by_statement(
    scoping: &Scoping,
    statement: &Statement<'_>,
    helpers: &FxHashSet<SymbolId>,
) -> Option<Captured> {
    let mut check = Check::statement(scoping, statement, helpers, false);
    Scan::new(&mut check).visit_statement(statement);
    check.captured
}

pub fn captured_by_helper(
    scoping: &Scoping,
    statement: &Statement<'_>,
    helpers: &FxHashSet<SymbolId>,
) -> Option<Captured> {
    let mut check = Check::statement(scoping, statement, helpers, true);
    Scan::new(&mut check).visit_statement(statement);
    check.captured
}

pub fn first_host_read<'k, 'a: 'k>(
    scoping: &Scoping,
    factories: impl IntoIterator<Item = (String, Factory<'k, 'a>)>,
    targets: &[&str],
) -> Option<(String, Captured)> {
    factories.into_iter().find_map(|(id, factory)| {
        let renamed: Vec<&str> = targets
            .iter()
            .enumerate()
            .filter(|(at, target)| {
                let kept = factory.params.items.get(*at).is_some_and(|item| {
                    matches!(&item.pattern, BindingPattern::BindingIdentifier(binding) if binding.name == **target)
                });
                **target != SHIELDED_HOST && !kept
            })
            .map(|(_, target)| *target)
            .collect();
        if renamed.is_empty() {
            return None;
        }
        let mut reads = HostReads {
            scoping,
            names: &renamed,
            found: None,
        };
        reads.visit_function_body(factory.body);
        reads
            .found
            .map(|name| (id, Captured::Host(name.to_string())))
    })
}

const SHIELDED_HOST: &str = "require";

struct HostReads<'s, 'n> {
    scoping: &'s Scoping,
    names: &'n [&'n str],
    found: Option<&'n str>,
}

impl HostReads<'_, '_> {
    fn note(&mut self, name: &str) {
        if self.found.is_none() {
            self.found = self.names.iter().find(|target| **target == name).copied();
        }
    }
}

impl<'a> VisitJs<'a> for HostReads<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if is_free(Some(self.scoping), it) {
            self.note(it.name.as_str());
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_direct_eval(self.scoping, &it.callee) {
            for name in closed_eval_names(it).unwrap_or_default() {
                self.note(&name);
            }
        }
        oxc_ast_visit::walk_js::walk_call_expression(self, it);
    }
}
const NODE_REQUIRE_MEMBERS: [&str; 7] = [
    "resolve",
    "main",
    "cache",
    "extensions",
    "call",
    "apply",
    "bind",
];

struct HostRequire<'s> {
    scoping: &'s Scoping,
    found: Option<String>,
    requests: Vec<String>,
}

impl HostRequire<'_> {
    fn note(&mut self, what: String) {
        self.found.get_or_insert(what);
    }

    fn is_host_require(&self, expr: &Expression<'_>) -> bool {
        is_host_require_name(self.scoping, expr)
    }

    fn judge(&mut self, call: &CallExpression<'_>) {
        let request = match call.arguments.as_slice() {
            [Argument::StringLiteral(literal)] => literal.value.as_str(),
            _ => None,
        };
        let Some(request) = request else {
            return self.note("a request that is not one literal".to_string());
        };
        match call.optional || !leaves_the_bundle(request) {
            true => self.note(format!("require({request:?})")),
            false => self.requests.push(request.to_string()),
        }
    }
}

impl<'a> VisitJs<'a> for HostRequire<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let reads = is_direct_eval(self.scoping, &it.callee)
            && closed_eval_names(it)
                .is_some_and(|names| names.iter().any(|name| name == SHIELDED_HOST));
        if reads {
            match evals_host_require(self.scoping, it) {
                true => self.note("read as a value".to_string()),
                false => self.note("read through eval".to_string()),
            }
            return;
        }
        let callee = it.callee.without_parentheses();
        if !calls_host_require(self.scoping, callee) {
            oxc_ast_visit::walk_js::walk_call_expression(self, it);
            return;
        }
        self.judge(it);
        if let Expression::ConditionalExpression(choice) = callee {
            self.visit_expression(&choice.test);
            for branch in [&choice.consequent, &choice.alternate] {
                if !self.is_host_require(branch.without_parentheses()) {
                    self.visit_expression(branch);
                }
            }
        }
        for argument in &it.arguments {
            self.visit_argument(argument);
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Typeof {
            let (prefix, last) = match it.argument.without_parentheses() {
                Expression::SequenceExpression(sequence) => match sequence.expressions.split_last()
                {
                    Some((last, prefix)) => (prefix, last),
                    None => (&[][..], &it.argument),
                },
                argument => (&[][..], argument),
            };
            if self.is_host_require(last.without_parentheses()) {
                for expression in prefix {
                    self.visit_expression(expression);
                }
                return;
            }
        }
        let deleted = it.operator == UnaryOperator::Delete
            && it
                .argument
                .without_parentheses()
                .as_member_expression()
                .is_some_and(|member| self.is_host_require(member.object()));
        if deleted {
            self.note("a deleted member".to_string());
        }
        oxc_ast_visit::walk_js::walk_unary_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        let written = it
            .as_member_expression()
            .is_some_and(|member| self.is_host_require(member.object()));
        if written {
            self.note("a write to a member".to_string());
        }
        oxc_ast_visit::walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if !self.is_host_require(it.object()) {
            oxc_ast_visit::walk_js::walk_member_expression(self, it);
            return;
        }
        match it {
            MemberExpression::StaticMemberExpression(member) => {
                let name = member.property.name.as_str();
                if NODE_REQUIRE_MEMBERS.contains(&name) || member.optional {
                    self.note(format!("require.{name}"));
                }
            }
            _ => self.note("a computed member".to_string()),
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == SHIELDED_HOST && is_free(Some(self.scoping), it) {
            self.note("read as a value".to_string());
        }
    }
}

fn evals_host_require(scoping: &Scoping, call: &CallExpression<'_>) -> bool {
    let [Argument::StringLiteral(payload)] = call.arguments.as_slice() else {
        return false;
    };
    let text = payload.value.as_str().unwrap_or_default().trim();
    let name = text.strip_suffix(';').unwrap_or(text).trim();
    name == SHIELDED_HOST
        && is_direct_eval(scoping, &call.callee)
        && is_closed_eval_at(scoping, call)
}

fn is_host_require_name(scoping: &Scoping, expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier(id) => id.name == SHIELDED_HOST && is_free(Some(scoping), id),
        Expression::CallExpression(call) => !call.optional && evals_host_require(scoping, call),
        _ => false,
    }
}

fn calls_host_require(scoping: &Scoping, callee: &Expression<'_>) -> bool {
    match callee.without_parentheses() {
        Expression::ConditionalExpression(choice) => [&choice.consequent, &choice.alternate]
            .into_iter()
            .any(|branch| is_host_require_name(scoping, branch.without_parentheses())),
        callee => is_host_require_name(scoping, callee),
    }
}

pub fn first_host_require_use<'k, 'a: 'k>(
    scoping: &Scoping,
    factories: impl IntoIterator<Item = (String, Factory<'k, 'a>)>,
) -> Option<(String, Captured)> {
    factories.into_iter().find_map(|(id, factory)| {
        let mut uses = HostRequire {
            scoping,
            found: None,
            requests: Vec::new(),
        };
        uses.visit_function_body(factory.body);
        uses.found.map(|what| (id, Captured::HostUse(what)))
    })
}

pub fn host_require_requests(scoping: &Scoping, program: &Program<'_>) -> Vec<String> {
    let mut uses = HostRequire {
        scoping,
        found: None,
        requests: Vec::new(),
    };
    uses.visit_program(program);
    uses.requests
}
