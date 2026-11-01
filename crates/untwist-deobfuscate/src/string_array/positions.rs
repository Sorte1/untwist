use super::*;

struct Positions<'t> {
    targets: &'t [Span],
    found: FxHashSet<Span>,
}

impl Positions<'_> {
    fn covers(&self, span: Span) -> bool {
        let at = self
            .targets
            .partition_point(|target| target.start < span.start);
        self.targets
            .get(at)
            .is_some_and(|target| target.end <= span.end)
    }

    fn consumes(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::ParenthesizedExpression(inner) => self.consumes(&inner.expression),
            Expression::SequenceExpression(sequence) => {
                if let Some(last) = sequence.expressions.last() {
                    self.consumes(last);
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.consumes(&conditional.consequent);
                self.consumes(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) => {
                self.consumes(&logical.left);
                self.consumes(&logical.right);
            }
            other => {
                let span = other.span();
                let known = self
                    .targets
                    .binary_search_by_key(&(span.start, span.end), |target| {
                        (target.start, target.end)
                    })
                    .is_ok();
                if known {
                    self.found.insert(span);
                }
            }
        }
    }
}

impl<'a> VisitJs<'a> for Positions<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.covers(it.span()) {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.covers(it.span()) {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.consumes(&it.callee);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.consumes(&it.callee);
        walk_js::walk_new_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.consumes(&it.tag);
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        self.consumes(it.object());
        walk_js::walk_member_expression(self, it);
    }

    fn visit_spread_element(&mut self, it: &SpreadElement<'a>) {
        self.consumes(&it.argument);
        walk_js::walk_spread_element(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.consumes(&it.right);
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        if let (true, Some(argument)) = (it.delegate, it.argument.as_ref()) {
            self.consumes(argument);
        }
        walk_js::walk_yield_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let pattern = !matches!(it.id, BindingPattern::BindingIdentifier(_));
        if let (true, Some(init)) = (pattern, it.init.as_ref()) {
            self.consumes(init);
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if matches!(
            it.left,
            AssignmentTarget::ArrayAssignmentTarget(_)
                | AssignmentTarget::ObjectAssignmentTarget(_)
        ) {
            self.consumes(&it.right);
        }
        walk_js::walk_assignment_expression(self, it);
    }
}

pub(super) fn reads_not_values(program: &Program<'_>, reads: &[Span]) -> FxHashSet<Span> {
    if reads.is_empty() {
        return FxHashSet::default();
    }
    let mut targets = reads.to_vec();
    targets.sort_unstable_by_key(|span| (span.start, span.end));
    let mut positions = Positions {
        targets: &targets,
        found: FxHashSet::default(),
    };
    positions.visit_program(program);
    positions.found
}
