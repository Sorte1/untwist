use oxc_ast::ast::{BinaryExpression, Expression, UnaryExpression};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::NESTING_DEPTH_CAP;
use crate::compile::entry::direct_value;
use crate::host::Host;
use crate::interp::Interp;
use crate::ops::{binary_op, unary_op};
use crate::stack;
use crate::value::{Abrupt, Eval, JsString, ObjKind, ObjRef, PropKey, Value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Konst {
    pub(crate) value: f64,
    pub(crate) weight: u64,
    pub(crate) depth: u32,
}

#[cfg(test)]
pub(crate) fn constant(expr: &Expression<'_>) -> Option<(f64, u64, u32)> {
    fold(expr)
        .ok()
        .map(|konst| (konst.value, konst.weight, konst.depth))
}

pub(crate) fn foldable_root(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ParenthesizedExpression(_) => true,
        Expression::UnaryExpression(unary) => unary_folds(unary.operator),
        Expression::BinaryExpression(binary) => binary_folds(binary.operator),
        _ => false,
    }
}

pub(crate) fn fold(expr: &Expression<'_>) -> Result<Konst, Span> {
    stack::maybe_grow(|| match expr {
        Expression::NumericLiteral(literal) => Ok(Konst {
            value: literal.value,
            weight: 1,
            depth: 0,
        }),
        Expression::ParenthesizedExpression(paren) => {
            let inner = fold(&paren.expression)?;
            enclose(expr, inner.value, 1, &[inner])
        }
        Expression::UnaryExpression(unary) => fold_unary(expr, unary),
        Expression::BinaryExpression(binary) => fold_binary(expr, binary),
        _ => Err(expr.span()),
    })
}

fn unary_folds(operator: UnaryOperator) -> bool {
    matches!(
        operator,
        UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
    )
}

fn binary_folds(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::Exponential
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
    )
}

fn fold_unary(expr: &Expression<'_>, unary: &UnaryExpression<'_>) -> Result<Konst, Span> {
    if !unary_folds(unary.operator) {
        return Err(expr.span());
    }
    let argument = fold(&unary.argument)?;
    let folded = unary_op(unary.operator, &Value::Number(argument.value), &mut Folding);
    enclose(expr, number(folded, expr)?, 2, &[argument])
}

fn fold_binary(expr: &Expression<'_>, binary: &BinaryExpression<'_>) -> Result<Konst, Span> {
    if !binary_folds(binary.operator) {
        return Err(expr.span());
    }
    let left = fold(&binary.left)?;
    let right = fold(&binary.right)?;
    let folded = binary_op(
        binary.operator,
        &Value::Number(left.value),
        &Value::Number(right.value),
        &mut Folding,
    );
    enclose(expr, number(folded, expr)?, 2, &[left, right])
}

fn number(folded: Eval<'_, Value<'_>>, expr: &Expression<'_>) -> Result<f64, Span> {
    match folded {
        Ok(Value::Number(value)) => Ok(value),
        _ => Err(expr.span()),
    }
}

fn enclose(expr: &Expression<'_>, value: f64, own: u64, parts: &[Konst]) -> Result<Konst, Span> {
    let weight = parts
        .iter()
        .try_fold(own, |total, part| total.checked_add(part.weight));
    let depth = parts
        .iter()
        .map(|part| part.depth)
        .max()
        .and_then(|deepest| deepest.checked_add(1));
    match (weight, depth) {
        (Some(weight), Some(depth)) => Ok(Konst {
            value,
            weight,
            depth,
        }),
        _ => Err(expr.span()),
    }
}

impl Konst {
    #[inline(always)]
    pub(crate) fn run<'a>(
        self,
        interp: &mut Interp<'a>,
        expr: &'a Expression<'a>,
    ) -> Eval<'a, Value<'a>> {
        let rest = self.weight - 1;
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_constant_run(
            interp.budget.steps_used(),
            self.weight,
            interp.budget.nesting_depth(),
            self.depth,
        );
        let nesting = u64::from(interp.budget.nesting_depth()) + u64::from(self.depth);
        if nesting > u64::from(NESTING_DEPTH_CAP) || interp.budget.steps_left() < rest {
            #[cfg(feature = "test-hooks")]
            crate::test_hooks::note_constant_fallback();
            return direct_value(interp, expr);
        }
        interp.budget.spend(rest)?;
        Ok(Value::Number(self.value))
    }
}

struct Folding;

fn refused<'a>() -> Abrupt<'a> {
    Bail::Unsupported(&"a constant folded at compile time").into()
}

impl<'a> Host<'a> for Folding {
    fn spend(&mut self, _: u64) -> EvalResult<()> {
        Ok(())
    }

    fn throw(&self, _: ErrorKind, _: &str) -> Abrupt<'a> {
        refused()
    }

    fn note(&mut self, _: &str) {}

    fn new_object(&mut self, _: ObjKind<'a>, _: Option<ObjRef<'a>>) -> Value<'a> {
        Value::Undefined
    }

    fn new_array(&mut self, _: Vec<Value<'a>>) -> Value<'a> {
        Value::Undefined
    }

    fn call_value(&mut self, _: &Value<'a>, _: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        Err(refused())
    }

    fn construct_value(&mut self, _: &Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        Err(refused())
    }

    fn get_property(&mut self, _: &Value<'a>, _: &PropKey) -> Eval<'a, Value<'a>> {
        Err(refused())
    }

    fn note_protocol_member(&mut self, _: &Value<'a>, _: &PropKey) -> EvalResult<()> {
        Err(Bail::Unsupported(&"a constant folded at compile time"))
    }

    fn function_source(&mut self, _: &Value<'a>) -> Eval<'a, JsString> {
        Err(refused())
    }

    fn enter_nesting(&mut self) -> EvalResult<()> {
        Ok(())
    }

    fn leave_nesting(&mut self) {}

    fn get_prototype_of(&mut self, _: &Value<'a>) -> Eval<'a, Value<'a>> {
        Err(refused())
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;

    use super::constant;

    fn folded(source: &str) -> Option<(f64, u64, u32)> {
        let alloc = Allocator::default();
        let parsed = untwist_core::parse::parse_program(&alloc, source);
        let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.first() else {
            panic!("an expression statement");
        };
        constant(&statement.expression)
    }

    #[test]
    fn a_constant_carries_the_walkers_step_weight_and_its_depth_below_the_root() {
        assert_eq!(folded("0x7;"), Some((7.0, 1, 0)));
        assert_eq!(folded("(0x7);"), Some((7.0, 1, 0)));
        assert_eq!(folded("-0x7;"), Some((-7.0, 3, 1)));
        assert_eq!(folded("1 + 2;"), Some((3.0, 4, 1)));
        assert_eq!(folded("-0x1da3+0x4*0x56f+0x2b*0x35;"), Some((256.0, 15, 3)));
        assert_eq!(
            folded("~(-0x1da3+0x4*0x56f+0x2b*0x35);"),
            Some((-257.0, 17, 4))
        );
        assert_eq!(
            folded("0x2 ** 0x34;"),
            Some((4_503_599_627_370_496.0, 4, 1))
        );
    }

    #[test]
    fn what_the_walker_would_not_fold_has_no_constant() {
        for source in [
            "2 ** 0.5;",
            "0x2 ** 0x40;",
            "0x1 < 0x2;",
            "-(0x1 === 0x1);",
            "-x;",
            "+'1';",
            "0x1 - '1';",
            "~[];",
            "(1, 2);",
            "1 && 2;",
            "typeof 1;",
            "void 0;",
            "!0;",
            "1n + 1n;",
        ] {
            assert_eq!(folded(source), None, "{source}");
        }
    }
}
