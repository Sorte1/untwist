use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_core::semantic::{has_module_syntax, is_foreign_global, may_call_eval};
use untwist_eval::fold::Const as Value;
use untwist_eval::fold::{const_to_expression, Const};

use super::base91;
use super::names::{names, Names};
use super::refs::{
    inside, judge_replaced, names_observed, no_op_helpers, scan, shared_elsewhere, Refs,
};
use super::templates::{matches_decoder, matches_getter, matches_prelude, PRELUDE_LENGTHS};
use super::tokens::FIXED;
use crate::pure_calls::literal_data;

pub const PASS: &str = "decode-js-confuser-strings";

const TABLE_LEN: usize = 91;
const PRELUDE_MARK: &str = "globalThis";
const PROGRAM_MARKS: [&str; 2] = ["return globalThis", "TextDecoder"];
const DECODER_MARKS: [&str; 3] = ["91", "0x5b", "0X5B"];
const MAX_DECODER_TEXT: usize = 4096;

static RELIED_ON: Reliance = Reliance {
    roots: &[
        "globalThis",
        "Object",
        "String",
        "Array",
        "TextDecoder",
        "Uint8Array",
    ],
    members: &[],
};

#[derive(Default)]
pub struct DecodeStrings {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DecodeStrings::default())
}

struct Pair {
    decoder: SymbolId,
    getter: SymbolId,
    returns_through: Option<SymbolId>,
    calls_decoder: Option<SymbolId>,
    slices: Option<SymbolId>,
    statements: [Span; 2],
    block: Span,
    top_level: bool,
    table: Vec<u16>,
}

struct PairScan<'s> {
    source: &'s str,
    scoping: &'s Scoping,
    names: &'s Names,
    pairs: Vec<Pair>,
    depth: usize,
    block: Span,
}

fn function_symbol<'k, 'a>(statement: &'k Statement<'a>) -> Option<(SymbolId, &'k Function<'a>)> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    if function.r#async || function.generator {
        return None;
    }
    Some((function.id.as_ref()?.symbol_id.get()?, function))
}

fn callee_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

fn last_return<'k, 'a>(function: &'k Function<'a>) -> Option<&'k CallExpression<'a>> {
    let body = function.body.as_ref()?;
    match body.statements.last()? {
        Statement::ReturnStatement(ret) => match ret.argument.as_ref()?.without_parentheses() {
            Expression::CallExpression(call) => Some(call),
            _ => None,
        },
        _ => None,
    }
}

fn returned_callee(function: &Function<'_>, scoping: &Scoping) -> Option<SymbolId> {
    callee_symbol(&last_return(function)?.callee, scoping)
}

fn getter_links(
    function: &Function<'_>,
    scoping: &Scoping,
) -> (Option<SymbolId>, Option<SymbolId>) {
    let Some(call) = last_return(function) else {
        return (None, None);
    };
    let slices = match call.arguments.as_slice() {
        [Argument::CallExpression(inner)] => match inner.callee.without_parentheses() {
            Expression::ComputedMemberExpression(member) => callee_symbol(&member.object, scoping),
            Expression::StaticMemberExpression(member) => callee_symbol(&member.object, scoping),
            _ => None,
        },
        _ => None,
    };
    (callee_symbol(&call.callee, scoping), slices)
}

fn count_refs(refs: &Refs<'_>, region: Span, symbol: SymbolId) -> usize {
    refs.by_symbol.get(&symbol).map_or(0, |list| {
        list.iter().filter(|r| inside(region, r.span)).count()
    })
}

fn text_of(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

impl PairScan<'_> {
    fn scan(&mut self, statements: &[Statement<'_>]) {
        for window in statements.windows(2) {
            let (Some((decoder, decoder_fn)), Some((getter, getter_fn))) =
                (function_symbol(&window[0]), function_symbol(&window[1]))
            else {
                continue;
            };
            let decoder_text = text_of(self.source, window[0].span());
            if decoder_text.len() > MAX_DECODER_TEXT
                || !DECODER_MARKS.iter().any(|m| decoder_text.contains(m))
            {
                continue;
            }
            let Some(table) = matches_decoder(self.source, window[0].span(), self.names) else {
                continue;
            };
            if table.len() != TABLE_LEN
                || !matches_getter(self.source, window[1].span(), self.names)
            {
                continue;
            }
            let (calls_decoder, slices) = getter_links(getter_fn, self.scoping);
            self.pairs.push(Pair {
                decoder,
                getter,
                returns_through: returned_callee(decoder_fn, self.scoping),
                calls_decoder,
                slices,
                statements: [window[0].span(), window[1].span()],
                block: self.block,
                top_level: self.depth == 0,
                table,
            });
        }
    }
}

impl<'p> VisitJs<'p> for PairScan<'_> {
    fn visit_program(&mut self, it: &Program<'p>) {
        self.block = it.span;
        self.scan(&it.body);
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'p>) {
        let saved = self.block;
        self.depth += 1;
        self.block = it.span;
        self.scan(&it.statements);
        walk_js::walk_function_body(self, it);
        self.depth -= 1;
        self.block = saved;
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'p>) {
        let saved = self.block;
        self.depth += 1;
        self.block = it.span;
        self.scan(&it.body);
        walk_js::walk_block_statement(self, it);
        self.depth -= 1;
        self.block = saved;
    }
}

struct Prelude {
    statements: Vec<Span>,
    span: Span,
    buffer_to_string: SymbolId,
    last_index: usize,
}

fn find_prelude(program: &Program<'_>, names: &Names) -> Option<Prelude> {
    let body = &program.body;
    for (index, statement) in body.iter().enumerate() {
        let Statement::FunctionDeclaration(_) = statement else {
            continue;
        };
        if !text_of(program.source_text, statement.span()).contains(PRELUDE_MARK) {
            continue;
        }
        for length in PRELUDE_LENGTHS {
            let Some(run) = body.get(index..index + length) else {
                continue;
            };
            let span = Span::new(run[0].span().start, run[length - 1].span().end);
            if !matches_prelude(program.source_text, span, names) {
                continue;
            }
            let (buffer_to_string, _) = function_symbol(&run[length - 1])?;
            return Some(Prelude {
                statements: run.iter().map(GetSpan::span).collect(),
                span,
                buffer_to_string,
                last_index: index + length - 1,
            });
        }
    }
    None
}

struct Packed {
    symbol: SymbolId,
    statement: Span,
    index: usize,
    units: Vec<u16>,
}

fn packed_string(program: &Program<'_>, symbol: SymbolId) -> Option<Packed> {
    for (index, statement) in program.body.iter().enumerate() {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            if id.symbol_id.get() != Some(symbol) {
                continue;
            }
            let Some(Expression::StringLiteral(literal)) = &declarator.init else {
                return None;
            };
            if declaration.kind != VariableDeclarationKind::Var {
                return None;
            }
            return Some(Packed {
                symbol,
                statement: statement.span(),
                index,
                units: literal.value.encode_utf16().collect(),
            });
        }
    }
    None
}

fn stable(symbol: SymbolId, refs: &Refs<'_>, scoping: &Scoping) -> bool {
    refs.stable(symbol, scoping)
}

struct Closure<'k> {
    region: Span,
    allowed: &'k [SymbolId],
    helpers: &'k FxHashSet<SymbolId>,
}

fn closed(refs: &Refs<'_>, scoping: &Scoping, closure: &Closure<'_>) -> bool {
    refs.all
        .iter()
        .filter(|(r, _)| inside(closure.region, r.span))
        .all(|(r, name)| match r.symbol {
            None => FIXED.contains(name) && !is_foreign_global(name),
            Some(symbol) => {
                inside(closure.region, scoping.symbol_span(symbol))
                    || closure.allowed.contains(&symbol)
                    || closure.helpers.contains(&symbol)
            }
        })
}

fn top_level_symbols(spans: &[Span], scoping: &Scoping) -> Vec<SymbolId> {
    let root = scoping.root_scope_id();
    scoping
        .symbol_ids()
        .filter(|symbol| scoping.symbol_scope_id(*symbol) == root)
        .filter(|symbol| {
            let declared = scoping.symbol_span(*symbol);
            spans.iter().any(|span| inside(*span, declared))
        })
        .collect()
}

fn ordered_before(program: &Program<'_>, upto: usize, machinery: &[Span]) -> bool {
    program.body[..upto].iter().all(|statement| {
        let span = statement.span();
        if machinery.iter().any(|m| inside(*m, span)) {
            return true;
        }
        match statement {
            Statement::FunctionDeclaration(_) => true,
            Statement::VariableDeclaration(declaration) => {
                declaration.declarations.iter().all(|declarator| {
                    matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                        && declarator
                            .init
                            .as_ref()
                            .is_none_or(|init| literal_data(init, true))
                })
            }
            _ => false,
        }
    })
}

#[derive(Default)]
struct Plan {
    folds: Vec<(Span, String)>,
    block_removals: Vec<Span>,
    top_removals: Vec<Span>,
    checked: Vec<Span>,
    symbols: Vec<SymbolId>,
    kept: Option<&'static str>,
}

fn refusal(reason: &'static str) -> Result<Plan, &'static str> {
    Err(reason)
}

fn decode_calls(pair: &Pair, packed: &Packed, refs: &Refs<'_>) -> (Vec<(Span, String)>, bool) {
    let calls = refs.calls_of(pair.getter);
    let mut folds = Vec::new();
    let mut every = true;
    for call in calls {
        let args = match call.args.as_slice() {
            [Some(Value::Num(start)), Some(Value::Num(length))] if !call.spread => {
                Some((*start, *length))
            }
            _ => None,
        };
        let decoded = args.and_then(|(start, length)| {
            let raw = base91::slice(&packed.units, start, length)?;
            base91::utf8(&base91::bytes(&pair.table, &raw))
        });
        match decoded {
            Some(text) if inside(pair.block, call.span) => folds.push((call.span, text)),
            _ => every = false,
        }
    }
    let refs_total = refs.by_symbol.get(&pair.getter).map_or(0, Vec::len);
    let complete = every && refs_total == calls.len() && refs_total == folds.len();
    (folds, complete)
}

fn analyse(
    program: &Program<'_>,
    scoping: &Scoping,
    nested_root: bool,
) -> Result<Plan, &'static str> {
    if !PROGRAM_MARKS
        .iter()
        .all(|m| program.source_text.contains(m))
    {
        return Ok(Plan::default());
    }
    let names = names(program, scoping);
    let mut pairs = PairScan {
        source: program.source_text,
        scoping,
        names: &names,
        pairs: Vec::new(),
        depth: 0,
        block: program.span,
    };
    pairs.visit_program(program);
    let pairs = pairs.pairs;
    if pairs.is_empty() {
        return Ok(Plan::default());
    }
    if has_module_syntax(program) {
        return refusal("the input is a module");
    }
    let refs = scan(program, scoping);
    if refs.has_with || may_call_eval(scoping) {
        return refusal("the program has a with statement or may call eval, which can rebind the machinery's names");
    }
    let Some(prelude) = find_prelude(program, &names) else {
        return refusal("its getGlobal and bufferToString prelude is not the js-confuser template");
    };
    let helpers = no_op_helpers(program, &refs, scoping);
    let prelude_symbols = top_level_symbols(&prelude.statements, scoping);
    if !prelude_symbols.iter().all(|s| stable(*s, &refs, scoping))
        || !closed(
            &refs,
            scoping,
            &Closure {
                region: prelude.span,
                allowed: &[],
                helpers: &helpers,
            },
        )
    {
        return refusal(
            "a name of its prelude is written elsewhere or its prelude reads a name from outside",
        );
    }
    let mut packed: Option<Packed> = None;
    let mut folds = Vec::new();
    let mut removals = Vec::new();
    let mut machinery: Vec<Span> = prelude.statements.clone();
    let mut kept_pairs = false;
    let mut top_symbols = prelude_symbols.clone();
    for pair in &pairs {
        if !stable(pair.decoder, &refs, scoping) || !stable(pair.getter, &refs, scoping) {
            return refusal("a decoder or getter function is redeclared or reassigned");
        }
        let decoder_refs = refs
            .by_symbol
            .get(&pair.decoder)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !decoder_refs
            .iter()
            .all(|r| inside(pair.statements[1], r.span))
        {
            return refusal("a decoder function is used outside its getter");
        }
        let linked = pair.returns_through == Some(prelude.buffer_to_string)
            && count_refs(&refs, pair.statements[0], prelude.buffer_to_string) == 1
            && pair.calls_decoder == Some(pair.decoder)
            && count_refs(&refs, pair.statements[1], pair.decoder) == 1;
        let Some(array) = pair.slices.filter(|_| linked) else {
            return refusal("a decoder does not return through bufferToString or a getter does not call its decoder on the packed string");
        };
        if count_refs(&refs, pair.statements[1], array) != 1 {
            return refusal("a getter reads its packed string twice");
        }
        let this_packed = match &packed {
            Some(found) if found.symbol == array => found,
            Some(_) => return refusal("getters read different packed strings"),
            None => {
                let Some(found) = packed_string(program, array) else {
                    return refusal(
                        "the packed string is not a top-level var with a string literal",
                    );
                };
                packed = Some(found);
                packed.as_ref().unwrap_or_else(|| unreachable!())
            }
        };
        let decoder_closed = closed(
            &refs,
            scoping,
            &Closure {
                region: pair.statements[0],
                allowed: &[prelude.buffer_to_string],
                helpers: &helpers,
            },
        );
        let getter_closed = closed(
            &refs,
            scoping,
            &Closure {
                region: pair.statements[1],
                allowed: &[pair.decoder, this_packed.symbol],
                helpers: &helpers,
            },
        );
        if !decoder_closed || !getter_closed {
            return refusal(
                "a decoder or getter reads a name other than the packed string and bufferToString",
            );
        }
        let getter_refs = refs
            .by_symbol
            .get(&pair.getter)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !getter_refs.iter().all(|r| inside(pair.block, r.span)) {
            return refusal("a getter is used outside the block that declares it");
        }
        let (decoded, every) = decode_calls(pair, this_packed, &refs);
        folds.extend(decoded);
        machinery.extend(pair.statements);
        if pair.top_level {
            top_symbols.extend([pair.decoder, pair.getter]);
        }
        match every {
            true => removals.extend(pair.statements),
            false => kept_pairs = true,
        }
    }
    let Some(packed) = packed else {
        return refusal("no packed string was found");
    };
    if !stable(packed.symbol, &refs, scoping) {
        return refusal("the packed string is reassigned");
    }
    machinery.push(packed.statement);
    top_symbols.push(packed.symbol);
    let upto = prelude.last_index.max(packed.index);
    if !ordered_before(program, upto, &machinery) {
        return refusal("code before the prelude may run a getter before its packed string is set");
    }
    let named: Vec<SymbolId> = top_symbols
        .iter()
        .copied()
        .chain(pairs.iter().flat_map(|pair| [pair.decoder, pair.getter]))
        .collect();
    if shared_elsewhere(program, scoping, nested_root, &named) {
        return refusal("another input names the machinery");
    }
    let is_top = |span: &Span| program.body.iter().any(|s| s.span() == *span);
    let block_removals: Vec<Span> = removals
        .iter()
        .copied()
        .filter(|span| !is_top(span))
        .collect();
    let mut top_removals: Vec<Span> = removals.iter().copied().filter(is_top).collect();
    let removed_spans: Vec<Span> = removals
        .iter()
        .copied()
        .chain(prelude.statements.iter().copied())
        .chain([packed.statement])
        .collect();
    let only_machinery_uses = prelude_symbols
        .iter()
        .chain([&packed.symbol])
        .all(|symbol| {
            refs.of(*symbol)
                .iter()
                .all(|r| removed_spans.iter().any(|span| inside(*span, r.span)))
        });
    let kept = match (kept_pairs, only_machinery_uses) {
        (true, _) => Some("a getter is used other than in a call with literal arguments that decodes, so it and the prelude are kept"),
        (false, false) => Some("the prelude or packed string is used outside the machinery, so they are kept"),
        _ => None,
    };
    if kept.is_none() {
        top_removals.extend(prelude.statements.iter().copied());
        top_removals.push(packed.statement);
    }
    Ok(Plan {
        folds,
        block_removals,
        top_removals,
        checked: machinery,
        symbols: top_symbols,
        kept,
    })
}

fn apply<'a>(
    plan: &Plan,
    removals_list: &[Span],
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let mut folds = ExprEdits::new();
    for (span, text) in &plan.folds {
        let Some(replacement) = const_to_expression(&Const::Str(text.clone()), *span, &*cx) else {
            return false;
        };
        if !folds.insert(*span, replacement) {
            return false;
        }
    }
    let wanted = folds.len();
    let applied = folds.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(applied);
    if applied != wanted {
        cx.note(format!("js-confuser string machinery kept: {applied} of {wanted} concealed strings could be put in place"), None);
        return false;
    }
    let mut removals = StmtEdits::new();
    for span in removals_list {
        removals.remove(*span);
    }
    let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(removed);
    true
}

impl<'a> Pass<'a> for DecodeStrings {
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
        let plan = match analyse(program, cx.scoping(), cx.opts.nested_root) {
            Ok(plan)
                if plan.folds.is_empty()
                    && plan.block_removals.is_empty()
                    && plan.top_removals.is_empty() =>
            {
                self.settled = true;
                return;
            }
            Ok(plan) => plan,
            Err(reason) => {
                self.settled = true;
                cx.note(
                    format!("js-confuser string concealing kept: {reason}"),
                    None,
                );
                return;
            }
        };
        if plan.folds.is_empty() {
            self.settled = true;
        }
        let replacements = plan
            .folds
            .iter()
            .filter_map(|(span, text)| {
                Some((
                    *span,
                    const_to_expression(&Const::Str(text.clone()), *span, &*cx)?,
                ))
            })
            .collect();
        let Some(judged) = judge_replaced(
            program,
            cx,
            replacements,
            (&RELIED_ON, &plan.checked),
            &plan.symbols,
        ) else {
            return;
        };
        if !cx.opts.unsafe_transforms && !judged.intact {
            cx.note("js-confuser string concealing kept: the program may replace a standard built-in the decoder uses (String, Array, Object, globalThis, TextDecoder, Uint8Array)", None);
            return;
        }
        if judged.reach.reach.is_some() || judged.reach.reach_by_code.is_some() {
            cx.note("js-confuser string concealing kept: code may reach the machinery's names through the global object or code text", None);
            return;
        }
        let mut removals = plan.block_removals.clone();
        if names_observed(&judged.reach) {
            cx.note(
                "js-confuser string machinery: code may observe the global object's names, so its top-level string machinery is kept",
                judged.reach.observed,
            );
        } else {
            removals.extend(plan.top_removals.iter().copied());
            if judged.reach.outside_premise.is_some() && !plan.top_removals.is_empty() {
                cx.note("js-confuser string machinery: the machinery was dropped under the outside-code premise", None);
            }
        }
        if !plan.folds.is_empty() {
            cx.note(
                format!(
                    "{} js-confuser concealed string(s) decoded",
                    plan.folds.len()
                ),
                None,
            );
        }
        if let Some(kept) = plan.kept {
            cx.note(format!("js-confuser string machinery kept: {kept}"), None);
        }
        if !apply(&plan, &removals, program, cx) {
            self.settled = true;
        }
    }
}
