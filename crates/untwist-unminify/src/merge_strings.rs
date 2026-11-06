use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN};
use oxc_str::Str;
use oxc_traverse::Ancestor;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct MergeStrings;

struct Chain {
    links: usize,
    starts_with_literal: bool,
    text: String,
}

fn mergeable<'e>(expr: &'e Expression<'_>) -> Option<&'e str> {
    match expr {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn link<'e, 'a>(expr: &'e Expression<'a>) -> Option<(&'e Expression<'a>, &'e str)> {
    let Expression::BinaryExpression(binary) = expr else {
        return None;
    };
    if binary.operator != BinaryOperator::Addition {
        return None;
    }
    Some((&binary.left, mergeable(&binary.right)?))
}

fn is_inner_link(cx: &Cx<'_, '_>) -> bool {
    match cx.tcx.parent() {
        Ancestor::BinaryExpressionLeft(parent) => {
            *parent.operator() == BinaryOperator::Addition && mergeable(parent.right()).is_some()
        }
        _ => false,
    }
}

fn spine<'e, 'a>(expr: &'e Expression<'a>) -> (usize, usize, &'e Expression<'a>) {
    let mut links = 0;
    let mut total = 0;
    let mut current = expr;
    while let Some((left, right)) = link(current) {
        links += 1;
        total += right.len();
        current = left;
    }
    (links, total, current)
}

fn push_rights(expr: &Expression<'_>, links: usize, text: &mut String) {
    if links == 1 {
        text.push_str(link(expr).map_or("", |(_, right)| right));
        return;
    }
    let mut rights: Vec<&str> = Vec::with_capacity(links);
    let mut current = expr;
    while let Some((left, right)) = link(current) {
        rights.push(right);
        current = left;
    }
    for right in rights.iter().rev() {
        text.push_str(right);
    }
}

fn chain(expr: &Expression<'_>) -> Option<Chain> {
    let (links, rights_len, base) = spine(expr);
    let first = mergeable(base);
    if links == 0 || (first.is_none() && links < 2) {
        return None;
    }
    let first = first.unwrap_or_default();
    let mut text = String::with_capacity(first.len() + rights_len);
    text.push_str(first);
    push_rights(expr, links, &mut text);
    Some(Chain {
        links,
        starts_with_literal: mergeable(base).is_some(),
        text,
    })
}

pub fn waits_for_its_chain_root(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> bool {
    link(expr).is_some_and(|(left, _)| mergeable(left).is_some()) && is_inner_link(cx)
}

pub fn literal_chain(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<(String, usize)> {
    if link(expr).is_none() || is_inner_link(cx) {
        return None;
    }
    let chain = chain(expr)?;
    chain
        .starts_with_literal
        .then_some((chain.text, chain.links))
}

fn lowest_link<'e, 'a>(expr: &'e mut Expression<'a>, links: usize) -> &'e mut Expression<'a> {
    let mut current = expr;
    for _ in 1..links {
        match current {
            Expression::BinaryExpression(binary) => current = &mut binary.left,
            _ => break,
        }
    }
    current
}

fn keep_lowest_link<'a>(expr: &mut Expression<'a>, chain: &Chain, cx: &mut Cx<'a, '_>) {
    let lowest = lowest_link(expr, chain.links);
    let Expression::BinaryExpression(binary) = lowest else {
        return;
    };
    let span = binary.right.span();
    let value = Str::from_str_in(&chain.text, cx);
    binary.right = Expression::new_string_literal(span, value, None, cx);
    let placeholder = Expression::new_null_literal(SPAN, cx);
    let kept = std::mem::replace(lowest, placeholder);
    *expr = kept;
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);

impl<'a> Pass<'a> for MergeStrings {
    fn name(&self) -> &'static str {
        "merge-strings"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if link(expr).is_none() || is_inner_link(cx) {
            return;
        }
        let Some(chain) = chain(expr) else {
            return;
        };
        if chain.starts_with_literal {
            let span = expr.span();
            let value = Str::from_str_in(&chain.text, cx);
            *expr = Expression::new_string_literal(span, value, None, cx);
            cx.changed_by(chain.links);
            return;
        }
        keep_lowest_link(expr, &chain, cx);
        cx.changed_by(chain.links - 1);
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(MergeStrings)
}
