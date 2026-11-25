use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{has_module_syntax, is_foreign_global, may_call_eval};
use untwist_eval::fold::{const_to_expression, fold_binary, Const};

use super::refs::{inside, literal_value, names_observed, scan, unpatched_outside, Refs};
use crate::calls::agnostic::{global_reach, ReachWatch};
use crate::pure_calls::literal_data;

pub const PASS: &str = "fold-js-confuser-helpers";

const HINTS: [&str; 2] = ["__p_", "return globalThis"];

const PREMISE_NOTE: &str =
    "js-confuser helpers: the machinery was dropped under the outside-code premise";

const CALCULATOR_OPERATORS: [BinaryOperator; 4] = [
    BinaryOperator::Addition,
    BinaryOperator::Subtraction,
    BinaryOperator::Multiplication,
    BinaryOperator::Division,
];

const FUNCTION_KEYS: [&str; 20] = [
    "length",
    "name",
    "prototype",
    "arguments",
    "caller",
    "constructor",
    "apply",
    "bind",
    "call",
    "toString",
    "__defineGetter__",
    "__defineSetter__",
    "hasOwnProperty",
    "__lookupGetter__",
    "__lookupSetter__",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "valueOf",
    "__proto__",
    "toLocaleString",
];

static FUNCTION_PROTOTYPES: Reliance = Reliance {
    roots: &["Function", "Object"],
    members: &[],
};

#[derive(Default)]
pub struct FoldHelpers {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(FoldHelpers::default())
}

struct Helper {
    predicate: bool,
    symbol: SymbolId,
    statement: Span,
    folds: Vec<(Span, Const)>,
    complete: bool,
}

fn declared_function<'k, 'a>(statement: &'k Statement<'a>) -> Option<(SymbolId, &'k Function<'a>)> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    if function.r#async || function.generator {
        return None;
    }
    Some((function.id.as_ref()?.symbol_id.get()?, function))
}

fn simple_params(function: &Function<'_>) -> Option<Vec<SymbolId>> {
    if function.params.rest.is_some() {
        return None;
    }
    function
        .params
        .items
        .iter()
        .map(|item| match (&item.pattern, &item.initializer) {
            (BindingPattern::BindingIdentifier(id), None) => id.symbol_id.get(),
            _ => None,
        })
        .collect()
}

fn predicate(statement: &Statement<'_>, refs: &Refs<'_>, scoping: &Scoping) -> Option<Helper> {
    let (symbol, function) = declared_function(statement)?;
    let body = function.body.as_ref()?;
    let empty = function.params.items.is_empty()
        && function.params.rest.is_none()
        && body.statements.is_empty()
        && body.directives.is_empty();
    let tests = refs.in_tests.get(&symbol)?;
    if !empty || !refs.stable(symbol, scoping) || refs.of(symbol).len() != tests.len() {
        return None;
    }
    let folds: Vec<(Span, Const)> = tests
        .iter()
        .filter(|(_, key)| !FUNCTION_KEYS.contains(&key.as_str()))
        .map(|(span, _)| (*span, Const::Bool(false)))
        .collect();
    let complete = folds.len() == tests.len();
    Some(Helper {
        predicate: true,
        symbol,
        statement: statement.span(),
        folds,
        complete,
    })
}

fn param_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

fn calculator_cases(
    function: &Function<'_>,
    scoping: &Scoping,
) -> Option<Vec<(String, BinaryOperator)>> {
    let params = simple_params(function)?;
    let [operator, left, right] = params.as_slice() else {
        return None;
    };
    let body = function.body.as_ref()?;
    let [Statement::SwitchStatement(switch)] = body.statements.as_slice() else {
        return None;
    };
    if !body.directives.is_empty() || param_symbol(&switch.discriminant, scoping) != Some(*operator)
    {
        return None;
    }
    switch
        .cases
        .iter()
        .map(|case| {
            let Some(Expression::StringLiteral(key)) = &case.test else {
                return None;
            };
            let [Statement::ReturnStatement(ret)] = case.consequent.as_slice() else {
                return None;
            };
            let Some(Expression::BinaryExpression(binary)) = ret.argument.as_ref() else {
                return None;
            };
            let operands = param_symbol(&binary.left, scoping) == Some(*left)
                && param_symbol(&binary.right, scoping) == Some(*right);
            (operands && CALCULATOR_OPERATORS.contains(&binary.operator))
                .then(|| Some((key.value.as_str()?.to_string(), binary.operator)))?
        })
        .collect()
}

fn calculator(statement: &Statement<'_>, refs: &Refs<'_>, scoping: &Scoping) -> Option<Helper> {
    let (symbol, function) = declared_function(statement)?;
    let cases = calculator_cases(function, scoping)?;
    let calls = refs.calls_of(symbol);
    if !refs.stable(symbol, scoping) || calls.is_empty() || refs.of(symbol).len() != calls.len() {
        return None;
    }
    let folds: Vec<(Span, Const)> = calls
        .iter()
        .filter(|call| !call.spread)
        .filter_map(|call| {
            let [Some(Const::Str(key)), Some(left @ Const::Num(_)), Some(right @ Const::Num(_))] =
                call.args.as_slice()
            else {
                return None;
            };
            let (_, operator) = cases.iter().find(|(case, _)| case == key)?;
            Some((call.span, fold_binary(*operator, left, right)?))
        })
        .collect();
    let complete = folds.len() == calls.len();
    Some(Helper {
        predicate: false,
        symbol,
        statement: statement.span(),
        folds,
        complete,
    })
}

fn literal_array(statement: &Statement<'_>, refs: &Refs<'_>, scoping: &Scoping) -> Option<Helper> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let (BindingPattern::BindingIdentifier(id), Some(Expression::ArrayExpression(array))) =
        (&declarator.id, &declarator.init)
    else {
        return None;
    };
    let symbol = id.symbol_id.get()?;
    let elements: Vec<Const> = array
        .elements
        .iter()
        .map(|element| element.as_expression().and_then(literal_value))
        .collect::<Option<_>>()?;
    let reads = refs.index_reads.get(&symbol)?;
    if !refs.stable(symbol, scoping) || refs.of(symbol).len() != reads.len() {
        return None;
    }
    let folds: Vec<(Span, Const)> = reads
        .iter()
        .filter(|(_, index)| index.fract() == 0.0 && *index >= 0.0)
        .filter_map(|(span, index)| Some((*span, elements.get(*index as usize)?.clone())))
        .collect();
    let complete = folds.len() == reads.len();
    Some(Helper {
        predicate: false,
        symbol,
        statement: statement.span(),
        folds,
        complete,
    })
}

fn repeats_a_read(helper: &Helper) -> bool {
    helper
        .folds
        .iter()
        .enumerate()
        .any(|(at, (_, value))| helper.folds[..at].iter().any(|(_, seen)| seen == value))
}

fn runs_no_code(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) => true,
        Statement::VariableDeclaration(declaration) => declaration.declarations.iter().all(|d| {
            matches!(d.id, BindingPattern::BindingIdentifier(_))
                && d.init.as_ref().is_none_or(|init| literal_data(init, true))
        }),
        _ => false,
    }
}

struct Plan {
    folds: Vec<(Span, Const)>,
    removals: Vec<Span>,
    notes: Vec<String>,
}

fn helpers(program: &Program<'_>, refs: &Refs<'_>, scoping: &Scoping) -> Vec<Helper> {
    let mut found = Vec::new();
    for (index, statement) in program.body.iter().enumerate() {
        if let Some(helper) = calculator(statement, refs, scoping) {
            found.push(helper);
            continue;
        }
        if let Some(helper) = literal_array(statement, refs, scoping) {
            if program.body[..index].iter().all(runs_no_code) && repeats_a_read(&helper) {
                found.push(helper);
            }
            continue;
        }
        if let Some(helper) = predicate(statement, refs, scoping) {
            found.push(helper);
        }
    }
    found
}

struct DeadBranches<'k> {
    falsy: &'k FxHashSet<Span>,
    dead: Vec<Span>,
    functions: Vec<(SymbolId, Span)>,
}

impl<'a> VisitJs<'a> for DeadBranches<'_> {
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        match it.test.without_parentheses() {
            test if self.falsy.contains(&test.span()) => self.dead.push(it.consequent.span()),
            Expression::UnaryExpression(unary)
                if unary.operator == UnaryOperator::LogicalNot
                    && self
                        .falsy
                        .contains(&unary.argument.without_parentheses().span()) =>
            {
                if let Some(alternate) = &it.alternate {
                    self.dead.push(alternate.span());
                }
            }
            _ => {}
        }
        walk_js::walk_if_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (true, Some(symbol)) = (
            it.is_declaration(),
            it.id.as_ref().and_then(|id| id.symbol_id.get()),
        ) {
            self.functions.push((symbol, it.span));
        }
        walk_js::walk_function(self, it, flags);
    }
}

fn dead_regions(program: &Program<'_>, refs: &Refs<'_>, found: &[Helper]) -> Vec<Span> {
    let falsy: FxHashSet<Span> = found
        .iter()
        .filter(|h| h.predicate)
        .flat_map(|h| h.folds.iter().map(|(span, _)| *span))
        .collect();
    if falsy.is_empty() {
        return Vec::new();
    }
    let mut branches = DeadBranches {
        falsy: &falsy,
        dead: Vec::new(),
        functions: Vec::new(),
    };
    branches.visit_program(program);
    let mut dead = branches.dead;
    let mut functions = branches.functions;
    loop {
        let before = dead.len();
        functions.retain(|(symbol, span)| {
            let uses = refs.of(*symbol);
            let unreached =
                !uses.is_empty() && uses.iter().all(|r| dead.iter().any(|d| inside(*d, r.span)));
            if unreached {
                dead.push(*span);
            }
            !unreached
        });
        if dead.len() == before {
            return dead;
        }
    }
}

fn analyse(program: &Program<'_>, scoping: &Scoping, unsafe_lane: bool) -> Plan {
    let mut plan = Plan {
        folds: Vec::new(),
        removals: Vec::new(),
        notes: Vec::new(),
    };
    if has_module_syntax(program) || !HINTS.iter().any(|hint| program.source_text.contains(hint)) {
        return plan;
    }
    let refs = scan(program, scoping);
    let mut found = helpers(program, &refs, scoping);
    if found.is_empty() {
        return plan;
    }
    let dead = dead_regions(program, &refs, &found);
    let skipped: Vec<Span> = found
        .iter()
        .map(|h| h.statement)
        .chain(dead.iter().copied())
        .collect();
    let predicates = found.iter().any(|h| h.predicate);
    if predicates
        && !unsafe_lane
        && !unpatched_outside(program, scoping, &FUNCTION_PROTOTYPES, &skipped)
    {
        plan.notes.push("js-confuser opaque predicates kept: the program may give Function.prototype or Object.prototype the tested key".to_string());
        found.retain(|h| !h.predicate);
        if found.is_empty() {
            return plan;
        }
    }
    if refs.has_with || may_call_eval(scoping) {
        plan.notes.push(
            "js-confuser helpers kept: the program has a with statement or may call eval"
                .to_string(),
        );
        return plan;
    }
    let names: Vec<&str> = found
        .iter()
        .map(|h| scoping.symbol_name(h.symbol))
        .collect();
    if names.iter().any(|name| is_foreign_global(name)) {
        plan.notes
            .push("js-confuser helpers kept: another input names them".to_string());
        return plan;
    }
    let fns: Vec<SymbolId> = found.iter().map(|h| h.symbol).collect();
    let statements: Vec<Span> = found.iter().map(|h| h.statement).collect();
    let watch = ReachWatch {
        names: &names,
        props: &names,
        fns: &fns,
        observe: true,
    };
    let excluded: Vec<Span> = statements
        .iter()
        .copied()
        .chain(dead.iter().copied())
        .collect();
    let reach = global_reach(program, scoping, (None, &excluded), watch);
    if reach.reach.is_some() || reach.reach_by_code.is_some() {
        plan.notes.push(
            "js-confuser helpers kept: code may reach them through the global object or code text"
                .to_string(),
        );
        return plan;
    }
    let removable = !names_observed(&reach);
    if !removable {
        plan.notes.push(
            "js-confuser helpers folded but kept: code may observe the global object's names"
                .to_string(),
        );
    }
    if removable && reach.outside_premise.is_some() {
        plan.notes.push(PREMISE_NOTE.to_string());
    }
    for helper in found {
        if helper.complete && removable {
            plan.removals.push(helper.statement);
        }
        plan.folds.extend(helper.folds);
    }
    plan
}

fn apply<'a>(plan: Plan, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
    let mut folds = ExprEdits::new();
    for (span, value) in &plan.folds {
        let Some(replacement) = const_to_expression(value, *span, &*cx) else {
            return;
        };
        if !folds.insert(*span, replacement) {
            return;
        }
    }
    let wanted = folds.len();
    let applied = folds.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(applied);
    if applied != wanted {
        return;
    }
    let mut removals = StmtEdits::new();
    for span in &plan.removals {
        removals.remove(*span);
    }
    let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(removed);
}

impl<'a> Pass<'a> for FoldHelpers {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.settled {
            return;
        }
        self.settled = true;
        let plan = analyse(program, cx.scoping(), cx.opts.unsafe_transforms);
        for note in &plan.notes {
            cx.note(note.clone(), None);
        }
        apply(plan, program, cx);
    }
}
