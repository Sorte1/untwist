use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::frame::{self, Frame, FrameScan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Found,
    Pure,
    Impure,
}

pub trait Reads {
    fn is_read(&self, expression: &Expression<'_>) -> bool;

    fn steady(&self, _name: Option<&IdentifierReference<'_>>) -> bool {
        true
    }
}

fn leaf<R: Reads + ?Sized>(name: Option<&IdentifierReference<'_>>, reads: &R) -> Order {
    if reads.steady(name) {
        Order::Pure
    } else {
        Order::Impure
    }
}

fn then_get<R: Reads + ?Sized>(object: Order, reads: &R) -> Order {
    match object {
        Order::Pure => leaf(None, reads),
        other => other,
    }
}

fn sequence<'e, 'a: 'e, R: Reads + ?Sized>(
    parts: impl IntoIterator<Item = &'e Expression<'a>>,
    reads: &R,
) -> Order {
    for part in parts {
        match order(part, reads) {
            Order::Pure => {}
            other => return other,
        }
    }
    Order::Pure
}

fn guarded<R: Reads + ?Sized>(
    first: &Expression<'_>,
    rest: &[&Expression<'_>],
    reads: &R,
) -> Order {
    match order(first, reads) {
        Order::Pure if rest.iter().all(|part| !contains(part, reads)) => Order::Pure,
        Order::Pure => Order::Impure,
        other => other,
    }
}

fn after_operands(operands: Order) -> Order {
    match operands {
        Order::Pure => Order::Impure,
        other => other,
    }
}

fn arguments_order<R: Reads + ?Sized>(arguments: &[Argument<'_>], reads: &R) -> Order {
    for argument in arguments {
        let Some(expression) = argument.as_expression() else {
            return Order::Impure;
        };
        match order(expression, reads) {
            Order::Pure => {}
            other => return other,
        }
    }
    Order::Pure
}

fn callee_order<R: Reads + ?Sized>(callee: &Expression<'_>, reads: &R) -> Order {
    match callee.without_parentheses() {
        Expression::StaticMemberExpression(member) => then_get(order(&member.object, reads), reads),
        Expression::ComputedMemberExpression(member) => {
            then_get(sequence([&member.object, &member.expression], reads), reads)
        }
        other => order(other, reads),
    }
}

fn target_order<R: Reads + ?Sized>(
    target: &AssignmentTarget<'_>,
    compound: bool,
    reads: &R,
) -> Order {
    let object = match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) if compound => {
            return leaf(Some(id), reads)
        }
        AssignmentTarget::AssignmentTargetIdentifier(_) => return Order::Pure,
        AssignmentTarget::StaticMemberExpression(member) => order(&member.object, reads),
        AssignmentTarget::ComputedMemberExpression(member) => {
            sequence([&member.object, &member.expression], reads)
        }
        _ => return Order::Impure,
    };
    if compound {
        then_get(object, reads)
    } else {
        object
    }
}

fn array_order<R: Reads + ?Sized>(array: &ArrayExpression<'_>, reads: &R) -> Order {
    for element in &array.elements {
        let order = match element {
            ArrayExpressionElement::Elision(_) => Order::Pure,
            ArrayExpressionElement::SpreadElement(_) => Order::Impure,
            element => element
                .as_expression()
                .map_or(Order::Impure, |expression| order(expression, reads)),
        };
        if order != Order::Pure {
            return order;
        }
    }
    Order::Pure
}

fn object_order<R: Reads + ?Sized>(object: &ObjectExpression<'_>, reads: &R) -> Order {
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Order::Impure;
        };
        if let Some(key) = property.key.as_expression() {
            match order(key, reads) {
                Order::Pure => {}
                other => return other,
            }
        }
        match order(&property.value, reads) {
            Order::Pure => {}
            other => return other,
        }
    }
    Order::Pure
}

pub fn order<R: Reads + ?Sized>(expression: &Expression<'_>, reads: &R) -> Order {
    if reads.is_read(expression) {
        return Order::Found;
    }
    match expression {
        Expression::Identifier(id) => leaf(Some(id), reads),
        Expression::ThisExpression(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => Order::Pure,
        Expression::ParenthesizedExpression(it) => order(&it.expression, reads),
        Expression::StaticMemberExpression(it) => then_get(order(&it.object, reads), reads),
        Expression::ComputedMemberExpression(it) => {
            then_get(sequence([&it.object, &it.expression], reads), reads)
        }
        Expression::AssignmentExpression(it) => match target_order(
            &it.left,
            it.operator != oxc_syntax::operator::AssignmentOperator::Assign,
            reads,
        ) {
            Order::Pure => after_operands(order(&it.right, reads)),
            other => other,
        },
        Expression::BinaryExpression(it) => after_operands(sequence([&it.left, &it.right], reads)),
        Expression::LogicalExpression(it) => guarded(&it.left, &[&it.right], reads),
        Expression::ConditionalExpression(it) => {
            guarded(&it.test, &[&it.consequent, &it.alternate], reads)
        }
        Expression::CallExpression(it) if !it.optional => match callee_order(&it.callee, reads) {
            Order::Pure => after_operands(arguments_order(&it.arguments, reads)),
            other => other,
        },
        Expression::NewExpression(it) => match order(&it.callee, reads) {
            Order::Pure => after_operands(arguments_order(&it.arguments, reads)),
            other => other,
        },
        Expression::ArrayExpression(it) => array_order(it, reads),
        Expression::ObjectExpression(it) => object_order(it, reads),
        Expression::SequenceExpression(it) => sequence(&it.expressions, reads),
        Expression::UnaryExpression(it) => match it.operator {
            oxc_syntax::operator::UnaryOperator::Typeof
            | oxc_syntax::operator::UnaryOperator::Void
            | oxc_syntax::operator::UnaryOperator::LogicalNot => order(&it.argument, reads),
            _ => after_operands(order(&it.argument, reads)),
        },
        _ => Order::Impure,
    }
}

pub fn statement_order<R: Reads + ?Sized>(statement: &Statement<'_>, reads: &R) -> Order {
    match statement {
        Statement::ExpressionStatement(it) => order(&it.expression, reads),
        Statement::IfStatement(it) => order(&it.test, reads),
        Statement::ThrowStatement(it) => order(&it.argument, reads),
        Statement::VariableDeclaration(it) => match it.declarations.first() {
            Some(VariableDeclarator {
                id: BindingPattern::BindingIdentifier(_),
                init: Some(init),
                ..
            }) => order(init, reads),
            _ => Order::Impure,
        },
        _ => Order::Impure,
    }
}

struct Contains<'r, R: ?Sized> {
    reads: &'r R,
    found: bool,
}

impl<'a, R: Reads + ?Sized> VisitJs<'a> for Contains<'_, R> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.found {
            return;
        }
        if self.reads.is_read(it) {
            self.found = true;
            return;
        }
        walk_js::walk_expression(self, it);
    }
}

pub fn contains<R: Reads + ?Sized>(expression: &Expression<'_>, reads: &R) -> bool {
    let mut visitor = Contains {
        reads,
        found: false,
    };
    visitor.visit_expression(expression);
    visitor.found
}

pub struct Uses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    nested: u32,
    pub inside: usize,
    pub closed_over: bool,
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.nested += 1;
        walk_js::walk_function(self, it, flags);
        self.nested -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nested += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.nested -= 1;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let hit = it.reference_id.get().is_some_and(|reference| {
            self.scoping.get_reference(reference).symbol_id() == Some(self.symbol)
        });
        if hit {
            self.inside += 1;
            self.closed_over |= self.nested > 0;
        }
    }
}

impl<'s> Uses<'s> {
    pub fn new(scoping: &'s Scoping, symbol: SymbolId) -> Self {
        Self {
            scoping,
            symbol,
            nested: 0,
            inside: 0,
            closed_over: false,
        }
    }
}

#[derive(Default)]
pub struct Plain<'a> {
    frame: Frame,
    loops: u32,
    breakables: u32,
    labels: Vec<&'a str>,
    pub bad: bool,
    pub seen_labels: Vec<&'a str>,
}

fn reserved(name: &str) -> bool {
    matches!(name, "yield" | "await")
}

impl<'a> Plain<'a> {
    pub fn check(&mut self, statement: &Statement<'a>) {
        self.visit_statement(statement);
    }

    pub fn check_expression(&mut self, expression: &Expression<'a>) {
        self.visit_expression(expression);
    }

    fn in_body(&self) -> bool {
        self.frame.is_own()
    }

    fn at_top(&self) -> bool {
        self.frame.is_top()
    }
}

impl<'a> FrameScan<'a> for Plain<'a> {
    fn frame(&mut self) -> &mut Frame {
        &mut self.frame
    }
}

impl<'a> VisitJs<'a> for Plain<'a> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.bad {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let (loops, breakables) = (self.loops, self.breakables);
        let labels = std::mem::take(&mut self.labels);
        self.loops = 0;
        self.breakables = 0;
        frame::walk_function(self, it, flags);
        self.labels = labels;
        self.loops = loops;
        self.breakables = breakables;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let (loops, breakables) = (self.loops, self.breakables);
        let labels = std::mem::take(&mut self.labels);
        self.loops = 0;
        self.breakables = 0;
        frame::walk_arrow(self, it);
        self.labels = labels;
        self.loops = loops;
        self.breakables = breakables;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        if self.at_top() {
            self.bad = true;
        }
        walk_js::walk_return_statement(self, it);
    }

    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        let known = match &it.label {
            Some(label) => self.labels.contains(&label.name.as_str()),
            None => self.breakables > 0,
        };
        self.bad |= !known;
    }

    fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
        let known = match &it.label {
            Some(label) => self.labels.contains(&label.name.as_str()),
            None => self.loops > 0,
        };
        self.bad |= !known;
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        let name = it.label.name.as_str();
        self.bad |= reserved(name);
        self.seen_labels.push(name);
        self.labels.push(name);
        walk_js::walk_labeled_statement(self, it);
        self.labels.pop();
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.breakables += 1;
        walk_js::walk_switch_statement(self, it);
        self.breakables -= 1;
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.enter_loop();
        walk_js::walk_for_statement(self, it);
        self.leave_loop();
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.enter_loop();
        walk_js::walk_for_in_statement(self, it);
        self.leave_loop();
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.bad |= it.r#await;
        self.enter_loop();
        walk_js::walk_for_of_statement(self, it);
        self.leave_loop();
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.enter_loop();
        walk_js::walk_while_statement(self, it);
        self.leave_loop();
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.enter_loop();
        walk_js::walk_do_while_statement(self, it);
        self.leave_loop();
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let name = it.name.as_str();
        self.bad |= reserved(name) || (name == "arguments" && self.in_body());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.bad |= reserved(it.name.as_str());
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.bad |= self.in_body();
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        self.bad |= self.at_top();
        walk_js::walk_yield_expression(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        self.bad |= self.at_top();
        walk_js::walk_await_expression(self, it);
    }
}

impl Plain<'_> {
    fn enter_loop(&mut self) {
        self.loops += 1;
        self.breakables += 1;
    }

    fn leave_loop(&mut self) {
        self.loops -= 1;
        self.breakables -= 1;
    }
}
