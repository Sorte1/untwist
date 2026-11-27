use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::super::names::Names;
use super::super::templates::{matches_decoder, matches_getter, matches_moved_getter};

const TABLE_LEN: usize = 91;
const DECODER_MARKS: [&str; 3] = ["91", "0x5b", "0X5B"];
const MAX_DECODER_TEXT: usize = 4096;

pub(super) struct Moved {
    pub(super) function: Option<SymbolId>,
    pub(super) span: Span,
    pub(super) first_param: usize,
}

pub(super) struct Pair {
    pub(super) decoder: SymbolId,
    pub(super) getter: SymbolId,
    pub(super) returns_through: Option<SymbolId>,
    pub(super) calls_decoder: Option<SymbolId>,
    pub(super) slices: Option<SymbolId>,
    pub(super) statements: [Span; 2],
    pub(super) block: Span,
    pub(super) top_level: bool,
    pub(super) decoding: Decoding,
    pub(super) moved: Option<Moved>,
}

pub(super) enum Decoding {
    Base91(Vec<u16>),
    Evaluated(String),
}

pub(super) struct PairScan<'s> {
    pub(super) source: &'s str,
    pub(super) scoping: &'s Scoping,
    pub(super) names: &'s Names,
    pub(super) pairs: Vec<Pair>,
    pub(super) depth: usize,
    pub(super) block: Span,
    pub(super) params: Option<(Option<SymbolId>, Span, Vec<SymbolId>)>,
}

pub(super) fn function_symbol<'k, 'a>(
    statement: &'k Statement<'a>,
) -> Option<(SymbolId, &'k Function<'a>)> {
    let Statement::FunctionDeclaration(function) = statement else {
        return None;
    };
    if function.r#async || function.generator {
        return None;
    }
    Some((function.id.as_ref()?.symbol_id.get()?, function))
}

pub(super) fn callee_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
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

pub(super) fn text_of(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

fn moved_function<'k, 'a>(
    statement: &'k Statement<'a>,
    scoping: &Scoping,
) -> Option<(SymbolId, &'k Function<'a>)> {
    let Statement::IfStatement(it) = statement else {
        return None;
    };
    let Expression::UnaryExpression(test) = &it.test else {
        return None;
    };
    let tested = match (&test.operator, &test.argument) {
        (UnaryOperator::LogicalNot, Expression::Identifier(id)) => {
            scoping.get_reference(id.reference_id.get()?).symbol_id()?
        }
        _ => return None,
    };
    let assigned = match &it.consequent {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only,
            _ => return None,
        },
        other => other,
    };
    let Statement::ExpressionStatement(assigned) = assigned else {
        return None;
    };
    let Expression::AssignmentExpression(assign) = &assigned.expression else {
        return None;
    };
    let (
        AssignmentTarget::AssignmentTargetIdentifier(target),
        Expression::FunctionExpression(function),
    ) = (&assign.left, &assign.right)
    else {
        return None;
    };
    let target = scoping
        .get_reference(target.reference_id.get()?)
        .symbol_id()?;
    let plain = it.alternate.is_none()
        && assign.operator == oxc_syntax::operator::AssignmentOperator::Assign
        && !function.r#async
        && !function.generator;
    (plain && target == tested).then_some((target, &**function))
}

struct Candidate<'k, 'a> {
    decoder: (SymbolId, &'k Function<'a>, Span),
    getter: (SymbolId, &'k Function<'a>, Span),
    decoding: Decoding,
}

impl PairScan<'_> {
    fn decoding(
        &self,
        function: &Function<'_>,
        statement: Span,
        moved: Option<SymbolId>,
    ) -> Option<Decoding> {
        let text = text_of(self.source, statement);
        if text.len() > MAX_DECODER_TEXT {
            return None;
        }
        let table = match moved {
            Some(_) => None,
            None => DECODER_MARKS
                .iter()
                .any(|m| text.contains(m))
                .then(|| matches_decoder(self.source, statement, self.names))
                .flatten()
                .filter(|table| table.len() == TABLE_LEN),
        };
        match (table, moved) {
            (Some(table), _) => Some(Decoding::Base91(table)),
            (None, None) if one_plain_param(function) => {
                Some(Decoding::Evaluated(text.to_string()))
            }
            (None, Some(symbol)) if one_plain_param(function) => {
                Some(Decoding::Evaluated(format!(
                    "var {} = {};",
                    self.scoping.symbol_name(symbol),
                    text_of(self.source, function.span)
                )))
            }
            _ => None,
        }
    }

    fn push(&mut self, found: Candidate<'_, '_>, moved: Option<Moved>) {
        let (calls_decoder, slices) = getter_links(found.getter.1, self.scoping);
        self.pairs.push(Pair {
            decoder: found.decoder.0,
            getter: found.getter.0,
            returns_through: returned_callee(found.decoder.1, self.scoping),
            calls_decoder,
            slices,
            statements: [found.decoder.2, found.getter.2],
            block: self.block,
            top_level: self.depth == 0,
            decoding: found.decoding,
            moved,
        });
    }

    fn scan(&mut self, statements: &[Statement<'_>]) {
        for window in statements.windows(2) {
            let (Some((decoder, decoder_fn)), Some((getter, getter_fn))) =
                (function_symbol(&window[0]), function_symbol(&window[1]))
            else {
                continue;
            };
            let Some(decoding) = self.decoding(decoder_fn, window[0].span(), None) else {
                continue;
            };
            if !matches_getter(self.source, window[1].span(), self.names) {
                continue;
            }
            let found = Candidate {
                decoder: (decoder, decoder_fn, window[0].span()),
                getter: (getter, getter_fn, window[1].span()),
                decoding,
            };
            self.push(found, None);
        }
    }

    fn scan_moved(&mut self, statements: &[Statement<'_>]) {
        let Some((function, span, params)) = self.params.clone() else {
            return;
        };
        let leading: Vec<(SymbolId, &Function<'_>, Span)> = statements
            .iter()
            .map_while(|s| moved_function(s, self.scoping).map(|(p, f)| (p, f, s.span())))
            .filter(|(p, _, _)| params.contains(p))
            .collect();
        for window in leading.windows(2) {
            for (decoder, getter) in [(window[0], window[1]), (window[1], window[0])] {
                if !matches_moved_getter(self.source, getter.2, self.names) {
                    continue;
                }
                let Some(decoding) = self.decoding(decoder.1, decoder.2, Some(decoder.0)) else {
                    continue;
                };
                let first_param = params
                    .iter()
                    .position(|p| *p == decoder.0 || *p == getter.0)
                    .unwrap_or(0);
                let found = Candidate {
                    decoder,
                    getter,
                    decoding,
                };
                self.push(
                    found,
                    Some(Moved {
                        function,
                        span,
                        first_param,
                    }),
                );
            }
        }
    }
}

fn param_symbols(function: &Function<'_>) -> Option<Vec<SymbolId>> {
    if function.params.rest.is_some() || function.r#async || function.generator {
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

impl<'p> VisitJs<'p> for PairScan<'_> {
    fn visit_program(&mut self, it: &Program<'p>) {
        self.block = it.span;
        self.scan(&it.body);
        walk_js::walk_program(self, it);
    }

    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        let saved = self.params.take();
        self.params = param_symbols(it).map(|params| {
            let symbol = it
                .id
                .as_ref()
                .filter(|_| it.is_declaration())
                .and_then(|id| id.symbol_id.get());
            (symbol, it.span, params)
        });
        walk_js::walk_function(self, it, flags);
        self.params = saved;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'p>) {
        let saved = self.params.take();
        walk_js::walk_arrow_function_expression(self, it);
        self.params = saved;
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'p>) {
        let saved = self.block;
        let params = self.params.take();
        self.depth += 1;
        self.block = it.span;
        self.scan(&it.statements);
        self.params = params;
        self.scan_moved(&it.statements);
        self.params = None;
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

pub(super) fn one_plain_param(function: &Function<'_>) -> bool {
    matches!(
        function.params.items.as_slice(),
        [param] if matches!(param.pattern, BindingPattern::BindingIdentifier(_)) && param.initializer.is_none()
    ) && function.params.rest.is_none()
        && function
            .body
            .as_ref()
            .is_some_and(|body| body.directives.is_empty())
}
