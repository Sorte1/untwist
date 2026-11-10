use oxc_allocator::{Allocator, CloneIn, TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_span::{SourceType, SPAN};
use oxc_str::Ident;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::print::{print, PrintStyle};

use crate::model::Runtime;
use crate::webpack::forms::FORMS;
use crate::webpack::locals::scoped;

const REQUIRE: &str = "$R";
const UNDEFINED: &str = "undefined";

#[derive(Default)]
struct Bound {
    names: FxHashSet<String>,
}

impl<'a> VisitJs<'a> for Bound {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.to_string());
    }
}

struct Rename<'n, 'a> {
    builder: AstBuilder<'a>,
    require: &'n str,
    bound: FxHashSet<String>,
    names: FxHashMap<String, String>,
}

impl<'a> Rename<'_, 'a> {
    fn renamed(&mut self, name: &str) -> Option<Ident<'a>> {
        let fresh = if name == self.require {
            REQUIRE.to_string()
        } else if self.bound.contains(name) {
            let next = format!("${}", self.names.len());
            self.names.entry(name.to_string()).or_insert(next).clone()
        } else {
            return None;
        };
        Some(Ident::from_str_in(&fresh, &self.builder))
    }

    fn folded(&self, expr: &mut Expression<'a>) -> Option<Expression<'a>> {
        match expr {
            Expression::ParenthesizedExpression(inner) => {
                Some(inner.expression.take_in(&self.builder))
            }
            Expression::UnaryExpression(unary) => {
                let Expression::NumericLiteral(number) = &unary.argument else {
                    return None;
                };
                match unary.operator {
                    UnaryOperator::LogicalNot if number.value == 0.0 || number.value == 1.0 => {
                        Some(Expression::new_boolean_literal(
                            SPAN,
                            number.value == 0.0,
                            &self.builder,
                        ))
                    }
                    UnaryOperator::Void if !self.bound.contains(UNDEFINED) => {
                        Some(Expression::new_identifier(
                            SPAN,
                            Ident::from_str_in(UNDEFINED, &self.builder),
                            &self.builder,
                        ))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn is_typeof(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Typeof)
}

fn literal_first(compare: &BinaryExpression<'_>) -> bool {
    let literal = matches!(
        compare.left,
        Expression::StringLiteral(_) | Expression::NumericLiteral(_)
    );
    literal
        && !matches!(
            compare.right,
            Expression::StringLiteral(_) | Expression::NumericLiteral(_)
        )
}

fn ordered(compare: &mut BinaryExpression<'_>) {
    let typed = (is_typeof(&compare.left) && matches!(compare.right, Expression::StringLiteral(_)))
        || (is_typeof(&compare.right) && matches!(compare.left, Expression::StringLiteral(_)));
    if typed {
        compare.operator = match compare.operator {
            BinaryOperator::Equality => BinaryOperator::StrictEquality,
            BinaryOperator::Inequality => BinaryOperator::StrictInequality,
            other => other,
        };
    }
    let symmetric = matches!(
        compare.operator,
        BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::BitwiseAnd
    );
    if symmetric
        && literal_first(compare)
        && (typed || compare.operator == BinaryOperator::BitwiseAnd)
    {
        std::mem::swap(&mut compare.left, &mut compare.right);
    }
}

impl<'a> Rename<'_, 'a> {
    fn block(&self, statement: Statement<'a>) -> Statement<'a> {
        match statement {
            Statement::BlockStatement(_) => statement,
            other => {
                let mut body = AVec::with_capacity_in(1, &self.builder);
                body.push(other);
                Statement::new_block_statement(SPAN, body, &self.builder)
            }
        }
    }

    fn effects(
        &self,
        expr: Expression<'a>,
        out: &mut AVec<'a, Statement<'a>>,
    ) -> Option<Expression<'a>> {
        match expr {
            Expression::SequenceExpression(sequence) => {
                let mut parts = sequence.unbox().expressions;
                let last = parts.pop();
                for part in parts {
                    self.push(
                        Statement::new_expression_statement(SPAN, part, &self.builder),
                        out,
                    );
                }
                last
            }
            Expression::ParenthesizedExpression(inner) => {
                self.effects(inner.unbox().expression, out)
            }
            other => Some(other),
        }
    }

    fn push(&self, statement: Statement<'a>, out: &mut AVec<'a, Statement<'a>>) {
        match statement {
            Statement::ExpressionStatement(statement) => {
                let Some(last) = self.effects(statement.unbox().expression, out) else {
                    return;
                };
                let statement = match last {
                    Expression::LogicalExpression(logical)
                        if logical.operator != LogicalOperator::Coalesce =>
                    {
                        let logical = logical.unbox();
                        let test = match logical.operator {
                            LogicalOperator::And => logical.left,
                            _ => Expression::new_unary_expression(
                                SPAN,
                                UnaryOperator::LogicalNot,
                                logical.left,
                                &self.builder,
                            ),
                        };
                        let then =
                            Statement::new_expression_statement(SPAN, logical.right, &self.builder);
                        Statement::new_if_statement(SPAN, test, then, None, &self.builder)
                    }
                    other => Statement::new_expression_statement(SPAN, other, &self.builder),
                };
                match statement {
                    Statement::IfStatement(_) => self.push(statement, out),
                    other => out.push(other),
                }
            }
            Statement::ReturnStatement(ret) => {
                let argument = ret
                    .unbox()
                    .argument
                    .and_then(|argument| self.effects(argument, out));
                out.push(Statement::new_return_statement(
                    SPAN,
                    argument,
                    &self.builder,
                ));
            }
            Statement::IfStatement(branch) => {
                let branch = branch.unbox();
                let Some(test) = self.effects(branch.test, out) else {
                    return;
                };
                let consequent = self.block(branch.consequent);
                let alternate = branch.alternate.map(|alternate| self.block(alternate));
                out.push(Statement::new_if_statement(
                    SPAN,
                    test,
                    consequent,
                    alternate,
                    &self.builder,
                ));
            }
            other => out.push(other),
        }
    }
}

impl<'a> VisitMut<'a> for Rename<'_, 'a> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        let old = std::mem::replace(it, AVec::new_in(&self.builder));
        for statement in old {
            self.push(statement, it);
        }
        walk_mut::walk_statements(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        while let Some(simpler) = self.folded(it) {
            *it = simpler;
        }
        if let Expression::BinaryExpression(compare) = it {
            ordered(compare);
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if let Some(name) = self.renamed(&it.name) {
            it.name = name;
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        if let Some(name) = self.renamed(&it.name) {
            it.name = name;
        }
    }
}

fn canonical(statement: &Statement<'_>, require: &str) -> Option<String> {
    let alloc = Allocator::default();
    let builder = AstBuilder::new(&alloc);
    let mut bound = Bound::default();
    bound.visit_statement(statement);
    let (_, free) = scoped(&alloc, statement.clone_in(&alloc));
    if free
        .iter()
        .any(|(name, _)| bound.names.contains(name) || name.starts_with('$'))
    {
        return None;
    }
    let mut body = AVec::with_capacity_in(1, &builder);
    body.push(statement.clone_in(&alloc));
    let mut program = Program::new(
        SPAN,
        SourceType::cjs(),
        "",
        AVec::new_in(&builder),
        None,
        AVec::new_in(&builder),
        body,
        &builder,
    );
    let mut rename = Rename {
        builder,
        require,
        bound: bound.names,
        names: FxHashMap::default(),
    };
    rename.visit_program(&mut program);
    Some(print(&program, PrintStyle::Compact))
}

fn unwrapped<'k, 'a>(statement: &'k Statement<'a>) -> &'k Statement<'a> {
    let Statement::ExpressionStatement(expression) = statement else {
        return statement;
    };
    let Expression::CallExpression(call) = expression.expression.without_parentheses() else {
        return statement;
    };
    let Expression::ArrowFunctionExpression(arrow) = call.callee.without_parentheses() else {
        return statement;
    };
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        return statement;
    };
    match (
        &body.statements[..],
        call.arguments.is_empty(),
        arrow.params.items.is_empty(),
    ) {
        ([only], true, true) if body.directives.is_empty() && !arrow.r#async => only,
        _ => statement,
    }
}

pub fn canonical_helper(statement: &Statement<'_>, require: &str) -> Option<String> {
    canonical(unwrapped(statement), require)
}

pub fn standard_helpers(runtime: &Runtime<'_>) -> bool {
    let mut last: FxHashMap<&str, &Statement<'_>> = FxHashMap::default();
    for helper in &runtime.helpers {
        last.insert(helper.name.as_str(), &helper.statement);
    }
    FORMS.iter().all(|(name, forms)| {
        last.get(name).is_none_or(|statement| {
            canonical_helper(statement, &runtime.require_name)
                .is_some_and(|text| forms.contains(&text.as_str()))
        })
    })
}
