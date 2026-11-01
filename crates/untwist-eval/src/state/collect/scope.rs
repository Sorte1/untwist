use oxc_ast::ast::{
    ArrowFunctionBody, ArrowFunctionExpression, AssignmentTarget, BindingPattern, BlockStatement,
    Class, Expression, ForStatementInit, ForStatementLeft, FormalParameters, Function, Statement,
    SwitchStatement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;

use crate::stack;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FrameKind {
    Outer,
    Call,
    Nested,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Freshness {
    Stale,
    Object,
    Function,
}

pub(super) struct Binding {
    pub(super) frame: usize,
    pub(super) fresh: Freshness,
    pub(super) declared_at: Span,
    pub(super) declarations: u32,
    pub(super) writes: u32,
    pub(super) bare_uses: u32,
    pub(super) non_call_use: bool,
    pub(super) first_use: u32,
    pub(super) last_use: u32,
}

pub(super) struct Frame<'a> {
    pub(super) names: FxHashMap<&'a str, usize>,
    pub(super) kind: FrameKind,
    pub(super) is_arrow: bool,
    pub(super) is_block: bool,
    pub(super) start: u32,
}

pub(super) struct Declared<'a> {
    pub(super) name: &'a str,
    pub(super) fresh: Freshness,
    pub(super) at: Span,
}

fn freshness_of(init: Option<&Expression<'_>>) -> Freshness {
    match init.map(Expression::without_parentheses) {
        Some(Expression::ObjectExpression(_) | Expression::ArrayExpression(_)) => Freshness::Object,
        Some(Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)) => {
            Freshness::Function
        }
        _ => Freshness::Stale,
    }
}

pub(super) fn push_pattern<'a>(pattern: &BindingPattern<'a>, out: &mut Vec<Declared<'a>>) {
    stack::maybe_grow(|| push_pattern_inner(pattern, out));
}

fn push_pattern_inner<'a>(pattern: &BindingPattern<'a>, out: &mut Vec<Declared<'a>>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => out.push(Declared {
            name: id.name.as_str(),
            fresh: Freshness::Stale,
            at: id.span,
        }),
        BindingPattern::ObjectPattern(node) => {
            for property in &node.properties {
                push_pattern(&property.value, out);
            }
            if let Some(rest) = &node.rest {
                push_pattern(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(node) => {
            for element in node.elements.iter().flatten() {
                push_pattern(element, out);
            }
            if let Some(rest) = &node.rest {
                push_pattern(&rest.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(node) => push_pattern(&node.left, out),
    }
}

fn push_declaration<'a>(decl: &VariableDeclaration<'a>, out: &mut Vec<Declared<'a>>) {
    for declarator in &decl.declarations {
        match &declarator.id {
            BindingPattern::BindingIdentifier(id) => out.push(Declared {
                name: id.name.as_str(),
                fresh: freshness_of(declarator.init.as_ref()),
                at: declarator.span,
            }),
            other => push_pattern(other, out),
        }
    }
}

struct DeclarationScan<'a> {
    out: Vec<Declared<'a>>,
    strict: bool,
    blocks: u32,
}

impl<'a> VisitJs<'a> for DeclarationScan<'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if let Some(id) = &it.id {
            if it.is_declaration() && !(self.strict && self.blocks > 0) {
                self.out.push(Declared {
                    name: id.name.as_str(),
                    fresh: Freshness::Function,
                    at: it.span,
                });
            }
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _it: &Class<'a>) {}

    fn visit_expression(&mut self, _it: &Expression<'a>) {}

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            push_declaration(it, &mut self.out);
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.blocks += 1;
        walk_js::walk_block_statement(self, it);
        self.blocks -= 1;
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.blocks += 1;
        walk_js::walk_switch_statement(self, it);
        self.blocks -= 1;
    }
}

fn push_lexical<'a>(statement: &Statement<'a>, strict: bool, out: &mut Vec<Declared<'a>>) {
    match statement {
        Statement::VariableDeclaration(decl) if decl.kind != VariableDeclarationKind::Var => {
            push_declaration(decl, out);
        }
        Statement::ClassDeclaration(class) => {
            if let Some(id) = &class.id {
                out.push(Declared {
                    name: id.name.as_str(),
                    fresh: Freshness::Stale,
                    at: class.span,
                });
            }
        }
        Statement::FunctionDeclaration(function) if strict => {
            if let Some(id) = &function.id {
                out.push(Declared {
                    name: id.name.as_str(),
                    fresh: Freshness::Function,
                    at: function.span,
                });
            }
        }
        _ => {}
    }
}

pub(super) fn declared_in_block<'a>(
    statements: &[Statement<'a>],
    strict: bool,
) -> Vec<Declared<'a>> {
    let mut out = Vec::new();
    for statement in statements {
        push_lexical(statement, strict, &mut out);
    }
    out
}

pub(super) fn declared_in_switch<'a>(it: &SwitchStatement<'a>, strict: bool) -> Vec<Declared<'a>> {
    let mut out = Vec::new();
    for case in &it.cases {
        for statement in &case.consequent {
            push_lexical(statement, strict, &mut out);
        }
    }
    out
}

pub(super) fn declared_in_for_init<'a>(init: Option<&ForStatementInit<'a>>) -> Vec<Declared<'a>> {
    let mut out = Vec::new();
    if let Some(ForStatementInit::VariableDeclaration(decl)) = init {
        if decl.kind != VariableDeclarationKind::Var {
            push_declaration(decl, &mut out);
        }
    }
    out
}

pub(super) fn declared_in_for_head<'a>(left: &ForStatementLeft<'a>) -> Vec<Declared<'a>> {
    let mut out = Vec::new();
    if let ForStatementLeft::VariableDeclaration(decl) = left {
        if decl.kind != VariableDeclarationKind::Var {
            push_declaration(decl, &mut out);
        }
    }
    out
}

pub(super) fn declared_in_arrow<'a>(
    it: &ArrowFunctionExpression<'a>,
    strict: bool,
) -> Vec<Declared<'a>> {
    match &it.body {
        ArrowFunctionBody::FunctionBody(body) => {
            declared_in_body(&it.params, &body.statements, strict)
        }
        _ => declared_in_body(&it.params, &[], strict),
    }
}

pub(super) fn declared_in_body<'a>(
    params: &FormalParameters<'a>,
    statements: &[Statement<'a>],
    strict: bool,
) -> Vec<Declared<'a>> {
    let mut scan = DeclarationScan {
        out: Vec::new(),
        strict,
        blocks: 0,
    };
    for param in &params.items {
        push_pattern(&param.pattern, &mut scan.out);
    }
    if let Some(rest) = &params.rest {
        push_pattern(&rest.rest.argument, &mut scan.out);
    }
    for statement in statements {
        if let Statement::VariableDeclaration(decl) = statement {
            if decl.kind != VariableDeclarationKind::Var {
                push_declaration(decl, &mut scan.out);
            }
        }
        scan.visit_statement(statement);
    }
    scan.out
}

pub(super) fn own_expression_name<'a>(function: &Function<'a>) -> Option<Declared<'a>> {
    if function.is_declaration() {
        return None;
    }
    function.id.as_ref().map(|id| Declared {
        name: id.name.as_str(),
        fresh: Freshness::Stale,
        at: function.span(),
    })
}
