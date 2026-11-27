use oxc_allocator::{Allocator, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span, SPAN};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::super::resolution::{Expected, Spans};
use super::build::{Build, Lifted, Plan};

fn index_statements<'k, 'a>(
    statement: &'k Statement<'a>,
    map: &mut FxHashMap<Span, &'k Statement<'a>>,
) {
    map.insert(statement.span(), statement);
    match statement {
        Statement::IfStatement(it) => {
            index_statements(&it.consequent, map);
            if let Some(alternate) = &it.alternate {
                index_statements(alternate, map);
            }
        }
        Statement::BlockStatement(block) => {
            for inner in &block.body {
                index_statements(inner, map);
            }
        }
        _ => {}
    }
}

pub(super) struct Installer<'b, 'a> {
    pub(super) builder: &'b AstBuilder<'a>,
    pub(super) alloc: &'a Allocator,
    pub(super) scoping: &'b Scoping,
    pub(super) spans: &'b Spans,
    pub(super) plans: &'b [Plan],
    pub(super) strings: &'b [u16],
    pub(super) helper_statements: &'b [Span],
    pub(super) remove_helpers: bool,
    pub(super) lifted: Lifted,
    pub(super) expected: Expected,
    pub(super) skip: &'b [SymbolId],
    pub(super) names_observed: bool,
    pub(super) failed: bool,
    pub(super) applied: usize,
    pub(super) plain: Option<&'b FxHashSet<Span>>,
}

impl<'a> Installer<'_, 'a> {
    fn install(&mut self, statements: &mut AVec<'a, Statement<'a>>, plan: &Plan) {
        let Some(at) = statements
            .iter()
            .position(|s| s.span() == plan.main_statement)
        else {
            self.failed = true;
            return;
        };
        let taken = statements.remove(at);
        let Statement::FunctionDeclaration(function) = &taken else {
            self.failed = true;
            return;
        };
        let mut index = FxHashMap::default();
        if let Some(body) = &function.body {
            for statement in &body.statements {
                if let Statement::WhileStatement(looped) = statement {
                    collect_cases(&looped.body, &mut index);
                }
            }
        }
        let mut lifted = std::mem::take(&mut self.lifted);
        let mut build = Build {
            builder: self.builder,
            alloc: self.alloc,
            scoping: self.scoping,
            spans: self.spans,
            plan,
            index,
            strings: self.strings,
            lifted: &mut lifted,
            names_observed: self.names_observed,
            failed: false,
            expected: &mut self.expected,
            skip: self.skip.to_vec(),
        };
        let built = build.emit(&plan.out, None, false);
        let mut replacement = build.finish(built, &plan.roots);
        let failed = build.failed;
        drop(build);
        self.lifted = lifted;
        if failed {
            self.failed = true;
            return;
        }
        let Some(first) = statements
            .iter()
            .position(|s| plan.entry.contains(&s.span()))
        else {
            self.failed = true;
            return;
        };
        let mut removed = 0;
        while first < statements.len() && plan.entry.contains(&statements[first].span()) {
            statements.remove(first);
            removed += 1;
        }
        if removed != plan.entry.len() {
            self.failed = true;
            return;
        }
        for (offset, statement) in replacement.drain(..).enumerate() {
            statements.insert(first + offset, statement);
        }
        self.applied += 1;
    }
}

fn collect_cases<'k, 'a>(body: &'k Statement<'a>, index: &mut FxHashMap<Span, &'k Statement<'a>>) {
    let inner = match body {
        Statement::BlockStatement(block) => match block.body.as_slice() {
            [only] => only,
            _ => return,
        },
        other => other,
    };
    let switch = match inner {
        Statement::SwitchStatement(switch) => switch,
        Statement::LabeledStatement(labeled) => match &labeled.body {
            Statement::SwitchStatement(switch) => switch,
            _ => return,
        },
        _ => return,
    };
    for case in &switch.cases {
        for statement in &case.consequent {
            index_statements(statement, index);
        }
    }
}

impl<'a> Installer<'_, 'a> {
    fn plain_params(&mut self, function: &mut Function<'a>) {
        let Some(rest) = function.params.rest.as_ref() else {
            return;
        };
        let BindingPattern::BindingIdentifier(id) = &rest.rest.argument else {
            return;
        };
        let rest_name = id.name.to_string();
        let mut names: Vec<String> = Vec::new();
        if let Some(body) = function.body.as_mut() {
            let found = body.statements.iter().position(|statement| {
                let Statement::ExpressionStatement(expression) = statement else {
                    return false;
                };
                let Expression::AssignmentExpression(assign) = &expression.expression else {
                    return false;
                };
                let AssignmentTarget::ArrayAssignmentTarget(target) = &assign.left else {
                    return false;
                };
                let simple = target.rest.is_none()
                    && target.elements.iter().all(|e| {
                        matches!(e, Some(AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(_)))
                    });
                simple && matches!(&assign.right, Expression::Identifier(r) if r.name == rest_name.as_str())
            });
            if let Some(at) = found {
                if let Statement::ExpressionStatement(expression) = &body.statements[at] {
                    if let Expression::AssignmentExpression(assign) = &expression.expression {
                        if let AssignmentTarget::ArrayAssignmentTarget(target) = &assign.left {
                            names = target
                                .elements
                                .iter()
                                .filter_map(|e| match e {
                                    Some(
                                        AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(t),
                                    ) => Some(t.name.to_string()),
                                    _ => None,
                                })
                                .collect();
                        }
                    }
                }
                body.statements.remove(at);
                for statement in body.statements.iter_mut().take(at) {
                    if let Statement::VariableDeclaration(declaration) = statement {
                        declaration.declarations.retain(|d| {
                            !matches!(&d.id, BindingPattern::BindingIdentifier(b) if names.iter().any(|n| n == b.name.as_str()))
                        });
                    }
                }
                body.statements.retain(|s| !matches!(s, Statement::VariableDeclaration(d) if d.declarations.is_empty()));
            }
        }
        function.params.rest = None;
        for name in names {
            let pattern = BindingPattern::new_binding_identifier(
                SPAN,
                oxc_str::Ident::from_str_in(&name, self.builder),
                self.builder,
            );
            let decorators: AVec<'a, Decorator<'a>> = AVec::new_in(self.builder);
            function.params.items.push(FormalParameter::new(
                SPAN,
                decorators,
                pattern,
                None,
                None,
                false,
                None,
                false,
                false,
                self.builder,
            ));
        }
    }
}

impl<'a> VisitMut<'a> for Installer<'_, 'a> {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        walk_mut::walk_function(self, it, flags);
        if self.plain.is_some_and(|spans| spans.contains(&it.span)) {
            self.plain_params(it);
        }
    }

    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_mut::walk_program(self, it);
        if self.plain.is_some() {
            return;
        }
        let plans = self.plans;
        if let Some(plan) = plans.iter().find(|p| p.body == it.span) {
            self.install(&mut it.body, plan);
        }
        if self.remove_helpers {
            let before = it.body.len();
            let helpers = self.helper_statements;
            it.body.retain(|s| !helpers.contains(&s.span()));
            self.failed |= before - it.body.len() != helpers.len();
        }
    }

    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        walk_mut::walk_function_body(self, it);
        if self.plain.is_some() {
            return;
        }
        let plans = self.plans;
        if let Some(plan) = plans.iter().find(|p| p.body == it.span) {
            self.install(&mut it.statements, plan);
        }
    }
}
