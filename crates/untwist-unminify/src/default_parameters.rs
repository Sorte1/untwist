use crate::transpile_util::{is_undefined_identifier, reference_symbol};
use oxc_allocator::{Box as ABox, Vec as AVec};
use oxc_ast::ast::*;
use oxc_span::SPAN;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::ast_util::scope::binding_symbol;
use untwist_core::pass::{Cx, Interest, Locality, Pass};

const MAX_PARAMETER_INDEX: usize = 256;

#[derive(Default)]
pub struct DefaultParameters;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DefaultParameters)
}

#[derive(Clone, Copy)]
enum DefaultForm {
    Expression,
    False,
    True,
    Plain,
}

impl<'a> Pass<'a> for DefaultParameters {
    fn name(&self) -> &'static str {
        "default-parameters"
    }

    fn locality(&self) -> Locality {
        Locality::Local
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_FUNCTION | Interest::EXIT_ARROW_FUNCTION_EXPRESSION
    }

    fn exit_function(&mut self, node: &mut Function<'a>, cx: &mut Cx<'a, '_>) {
        let Function { params, body, .. } = node;
        let Some(body) = body.as_deref_mut() else {
            return;
        };
        restore_parameters(params, &mut body.statements, cx);
    }

    fn exit_arrow_function_expression(
        &mut self,
        node: &mut ArrowFunctionExpression<'a>,
        cx: &mut Cx<'a, '_>,
    ) {
        let ArrowFunctionExpression { params, body, .. } = node;
        let ArrowFunctionBody::FunctionBody(body) = body else {
            return;
        };
        restore_parameters(params, &mut body.statements, cx);
    }
}

fn restore_parameters<'a>(
    params: &mut FormalParameters<'a>,
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) {
    if params.rest.is_some() {
        return;
    }
    while restore_one(params, stmts, cx) {
        cx.changed();
    }
}

fn restore_one<'a>(
    params: &mut FormalParameters<'a>,
    stmts: &mut AVec<'a, Statement<'a>>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let Some(first) = stmts.first_mut() else {
        return false;
    };
    match first {
        Statement::VariableDeclaration(declaration) => {
            let Some((index, form)) = classify_declaration(declaration, cx) else {
                return false;
            };
            let initialised = !matches!(form, DefaultForm::Plain);
            if cx.opts.extended && !keeps_length(params, index, initialised) {
                return false;
            }
            let Some(parameter) = take_parameter(declaration, form, cx) else {
                return false;
            };
            stmts.remove(0);
            place_parameter(params, index, parameter, cx);
            true
        }
        Statement::IfStatement(statement) => {
            let Some(symbol) = loose_default_symbol(statement, cx) else {
                return false;
            };
            let Some(position) = parameter_position(params, symbol) else {
                return false;
            };
            if cx.opts.extended && !keeps_length(params, position, true) {
                return false;
            }
            let Some(value) = take_loose_default_value(statement, cx) else {
                return false;
            };
            stmts.remove(0);
            params.items[position].initializer = Some(ABox::new_in(value, cx));
            true
        }
        _ => false,
    }
}

fn function_length(initialised: impl Iterator<Item = bool>) -> usize {
    let mut length = 0;
    for has_initializer in initialised {
        if has_initializer {
            break;
        }
        length += 1;
    }
    length
}

fn keeps_length(params: &FormalParameters<'_>, index: usize, initialised: bool) -> bool {
    let before = params.items.iter().map(|item| item.initializer.is_some());
    let count = params.items.len().max(index + 1);
    let after = (0..count).map(|position| match params.items.get(position) {
        _ if position == index => initialised,
        Some(item) => item.initializer.is_some(),
        None => false,
    });
    function_length(before) == function_length(after)
}

fn classify_declaration(
    declaration: &VariableDeclaration<'_>,
    cx: &Cx<'_, '_>,
) -> Option<(usize, DefaultForm)> {
    if declaration.declarations.len() != 1 {
        return None;
    }
    let declarator = &declaration.declarations[0];
    if matches!(declarator.id, BindingPattern::AssignmentPattern(_)) {
        return None;
    }
    let init = declarator.init.as_ref()?;
    match init {
        Expression::ConditionalExpression(conditional) => {
            if let Some(index) = argument_check_and(&conditional.test, cx) {
                if is_arguments_index(&conditional.consequent, index) {
                    return Some((index, DefaultForm::Expression));
                }
            }
            let index = argument_length_greater(&conditional.test)?;
            if is_arguments_index(&conditional.consequent, index)
                && is_undefined_identifier(&conditional.alternate, cx)
            {
                return Some((index, DefaultForm::Plain));
            }
            None
        }
        Expression::LogicalExpression(logical) => match logical.operator {
            LogicalOperator::And => {
                let index = argument_check_and(&logical.left, cx)?;
                is_arguments_index(&logical.right, index).then_some((index, DefaultForm::False))
            }
            LogicalOperator::Or => {
                let index = argument_check_or(&logical.left, cx)?;
                is_arguments_index(&logical.right, index).then_some((index, DefaultForm::True))
            }
            LogicalOperator::Coalesce => None,
        },
        _ => None,
    }
}

fn argument_check_and(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<usize> {
    let Expression::LogicalExpression(logical) = expr else {
        return None;
    };
    if logical.operator != LogicalOperator::And {
        return None;
    }
    let index = argument_length_greater(&logical.left)?;
    argument_undefined_check(&logical.right, index, BinaryOperator::StrictInequality, cx)
        .then_some(index)
}

fn argument_check_or(expr: &Expression<'_>, cx: &Cx<'_, '_>) -> Option<usize> {
    let Expression::LogicalExpression(logical) = expr else {
        return None;
    };
    if logical.operator != LogicalOperator::Or {
        return None;
    }
    let index = binary_arguments_length(&logical.left, BinaryOperator::LessEqualThan)?;
    argument_undefined_check(&logical.right, index, BinaryOperator::StrictEquality, cx)
        .then_some(index)
}

fn argument_undefined_check(
    expr: &Expression<'_>,
    index: usize,
    operator: BinaryOperator,
    cx: &Cx<'_, '_>,
) -> bool {
    let Expression::BinaryExpression(binary) = expr else {
        return false;
    };
    binary.operator == operator
        && is_arguments_index(&binary.left, index)
        && is_undefined_identifier(&binary.right, cx)
}

fn argument_length_greater(expr: &Expression<'_>) -> Option<usize> {
    binary_arguments_length(expr, BinaryOperator::GreaterThan)
}

fn binary_arguments_length(expr: &Expression<'_>, operator: BinaryOperator) -> Option<usize> {
    let Expression::BinaryExpression(binary) = expr else {
        return None;
    };
    if binary.operator != operator || !is_arguments_length(&binary.left) {
        return None;
    }
    literal_index(&binary.right)
}

fn literal_index(expr: &Expression<'_>) -> Option<usize> {
    let Expression::NumericLiteral(number) = expr else {
        return None;
    };
    let value = number.value;
    if value < 0.0 || value.fract() != 0.0 || value > MAX_PARAMETER_INDEX as f64 {
        return None;
    }
    Some(value as usize)
}

fn is_arguments(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::Identifier(identifier) if identifier.name.as_str() == "arguments"
    )
}

fn is_arguments_length(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::StaticMemberExpression(member) => {
            !member.optional
                && is_arguments(&member.object)
                && member.property.name.as_str() == "length"
        }
        Expression::ComputedMemberExpression(member) => {
            !member.optional
                && is_arguments(&member.object)
                && matches!(
                    &member.expression,
                    Expression::StringLiteral(text) if text.value.as_str() == "length"
                )
        }
        _ => false,
    }
}

fn is_arguments_index(expr: &Expression<'_>, index: usize) -> bool {
    let Expression::ComputedMemberExpression(member) = expr else {
        return false;
    };
    !member.optional
        && is_arguments(&member.object)
        && literal_index(&member.expression) == Some(index)
}

fn take_parameter<'a>(
    declaration: &mut VariableDeclaration<'a>,
    form: DefaultForm,
    cx: &mut Cx<'a, '_>,
) -> Option<FormalParameter<'a>> {
    let declarator = declaration.declarations.get_mut(0)?;
    let span = declarator.span;
    let value = match form {
        DefaultForm::Plain => None,
        DefaultForm::False => Some(Expression::new_boolean_literal(span, false, cx)),
        DefaultForm::True => Some(Expression::new_boolean_literal(span, true, cx)),
        DefaultForm::Expression => match declarator.init.take() {
            Some(Expression::ConditionalExpression(conditional)) => {
                Some(conditional.unbox().alternate)
            }
            other => {
                declarator.init = other;
                return None;
            }
        },
    };
    let declarator = declaration.declarations.remove(0);
    let VariableDeclarator { id, .. } = declarator;
    let initializer = value.map(|expression| ABox::new_in(expression, cx));
    let decorators: AVec<'a, Decorator<'a>> = AVec::new_in(cx);
    Some(FormalParameter::new(
        span,
        decorators,
        id,
        None,
        initializer,
        false,
        None,
        false,
        false,
        cx,
    ))
}

fn place_parameter<'a>(
    params: &mut FormalParameters<'a>,
    index: usize,
    parameter: FormalParameter<'a>,
    cx: &mut Cx<'a, '_>,
) {
    while params.items.len() < index {
        let uid = cx
            .tcx
            .generate_uid_in_current_scope("param", SymbolFlags::FunctionScopedVariable);
        let pattern = uid.create_binding_pattern(cx.tcx);
        let decorators: AVec<'a, Decorator<'a>> = AVec::new_in(cx);
        params.items.push(FormalParameter::new(
            SPAN, decorators, pattern, None, None, false, None, false, false, cx,
        ));
    }
    if index < params.items.len() {
        params.items[index] = parameter;
    } else {
        params.items.push(parameter);
    }
}

fn parameter_position(params: &FormalParameters<'_>, symbol: SymbolId) -> Option<usize> {
    params.items.iter().position(|item| {
        item.initializer.is_none() && binding_symbol(&item.pattern) == Some(symbol)
    })
}

fn loose_default_symbol(statement: &IfStatement<'_>, cx: &Cx<'_, '_>) -> Option<SymbolId> {
    if statement.alternate.is_some() {
        return None;
    }
    let Expression::BinaryExpression(test) = &statement.test else {
        return None;
    };
    if test.operator != BinaryOperator::StrictEquality || !is_undefined_identifier(&test.right, cx)
    {
        return None;
    }
    let Expression::Identifier(name) = &test.left else {
        return None;
    };
    let symbol = reference_symbol(name, cx)?;
    let Statement::BlockStatement(block) = &statement.consequent else {
        return None;
    };
    if block.body.len() != 1 {
        return None;
    }
    let Statement::ExpressionStatement(inner) = &block.body[0] else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &inner.expression else {
        return None;
    };
    if assignment.operator != AssignmentOperator::Assign {
        return None;
    }
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (reference_symbol(target, cx) == Some(symbol)).then_some(symbol)
}

fn take_loose_default_value<'a>(
    statement: &mut IfStatement<'a>,
    cx: &mut Cx<'a, '_>,
) -> Option<Expression<'a>> {
    let Statement::BlockStatement(block) = &mut statement.consequent else {
        return None;
    };
    let first = block.body.first_mut()?;
    let Statement::ExpressionStatement(inner) = first else {
        return None;
    };
    let taken = std::mem::replace(
        &mut inner.expression,
        Expression::new_null_literal(SPAN, cx),
    );
    match taken {
        Expression::AssignmentExpression(assignment) => Some(assignment.unbox().right),
        other => {
            inner.expression = other;
            None
        }
    }
}
