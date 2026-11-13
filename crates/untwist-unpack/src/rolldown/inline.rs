use std::cell::Cell;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::Span;
use oxc_syntax::scope::ScopeId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::esbuild::shape::{lazy_helper, simple_params};
use crate::matcher::{anonymous_factory, Factory};
use crate::rolldown::shape::commonjs_min;

pub struct InlineSite<'k, 'a> {
    pub statement: usize,
    pub declarator: usize,
    pub span: Span,
    pub function: Factory<'k, 'a>,
    pub scope: &'k Cell<Option<ScopeId>>,
    pub hint: Option<&'a str>,
}

fn literal_helper(callee: &Expression<'_>) -> bool {
    lazy_helper(callee).is_some_and(|(params, inner)| commonjs_min(&params, &inner))
}

fn factory<'k, 'a>(
    argument: &'k Argument<'a>,
) -> Option<(Factory<'k, 'a>, &'k Cell<Option<ScopeId>>)> {
    let expression = argument.as_expression()?.without_parentheses();
    let scope = match expression {
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            &function.scope_id
        }
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => &arrow.scope_id,
        _ => return None,
    };
    let found = anonymous_factory(expression)?;
    simple_params(found.params)
        .is_some_and(|params| params.len() <= 2)
        .then_some((found, scope))
}

struct Finder<'k, 'a, 'n> {
    helpers: &'n FxHashSet<&'n str>,
    statement: usize,
    declarator: usize,
    hint: Option<&'a str>,
    found: Vec<InlineSite<'k, 'a>>,
}

impl<'k, 'a> Finder<'k, 'a, '_> {
    fn inline(&self, call: &'k CallExpression<'a>) -> Option<InlineSite<'k, 'a>> {
        if call.optional || !call.arguments.is_empty() {
            return None;
        }
        let Expression::CallExpression(lazy) = call.callee.without_parentheses() else {
            return None;
        };
        let [argument] = &lazy.arguments[..] else {
            return None;
        };
        let helper = match lazy.callee.without_parentheses() {
            Expression::Identifier(id) => self.helpers.contains(id.name.as_str()),
            other => literal_helper(other),
        };
        if lazy.optional || !helper {
            return None;
        }
        let (function, scope) = factory(argument)?;
        Some(InlineSite {
            statement: self.statement,
            declarator: self.declarator,
            span: call.span,
            function,
            scope,
            hint: self.hint,
        })
    }

    fn expression(&mut self, expression: &'k Expression<'a>) {
        let mut stack = vec![expression];
        while let Some(expression) = stack.pop() {
            if let Expression::CallExpression(call) = expression.without_parentheses() {
                if let Some(site) = self.inline(call) {
                    self.found.push(site);
                    continue;
                }
            }
            let mut children = Children { found: Vec::new() };
            children.collect(expression);
            stack.extend(children.found.into_iter().rev());
        }
    }
}

struct Children<'k, 'a> {
    found: Vec<&'k Expression<'a>>,
}

impl<'k, 'a> Children<'k, 'a> {
    fn collect(&mut self, expression: &'k Expression<'a>) {
        match expression.without_parentheses() {
            Expression::CallExpression(call) => {
                self.found.push(&call.callee);
                self.found
                    .extend(call.arguments.iter().filter_map(Argument::as_expression));
            }
            Expression::SequenceExpression(sequence) => {
                self.found.extend(sequence.expressions.iter())
            }
            Expression::LogicalExpression(logical) => {
                self.found.push(&logical.left);
                self.found.push(&logical.right);
            }
            Expression::ConditionalExpression(choice) => {
                self.found.push(&choice.test);
                self.found.push(&choice.consequent);
                self.found.push(&choice.alternate);
            }
            Expression::StaticMemberExpression(member) => self.found.push(&member.object),
            Expression::ComputedMemberExpression(member) => {
                self.found.push(&member.object);
                self.found.push(&member.expression);
            }
            Expression::BinaryExpression(binary) => {
                self.found.push(&binary.left);
                self.found.push(&binary.right);
            }
            Expression::UnaryExpression(unary) => self.found.push(&unary.argument),
            Expression::AssignmentExpression(assign) => self.found.push(&assign.right),
            _ => {}
        }
    }
}

pub fn inline_sites<'k, 'a>(
    list: &'k [Statement<'a>],
    helpers: &FxHashSet<&str>,
) -> Vec<InlineSite<'k, 'a>> {
    let mut finder = Finder {
        helpers,
        statement: 0,
        declarator: 0,
        hint: None,
        found: Vec::new(),
    };
    for (index, statement) in list.iter().enumerate() {
        finder.statement = index;
        match statement {
            Statement::VariableDeclaration(declaration) => {
                for (at, declarator) in declaration.declarations.iter().enumerate() {
                    finder.declarator = at;
                    finder.hint = match &declarator.id {
                        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
                        _ => None,
                    };
                    if let Some(init) = &declarator.init {
                        finder.expression(init);
                    }
                }
            }
            Statement::ExpressionStatement(statement) => {
                finder.declarator = 0;
                finder.hint = None;
                finder.expression(&statement.expression);
            }
            _ => {}
        }
    }
    finder.found
}

struct Replace<'a, 'r> {
    builder: AstBuilder<'a>,
    sites: &'r FxHashMap<Span, &'a str>,
}

impl<'a> VisitMut<'a> for Replace<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        match self.sites.get(&it.span) {
            Some(&name) => it.callee = Expression::new_identifier(it.span, name, &self.builder),
            None => walk_mut::walk_call_expression(self, it),
        }
    }
}

pub fn replace_inline<'a>(
    alloc: &'a Allocator,
    statement: &mut Statement<'a>,
    sites: &[(Span, &str)],
) {
    if sites.is_empty() {
        return;
    }
    let sites: FxHashMap<Span, &'a str> = sites
        .iter()
        .map(|&(span, name)| (span, alloc.alloc_str(name) as &str))
        .collect();
    let mut replace = Replace {
        builder: AstBuilder::new(alloc),
        sites: &sites,
    };
    replace.visit_statement(statement);
}
