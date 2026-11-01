use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::BinaryOperator;

use super::{
    declared, is_named, is_number, is_string, single_var, var_declaration, Declared, ALPHABET,
};
use crate::ast_util::prop::member_parts;

const READ_OFFSET: f64 = 10.0;
const READER: &str = "charCodeAt";

fn reader_declarations<'k, 'a>(
    statements: &'k [Statement<'a>],
) -> Option<(&'k str, &'k [Statement<'a>])> {
    let first = statements.first()?;
    if let Some(declaration) = var_declaration(first) {
        if let [a, b, c] = declaration.declarations.as_slice() {
            let reader = reader_name([declared(a)?, declared(b)?, declared(c)?])?;
            return Some((reader, &statements[1..]));
        }
    }
    let [a, b, c, ..] = statements else {
        return None;
    };
    let reader = reader_name([single_var(a)?, single_var(b)?, single_var(c)?])?;
    Some((reader, &statements[3..]))
}

fn reader_name<'k>(declared: [Declared<'k, '_>; 3]) -> Option<&'k str> {
    let [(output, Some(output_init)), (_, Some(text_init)), (reader, Some(source))] = declared
    else {
        return None;
    };
    let Expression::BinaryExpression(sum) = source else {
        return None;
    };
    let shaped = is_string(output_init, "")
        && is_string(text_init, "")
        && sum.operator == BinaryOperator::Addition
        && is_named(&sum.left, output)
        && matches!(sum.right, Expression::Identifier(_));
    shaped.then_some(reader)
}

pub(super) fn helper_target<'k, 'a>(
    function: &'k Function<'a>,
    statements: &'k [Statement<'a>],
) -> Option<&'k Function<'a>> {
    let (_, Some(Expression::StringLiteral(alphabet))) = single_var(statements.first()?)? else {
        return None;
    };
    if alphabet.value.len() != ALPHABET.len() || alphabet.value != ALPHABET {
        return None;
    }
    let (reader, rest) = reader_declarations(&statements[1..])?;
    let Some(Statement::ForStatement(scan)) = rest.first() else {
        return None;
    };
    let mut search = SelfRead {
        reader,
        found: false,
    };
    if let Some(test) = &scan.test {
        search.visit_expression(test);
    }
    if let Some(update) = &scan.update {
        search.visit_expression(update);
    }
    search.found.then_some(function)
}

struct SelfRead<'r> {
    reader: &'r str,
    found: bool,
}

impl SelfRead<'_> {
    fn is_offset_read(&self, expr: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expr else {
            return false;
        };
        let Some((object, key)) = member_parts(&call.callee) else {
            return false;
        };
        let offset = matches!(call.arguments.as_slice(),
            [Argument::BinaryExpression(sum)]
                if sum.operator == BinaryOperator::Addition && is_number(&sum.right, READ_OFFSET));
        !call.optional && key == READER && is_named(object, self.reader) && offset
    }

    fn is_self_read_test(&self, it: &BinaryExpression<'_>) -> bool {
        let Expression::BinaryExpression(difference) = &it.left else {
            return false;
        };
        it.operator == BinaryOperator::StrictInequality
            && is_number(&it.right, 0.0)
            && difference.operator == BinaryOperator::Subtraction
            && is_number(&difference.right, READ_OFFSET)
            && self.is_offset_read(&difference.left)
    }
}

impl<'a> VisitJs<'a> for SelfRead<'_> {
    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        if self.found || self.is_self_read_test(it) {
            self.found = true;
            return;
        }
        walk_js::walk_binary_expression(self, it);
    }
}
