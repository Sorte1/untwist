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

type Comparison = (GuardOperand, GuardOperand, bool);

fn comparison(test: &Expression<'_>) -> Option<Comparison> {
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

#[derive(Clone, Debug, Default)]
pub struct EscapeGuards {
    pub values: Vec<Vec<BranchGuard>>,
    pub dynamic: Vec<Vec<BranchGuard>>,
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

    fn enter(&mut self, (left, right, _): &Comparison, taken_when_equal: bool) {
        self.stack.push(BranchGuard {
            left: left.clone(),
            right: right.clone(),
            taken_when_equal,
        });
    }
}

impl<'a> VisitJs<'a> for Enclosing<'_> {
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        let Some(parts) = comparison(&it.test) else {
            walk_js::walk_if_statement(self, it);
            return;
        };
        self.visit_expression(&it.test);
        self.enter(&parts, parts.2);
        self.visit_statement(&it.consequent);
        self.stack.pop();
        if let Some(alternate) = &it.alternate {
            self.enter(&parts, !parts.2);
            self.visit_statement(alternate);
            self.stack.pop();
        }
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        let Some(parts) = comparison(&it.test) else {
            walk_js::walk_conditional_expression(self, it);
            return;
        };
        self.visit_expression(&it.test);
        self.enter(&parts, parts.2);
        self.visit_expression(&it.consequent);
        self.stack.pop();
        self.enter(&parts, !parts.2);
        self.visit_expression(&it.alternate);
        self.stack.pop();
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
