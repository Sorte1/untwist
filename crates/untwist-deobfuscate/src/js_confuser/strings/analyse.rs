use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::semantic::{has_module_syntax, is_foreign_global, may_call_eval};
use untwist_eval::fold::Const;

use super::super::base91;
use super::super::names::{names, Names};
use super::super::refs::{inside, no_op_helpers, scan, shared_elsewhere, Refs};
use super::super::templates::{matches_prelude, PRELUDE_LENGTHS};
use super::super::tokens::FIXED;
use super::moved::{declared_bare, find_moved_prelude, literal_array, written_once, LiteralArray};
use super::pairs::{function_symbol, text_of, Decoding, Moved, Pair, PairScan};
use super::sandbox::{outside_symbols, Sandbox};
use crate::pure_calls::literal_data;

const PRELUDE_MARK: &str = "globalThis";
const PROGRAM_MARKS: [&str; 2] = ["return globalThis", "TextDecoder"];

#[derive(Default)]
pub(super) struct Plan {
    pub(super) folds: Vec<(Span, String)>,
    pub(super) block_removals: Vec<Span>,
    pub(super) top_removals: Vec<Span>,
    pub(super) checked: Vec<Span>,
    pub(super) symbols: Vec<SymbolId>,
    pub(super) kept: Option<&'static str>,
    pub(super) evaluated: bool,
    pub(super) dead_vars: Vec<SymbolId>,
    pub(super) array_reads: Vec<(Span, Const)>,
}

struct Prelude {
    statements: Vec<Span>,
    span: Span,
    buffer_to_string: SymbolId,
    last_index: usize,
    vars: Vec<SymbolId>,
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
                vars: Vec::new(),
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
    write: Option<Span>,
}

fn assigned_string(
    program: &Program<'_>,
    symbol: SymbolId,
    (refs, scoping): (&Refs<'_>, &Scoping),
) -> Option<Packed> {
    let write = written_once(symbol, refs, scoping)?;
    if !declared_bare(program, symbol) {
        return None;
    }
    program
        .body
        .iter()
        .enumerate()
        .find_map(|(index, statement)| {
            let Statement::ExpressionStatement(expression) = statement else {
                return None;
            };
            let Expression::AssignmentExpression(assign) = &expression.expression else {
                return None;
            };
            let (
                AssignmentTarget::AssignmentTargetIdentifier(target),
                Expression::StringLiteral(literal),
            ) = (&assign.left, &assign.right)
            else {
                return None;
            };
            (target.span == write
                && assign.operator == oxc_syntax::operator::AssignmentOperator::Assign)
                .then(|| Packed {
                    symbol,
                    statement: statement.span(),
                    index,
                    units: literal.value.encode_utf16().collect(),
                    write: Some(write),
                })
        })
}

fn packed_string(
    program: &Program<'_>,
    symbol: SymbolId,
    (refs, scoping): (&Refs<'_>, &Scoping),
) -> Option<Packed> {
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
            if declaration.kind != VariableDeclarationKind::Var {
                return None;
            }
            return match &declarator.init {
                Some(Expression::StringLiteral(literal)) => Some(Packed {
                    symbol,
                    statement: statement.span(),
                    index,
                    units: literal.value.encode_utf16().collect(),
                    write: None,
                }),
                None => assigned_string(program, symbol, (refs, scoping)),
                Some(_) => None,
            };
        }
    }
    None
}

fn closed(
    (refs, scoping): (&Refs<'_>, &Scoping),
    region: Span,
    allowed: &dyn Fn(SymbolId) -> bool,
    free: bool,
) -> bool {
    refs.all
        .iter()
        .filter(|(r, _)| inside(region, r.span))
        .all(|(r, name)| match r.symbol {
            None => free && FIXED.contains(name) && !is_foreign_global(name),
            Some(symbol) => inside(region, scoping.symbol_span(symbol)) || allowed(symbol),
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

fn count_refs(refs: &Refs<'_>, region: Span, symbol: SymbolId) -> usize {
    refs.of(symbol)
        .iter()
        .filter(|r| inside(region, r.span))
        .count()
}

struct Env<'k, 'p> {
    program: &'k Program<'p>,
    scoping: &'k Scoping,
    refs: &'k Refs<'p>,
    helpers: &'k FxHashSet<SymbolId>,
    array: Option<&'k LiteralArray>,
    prelude: &'k Prelude,
    strict: bool,
}

impl Env<'_, '_> {
    fn array_symbol(&self) -> Option<SymbolId> {
        self.array.map(|a| a.symbol)
    }

    fn own_writes(&self, symbol: SymbolId, own: Span, moved: bool) -> bool {
        if !moved {
            return self.refs.stable(symbol, self.scoping);
        }
        let writes: Vec<Span> = self
            .refs
            .of(symbol)
            .iter()
            .filter(|r| r.write)
            .map(|r| r.span)
            .collect();
        self.scoping.symbol_redeclarations(symbol).is_empty()
            && matches!(writes.as_slice(), [only] if inside(own, *only))
            && count_refs(self.refs, own, symbol) == 2
    }

    fn moved_fits(&self, moved: &Moved) -> bool {
        let Some(function) = moved.function else {
            return false;
        };
        let calls = self.refs.calls_of(function);
        !text_of(self.program.source_text, moved.span).contains("arguments")
            && self.refs.stable(function, self.scoping)
            && self.refs.of(function).len() == calls.len()
            && calls
                .iter()
                .all(|call| !call.spread && call.args.len() <= moved.first_param)
    }

    fn sandbox(&self, bytes: bool) -> Sandbox<'_> {
        Sandbox {
            source: self.program.source_text,
            scoping: self.scoping,
            strict: self.strict,
            array: self.array.map(|a| (a.symbol, a.init)),
            helpers: self.helpers,
            bytes: bytes.then_some(self.prelude.buffer_to_string),
        }
    }
}

struct Linked {
    packed: SymbolId,
    bytes: bool,
}

fn check_pair(env: &Env<'_, '_>, pair: &Pair) -> Result<Linked, &'static str> {
    let moved = pair.moved.is_some();
    let [decoder_at, getter_at] = pair.statements;
    if !env.own_writes(pair.decoder, decoder_at, moved)
        || !env.own_writes(pair.getter, getter_at, moved)
    {
        return Err("a decoder or getter function is redeclared or reassigned");
    }
    let own_decoder_refs = if moved { 2 } else { 0 };
    let decoder_refs = env.refs.of(pair.decoder);
    let outside_getter = decoder_refs
        .iter()
        .filter(|r| !inside(getter_at, r.span))
        .count();
    if outside_getter != own_decoder_refs
        || count_refs(env.refs, decoder_at, pair.decoder) != own_decoder_refs
    {
        return Err("a decoder function is used outside its getter");
    }
    if pair.moved.as_ref().is_some_and(|m| !env.moved_fits(m)) {
        return Err("a getter moved into a parameter may be passed in by a caller or read through arguments");
    }
    let buffer = env.prelude.buffer_to_string;
    let through = count_refs(env.refs, decoder_at, buffer);
    let returns_bytes = pair.returns_through == Some(buffer) && through == 1;
    let evaluated = matches!(pair.decoding, Decoding::Evaluated(_));
    let returns = match evaluated {
        true => through == 0 || returns_bytes,
        false => returns_bytes,
    };
    let linked = returns
        && pair.calls_decoder == Some(pair.decoder)
        && count_refs(env.refs, getter_at, pair.decoder) == 1;
    let Some(packed) = pair.slices.filter(|_| linked) else {
        return Err("a decoder does not return through bufferToString or a getter does not call its decoder on the packed string");
    };
    if count_refs(env.refs, getter_at, packed) != 1 {
        return Err("a getter reads its packed string twice");
    }
    let array = env.array_symbol();
    let bytes = evaluated && returns_bytes;
    let decoder_allowed = |symbol: SymbolId| {
        env.helpers.contains(&symbol)
            || Some(symbol) == array
            || (symbol == buffer && (bytes || !evaluated))
            || (moved && symbol == pair.decoder)
    };
    let getter_allowed = |symbol: SymbolId| {
        env.helpers.contains(&symbol)
            || Some(symbol) == array
            || symbol == pair.decoder
            || symbol == packed
            || (moved && symbol == pair.getter)
    };
    let scope = (env.refs, env.scoping);
    if !closed(scope, decoder_at, &decoder_allowed, !evaluated)
        || !closed(scope, getter_at, &getter_allowed, true)
    {
        return Err(
            "a decoder or getter reads a name other than the packed string and bufferToString",
        );
    }
    if !env
        .refs
        .of(pair.getter)
        .iter()
        .all(|r| inside(pair.block, r.span))
    {
        return Err("a getter is used outside the block that declares it");
    }
    Ok(Linked { packed, bytes })
}

fn decode_calls(
    env: &Env<'_, '_>,
    pair: &Pair,
    packed: &Packed,
    bytes: bool,
) -> (Vec<(Span, String)>, bool) {
    let calls = env.refs.calls_of(pair.getter);
    let sandbox = env.sandbox(bytes);
    let name = env.scoping.symbol_name(pair.decoder);
    let outside: Vec<SymbolId> = match &pair.decoding {
        Decoding::Evaluated(_) => outside_symbols(env.refs, env.scoping, pair.statements[0])
            .into_iter()
            .filter(|s| *s != pair.decoder)
            .collect(),
        Decoding::Base91(_) => Vec::new(),
    };
    let array = env.array.map(|a| (a.symbol, a.values.as_slice()));
    let mut folds = Vec::new();
    let mut every = true;
    for call in calls {
        let args = match call.resolved(array).as_slice() {
            [Some(Const::Num(start)), Some(Const::Num(length))] if !call.spread => {
                Some((*start, *length))
            }
            _ => None,
        };
        let decoded = args.and_then(|(start, length)| {
            let raw = base91::slice(&packed.units, start, length)?;
            match &pair.decoding {
                Decoding::Base91(table) => base91::utf8(&base91::bytes(table, &raw)),
                Decoding::Evaluated(text) => sandbox.decode(&outside, (text, name), &raw),
            }
        });
        match decoded {
            Some(text) if inside(pair.block, call.span) => folds.push((call.span, text)),
            _ => every = false,
        }
    }
    let own = if pair.moved.is_some() { 2 } else { 0 };
    let refs_total = env.refs.of(pair.getter).len() - own.min(env.refs.of(pair.getter).len());
    let complete = every && refs_total == calls.len() && refs_total == folds.len();
    (folds, complete)
}

fn locate_prelude(
    program: &Program<'_>,
    (refs, scoping, names): (&Refs<'_>, &Scoping, &Names),
    helpers: &FxHashSet<SymbolId>,
    array: Option<&LiteralArray>,
) -> Option<Prelude> {
    if let Some(found) = find_prelude(program, names) {
        return Some(found);
    }
    let sandbox = Sandbox {
        source: program.source_text,
        scoping,
        strict: program.has_use_strict_directive(),
        array: array.map(|a| (a.symbol, a.init)),
        helpers,
        bytes: None,
    };
    let moved = find_moved_prelude(program, (refs, scoping, names), &sandbox)?;
    Some(Prelude {
        statements: moved.statements,
        span: moved.span,
        buffer_to_string: moved.buffer_to_string,
        last_index: moved.last_index,
        vars: moved.vars,
    })
}

pub(super) fn analyse(
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
    let refs = scan(program, scoping);
    let array = literal_array(program, &refs, scoping);
    let names = match &array {
        Some(found) => names(program, scoping).with_literals(found.symbol, &found.values),
        None => names(program, scoping),
    };
    let mut scanner = PairScan {
        source: program.source_text,
        scoping,
        names: &names,
        pairs: Vec::new(),
        depth: 0,
        block: program.span,
        params: None,
    };
    scanner.visit_program(program);
    let pairs = scanner.pairs;
    if pairs.is_empty() {
        return Ok(Plan::default());
    }
    if has_module_syntax(program) {
        return Err("the input is a module");
    }
    if refs.has_with || may_call_eval(scoping) {
        return Err("the program has a with statement or may call eval, which can rebind the machinery's names");
    }
    let helpers = no_op_helpers(program, &refs, scoping);
    let Some(prelude) = locate_prelude(program, (&refs, scoping, &names), &helpers, array.as_ref())
    else {
        return Err("its getGlobal and bufferToString prelude is not the js-confuser template");
    };
    let env = Env {
        program,
        scoping,
        refs: &refs,
        helpers: &helpers,
        array: array.as_ref(),
        prelude: &prelude,
        strict: program.has_use_strict_directive(),
    };
    plan(&env, &pairs, nested_root)
}

fn prelude_closed(env: &Env<'_, '_>, prelude_symbols: &[SymbolId]) -> bool {
    let array = env.array_symbol();
    let vars = &env.prelude.vars;
    let allowed = |symbol: SymbolId| {
        env.helpers.contains(&symbol) || Some(symbol) == array || vars.contains(&symbol)
    };
    prelude_symbols
        .iter()
        .all(|s| env.refs.stable(*s, env.scoping))
        && closed((env.refs, env.scoping), env.prelude.span, &allowed, true)
}

fn plan(env: &Env<'_, '_>, pairs: &[Pair], nested_root: bool) -> Result<Plan, &'static str> {
    let (program, refs, scoping, prelude) = (env.program, env.refs, env.scoping, env.prelude);
    let prelude_symbols = top_level_symbols(&prelude.statements, scoping);
    if !prelude_closed(env, &prelude_symbols) {
        return Err(
            "a name of its prelude is written elsewhere or its prelude reads a name from outside",
        );
    }
    let mut packed: Option<Packed> = None;
    let mut folds = Vec::new();
    let mut removals = Vec::new();
    let mut machinery: Vec<Span> = prelude.statements.clone();
    let mut kept_pairs = false;
    let mut any_evaluated = false;
    let mut top_symbols: Vec<SymbolId> = prelude_symbols
        .iter()
        .chain(prelude.vars.iter())
        .copied()
        .collect();
    for pair in pairs {
        let linked = check_pair(env, pair)?;
        let this_packed = match &packed {
            Some(found) if found.symbol == linked.packed => found,
            Some(_) => return Err("getters read different packed strings"),
            None => {
                let Some(found) = packed_string(program, linked.packed, (refs, scoping)) else {
                    return Err("the packed string is not a top-level var with a string literal");
                };
                packed.insert(found)
            }
        };
        let (decoded, every) = decode_calls(env, pair, this_packed, linked.bytes);
        any_evaluated |= matches!(pair.decoding, Decoding::Evaluated(_));
        folds.extend(decoded);
        machinery.extend(pair.statements);
        if pair.top_level {
            top_symbols.extend([pair.decoder, pair.getter]);
        }
        if let Some(function) = pair.moved.as_ref().and_then(|m| m.function) {
            top_symbols.push(function);
        }
        match every {
            true => removals.extend(pair.statements),
            false => kept_pairs = true,
        }
    }
    let Some(packed) = packed else {
        return Err("no packed string was found");
    };
    let packed_writes_ok = scoping.symbol_redeclarations(packed.symbol).is_empty()
        && refs
            .of(packed.symbol)
            .iter()
            .filter(|r| r.write)
            .all(|r| Some(r.span) == packed.write);
    if !packed_writes_ok {
        return Err("the packed string is reassigned");
    }
    machinery.push(packed.statement);
    top_symbols.push(packed.symbol);
    let upto = prelude.last_index.max(packed.index);
    let inert: Vec<Span> = machinery
        .iter()
        .copied()
        .chain(env.array.map(|a| a.statement))
        .collect();
    if !ordered_before(program, upto, &inert) {
        return Err("code before the prelude may run a getter before its packed string is set");
    }
    let named: Vec<SymbolId> = top_symbols
        .iter()
        .copied()
        .chain(pairs.iter().flat_map(|pair| [pair.decoder, pair.getter]))
        .collect();
    if shared_elsewhere(program, scoping, nested_root, &named) {
        return Err("another input names the machinery");
    }
    let is_top = |span: &Span| program.body.iter().any(|s| s.span() == *span);
    let block_removals: Vec<Span> = removals.iter().copied().filter(|s| !is_top(s)).collect();
    let mut top_removals: Vec<Span> = removals.iter().copied().filter(is_top).collect();
    let removed_spans: Vec<Span> = removals
        .iter()
        .copied()
        .chain(prelude.statements.iter().copied())
        .chain([packed.statement])
        .collect();
    let only_machinery_uses = prelude_symbols
        .iter()
        .chain(prelude.vars.iter())
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
    let dead_vars: Vec<SymbolId> = prelude
        .vars
        .iter()
        .copied()
        .chain(packed.write.map(|_| packed.symbol))
        .collect();
    let array_reads: Vec<(Span, Const)> = env
        .array
        .and_then(|array| {
            let reads = refs.index_reads.get(&array.symbol)?;
            Some(
                reads
                    .iter()
                    .filter(|(span, _)| {
                        !folds.iter().any(|(fold, _)| inside(*fold, *span))
                            && !machinery.iter().any(|m| inside(*m, *span))
                    })
                    .filter_map(|(span, index)| {
                        let whole = index.fract() == 0.0 && *index >= 0.0;
                        whole
                            .then(|| array.values.get(*index as usize).cloned())
                            .flatten()
                            .map(|value| (*span, value))
                    })
                    .collect(),
            )
        })
        .unwrap_or_default();
    Ok(Plan {
        folds,
        block_removals,
        top_removals,
        checked: machinery,
        symbols: top_symbols,
        kept,
        evaluated: any_evaluated,
        dead_vars,
        array_reads,
    })
}
