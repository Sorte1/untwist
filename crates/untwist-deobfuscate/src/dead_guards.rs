use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_eval::decode::Decoded;
use untwist_eval::value::JsString;

#[derive(Clone, Debug, PartialEq)]
pub enum GuardOperand {
    Text(String),
    Call(Span),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchGuard {
    pub left: GuardOperand,
    pub right: GuardOperand,
    pub taken_when_equal: bool,
}

fn operand(expr: &Expression<'_>) -> Option<GuardOperand> {
    match expr.without_parentheses() {
        Expression::StringLiteral(text) => {
            Some(GuardOperand::Text(text.value.as_str()?.to_string()))
        }
        Expression::CallExpression(call)
            if !call.optional && matches!(call.callee, Expression::Identifier(_)) =>
        {
            Some(GuardOperand::Call(call.span))
        }
        _ => None,
    }
}

fn comparison(test: &Expression<'_>) -> Option<(GuardOperand, GuardOperand, bool)> {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return None;
    };
    let equal = match binary.operator {
        BinaryOperator::StrictEquality | BinaryOperator::Equality => true,
        BinaryOperator::StrictInequality | BinaryOperator::Inequality => false,
        _ => return None,
    };
    let left = operand(&binary.left)?;
    let right = operand(&binary.right)?;
    let calls = [&left, &right]
        .into_iter()
        .filter(|side| matches!(side, GuardOperand::Call(_)))
        .count();
    (calls > 0).then_some((left, right, equal))
}

struct Enclosing<'e> {
    escapes: &'e FxHashSet<Span>,
    stack: Vec<BranchGuard>,
    found: FxHashMap<Span, Vec<BranchGuard>>,
}

impl Enclosing<'_> {
    fn note(&mut self, span: Span) {
        if self.escapes.contains(&span) {
            self.found.insert(span, self.stack.clone());
        }
    }

    fn guarded<'a>(
        &mut self,
        test: &Expression<'a>,
        branches: (&dyn Fn(&mut Self), Option<&dyn Fn(&mut Self)>),
    ) -> bool {
        let Some((left, right, equal)) = comparison(test) else {
            return false;
        };
        self.visit_expression(test);
        let (consequent, alternate) = branches;
        for (visit, taken_when_equal) in [(Some(consequent), equal), (alternate, !equal)] {
            let Some(visit) = visit else {
                continue;
            };
            self.stack.push(BranchGuard {
                left: left.clone(),
                right: right.clone(),
                taken_when_equal,
            });
            visit(self);
            self.stack.pop();
        }
        true
    }
}

impl<'a> VisitJs<'a> for Enclosing<'_> {
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        let consequent = |this: &mut Self| this.visit_statement(&it.consequent);
        let alternate = it
            .alternate
            .as_ref()
            .map(|alternate| move |this: &mut Self| this.visit_statement(alternate));
        let alternate_ref = alternate.as_ref().map(|f| f as &dyn Fn(&mut Self));
        if !self.guarded(&it.test, (&consequent, alternate_ref)) {
            walk_js::walk_if_statement(self, it);
        }
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        let consequent = |this: &mut Self| this.visit_expression(&it.consequent);
        let alternate = |this: &mut Self| this.visit_expression(&it.alternate);
        if !self.guarded(&it.test, (&consequent, Some(&alternate))) {
            walk_js::walk_conditional_expression(self, it);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.note(it.span);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.note(it.span);
    }
}

pub fn enclosing_guards(
    program: &Program<'_>,
    escapes: &FxHashSet<Span>,
) -> FxHashMap<Span, Vec<BranchGuard>> {
    let mut scan = Enclosing {
        escapes,
        stack: Vec::new(),
        found: FxHashMap::default(),
    };
    scan.visit_program(program);
    scan.found
}

fn text_of(operand: &GuardOperand, values: &FxHashMap<Span, &Decoded>) -> Option<JsString> {
    match operand {
        GuardOperand::Text(text) => Some(JsString::from(text.as_str())),
        GuardOperand::Call(span) => match values.get(span)? {
            Decoded::Str(text) => Some(text.clone()),
            _ => None,
        },
    }
}

fn never_taken(guard: &BranchGuard, values: &FxHashMap<Span, &Decoded>) -> bool {
    let (Some(left), Some(right)) = (text_of(&guard.left, values), text_of(&guard.right, values))
    else {
        return false;
    };
    (left == right) != guard.taken_when_equal
}

pub fn all_dead(escapes: &[Vec<BranchGuard>], values: &FxHashMap<Span, &Decoded>) -> bool {
    escapes
        .iter()
        .all(|guards| guards.iter().any(|guard| never_taken(guard, values)))
}
