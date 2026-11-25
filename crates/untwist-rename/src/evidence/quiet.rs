use oxc_ast::ast::*;
use oxc_syntax::operator::UnaryOperator;

pub fn quiet_statements(statements: &[Statement<'_>]) -> bool {
    let mut cached = Vec::new();
    collect_returned_paths(statements, &mut cached);
    statements
        .iter()
        .all(|statement| quiet_statement(statement, &cached))
}

fn collect_returned_paths(statements: &[Statement<'_>], out: &mut Vec<String>) {
    for statement in statements {
        match statement {
            Statement::ReturnStatement(ret) => {
                if let Some(path) = ret.argument.as_ref().and_then(member_path) {
                    out.push(path);
                }
            }
            Statement::BlockStatement(block) => collect_returned_paths(&block.body, out),
            Statement::IfStatement(branch) => {
                collect_returned_paths(std::slice::from_ref(&branch.consequent), out);
                if let Some(alternate) = &branch.alternate {
                    collect_returned_paths(std::slice::from_ref(alternate), out);
                }
            }
            _ => {}
        }
    }
}

fn member_path(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.to_string()),
        Expression::ThisExpression(_) => Some("this".to_string()),
        Expression::StaticMemberExpression(member) => Some(format!(
            "{}.{}",
            member_path(&member.object)?,
            member.property.name
        )),
        _ => None,
    }
}

fn target_path(target: &AssignmentTarget<'_>) -> Option<String> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => Some(format!(
            "{}.{}",
            member_path(&member.object)?,
            member.property.name
        )),
        _ => None,
    }
}

fn quiet_statement(statement: &Statement<'_>, cached: &[String]) -> bool {
    match statement {
        Statement::ReturnStatement(_)
        | Statement::EmptyStatement(_)
        | Statement::FunctionDeclaration(_)
        | Statement::ThrowStatement(_) => true,
        Statement::VariableDeclaration(declaration) => {
            declaration.kind != VariableDeclarationKind::Using
                && declaration.kind != VariableDeclarationKind::AwaitUsing
                && declaration
                    .declarations
                    .iter()
                    .all(|declarator| declarator.init.as_ref().is_none_or(is_pure))
        }
        Statement::BlockStatement(block) => block
            .body
            .iter()
            .all(|statement| quiet_statement(statement, cached)),
        Statement::IfStatement(branch) => {
            is_pure(&branch.test)
                && quiet_statement(&branch.consequent, cached)
                && branch
                    .alternate
                    .as_ref()
                    .is_none_or(|alternate| quiet_statement(alternate, cached))
        }
        Statement::ExpressionStatement(statement) => {
            is_cache_fill(&statement.expression, cached) || is_pure(&statement.expression)
        }
        _ => false,
    }
}

fn is_cache_fill(expr: &Expression<'_>, cached: &[String]) -> bool {
    match expr.without_parentheses() {
        Expression::AssignmentExpression(assign) => {
            target_path(&assign.left).is_some_and(|path| cached.contains(&path))
        }
        Expression::LogicalExpression(logical) => {
            is_pure(&logical.left) && is_cache_fill(&logical.right, cached)
        }
        _ => false,
    }
}

fn is_pure(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.iter().all(is_pure),
        Expression::StaticMemberExpression(member) => is_pure(&member.object),
        Expression::ComputedMemberExpression(member) => {
            is_pure(&member.object) && is_pure(&member.expression)
        }
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_pure(&unary.argument)
        }
        Expression::BinaryExpression(binary) => is_pure(&binary.left) && is_pure(&binary.right),
        Expression::LogicalExpression(logical) => is_pure(&logical.left) && is_pure(&logical.right),
        Expression::ConditionalExpression(conditional) => {
            is_pure(&conditional.test)
                && is_pure(&conditional.consequent)
                && is_pure(&conditional.alternate)
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().all(is_pure),
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
            element.as_expression().is_some_and(is_pure)
                || matches!(element, ArrayExpressionElement::Elision(_))
        }),
        Expression::ObjectExpression(object) => {
            object.properties.iter().all(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    !property.computed && is_pure(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(_) => false,
            })
        }
        _ => false,
    }
}
