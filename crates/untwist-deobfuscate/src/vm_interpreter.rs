use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UpdateOperator;

const MIN_SWITCH_CASES: usize = 16;
const MIN_BYTECODE_TEXT: usize = 128;
const MIN_BYTECODE_ITEMS: usize = 128;
const MAX_OPCODE_SPREAD: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dispatch {
    Switch(usize),
    Table,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmInterpreter {
    pub span: Span,
    pub dispatch: Dispatch,
    pub bytecode: usize,
}

pub fn vm_interpreters(program: &Program<'_>) -> Vec<VmInterpreter> {
    let Some(bytecode) = largest_bytecode(program) else {
        return Vec::new();
    };
    let mut loops = Loops::default();
    loops.visit_program(program);
    loops
        .found
        .into_iter()
        .map(|(span, dispatch)| VmInterpreter {
            span,
            dispatch,
            bytecode,
        })
        .collect()
}

fn largest_bytecode(program: &Program<'_>) -> Option<usize> {
    let mut literals = Bytecode::default();
    literals.visit_program(program);
    (literals.largest > 0).then_some(literals.largest)
}

#[derive(Default)]
struct Bytecode {
    largest: usize,
}

fn is_encoded_text(text: &str) -> bool {
    text.len() >= MIN_BYTECODE_TEXT
        && text.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=' | b'-' | b'_')
        })
}

fn is_byte_array(array: &ArrayExpression<'_>) -> bool {
    array.elements.len() >= MIN_BYTECODE_ITEMS
        && array.elements.iter().all(|element| {
            matches!(
                element,
                ArrayExpressionElement::NumericLiteral(number)
                    if number.value.fract() == 0.0 && (0.0..=65535.0).contains(&number.value)
            )
        })
}

impl<'a> VisitJs<'a> for Bytecode {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if let Some(text) = it.value.as_str().filter(|text| is_encoded_text(text)) {
            self.largest = self.largest.max(text.len());
        }
    }

    fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
        if is_byte_array(it) {
            self.largest = self.largest.max(it.elements.len());
        }
        walk_js::walk_array_expression(self, it);
    }
}

#[derive(Default)]
struct Loops {
    found: Vec<(Span, Dispatch)>,
}

fn is_always_true(test: &Expression<'_>) -> bool {
    match test.without_parentheses() {
        Expression::BooleanLiteral(value) => value.value,
        Expression::NumericLiteral(value) => value.value != 0.0 && !value.value.is_nan(),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            is_always_false(&unary.argument)
        }
        _ => false,
    }
}

fn is_always_false(test: &Expression<'_>) -> bool {
    match test.without_parentheses() {
        Expression::BooleanLiteral(value) => !value.value,
        Expression::NumericLiteral(value) => value.value == 0.0,
        Expression::ArrayExpression(_) | Expression::ObjectExpression(_) => false,
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            matches!(
                unary.argument.without_parentheses(),
                Expression::ArrayExpression(_) | Expression::ObjectExpression(_)
            ) || is_always_true(&unary.argument)
        }
        _ => false,
    }
}

fn loop_body<'s, 'a>(stmt: &'s Statement<'a>) -> Option<&'s Statement<'a>> {
    match stmt {
        Statement::WhileStatement(it) if is_always_true(&it.test) => Some(&it.body),
        Statement::DoWhileStatement(it) if is_always_true(&it.test) => Some(&it.body),
        Statement::ForStatement(it) if it.test.as_ref().is_none_or(is_always_true) => {
            Some(&it.body)
        }
        _ => None,
    }
}

impl<'a> VisitJs<'a> for Loops {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if let Some(body) = loop_body(it) {
            if let Some(dispatch) = interpreter_dispatch(body) {
                self.found.push((it.span(), dispatch));
            }
        }
        walk_js::walk_statement(self, it);
    }
}

fn interpreter_dispatch(body: &Statement<'_>) -> Option<Dispatch> {
    let mut fetches = Fetches::default();
    fetches.visit_statement(body);
    if fetches.names.is_empty() {
        return None;
    }
    let mut dispatches = Dispatches {
        names: &fetches.names,
        found: None,
    };
    dispatches.visit_statement(body);
    dispatches.found
}

fn is_fetch(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::ComputedMemberExpression(member) => matches!(
            member.expression.without_parentheses(),
            Expression::UpdateExpression(update) if update.operator == UpdateOperator::Increment
        ),
        Expression::CallExpression(call) => {
            call.arguments.is_empty()
                && callee_name(&call.callee)
                    .is_some_and(|name| name.to_ascii_lowercase().starts_with("read"))
        }
        _ => false,
    }
}

fn callee_name<'s>(callee: &'s Expression<'_>) -> Option<&'s str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

#[derive(Default)]
struct Fetches<'a> {
    names: Vec<&'a str>,
}

impl<'a> VisitJs<'a> for Fetches<'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if is_fetch(init) {
                self.names.push(id.name.as_str());
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if is_fetch(&it.right) {
                self.names.push(id.name.as_str());
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }
}

struct Dispatches<'n, 'a> {
    names: &'n [&'a str],
    found: Option<Dispatch>,
}

impl Dispatches<'_, '_> {
    fn is_fetched(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id) if self.names.contains(&id.name.as_str()))
    }
}

fn dense_opcodes(cases: &[SwitchCase<'_>]) -> bool {
    let values: Vec<f64> = cases
        .iter()
        .filter_map(|case| match case.test.as_ref()?.without_parentheses() {
            Expression::NumericLiteral(number)
                if number.value.fract() == 0.0 && number.value >= 0.0 =>
            {
                Some(number.value)
            }
            _ => None,
        })
        .collect();
    if values.len() < MIN_SWITCH_CASES || values.len() + 1 < cases.len() {
        return false;
    }
    let highest = values.iter().copied().fold(0.0, f64::max);
    highest + 1.0 <= values.len() as f64 * MAX_OPCODE_SPREAD
}

impl<'a> VisitJs<'a> for Dispatches<'_, 'a> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        if self.found.is_none() && self.is_fetched(&it.discriminant) && dense_opcodes(&it.cases) {
            self.found = Some(Dispatch::Switch(it.cases.len()));
        }
        walk_js::walk_switch_statement(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.found.is_none() {
            if let Expression::ComputedMemberExpression(member) = it.callee.without_parentheses() {
                if self.is_fetched(&member.expression) {
                    self.found = Some(Dispatch::Table);
                }
            }
        }
        walk_js::walk_call_expression(self, it);
    }
}
