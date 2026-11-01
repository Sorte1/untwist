use oxc_ast::ast::*;
use oxc_semantic::{Scoping, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default)]
pub(super) struct NumericKeys {
    declared: FxHashSet<SymbolId>,
    spoiled: FxHashSet<SymbolId>,
    depends: FxHashMap<SymbolId, Vec<SymbolId>>,
    required: Vec<SymbolId>,
}

fn always_numeric(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::Exponential
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::BitwiseAnd
    )
}

pub(super) fn numeric_deps(
    expr: &Expression<'_>,
    scoping: &Scoping,
    deps: &mut Vec<SymbolId>,
) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(_) | Expression::UpdateExpression(_) => true,
        Expression::UnaryExpression(unary) => matches!(
            unary.operator,
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
        ),
        Expression::BinaryExpression(binary) if always_numeric(binary.operator) => true,
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            numeric_deps(&binary.left, scoping, deps) && numeric_deps(&binary.right, scoping, deps)
        }
        Expression::ConditionalExpression(conditional) => {
            numeric_deps(&conditional.consequent, scoping, deps)
                && numeric_deps(&conditional.alternate, scoping, deps)
        }
        Expression::Identifier(id) => {
            let symbol = id
                .reference_id
                .get()
                .and_then(|r| scoping.get_reference(r).symbol_id());
            symbol.map(|symbol| deps.push(symbol)).is_some()
        }
        _ => false,
    }
}

fn is_number(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::NumericLiteral(_))
}

pub(super) fn numeric_overwrite(value: &Expression<'_>) -> bool {
    match value.without_parentheses() {
        Expression::BinaryExpression(binary) => {
            always_numeric(binary.operator)
                && binary.operator != BinaryOperator::Exponential
                && (is_number(&binary.left) || is_number(&binary.right))
        }
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) && is_number(&unary.argument)
        }
        _ => false,
    }
}

fn simple_parameters(params: &FormalParameters<'_>) -> bool {
    params.rest.is_none()
        && params.items.iter().all(|item| {
            item.initializer.is_none()
                && matches!(item.pattern, BindingPattern::BindingIdentifier(_))
        })
}

pub(super) fn overwritten_parameter(
    function: &Function<'_>,
    scoping: &Scoping,
) -> Option<SymbolId> {
    if !simple_parameters(&function.params) {
        return None;
    }
    let Some(Statement::ExpressionStatement(first)) = function.body.as_ref()?.statements.first()
    else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = first.expression.without_parentheses()
    else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign || !numeric_overwrite(&assignment.right) {
        return None;
    }
    let symbol = target
        .reference_id
        .get()
        .and_then(|r| scoping.get_reference(r).symbol_id())?;
    function
        .params
        .items
        .iter()
        .filter_map(|item| item.pattern.get_binding_identifier())
        .any(|id| id.symbol_id.get() == Some(symbol))
        .then_some(symbol)
}

impl NumericKeys {
    pub(super) fn declare(&mut self, symbol: SymbolId) {
        self.declared.insert(symbol);
    }

    pub(super) fn spoil(&mut self, symbol: SymbolId) {
        self.spoiled.insert(symbol);
    }

    pub(super) fn assign(&mut self, symbol: SymbolId, value: &Expression<'_>, scoping: &Scoping) {
        let mut deps = Vec::new();
        if numeric_deps(value, scoping, &mut deps) {
            self.depends.entry(symbol).or_default().extend(deps);
        } else {
            self.spoiled.insert(symbol);
        }
    }

    pub(super) fn require(&mut self, symbol: SymbolId) {
        self.required.push(symbol);
    }

    fn numeric(&self, symbol: &SymbolId) -> bool {
        self.declared.contains(symbol) && !self.spoiled.contains(symbol)
    }

    pub(super) fn holds(mut self) -> bool {
        loop {
            let newly: Vec<SymbolId> = self
                .depends
                .iter()
                .filter(|(symbol, deps)| {
                    !self.spoiled.contains(symbol) && deps.iter().any(|dep| !self.numeric(dep))
                })
                .map(|(symbol, _)| *symbol)
                .collect();
            if newly.is_empty() {
                break;
            }
            self.spoiled.extend(newly);
        }
        self.required.iter().all(|symbol| self.numeric(symbol))
    }
}
