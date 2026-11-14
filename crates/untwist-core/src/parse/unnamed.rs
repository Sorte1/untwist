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

fn skip_trivia(bytes: &[u8], mut at: usize) -> usize {
    loop {
        match (bytes.get(at), bytes.get(at + 1)) {
            (Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c), _) => at += 1,
            (Some(b'/'), Some(b'*')) => {
                at = bytes[at + 2..]
                    .windows(2)
                    .position(|pair| pair == b"*/")
                    .map_or(bytes.len(), |end| at + 2 + end + 2);
            }
            (Some(b'/'), Some(b'/')) => {
                at = bytes[at..]
                    .iter()
                    .position(|&byte| byte == b'\n')
                    .map_or(bytes.len(), |end| at + end);
            }
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
