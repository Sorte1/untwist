mod judge;
mod prefix;
mod scope;
mod self_rewrite;
mod template_arguments;
mod uses;
mod walk;

use oxc_ast::ast::{Expression, Function};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;

use crate::mutants::void_operands_unchecked;
use judge::Walker;
use scope::FrameKind;
pub(super) use self_rewrite::{self_rewrite_target, template_rewrite};

pub(super) struct Trigger<'a> {
    pub(super) span: Span,
    pub(super) kind: TriggerKind<'a>,
}

pub(super) enum TriggerKind<'a> {
    This,
    Identifier,
    Unrecognized,
    Member(MemberPeel<'a>),
    CapturedMemo(MemberPeel<'a>),
    Monitored,
}

#[derive(Clone, Copy)]
pub(super) struct MemberPeel<'a> {
    pub(super) root: &'a str,
    pub(super) segment: Option<&'a str>,
    pub(super) container_span: Span,
    pub(super) final_key: FinalKey<'a>,
}

#[derive(Clone, Copy)]
pub(super) enum FinalKey<'a> {
    Static(&'a str),
    Computed(Span),
}

impl MemberPeel<'_> {
    pub(super) fn container_key(&self) -> (&str, Option<&str>) {
        (self.root, self.segment)
    }
}

pub(super) fn collect_for_function<'a>(
    function: &Function<'a>,
    own_name: Option<&'a str>,
    machinery: &[&'a str],
    strict: bool,
) -> Collected<'a> {
    let Some(body) = &function.body else {
        return Collected::default();
    };
    let rewrite = own_name.and_then(|name| template_rewrite(function, name));
    let template = own_name.filter(|_| rewrite.is_some());
    let call_target = template
        .and_then(|name| self_rewrite_target(&body.statements, Some(name)))
        .map(|inner| inner.span);
    let kind = if template.is_some() {
        FrameKind::Outer
    } else {
        FrameKind::Call
    };
    let mut walker = Walker::new(machinery, template, call_target);
    walker.strict = strict;
    walker.outer_arity = template.map(|_| function.params.items.len());
    walker.enter_root(function, kind);
    let mut collected = walker.finish();
    collected.rewrite = rewrite;
    collected
}

pub(super) fn rotator_function<'a>(iife: &'a Expression<'a>) -> Option<&'a Function<'a>> {
    let call = match iife {
        Expression::CallExpression(call) => call,
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            return rotator_function(&unary.argument)
        }
        Expression::ParenthesizedExpression(inner) => return rotator_function(&inner.expression),
        _ => return None,
    };
    match &call.callee {
        Expression::FunctionExpression(function) => Some(function),
        Expression::ParenthesizedExpression(inner) => match &inner.expression {
            Expression::FunctionExpression(function) => Some(function),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn collect_for_rotator<'a>(
    function: &Function<'a>,
    machinery: &[&'a str],
    strict: bool,
) -> Collected<'a> {
    let mut walker = Walker::new(&[], None, None);
    walker.watched = machinery;
    walker.strict = strict;
    walker.enter_root(function, FrameKind::Call);
    walker.finish()
}

pub(super) struct RecognizedGate<'a> {
    pub(super) root: &'a str,
    pub(super) segment: Option<&'a str>,
    pub(super) flag_key: &'a str,
    pub(super) region: Span,
    pub(super) test: Span,
    pub(super) conditional: bool,
}

pub(super) struct MemberAccess<'a> {
    pub(super) peel: MemberPeel<'a>,
    pub(super) is_write: bool,
    pub(super) local: bool,
    pub(super) at: u32,
}

#[derive(Clone, Copy)]
pub(super) enum Allowance {
    Never,
    Always,
    AfterGates(u32),
}

pub(super) struct BareUse<'a> {
    pub(super) name: &'a str,
    pub(super) allowance: Allowance,
}

#[derive(Default)]
pub(super) struct Collected<'a> {
    pub(super) triggers: Vec<Trigger<'a>>,
    pub(super) gates: Vec<RecognizedGate<'a>>,
    pub(super) accesses: Vec<MemberAccess<'a>>,
    pub(super) arguments_stores: Vec<(&'a str, &'a str)>,
    pub(super) captures: Vec<Span>,
    pub(super) bare_uses: Vec<BareUse<'a>>,
    pub(super) receivers: Vec<(&'a str, &'a str)>,
    pub(super) function_writes: Vec<(&'a str, &'a str, bool)>,
    pub(super) rewrite: Option<Span>,
}

pub(super) fn unwrap_parens<'a, 'n>(expr: &'n Expression<'a>) -> &'n Expression<'a> {
    match expr {
        Expression::ParenthesizedExpression(node) => unwrap_parens(&node.expression),
        _ => expr,
    }
}

fn as_identifier<'a>(expr: &Expression<'a>) -> Option<&'a str> {
    match unwrap_parens(expr) {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

pub(super) fn member_peel_from<'a>(
    member: &oxc_ast::ast::MemberExpression<'a>,
) -> Option<MemberPeel<'a>> {
    let final_key = match member.static_property_name().and_then(|name| name.as_str()) {
        Some(text) => FinalKey::Static(text),
        None => match member {
            oxc_ast::ast::MemberExpression::ComputedMemberExpression(inner) => {
                FinalKey::Computed(inner.expression.span())
            }
            _ => return None,
        },
    };
    match unwrap_parens(member.object()) {
        Expression::Identifier(id) => Some(MemberPeel {
            root: id.name.as_str(),
            segment: None,
            container_span: id.span,
            final_key,
        }),
        other => {
            let inner = other.as_member_expression()?;
            let root = as_identifier(inner.object())?;
            let segment = inner
                .static_property_name()
                .and_then(|name| name.as_str())?;
            Some(MemberPeel {
                root,
                segment: Some(segment),
                container_span: other.span(),
                final_key,
            })
        }
    }
}

fn member_peel<'a>(expr: &Expression<'a>) -> Option<MemberPeel<'a>> {
    member_peel_from(expr.as_member_expression()?)
}

pub(super) fn gate_guard<'a>(test: &Expression<'a>) -> Option<MemberPeel<'a>> {
    match test {
        Expression::BinaryExpression(node) => {
            use oxc_syntax::operator::BinaryOperator;
            if !matches!(
                node.operator,
                BinaryOperator::StrictEquality | BinaryOperator::Equality
            ) {
                return None;
            }
            if is_undefined(&node.left) {
                member_peel(&node.right)
            } else if is_undefined(&node.right) {
                member_peel(&node.left)
            } else {
                None
            }
        }
        Expression::UnaryExpression(node) if node.operator == UnaryOperator::LogicalNot => {
            member_peel(&node.argument)
        }
        Expression::ParenthesizedExpression(node) => gate_guard(&node.expression),
        _ => None,
    }
}

fn is_undefined(expr: &Expression<'_>) -> bool {
    match unwrap_parens(expr) {
        Expression::Identifier(id) => id.name.as_str() == "undefined",
        Expression::UnaryExpression(node) if node.operator == UnaryOperator::Void => {
            void_operands_unchecked() || runs_no_code(&node.argument)
        }
        _ => false,
    }
}

fn runs_no_code(expr: &Expression<'_>) -> bool {
    match unwrap_parens(expr) {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::UnaryExpression(node) if node.operator == UnaryOperator::Void => {
            runs_no_code(&node.argument)
        }
        _ => false,
    }
}
