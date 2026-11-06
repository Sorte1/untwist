use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, AssignmentTargetPattern, BindingPattern, Expression,
    Function, SimpleAssignmentTarget, Statement, VariableDeclarator,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::stack;

#[derive(Default)]
pub(super) struct Helpers<'a> {
    pub(super) bindings: FxHashMap<&'a str, Span>,
    pub(super) installed: FxHashMap<&'a str, Vec<Span>>,
    pub(super) poisoned: FxHashSet<&'a str>,
}

enum Installed<'a> {
    Function(Span),
    Named(&'a str),
    Other,
}

struct Collector<'s, 'a> {
    decoder: &'s str,
    writes: FxHashMap<&'a str, u32>,
    candidates: Vec<(&'a str, Span)>,
    members: Vec<(&'a str, Installed<'a>)>,
    patterns: bool,
}

impl<'a> Collector<'_, 'a> {
    fn written(&mut self, name: &'a str) {
        *self.writes.entry(name).or_default() += 1;
    }

    fn member_write(&mut self, node: &AssignmentExpression<'a>) {
        let Some(member) = node.left.as_member_expression() else {
            return;
        };
        let on_decoder =
            matches!(member.object(), Expression::Identifier(id) if id.name == self.decoder);
        let Some(key) = member
            .static_property_name()
            .and_then(|name| name.as_str())
            .filter(|_| on_decoder)
        else {
            return;
        };
        let value = match (&node.right, node.operator) {
            (Expression::FunctionExpression(function), AssignmentOperator::Assign) => {
                Installed::Function(function.span)
            }
            (Expression::Identifier(id), AssignmentOperator::Assign) => {
                Installed::Named(id.name.as_str())
            }
            _ => Installed::Other,
        };
        self.members.push((key, value));
    }
}

impl<'a> VisitJs<'a> for Collector<'_, 'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let (true, Some(id)) = (it.is_declaration(), &it.id) {
            self.written(id.name.as_str());
            self.candidates.push((id.name.as_str(), it.span));
        }
        walk_js::walk_function(self, it, flags);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            self.written(id.name.as_str());
            if let Expression::FunctionExpression(function) = init {
                self.candidates.push((id.name.as_str(), function.span()));
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.member_write(it);
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.written(id.name.as_str());
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_pattern(&mut self, it: &AssignmentTargetPattern<'a>) {
        self.patterns = true;
        walk_js::walk_assignment_target_pattern(self, it);
    }
}

pub(super) fn helpers<'a>(function: &Function<'a>, decoder: &str) -> Helpers<'a> {
    let mut collector = Collector {
        decoder,
        writes: FxHashMap::default(),
        candidates: Vec::new(),
        members: Vec::new(),
        patterns: false,
    };
    collector.visit_function(function, ScopeFlags::Function);
    if collector.patterns {
        return Helpers::default();
    }
    let mut helpers = Helpers::default();
    for (name, span) in &collector.candidates {
        if collector.writes.get(name) == Some(&1) {
            helpers.bindings.insert(name, *span);
        }
    }
    for (key, value) in collector.members {
        let span = match value {
            Installed::Function(span) => Some(span),
            Installed::Named(name) => helpers.bindings.get(name).copied(),
            Installed::Other => None,
        };
        match span {
            Some(span) => helpers.installed.entry(key).or_default().push(span),
            None => {
                helpers.poisoned.insert(key);
            }
        }
    }
    helpers
}
