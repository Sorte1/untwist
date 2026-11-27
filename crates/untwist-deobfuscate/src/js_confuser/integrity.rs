use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{has_module_syntax, is_foreign_global, may_call_eval};
use untwist_eval::fold::{const_to_expression, Const};

use super::names::{names, Names};
use super::refs::{
    inside, judge_replaced, names_observed, scan, shared_elsewhere, Refs, FUNCTION_KEYS,
};
use super::templates::{matches_integrity, INTEGRITY_LENGTHS};
use super::tokens::FIXED;
use crate::pure_calls::literal_data;

pub const PASS: &str = "remove-js-confuser-integrity";

const MARK: &str = "|\\\\{|\\\\}|\\\\(|\\\\)|";
const SENSITIVE: [char; 11] = [' ', '\n', ';', ',', '{', '}', '(', ')', '.', '[', ']'];
const KEY_MENTIONS: usize = 2;

static HASHED: Reliance = Reliance {
    roots: &["Math", "Function", "Object", "String", "RegExp"],
    members: &[],
};

#[derive(Default)]
pub struct RemoveIntegrity {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(RemoveIntegrity::default())
}

pub(super) fn cyrb53(text: &str, seed: u32) -> f64 {
    let mut h1: u32 = 0xdead_beef ^ seed;
    let mut h2: u32 = 0x41c6_ce57 ^ seed;
    for unit in text.encode_utf16() {
        h1 = (h1 ^ u32::from(unit)).wrapping_mul(2_654_435_761);
        h2 = (h2 ^ u32::from(unit)).wrapping_mul(1_597_334_677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2_246_822_507)
        ^ (h2 ^ (h2 >> 13)).wrapping_mul(3_266_489_909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2_246_822_507)
        ^ (h1 ^ (h1 >> 13)).wrapping_mul(3_266_489_909);
    4_294_967_296.0 * f64::from(h2 & 0x001f_ffff) + f64::from(h1)
}

fn text_of(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

struct Region {
    span: Span,
    statements: Vec<Span>,
    hash: SymbolId,
    last_index: usize,
}

fn find_region(program: &Program<'_>, names: &Names) -> Option<Region> {
    let body = &program.body;
    for (index, statement) in body.iter().enumerate() {
        if !matches!(statement, Statement::FunctionDeclaration(_)) {
            continue;
        }
        for length in INTEGRITY_LENGTHS {
            let Some(run) = body.get(index..index + length) else {
                continue;
            };
            let span = Span::new(run[0].span().start, run[length - 1].span().end);
            if !matches_integrity(program.source_text, span, names) {
                continue;
            }
            let Statement::FunctionDeclaration(hash) = &run[length - 1] else {
                return None;
            };
            return Some(Region {
                span,
                statements: run.iter().map(GetSpan::span).collect(),
                hash: hash.id.as_ref()?.symbol_id.get()?,
                last_index: index + length - 1,
            });
        }
    }
    None
}

struct Guard {
    owner: SymbolId,
    key: String,
    cache: Span,
    test: Span,
    target: SymbolId,
    seed: f64,
    expected: f64,
    local: SymbolId,
}

fn symbol_of(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

fn own_member<'k>(expr: &'k Expression<'_>, scoping: &Scoping) -> Option<(SymbolId, &'k str)> {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) => Some((
            symbol_of(&member.object, scoping)?,
            member.property.name.as_str(),
        )),
        _ => None,
    }
}

fn cached_hash<'k, 'a>(
    init: &'k Expression<'a>,
    scoping: &Scoping,
) -> Option<(SymbolId, &'k str, &'k CallExpression<'a>)> {
    let Expression::LogicalExpression(logical) = init.without_parentheses() else {
        return None;
    };
    if logical.operator != LogicalOperator::Or {
        return None;
    }
    let (owner, key) = own_member(&logical.left, scoping)?;
    let Expression::AssignmentExpression(assign) = logical.right.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(target) = &assign.left else {
        return None;
    };
    let same_target =
        symbol_of(&target.object, scoping) == Some(owner) && target.property.name.as_str() == key;
    let Expression::CallExpression(call) = assign.right.without_parentheses() else {
        return None;
    };
    (assign.operator == AssignmentOperator::Assign && same_target).then_some((owner, key, call))
}

fn guard(function: &Function<'_>, hash: SymbolId, scoping: &Scoping) -> Option<Guard> {
    let own = function.id.as_ref()?.symbol_id.get()?;
    let body = function.body.as_ref()?;
    let [Statement::VariableDeclaration(cache), Statement::IfStatement(check)] =
        body.statements.as_slice()
    else {
        return None;
    };
    let [declarator] = cache.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(local) = &declarator.id else {
        return None;
    };
    let (owner, key, call) = cached_hash(declarator.init.as_ref()?, scoping)?;
    let [Argument::Identifier(target), Argument::NumericLiteral(seed)] = call.arguments.as_slice()
    else {
        return None;
    };
    let Expression::BinaryExpression(test) = check.test.without_parentheses() else {
        return None;
    };
    let Expression::NumericLiteral(expected) = &test.right else {
        return None;
    };
    let local = local.symbol_id.get()?;
    let linked = owner == own
        && symbol_of(&call.callee, scoping) == Some(hash)
        && test.operator == BinaryOperator::StrictEquality
        && symbol_of(&test.left, scoping) == Some(local);
    linked.then(|| {
        Some(Guard {
            owner,
            key: key.to_string(),
            cache: cache.span,
            test: test.span,
            target: scoping
                .get_reference(target.reference_id.get()?)
                .symbol_id()?,
            seed: seed.value,
            expected: expected.value,
            local,
        })
    })?
}

struct Guards<'s> {
    scoping: &'s Scoping,
    hash: SymbolId,
    found: Vec<Guard>,
}

impl<'a> VisitJs<'a> for Guards<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.is_declaration() {
            if let Some(found) = guard(it, self.hash, self.scoping) {
                self.found.push(found);
            }
        }
        walk_js::walk_function(self, it, flags);
    }
}

fn closed(refs: &Refs<'_>, scoping: &Scoping, region: Span) -> bool {
    refs.all
        .iter()
        .filter(|(r, _)| inside(region, r.span))
        .all(|(r, name)| match r.symbol {
            None => FIXED.contains(name) && !is_foreign_global(name),
            Some(symbol) => inside(region, scoping.symbol_span(symbol)),
        })
}

fn hashed(source: &str, span: Span) -> String {
    text_of(source, span)
        .chars()
        .filter(|c| !SENSITIVE.contains(c))
        .collect()
}

fn runs_no_code(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => true,
        Statement::VariableDeclaration(declaration) => declaration.declarations.iter().all(|d| {
            matches!(d.id, BindingPattern::BindingIdentifier(_))
                && d.init.as_ref().is_none_or(|init| literal_data(init, true))
        }),
        _ => false,
    }
}

#[derive(Default)]
struct Plan {
    tests: Vec<Span>,
    caches: Vec<Span>,
    region: Vec<Span>,
    symbols: Vec<SymbolId>,
    watched: Vec<SymbolId>,
}

fn function_spans(program: &Program<'_>) -> rustc_hash::FxHashMap<SymbolId, Span> {
    struct Spans {
        out: rustc_hash::FxHashMap<SymbolId, Span>,
    }
    impl<'a> VisitJs<'a> for Spans {
        fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
            if let (true, Some(symbol)) = (
                it.is_declaration(),
                it.id.as_ref().and_then(|id| id.symbol_id.get()),
            ) {
                self.out.insert(symbol, it.span);
            }
            walk_js::walk_function(self, it, flags);
        }
    }
    let mut spans = Spans {
        out: rustc_hash::FxHashMap::default(),
    };
    spans.visit_program(program);
    spans.out
}

fn analyse(
    program: &Program<'_>,
    scoping: &Scoping,
    nested_root: bool,
) -> Result<Plan, &'static str> {
    if has_module_syntax(program) || !program.source_text.contains(MARK) {
        return Ok(Plan::default());
    }
    let names = names(program, scoping);
    let Some(region) = find_region(program, &names) else {
        return Ok(Plan::default());
    };
    let mut guards = Guards {
        scoping,
        hash: region.hash,
        found: Vec::new(),
    };
    guards.visit_program(program);
    let refs = scan(program, scoping);
    if refs.has_with || may_call_eval(scoping) {
        return Err("the program has a with statement or may call eval");
    }
    if !closed(&refs, scoping, region.span) {
        return Err("its hash functions read a name from outside");
    }
    let region_symbols: Vec<SymbolId> = scoping
        .symbol_ids()
        .filter(|s| scoping.symbol_scope_id(*s) == scoping.root_scope_id())
        .filter(|s| {
            region
                .statements
                .iter()
                .any(|span| inside(*span, scoping.symbol_span(*s)))
        })
        .collect();
    if !region_symbols.iter().all(|s| refs.stable(*s, scoping)) {
        return Err("a name of its hash functions is written elsewhere");
    }
    if !program.body[..region.last_index]
        .iter()
        .all(|s| runs_no_code(s) || region.statements.contains(&s.span()))
    {
        return Err("code before its hash functions may call a checked function");
    }
    let functions = function_spans(program);
    let mut plan = Plan::default();
    for found in &guards.found {
        let Some(target) = functions.get(&found.target) else {
            return Err("a checked function is not a function declaration");
        };
        let Some(owner) = functions.get(&found.owner) else {
            return Err("a guard is not in a function declaration");
        };
        let target_uses_inside = refs.of(found.target).iter().all(|r| inside(*owner, r.span));
        let mentions = program.source_text.matches(found.key.as_str()).count();
        let cyrb_seed =
            found.seed.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&found.seed);
        if FUNCTION_KEYS.contains(&found.key.as_str()) {
            return Err("a cache key is a property functions already have");
        }
        if !target_uses_inside
            || mentions != KEY_MENTIONS
            || !cyrb_seed
            || refs.of(found.local).len() != 1
        {
            return Err("a checked function or its cache key is used elsewhere");
        }
        if !refs.stable(found.target, scoping) || !refs.stable(found.owner, scoping) {
            return Err("a checked function is reassigned");
        }
        if cyrb53(&hashed(program.source_text, *target), found.seed as u32) != found.expected {
            return Err("a checked function's source text does not hash to its expected value, so the check fails on the input");
        }
        plan.tests.push(found.test);
        plan.caches.push(found.cache);
        plan.watched.extend([found.target, found.owner]);
    }
    let named: Vec<SymbolId> = region_symbols
        .iter()
        .chain(plan.watched.iter())
        .copied()
        .collect();
    if shared_elsewhere(program, scoping, nested_root, &named) {
        return Err("another input names its hash functions or a checked function");
    }
    let removed: Vec<Span> = region
        .statements
        .iter()
        .chain(plan.caches.iter())
        .copied()
        .collect();
    let unused_elsewhere = region_symbols.iter().all(|s| {
        refs.of(*s)
            .iter()
            .all(|r| removed.iter().any(|span| inside(*span, r.span)))
    });
    if unused_elsewhere {
        plan.region = region.statements;
    }
    plan.symbols = region_symbols;
    Ok(plan)
}

impl<'a> Pass<'a> for RemoveIntegrity {
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
        let plan = match analyse(program, cx.scoping(), cx.opts.nested_root) {
            Ok(plan) if plan.tests.is_empty() && plan.region.is_empty() => return,
            Ok(plan) => plan,
            Err(reason) => {
                cx.note(format!("js-confuser integrity checks kept: {reason}"), None);
                return;
            }
        };
        let replacements: Vec<(Span, Expression<'a>)> = plan
            .tests
            .iter()
            .filter_map(|span| Some((*span, const_to_expression(&Const::Bool(true), *span, &*cx)?)))
            .collect();
        if replacements.len() != plan.tests.len() {
            return;
        }
        let stripped: Vec<Span> = plan
            .region
            .iter()
            .chain(plan.caches.iter())
            .copied()
            .collect();
        let symbols: Vec<SymbolId> = plan
            .symbols
            .iter()
            .chain(plan.watched.iter())
            .copied()
            .collect();
        let Some(judged) =
            judge_replaced(program, cx, replacements, (&HASHED, &stripped), &symbols)
        else {
            return;
        };
        if !cx.opts.unsafe_transforms && !judged.intact {
            cx.note("js-confuser integrity checks kept: the program may replace a built-in the hash reads (Math.imul, Function.prototype.toString, String.prototype.replace, RegExp)", None);
            return;
        }
        if judged.reach.reach.is_some() || judged.reach.reach_by_code.is_some() {
            cx.note("js-confuser integrity checks kept: code may reach its hash functions or a checked function through the global object or code text", None);
            return;
        }
        let mut folds = ExprEdits::new();
        for span in &plan.tests {
            let Some(value) = const_to_expression(&Const::Bool(true), *span, &*cx) else {
                return;
            };
            folds.insert(*span, value);
        }
        let wanted = folds.len();
        let applied = folds.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(applied);
        if applied != wanted {
            return;
        }
        let mut removals = StmtEdits::new();
        for span in &plan.caches {
            removals.remove(*span);
        }
        if names_observed(&judged.reach) {
            cx.note("js-confuser integrity: code may observe the global object's names, so its hash functions are kept", None);
        } else {
            for span in &plan.region {
                removals.remove(*span);
            }
        }
        let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
        cx.changed_by(removed);
        if !plan.tests.is_empty() {
            cx.note(format!("{} js-confuser integrity check(s) removed: each checked function's source text hashes to its expected value, so the check passes on the input", plan.tests.len()), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyrb53_matches_js_confuser() {
        let source =
            "function __p_1lka(first,second){return function(value){return second(first(value))}}";
        let text = hashed(source, Span::new(0, source.len() as u32));
        assert_eq!(cyrb53(&text, 8_843_169), 1_724_188_293_286_704.0);
    }
}
