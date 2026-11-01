use oxc_ast::ast::*;

use crate::matcher::{anonymous_factory, unwrap_statement_expression, Factory};

const BUN_PARAMS: [&str; 5] = ["exports", "require", "module", "__filename", "__dirname"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Iife,
    Call,
    Umd,
    Bun,
}

pub struct Wrapper<'k, 'a> {
    pub shape: Shape,
    pub index: usize,
    pub function: Factory<'k, 'a>,
    pub arguments: &'k [Argument<'a>],
}

impl<'k, 'a> Wrapper<'k, 'a> {
    pub fn body(&self) -> &'k [Statement<'a>] {
        &self.function.body.statements
    }

    pub fn umd_factory(&self) -> Option<Factory<'k, 'a>> {
        if self.shape != Shape::Umd {
            return None;
        }
        self.arguments
            .iter()
            .filter_map(|argument| argument.as_expression())
            .find_map(anonymous_factory)
    }
}

pub struct Frame<'k, 'a> {
    pub top: &'k [Statement<'a>],
    pub wrapper: Option<Wrapper<'k, 'a>>,
}

impl<'k, 'a> Frame<'k, 'a> {
    pub fn of(program: &'k Program<'a>) -> Self {
        Self::of_statements(&program.body)
    }

    pub fn of_statements(top: &'k [Statement<'a>]) -> Self {
        let mut found = top
            .iter()
            .enumerate()
            .filter_map(|(index, statement)| wrapper_of(index, statement));
        let wrapper = match (found.next(), found.next()) {
            (Some(only), None) => Some(only),
            _ => None,
        };
        Self { top, wrapper }
    }

    pub fn inner(&self) -> &'k [Statement<'a>] {
        self.wrapper.as_ref().map_or(self.top, Wrapper::body)
    }

    pub fn lists(&self) -> impl Iterator<Item = &'k [Statement<'a>]> + '_ {
        std::iter::once(self.top).chain(self.wrapper.as_ref().map(Wrapper::body))
    }
}

fn wrapper_of<'k, 'a>(index: usize, statement: &'k Statement<'a>) -> Option<Wrapper<'k, 'a>> {
    let expression = match statement {
        Statement::ExpressionStatement(statement) => {
            unwrap_statement_expression(&statement.expression)
        }
        Statement::VariableDeclaration(declaration) if declaration.declarations.len() == 1 => {
            declaration.declarations[0].init.as_ref()?
        }
        _ => return None,
    };
    let expression = expression.without_parentheses();
    if let Some(function) = bun_function(expression) {
        return Some(Wrapper {
            shape: Shape::Bun,
            index,
            function,
            arguments: &[],
        });
    }
    let Expression::CallExpression(call) = expression else {
        return None;
    };
    let (shape, function) = called_function(&call.callee)?;
    let shape = if shape == Shape::Iife && holds_a_function(&call.arguments) {
        Shape::Umd
    } else {
        shape
    };
    Some(Wrapper {
        shape,
        index,
        function,
        arguments: &call.arguments,
    })
}

fn called_function<'k, 'a>(callee: &'k Expression<'a>) -> Option<(Shape, Factory<'k, 'a>)> {
    let callee = callee.without_parentheses();
    if let Some(function) = anonymous_factory(callee) {
        return Some((Shape::Iife, function));
    }
    let Expression::StaticMemberExpression(member) = callee else {
        return None;
    };
    if !matches!(member.property.name.as_str(), "call" | "apply") {
        return None;
    }
    anonymous_factory(member.object.without_parentheses()).map(|function| (Shape::Call, function))
}

fn holds_a_function(arguments: &[Argument<'_>]) -> bool {
    arguments
        .iter()
        .filter_map(|argument| argument.as_expression())
        .any(|argument| anonymous_factory(argument).is_some())
}

fn bun_function<'k, 'a>(expression: &'k Expression<'a>) -> Option<Factory<'k, 'a>> {
    let Expression::FunctionExpression(function) = expression else {
        return None;
    };
    let names = crate::matcher::param_names(&function.params);
    if names != BUN_PARAMS || function.params.rest.is_some() {
        return None;
    }
    anonymous_factory(expression)
}
