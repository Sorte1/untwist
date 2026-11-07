use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::inline::returned_expression;

use super::eval::{literal_argument, literal_data};
use crate::bindings::{is_block_function, resolved_symbol};

pub(super) struct Func<'p> {
    pub(super) symbol: SymbolId,
    pub(super) statement: Span,
    pub(super) hoisted: bool,
    pub(super) params: Vec<SymbolId>,
    pub(super) body: &'p Expression<'p>,
}

pub(super) struct Constant<'p> {
    pub(super) statement: Span,
    pub(super) init: &'p Expression<'p>,
}

#[derive(Default)]
pub(super) struct Declared<'p> {
    pub(super) funcs: Vec<Func<'p>>,
    pub(super) constants: FxHashMap<SymbolId, Constant<'p>>,
    pub(super) called: FxHashSet<SymbolId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    Function,
    Block,
}

struct Scan<'s, 'p> {
    scoping: &'s Scoping,
    found: Declared<'p>,
}

pub(super) fn declared<'p>(program: &'p Program<'p>, scoping: &Scoping) -> Declared<'p> {
    let mut scan = Scan {
        scoping,
        found: Declared::default(),
    };
    scan.visit_program(program);
    scan.found
}

pub(super) fn simple_params(params: &FormalParameters<'_>) -> Option<Vec<SymbolId>> {
    if params.rest.is_some() {
        return None;
    }
    let mut names = FxHashSet::default();
    let mut symbols = Vec::with_capacity(params.items.len());
    for item in &params.items {
        let BindingPattern::BindingIdentifier(id) = &item.pattern else {
            return None;
        };
        if item.initializer.is_some() || !names.insert(id.name.as_str()) {
            return None;
        }
        symbols.push(id.symbol_id.get()?);
    }
    Some(symbols)
}

fn function_body<'k, 'p>(function: &'k Function<'p>) -> Option<&'k Expression<'p>> {
    if function.r#async || function.generator {
        return None;
    }
    returned_expression(function)
}

fn arrow_body<'k, 'p>(arrow: &'k ArrowFunctionExpression<'p>) -> Option<&'k Expression<'p>> {
    if arrow.r#async {
        return None;
    }
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        return arrow.body.as_expression();
    };
    let [Statement::ReturnStatement(statement)] = body.statements.as_slice() else {
        return None;
    };
    statement.argument.as_ref()
}

fn initialised_function<'k, 'p>(
    init: &'k Expression<'p>,
) -> Option<(&'k Expression<'p>, Vec<SymbolId>)> {
    match init {
        Expression::ArrowFunctionExpression(arrow) => {
            Some((arrow_body(arrow)?, simple_params(&arrow.params)?))
        }
        Expression::FunctionExpression(function) => {
            Some((function_body(function)?, simple_params(&function.params)?))
        }
        _ => None,
    }
}

impl<'p> Scan<'_, 'p> {
    fn list(&mut self, statements: &AVec<'p, Statement<'p>>, level: Level) {
        for statement in statements {
            let statement: &'p Statement<'p> = self.alloc(statement);
            match statement {
                Statement::FunctionDeclaration(function) => {
                    self.function_declaration(function, statement.span());
                }
                Statement::VariableDeclaration(declaration) => {
                    self.variable(declaration, statement.span(), level);
                }
                _ => {}
            }
        }
    }

    fn function_declaration(&mut self, function: &'p Function<'p>, statement: Span) {
        let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) else {
            return;
        };
        if is_block_function(self.scoping, symbol) {
            return;
        }
        let (Some(body), Some(params)) = (function_body(function), simple_params(&function.params))
        else {
            return;
        };
        self.found.funcs.push(Func {
            symbol,
            statement,
            hoisted: true,
            params,
            body,
        });
    }

    fn variable(
        &mut self,
        declaration: &'p VariableDeclaration<'p>,
        statement: Span,
        level: Level,
    ) {
        let lexical = matches!(
            declaration.kind,
            VariableDeclarationKind::Let | VariableDeclarationKind::Const
        );
        if !lexical
            && (declaration.kind != VariableDeclarationKind::Var || level != Level::Function)
        {
            return;
        }
        let [declarator] = declaration.declarations.as_slice() else {
            return;
        };
        let (BindingPattern::BindingIdentifier(id), Some(init)) =
            (&declarator.id, &declarator.init)
        else {
            return;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return;
        };
        if let Some((body, params)) = initialised_function(init) {
            self.found.funcs.push(Func {
                symbol,
                statement,
                hoisted: false,
                params,
                body,
            });
            return;
        }
        if literal_data(init, false) {
            self.found
                .constants
                .insert(symbol, Constant { statement, init });
        }
    }
}

impl<'p> VisitJs<'p> for Scan<'_, 'p> {
    fn visit_call_expression(&mut self, it: &CallExpression<'p>) {
        if let Expression::Identifier(callee) = &it.callee {
            let literal = it.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expr| literal_argument(expr).is_some())
            });
            if let Some(symbol) = resolved_symbol(callee, self.scoping).filter(|_| literal) {
                self.found.called.insert(symbol);
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_program(&mut self, it: &Program<'p>) {
        self.list(&it.body, Level::Function);
        walk_js::walk_program(self, it);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'p>) {
        self.list(&it.statements, Level::Function);
        walk_js::walk_function_body(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'p>) {
        self.list(&it.body, Level::Block);
        walk_js::walk_block_statement(self, it);
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'p>) {
        self.list(&it.body, Level::Block);
        walk_js::walk_static_block(self, it);
    }
}
