use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::GetSpan;

struct Unnamed<'a, 's> {
    builder: AstBuilder<'a>,
    source: &'s str,
}

impl<'a> Unnamed<'a, '_> {
    fn parenthesised(&self, outer: u32, name: &IdentifierReference<'_>) -> bool {
        outer < name.span.start && self.source.as_bytes().get(outer as usize) == Some(&b'(')
    }

    fn hide(&self, value: &mut Expression<'a>) {
        if !value.is_anonymous_function_definition() {
            return;
        }
        let span = value.span();
        let hidden = std::mem::replace(value, Expression::new_number_0(span, &self.builder));
        let parts = AVec::from_array_in(
            [Expression::new_number_0(span, &self.builder), hidden],
            &self.builder,
        );
        *value = Expression::new_sequence_expression(span, parts, &self.builder);
    }
}

fn naming(operator: AssignmentOperator) -> bool {
    matches!(
        operator,
        AssignmentOperator::Assign
            | AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalNullish
    )
}

impl<'a> VisitMut<'a> for Unnamed<'a, '_> {
    fn visit_assignment_expression(&mut self, it: &mut AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(name) = &it.left {
            if naming(it.operator) && self.parenthesised(it.span.start, name) {
                self.hide(&mut it.right);
            }
        }
        walk_mut::walk_assignment_expression(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &mut AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(name) = &it.binding {
            if self.parenthesised(it.span.start, name) {
                self.hide(&mut it.init);
            }
        }
        walk_mut::walk_assignment_target_with_default(self, it);
    }
}

const LINE_SEPARATOR: &[u8] = "\u{2028}".as_bytes();
const PARAGRAPH_SEPARATOR: &[u8] = "\u{2029}".as_bytes();

fn line_terminator(bytes: &[u8]) -> Option<usize> {
    match bytes {
        [b'\n' | b'\r', ..] => Some(1),
        _ if bytes.starts_with(LINE_SEPARATOR) || bytes.starts_with(PARAGRAPH_SEPARATOR) => {
            Some(LINE_SEPARATOR.len())
        }
        _ => None,
    }
}

fn line_end(bytes: &[u8], at: usize) -> usize {
    (at..bytes.len())
        .find(|&end| line_terminator(&bytes[end..]).is_some())
        .unwrap_or(bytes.len())
}

fn block_end(bytes: &[u8], at: usize) -> (usize, bool) {
    let body = &bytes[at + 2..];
    let close = body.windows(2).position(|pair| pair == b"*/");
    let inside = close.map_or(body, |end| &body[..end]);
    let breaks = (0..inside.len()).any(|index| line_terminator(&inside[index..]).is_some());
    (close.map_or(bytes.len(), |end| at + 2 + end + 2), breaks)
}

fn skip_trivia(bytes: &[u8], mut at: usize) -> usize {
    let mut line_start = false;
    loop {
        let rest = bytes.get(at..).unwrap_or_default();
        if let Some(width) = line_terminator(rest) {
            at += width;
            line_start = true;
            continue;
        }
        match rest {
            [b' ' | b'\t' | 0x0b | 0x0c, ..] => at += 1,
            [b'/', b'*', ..] => {
                let (end, breaks) = block_end(bytes, at);
                at = end;
                line_start |= breaks;
            }
            [b'/', b'/', ..] | [b'<', b'!', b'-', b'-', ..] => at = line_end(bytes, at),
            [b'-', b'-', b'>', ..] if line_start => at = line_end(bytes, at),
            _ => return at,
        }
    }
}

fn assigns_after(bytes: &[u8], close: usize) -> bool {
    let at = skip_trivia(bytes, close + 1);
    match bytes.get(at..).unwrap_or_default() {
        [b'=', b'=' | b'>', ..] => false,
        [b'=', ..] => true,
        [byte, ..] if !byte.is_ascii() => true,
        [b'|', b'|', b'=', ..] | [b'&', b'&', b'=', ..] | [b'?', b'?', b'=', ..] => true,
        _ => false,
    }
}

fn may_hide_names(source: &str) -> bool {
    let bytes = source.as_bytes();
    closing_parens(bytes).any(|close| assigns_after(bytes, close))
}

fn closing_parens(bytes: &[u8]) -> impl Iterator<Item = usize> + '_ {
    bytes
        .iter()
        .enumerate()
        .filter(|(_, &byte)| byte == b')')
        .map(|(at, _)| at)
}

pub(super) fn keep_unnamed<'a>(alloc: &'a Allocator, source: &str, program: &mut Program<'a>) {
    if !may_hide_names(source) {
        return;
    }
    let mut unnamed = Unnamed {
        builder: AstBuilder::new(alloc),
        source,
    };
    unnamed.visit_program(program);
}
