use std::cell::{OnceCell, RefCell};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, Function, LogicalExpression, Program,
    Statement, VariableDeclarationKind,
};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::parse::parse_program;
use untwist_core::print::{expression_text, function_text};
use untwist_core::semantic::builtins::{patched_by_another_input, scan, Reliance};
use untwist_core::semantic::{
    has_module_syntax, is_constant_binding_in, observing_open, AnnexBWrites, Strictness,
};
use untwist_eval::closed_script_value;
use untwist_eval::fold::Const;

use super::resolved;
use crate::bindings::reaches_a_script_global;
use crate::calls::agnostic::names_reachable_from;
use crate::pure_calls::literal_data;

const FUEL: u64 = 5_000_000;
const HEAP: u64 = 1 << 24;

const GLOBALS: [&str; 5] = ["Array", "Math", "String", "Number", "Object"];

const USE_STRICT: &str = "\"use strict\";\n";

const IS_ARRAY: &str = "var Array = { isArray: function (value) { return Object.prototype.toString.call(value) === \"[object Array]\"; } };\n";

static CLOSED_GLOBALS: Reliance = Reliance {
    roots: &GLOBALS,
    members: &[],
};

pub struct Roots<'p> {
    program: &'p Program<'p>,
    functions: FxHashMap<SymbolId, &'p Function<'p>>,
    data: FxHashMap<SymbolId, (usize, &'p Expression<'p>)>,
    order: FxHashMap<Span, usize>,
    builtins: OnceCell<bool>,
    annex_b: OnceCell<AnnexBWrites>,
    decided: RefCell<FxHashMap<(Span, Span), Option<bool>>>,
    unsafe_lane: bool,
}

impl<'p> Roots<'p> {
    pub fn of(program: &'p Program<'p>, unsafe_lane: bool) -> Self {
        let mut roots = Self {
            program,
            functions: FxHashMap::default(),
            data: FxHashMap::default(),
            order: FxHashMap::default(),
            builtins: OnceCell::new(),
            annex_b: OnceCell::new(),
            decided: RefCell::default(),
            unsafe_lane,
        };
        for (index, statement) in program.body.iter().enumerate() {
            roots.record(index, statement);
        }
        roots
    }

    fn record(&mut self, index: usize, statement: &'p Statement<'p>) {
        match statement {
            Statement::FunctionDeclaration(function) => {
                if let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) {
                    self.functions.insert(symbol, function);
                }
            }
            Statement::VariableDeclaration(declaration) => {
                self.order.insert(declaration.span, index);
                if declaration.kind != VariableDeclarationKind::Const {
                    return;
                }
                for declarator in &declaration.declarations {
                    let (BindingPattern::BindingIdentifier(id), Some(init)) =
                        (&declarator.id, &declarator.init)
                    else {
                        continue;
                    };
                    if let Some(symbol) = id.symbol_id.get() {
                        if literal_data(init, true) {
                            self.data.insert(symbol, (index, init));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn decide(&self, test: &Expression<'_>, seed: Span, scoping: &Scoping) -> Option<bool> {
        let key = (test.span(), seed);
        if let Some(known) = self.decided.borrow().get(&key) {
            return *known;
        }
        let decided = self.decide_afresh(test, seed, scoping);
        self.decided.borrow_mut().insert(key, decided);
        decided
    }

    fn decide_afresh(&self, test: &Expression<'_>, seed: Span, scoping: &Scoping) -> Option<bool> {
        let index = *self.order.get(&seed)?;
        let shape = Shape::of(test, scoping)?;
        let function = *self.functions.get(&shape.symbol)?;
        if !self.fixed(shape.symbol, scoping) {
            return None;
        }
        let references = scoping.get_resolved_references(shape.symbol);
        let mut count = 0;
        for reference in references {
            if reference.is_write() {
                return None;
            }
            count += 1;
        }
        if count != shape.reads || function.r#async || function.generator {
            return None;
        }
        let text = function_text(function, false);
        let free = closed_reads(function, scoping, &self.data, &text)?;
        let mut inits = Vec::new();
        let mut named: FxHashSet<SymbolId> = FxHashSet::default();
        named.insert(shape.symbol);
        for (symbol, reads) in free.reads {
            named.insert(symbol);
            let (declared, init) = self.data.get(&symbol)?;
            let total = scoping.get_resolved_references(symbol).count();
            if *declared >= index || total != reads {
                return None;
            }
            let name = scoping.symbol_name(symbol);
            if GLOBALS.contains(&name) || !self.fixed(symbol, scoping) {
                return None;
            }
            inits.push((name.to_string(), *init));
        }
        if !self.builtins_hold(scoping) {
            return None;
        }
        if !self.unsafe_lane && self.reached_through_the_global_object(&named, scoping) {
            return None;
        }
        let script = Script {
            strict: self.is_strict(function),
            arrays: free.arrays,
            function: &text,
        };
        let value = evaluate(&inits, &script, shape.call)?;
        Some(value == shape.expected)
    }

    fn fixed(&self, symbol: SymbolId, scoping: &Scoping) -> bool {
        let annex_b = self
            .annex_b
            .get_or_init(|| AnnexBWrites::of(self.program, scoping));
        is_constant_binding_in(scoping, annex_b, symbol)
    }

    fn is_strict(&self, function: &Function<'_>) -> bool {
        self.program.source_type.is_module()
            || Strictness::of_program(self.program)
                .in_function(function)
                .is_strict()
    }

    fn reached_through_the_global_object(
        &self,
        symbols: &FxHashSet<SymbolId>,
        scoping: &Scoping,
    ) -> bool {
        if self.program.source_type.is_module() || has_module_syntax(self.program) {
            return false;
        }
        let mut names: Vec<&str> = symbols
            .iter()
            .map(|symbol| scoping.symbol_name(*symbol))
            .collect();
        names.sort_unstable();
        names.dedup();
        observing_open(|| {
            names_reachable_from(self.program, scoping, &names)
                || reaches_a_script_global(symbols, &[], self.program, scoping)
        })
    }

    fn builtins_hold(&self, scoping: &Scoping) -> bool {
        *self.builtins.get_or_init(|| {
            !patched_by_another_input(&CLOSED_GLOBALS)
                && scan(
                    self.program,
                    scoping,
                    has_module_syntax(self.program),
                    false,
                    &CLOSED_GLOBALS,
                )
                .verdict
                .allows_safe(self.unsafe_lane)
        })
    }
}

struct Shape<'e, 'a> {
    symbol: SymbolId,
    reads: usize,
    call: &'e Expression<'a>,
    expected: f64,
}

impl<'e, 'a> Shape<'e, 'a> {
    fn of(test: &'e Expression<'a>, scoping: &Scoping) -> Option<Self> {
        match test.without_parentheses() {
            Expression::LogicalExpression(logical) => Self::guarded(logical, scoping),
            other => Self::comparison(other, scoping),
        }
    }

    fn guarded(logical: &'e LogicalExpression<'a>, scoping: &Scoping) -> Option<Self> {
        if logical.operator != LogicalOperator::And {
            return None;
        }
        let guarded = typeof_function(&logical.left, scoping)?;
        let mut shape = Self::comparison(&logical.right, scoping)?;
        if guarded != shape.symbol {
            return None;
        }
        shape.reads += 1;
        Some(shape)
    }

    fn comparison(expr: &'e Expression<'a>, scoping: &Scoping) -> Option<Self> {
        let Expression::BinaryExpression(binary) = expr.without_parentheses() else {
            return None;
        };
        if !matches!(
            binary.operator,
            BinaryOperator::StrictEquality | BinaryOperator::Equality
        ) {
            return None;
        }
        let (call, expected) = match (
            binary.left.without_parentheses(),
            binary.right.without_parentheses(),
        ) {
            (Expression::CallExpression(_), Expression::NumericLiteral(literal)) => {
                (&binary.left, literal.value)
            }
            (Expression::NumericLiteral(literal), Expression::CallExpression(_)) => {
                (&binary.right, literal.value)
            }
            _ => return None,
        };
        let Expression::CallExpression(inner) = call.without_parentheses() else {
            return None;
        };
        let symbol = literal_call(inner, scoping)?;
        Some(Self {
            symbol,
            reads: 1,
            call,
            expected,
        })
    }
}

fn literal_call(call: &CallExpression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    if call.optional {
        return None;
    }
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    let literal = |argument: &Argument<'_>| matches!(argument, Argument::NumericLiteral(_));
    call.arguments
        .iter()
        .all(literal)
        .then(|| resolved(callee, scoping))
        .flatten()
}

fn typeof_function(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::BinaryExpression(binary) = expr.without_parentheses() else {
        return None;
    };
    if binary.operator != BinaryOperator::StrictEquality {
        return None;
    }
    let Expression::UnaryExpression(unary) = binary.left.without_parentheses() else {
        return None;
    };
    let Expression::StringLiteral(kind) = binary.right.without_parentheses() else {
        return None;
    };
    if unary.operator != UnaryOperator::Typeof || kind.value != "function" {
        return None;
    }
    let Expression::Identifier(id) = unary.argument.without_parentheses() else {
        return None;
    };
    resolved(id, scoping)
}

struct Closed {
    reads: Vec<(SymbolId, usize)>,
    arrays: bool,
}

fn closed_reads(
    function: &Function<'_>,
    scoping: &Scoping,
    data: &FxHashMap<SymbolId, (usize, &Expression<'_>)>,
    text: &str,
) -> Option<Closed> {
    let inside = function.scope_id.get()?;
    let within = |scope| {
        scoping
            .scope_ancestors(scope)
            .any(|ancestor| ancestor == inside)
    };
    let unresolved: FxHashMap<ReferenceId, &str> = scoping
        .root_unresolved_references()
        .iter()
        .flat_map(|(name, ids)| ids.iter().map(move |id| (*id, name.as_str())))
        .collect();
    let mut reads: FxHashMap<SymbolId, usize> = FxHashMap::default();
    for index in 0..scoping.references_len() {
        let id = ReferenceId::from_usize(index);
        let reference = scoping.get_reference(id);
        if !within(reference.scope_id()) {
            continue;
        }
        match reference.symbol_id() {
            Some(symbol) if within(scoping.symbol_scope_id(symbol)) => {}
            Some(symbol) if data.contains_key(&symbol) => *reads.entry(symbol).or_default() += 1,
            Some(_) => return None,
            None => {
                if !GLOBALS.contains(unresolved.get(&id)?) {
                    return None;
                }
            }
        }
    }
    if words(text, "this").next().is_some() {
        return None;
    }
    let mut arrays = false;
    for global in GLOBALS {
        for after in words(text, global) {
            let member = after.strip_prefix('.')?;
            let end = member
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                .unwrap_or(member.len());
            match (global, &member[..end]) {
                ("Math", "random") | (_, "") => return None,
                ("Array", "isArray") => arrays = true,
                ("Array", _) => return None,
                _ => {}
            }
        }
    }
    let mut reads: Vec<(SymbolId, usize)> = reads.into_iter().collect();
    reads.sort_unstable_by_key(|(symbol, _)| *symbol);
    Some(Closed { reads, arrays })
}

fn words<'t>(text: &'t str, word: &'t str) -> impl Iterator<Item = &'t str> + 't {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    text.match_indices(word).filter_map(move |(at, _)| {
        let before = text[..at].chars().next_back();
        let after = &text[at + word.len()..];
        let separate = before.is_none_or(|c| !is_name(c) && c != '.')
            && after.chars().next().is_none_or(|c| !is_name(c));
        separate.then_some(after)
    })
}

struct Script<'t> {
    strict: bool,
    arrays: bool,
    function: &'t str,
}

fn evaluate(
    inits: &[(String, &Expression<'_>)],
    script: &Script<'_>,
    call: &Expression<'_>,
) -> Option<f64> {
    let mut text = String::new();
    if script.strict {
        text.push_str(USE_STRICT);
    }
    if script.arrays {
        text.push_str(IS_ARRAY);
    }
    for (name, init) in inits {
        text.push_str(&format!("var {name} = {};\n", expression_text(init)));
    }
    text.push_str(script.function);
    text.push_str(&format!("\n{};\n", expression_text(call)));
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &text);
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    match closed_script_value(&parsed.program, FUEL, HEAP)? {
        Const::Num(value) => Some(value),
        _ => None,
    }
}
