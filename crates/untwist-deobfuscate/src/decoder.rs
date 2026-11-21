use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::{call_parts, var_function_or_declaration};
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::rename::Renames;
use untwist_core::pass::{Cx, Interest, Pass};

use crate::init_order::{early_or_written, pass_done, read_before_declaration};
use crate::notes;
use crate::probe::contains_expression;
use crate::proof::{
    declared_text, reachable_by_name, redeclared, watch_texts, writes_outside, TextWatch, Watched,
};
use crate::stamp::{
    declaration_symbol, decoder_index, decoder_name, owning_unit, reached_outside_its_block,
    string_array_unit, IndexAllocator, RefusedByDecode,
};
use crate::wrapper_speculation::{
    branched_writes_outside, declared_by_function, guarded_writes_outside,
};

const SHORT_DECODER_BODY: usize = 4;

const LEADING_INDEX_STATEMENTS: usize = 2;

pub struct DecoderShape {
    pub statement: Span,
    pub symbol: SymbolId,
    pub already_stamped: Option<usize>,
    pub bound: bool,
}

fn array_alias<'k, 'a>(stmt: &'k Statement<'a>) -> Option<&'k str> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = declaration.declarations.first()?;
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let (callee, arguments) = call_parts(declarator.init.as_ref()?)?;
    if !arguments.is_empty() {
        return None;
    }
    string_array_unit(identifier_name(callee)?)?;
    Some(id.name.as_str())
}

fn indexes_through_alias(statements: &[Statement<'_>]) -> bool {
    let Some(position) = statements
        .iter()
        .position(|statement| array_alias(statement).is_some())
    else {
        return false;
    };
    array_alias(&statements[position]).is_some_and(|alias| {
        statements[position + 1..]
            .iter()
            .any(|statement| indexes_alias(statement, alias))
    })
}

fn is_array_call(expr: &Expression<'_>) -> bool {
    call_parts(expr).is_some_and(|(callee, arguments)| {
        arguments.is_empty()
            && identifier_name(callee)
                .and_then(string_array_unit)
                .is_some()
    })
}

fn returns_an_array_index(statements: &[Statement<'_>]) -> bool {
    statements.len() <= SHORT_DECODER_BODY
        && statements.iter().any(|statement| {
            matches!(statement, Statement::ReturnStatement(_)) && indexes_array_call(statement)
        })
}

fn indexes_array_call(stmt: &Statement<'_>) -> bool {
    contains_expression(stmt, |expr| {
        let Expression::ComputedMemberExpression(member) = expr else {
            return false;
        };
        is_array_call(member.object.without_parentheses())
    })
}

fn indexes_alias(stmt: &Statement<'_>, alias: &str) -> bool {
    contains_expression(stmt, |expr| {
        let Expression::ComputedMemberExpression(member) = expr else {
            return false;
        };
        identifier_name(&member.object) == Some(alias)
    })
}

fn declares_an_array_index(statements: &[Statement<'_>]) -> bool {
    statements
        .iter()
        .take(LEADING_INDEX_STATEMENTS)
        .any(|statement| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return false;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                return false;
            };
            matches!(
                declarator.init.as_ref().map(Expression::without_parentheses),
                Some(Expression::ComputedMemberExpression(member))
                    if is_array_call(member.object.without_parentheses())
            )
        })
}

pub fn match_decoder(stmt: &Statement<'_>) -> Option<DecoderShape> {
    let declaration = var_function_or_declaration(stmt)?;
    let body = declaration.function.body.as_ref()?;
    let statements = body.statements.as_slice();
    if !indexes_through_alias(statements)
        && !returns_an_array_index(statements)
        && !declares_an_array_index(statements)
    {
        return None;
    }
    owning_unit(declaration.function)?;
    Some(DecoderShape {
        statement: stmt.span(),
        symbol: declaration_symbol(stmt)?,
        already_stamped: decoder_index(declaration.name),
        bound: matches!(stmt, Statement::VariableDeclaration(_)),
    })
}

struct Search<'a, const WATCHING: bool> {
    found: Vec<(DecoderShape, &'a str)>,
    watch: TextWatch<'a>,
}

impl<'a, const WATCHING: bool> VisitJs<'a> for Search<'a, WATCHING> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(shape) = match_decoder(it) {
            self.found.push((shape, declared_text(it)));
        }
        walk_js::walk_statement(self, it);
    }

    watch_texts!();
}

fn searched<'a, const WATCHING: bool>(
    program: &Program<'a>,
    watching: Vec<&'a str>,
) -> (Vec<(DecoderShape, &'a str)>, TextWatch<'a>) {
    let mut search = Search::<WATCHING> {
        found: Vec::new(),
        watch: TextWatch::over(watching),
    };
    search.visit_program(program);
    (search.found, search.watch)
}

fn top_level_decoders<'a>(program: &Program<'a>) -> Vec<&'a str> {
    program
        .body
        .iter()
        .filter(|statement| {
            match_decoder(statement).is_some_and(|shape| shape.already_stamped.is_none())
        })
        .map(declared_text)
        .collect()
}

fn unstamped_decoders(
    program: &Program<'_>,
    scoping: &Scoping,
) -> (Vec<DecoderShape>, Watched, usize) {
    let watching = top_level_decoders(program);
    let (found, watch) = if watching.is_empty() {
        searched::<false>(program, watching)
    } else {
        searched::<true>(program, watching)
    };
    let total = found.len();
    let (unstamped, names): (Vec<DecoderShape>, Vec<&str>) = found
        .into_iter()
        .filter(|(shape, _)| shape.already_stamped.is_none())
        .unzip();
    let reachable: Vec<bool> = unstamped
        .iter()
        .map(|shape| reachable_by_name(scoping, shape.symbol))
        .collect();
    let watched = watch.resolve(program, &names, &reachable);
    (unstamped, watched, total)
}

#[derive(Default)]
pub struct FindDecoders {
    refused_by_decode: RefusedByDecode,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(FindDecoders::default())
}

pub fn boxed_with<'a>(refused_by_decode: RefusedByDecode) -> Box<dyn Pass<'a> + 'a> {
    Box::new(FindDecoders { refused_by_decode })
}

impl<'a> Pass<'a> for FindDecoders {
    fn name(&self) -> &'static str {
        "find-decoders"
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        let (unstamped, watched, total) = unstamped_decoders(program, cx.scoping());
        if unstamped.is_empty() {
            return;
        }
        let refused = self.refused_by_decode.borrow();
        let bound: Vec<(Span, SymbolId)> = unstamped
            .iter()
            .filter(|shape| shape.bound && !refused.contains(&shape.statement))
            .map(|shape| (shape.statement, shape.symbol))
            .collect();
        let early = read_before_declaration(program, cx.scoping(), &bound);
        let speculated = speculated_decoders(program, cx, &unstamped, &watched);
        let guarded = guarded_decoders(program, cx.scoping(), &unstamped, &watched, &speculated);
        let mut indices = IndexAllocator::decoders(cx.scoping());
        let mut renames = Renames::new();
        let mut deferred = Vec::new();
        for (index, shape) in unstamped.iter().enumerate() {
            if refused.contains(&shape.statement) {
                continue;
            }
            if let Some(note) = early_or_written(cx.scoping(), &early, shape.symbol) {
                cx.note(note, Some(shape.statement));
                continue;
            }
            let written = speculated.get(&shape.symbol);
            let unwritten = written.is_some() || guarded.contains(&shape.symbol);
            let checked = (cx.scoping(), &watched, unwritten);
            if let Some((note, span)) = decoder_refusal(checked, shape, index) {
                cx.note(note, Some(span));
                continue;
            }
            if let Some(writes) = written {
                deferred.push(cx.scoping().symbol_span(shape.symbol));
                deferred.extend(writes);
            }
            let name = decoder_name(indices.allocate());
            renames.insert(shape.symbol, Ident::from_str_in(&name, &*cx));
        }
        drop(refused);
        crate::legacy_reach::defer_escapes(&deferred);
        if renames.is_empty() {
            return;
        }
        let count = renames.apply(program, cx.scoping());
        cx.changed_by(count);
        pass_done(cx.opts.shares_init_order, count > 0);
        cx.progress(notes::decoders_found(total), None);
    }
}

fn speculated_decoders(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    unstamped: &[DecoderShape],
    watched: &Watched,
) -> FxHashMap<SymbolId, Vec<Span>> {
    if !cx.opts.unsafe_transforms || !crate::legacy_reach::speculates_escapes() {
        return FxHashMap::default();
    }
    let scoping = cx.scoping();
    let homes: FxHashMap<SymbolId, Span> = unstamped
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            !shape.bound
                && declared_by_function(scoping, shape.symbol)
                && writes_outside(
                    scoping,
                    shape.symbol,
                    shape.statement,
                    &watched.writes,
                    *index,
                )
        })
        .map(|(_, shape)| (shape.symbol, shape.statement))
        .collect();
    if homes.is_empty() {
        return FxHashMap::default();
    }
    let candidates: FxHashSet<SymbolId> = homes.keys().copied().collect();
    branched_writes_outside(program, scoping, &candidates, &homes)
}

fn guarded_decoders(
    program: &Program<'_>,
    scoping: &Scoping,
    unstamped: &[DecoderShape],
    watched: &Watched,
    speculated: &FxHashMap<SymbolId, Vec<Span>>,
) -> FxHashSet<SymbolId> {
    let homes: FxHashMap<SymbolId, Span> = unstamped
        .iter()
        .enumerate()
        .filter(|(index, shape)| {
            !shape.bound
                && !speculated.contains_key(&shape.symbol)
                && declared_by_function(scoping, shape.symbol)
                && writes_outside(
                    scoping,
                    shape.symbol,
                    shape.statement,
                    &watched.writes,
                    *index,
                )
        })
        .map(|(_, shape)| (shape.symbol, shape.statement))
        .collect();
    if homes.is_empty() {
        return FxHashSet::default();
    }
    let candidates: FxHashSet<SymbolId> = homes.keys().copied().collect();
    guarded_writes_outside(program, scoping, &candidates, &homes)
}

fn decoder_refusal(
    (scoping, watched, speculated): (&Scoping, &Watched, bool),
    shape: &DecoderShape,
    index: usize,
) -> Option<(&'static str, Span)> {
    if !speculated
        && writes_outside(
            scoping,
            shape.symbol,
            shape.statement,
            &watched.writes,
            index,
        )
    {
        return Some((notes::DECODER_WRITTEN_OUTSIDE, shape.statement));
    }
    if let Some(span) = watched.spelled.get(index).copied().flatten() {
        return Some((notes::MACHINERY_NAME_SPELLED, span));
    }
    if let Some(note) = reached_outside_its_block(scoping, shape.symbol) {
        return Some((note, shape.statement));
    }
    redeclared(scoping, shape.symbol).map(|span| (notes::MACHINERY_REDECLARED, span))
}
