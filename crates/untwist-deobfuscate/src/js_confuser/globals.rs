use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{has_module_syntax, is_foreign_global, may_call_eval};
use untwist_eval::fold::Const;

use super::names::{names, Names};
use super::refs::{
    inside, judge_replaced, names_observed, no_op_helpers, scan, shared_elsewhere, Refs,
};
use super::templates::matches_get_global;
use super::tokens::FIXED;
use crate::pure_calls::literal_data;

pub const PASS: &str = "inline-js-confuser-globals";

const GETTER_MARK: &str = "return globalThis";

const STANDARD_GLOBALS: [&str; 40] = [
    "console",
    "Math",
    "JSON",
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Symbol",
    "Date",
    "RegExp",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "Promise",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "Reflect",
    "Proxy",
    "BigInt",
    "Function",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "encodeURIComponent",
    "decodeURIComponent",
    "encodeURI",
    "decodeURI",
    "escape",
    "unescape",
    "setTimeout",
    "clearTimeout",
    "setInterval",
    "clearInterval",
    "globalThis",
];

static GLOBAL_OBJECT: Reliance = Reliance {
    roots: &["globalThis", "Object", "String"],
    members: &[],
};

#[derive(Default)]
pub struct InlineGlobals {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InlineGlobals::default())
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

fn symbol_of(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

struct Getter {
    symbol: SymbolId,
    statement: Span,
}

fn getters(
    program: &Program<'_>,
    (refs, scoping): (&Refs<'_>, &Scoping),
    names: &Names,
    helpers: &FxHashSet<SymbolId>,
) -> Vec<Getter> {
    program
        .body
        .iter()
        .filter_map(declared_function)
        .filter(|(_, function)| matches_get_global(program.source_text, function.span, names))
        .filter(|(symbol, function)| {
            refs.stable(*symbol, scoping)
                && refs
                    .all
                    .iter()
                    .filter(|(r, _)| inside(function.span, r.span))
                    .all(|(r, name)| match r.symbol {
                        None => FIXED.contains(name) && !is_foreign_global(name),
                        Some(s) => {
                            inside(function.span, scoping.symbol_span(s)) || helpers.contains(&s)
                        }
                    })
        })
        .map(|(symbol, function)| Getter {
            symbol,
            statement: function.span,
        })
        .collect()
}

struct Holder {
    symbol: SymbolId,
    statement: Span,
    index: usize,
}

fn holder(
    program: &Program<'_>,
    getter: &Getter,
    refs: &Refs<'_>,
    scoping: &Scoping,
) -> Option<Holder> {
    let calls = refs.calls_of(getter.symbol);
    let [call] = calls else {
        return None;
    };
    if refs.of(getter.symbol).len() != 1 || !call.args.is_empty() {
        return None;
    }
    program
        .body
        .iter()
        .enumerate()
        .find_map(|(index, statement)| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return None;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                return None;
            };
            let (BindingPattern::BindingIdentifier(id), Some(Expression::CallExpression(init))) =
                (&declarator.id, &declarator.init)
            else {
                return None;
            };
            let symbol = id.symbol_id.get()?;
            (init.span == call.span
                && init.arguments.is_empty()
                && declaration.kind == VariableDeclarationKind::Var
                && refs.stable(symbol, scoping))
            .then_some(Holder {
                symbol,
                statement: statement.span(),
                index,
            })
        })
}

struct Mapper {
    symbol: SymbolId,
    statement: Span,
    cases: Vec<(String, String)>,
}

fn mapper_cases(
    function: &Function<'_>,
    holder: SymbolId,
    scoping: &Scoping,
) -> Option<Vec<(String, String)>> {
    if function.params.rest.is_some() {
        return None;
    }
    let [param] = function.params.items.as_slice() else {
        return None;
    };
    let (BindingPattern::BindingIdentifier(param), None) = (&param.pattern, &param.initializer)
    else {
        return None;
    };
    let body = function.body.as_ref()?;
    let [Statement::SwitchStatement(switch)] = body.statements.as_slice() else {
        return None;
    };
    if !body.directives.is_empty()
        || symbol_of(&switch.discriminant, scoping) != param.symbol_id.get()
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
            let (object, name) = match ret.argument.as_ref()? {
                Expression::ComputedMemberExpression(member) => match &member.expression {
                    Expression::StringLiteral(name) => {
                        (&member.object, name.value.as_str()?.to_string())
                    }
                    _ => return None,
                },
                Expression::StaticMemberExpression(member) => {
                    (&member.object, member.property.name.to_string())
                }
                _ => return None,
            };
            (symbol_of(object, scoping) == Some(holder))
                .then(|| Some((key.value.as_str()?.to_string(), name)))?
        })
        .collect()
}

fn mappers(
    program: &Program<'_>,
    holder: &Holder,
    refs: &Refs<'_>,
    scoping: &Scoping,
) -> Vec<Mapper> {
    program
        .body
        .iter()
        .filter_map(declared_function)
        .filter_map(|(symbol, function)| {
            let cases = mapper_cases(function, holder.symbol, scoping)?;
            refs.stable(symbol, scoping).then_some(Mapper {
                symbol,
                statement: function.span,
                cases,
            })
        })
        .collect()
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
    folds: Vec<(Span, String)>,
    removals: Vec<Span>,
    note: Option<&'static str>,
    checked: Vec<Span>,
    symbols: Vec<SymbolId>,
}

fn bound_names(scoping: &Scoping) -> FxHashSet<&str> {
    scoping.symbol_names().collect()
}

fn analyse(program: &Program<'_>, scoping: &Scoping, nested_root: bool) -> Option<Plan> {
    if has_module_syntax(program) || !program.source_text.contains(GETTER_MARK) {
        return None;
    }
    let refs = scan(program, scoping);
    let names = names(program, scoping);
    let helpers = no_op_helpers(program, &refs, scoping);
    let found = getters(program, (&refs, scoping), &names, &helpers);
    let (getter, holder) = found
        .iter()
        .find_map(|g| Some((g, holder(program, g, &refs, scoping)?)))?;
    let mappers = mappers(program, &holder, &refs, scoping);
    if mappers.is_empty() {
        return None;
    }
    let refuse = |note| {
        Some(Plan {
            folds: Vec::new(),
            removals: Vec::new(),
            note: Some(note),
            checked: Vec::new(),
            symbols: Vec::new(),
        })
    };
    if refs.has_with || may_call_eval(scoping) {
        return refuse("the program has a with statement or may call eval");
    }
    if !program.body[..holder.index].iter().all(runs_no_code) {
        return refuse("code before the global object is fetched may call a mapper");
    }
    let skipped: Vec<Span> = [getter.statement, holder.statement]
        .into_iter()
        .chain(mappers.iter().map(|m| m.statement))
        .collect();
    let bound = bound_names(scoping);
    let machinery: Vec<SymbolId> = [getter.symbol, holder.symbol]
        .into_iter()
        .chain(mappers.iter().map(|m| m.symbol))
        .collect();
    if shared_elsewhere(program, scoping, nested_root, &machinery) {
        return refuse("another input names its machinery");
    }
    let mut folds = Vec::new();
    let mut complete = true;
    for mapper in &mappers {
        let calls = refs.calls_of(mapper.symbol);
        if calls.len() != refs.of(mapper.symbol).len() {
            complete = false;
        }
        for call in calls {
            let name = match call.args.as_slice() {
                [Some(Const::Str(key))] if !call.spread => mapper
                    .cases
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, n)| n.as_str()),
                _ => None,
            };
            match name {
                Some(name)
                    if STANDARD_GLOBALS.contains(&name)
                        && !bound.contains(name)
                        && !is_foreign_global(name) =>
                {
                    folds.push((call.span, name.to_string()))
                }
                _ => complete = false,
            }
        }
    }
    let holder_uses = refs
        .of(holder.symbol)
        .iter()
        .all(|r| mappers.iter().any(|m| inside(m.statement, r.span)));
    let removals = if complete && holder_uses {
        skipped.clone()
    } else {
        Vec::new()
    };
    let note = (!folds.is_empty() && removals.is_empty())
        .then_some("its getter, holder or mapper is still used, so they are kept");
    Some(Plan {
        folds,
        removals,
        note,
        checked: skipped,
        symbols: machinery,
    })
}

fn apply<'a>(plan: Plan, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
    let mut folds = ExprEdits::new();
    for (span, name) in &plan.folds {
        let Some(name) = STANDARD_GLOBALS
            .iter()
            .find(|global| **global == name.as_str())
        else {
            return;
        };
        let replacement = Expression::new_identifier(*span, *name, &*cx);
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

impl<'a> Pass<'a> for InlineGlobals {
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
        let Some(plan) = analyse(program, cx.scoping(), cx.opts.nested_root) else {
            return;
        };
        if plan.folds.is_empty() {
            if let Some(note) = plan.note {
                cx.note(format!("js-confuser global concealing kept: {note}"), None);
            }
            return;
        }
        let replacements = plan
            .folds
            .iter()
            .filter_map(|(span, name)| {
                let name = STANDARD_GLOBALS
                    .iter()
                    .find(|global| **global == name.as_str())?;
                Some((*span, Expression::new_identifier(*span, *name, &*cx)))
            })
            .collect();
        let Some(judged) = judge_replaced(
            program,
            cx,
            replacements,
            (&GLOBAL_OBJECT, &plan.checked),
            &plan.symbols,
        ) else {
            return;
        };
        if !cx.opts.unsafe_transforms && !judged.intact {
            cx.note("js-confuser global concealing kept: the program may replace globalThis, Object or String, which its global getter reads", None);
            return;
        }
        if judged.reach.reach.is_some() || judged.reach.reach_by_code.is_some() {
            cx.note("js-confuser global concealing kept: code may reach its machinery through the global object or code text", None);
            return;
        }
        let mut plan = plan;
        if !plan.removals.is_empty() && names_observed(&judged.reach) {
            plan.removals.clear();
            cx.note("js-confuser global concealing: code may observe the global object's names, so its machinery is kept", None);
        } else if !plan.removals.is_empty() && judged.reach.outside_premise.is_some() {
            cx.note("js-confuser global concealing: the machinery was dropped under the outside-code premise", None);
        }
        if let Some(note) = plan.note {
            cx.note(format!("js-confuser global concealing: {note}"), None);
        }
        apply(plan, program, cx);
    }
}
