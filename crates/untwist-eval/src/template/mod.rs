use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentTarget, AssignmentTargetPattern,
    BindingIdentifier, BindingPattern, ComputedMemberExpression, Expression, Function,
    SimpleAssignmentTarget, Statement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::scope::ScopeFlags;
use untwist_core::ast_util::matcher::infinite_loop_body;

use crate::stack;

mod dead;
mod terms;

pub use dead::dead_tail_provable;

pub(crate) fn identifier_name<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    match expr {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

type MemberCall<'a> = (&'a Expression<'a>, &'a str, &'a [Argument<'a>]);

fn member_call<'a>(expr: &'a Expression<'a>) -> Option<MemberCall<'a>> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    if call.optional {
        return None;
    }
    let member = call.callee.as_member_expression()?;
    if member.optional() {
        return None;
    }
    Some((
        member.object(),
        member
            .static_property_name()
            .and_then(|name| name.as_str())?,
        call.arguments.as_slice(),
    ))
}

fn rotates(stmt: &Statement<'_>, array: &str) -> bool {
    let stmt = match stmt {
        Statement::BlockStatement(block) if block.body.len() == 1 => &block.body[0],
        other => other,
    };
    let Statement::ExpressionStatement(expression) = stmt else {
        return false;
    };
    let Some((object, "push", [argument])) = member_call(&expression.expression) else {
        return false;
    };
    let inner = argument.as_expression().and_then(member_call);
    identifier_name(object) == Some(array)
        && matches!(inner, Some((o, "shift", [])) if identifier_name(o) == Some(array))
}

fn is_unlabelled_break(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::BreakStatement(b) => b.label.is_none(),
        Statement::BlockStatement(b) => b.body.len() == 1 && is_unlabelled_break(&b.body[0]),
        _ => false,
    }
}

fn infinite_body<'a>(stmt: &'a Statement<'a>) -> Option<&'a Statement<'a>> {
    if let Statement::ForStatement(node) = stmt {
        if node.init.is_some() || node.update.is_some() {
            return None;
        }
    }
    infinite_loop_body(stmt)
}

fn array_local<'a>(prelude: &'a [Statement<'a>], array_param: &str) -> Option<&'a str> {
    prelude.iter().find_map(|stmt| {
        let Statement::VariableDeclaration(declaration) = stmt else {
            return None;
        };
        declaration.declarations.iter().find_map(|d| {
            let BindingPattern::BindingIdentifier(id) = &d.id else {
                return None;
            };
            let Some(Expression::CallExpression(call)) = &d.init else {
                return None;
            };
            let calls_array = identifier_name(&call.callee) == Some(array_param);
            (calls_array && call.arguments.is_empty()).then_some(id.name.as_str())
        })
    })
}

fn checksum_test<'a>(
    stmts: &'a [Statement<'a>],
    array: &str,
    expected: &str,
) -> Option<&'a Expression<'a>> {
    let [Statement::VariableDeclaration(declaration), Statement::IfStatement(test)] = stmts else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    let BindingPattern::BindingIdentifier(accumulator) = &declarator.id else {
        return None;
    };
    let Expression::BinaryExpression(compare) = &test.test else {
        return None;
    };
    let shaped = compare.operator == BinaryOperator::StrictEquality
        && identifier_name(&compare.left) == Some(accumulator.name.as_str())
        && identifier_name(&compare.right) == Some(expected)
        && is_unlabelled_break(&test.consequent)
        && test
            .alternate
            .as_ref()
            .is_some_and(|alternate| rotates(alternate, array));
    shaped.then_some(declarator.init.as_ref()).flatten()
}

struct Rebinds<'n> {
    array: &'n str,
    expected: &'n str,
    array_bindings: usize,
    nested: usize,
    found: bool,
}

impl<'a> VisitJs<'a> for Rebinds<'_> {
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
        self.nested += 1;
        walk_js::walk_function(self, it, flags);
        self.nested -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nested += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.nested -= 1;
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.nested > 0 {
            return;
        }
        let name = it.name.as_str();
        if name == self.array {
            self.array_bindings += 1;
        }
        self.found |= name == self.expected;
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            let name = id.name.as_str();
            self.found |= name == self.array || name == self.expected;
        }
        walk_js::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_pattern(&mut self, it: &AssignmentTargetPattern<'a>) {
        self.found = true;
        walk_js::walk_assignment_target_pattern(self, it);
    }
}

#[derive(Default)]
struct IndexRead {
    found: bool,
}

impl<'a> VisitJs<'a> for IndexRead {
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

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_computed_member_expression(&mut self, _it: &ComputedMemberExpression<'a>) {
        self.found = true;
    }
}

fn prelude_indexes(prelude: &[Statement<'_>]) -> bool {
    let mut scan = IndexRead::default();
    for statement in prelude {
        scan.visit_statement(statement);
    }
    scan.found
}

fn names_stay_bound(statements: &[Statement<'_>], array: &str, expected: &str) -> bool {
    let mut scan = Rebinds {
        array,
        expected,
        array_bindings: 0,
        nested: 0,
        found: false,
    };
    for statement in statements {
        scan.visit_statement(statement);
    }
    !scan.found && scan.array_bindings == 1
}

pub fn template_exact<'a>(
    statements: &'a [Statement<'a>],
    array_param: &str,
    expected: &str,
) -> bool {
    let Some((last, prelude)) = statements.split_last() else {
        return false;
    };
    let declarations = prelude.iter().all(|s| {
        matches!(
            s,
            Statement::VariableDeclaration(_) | Statement::FunctionDeclaration(_)
        )
    });
    let (Some(array), Some(Statement::BlockStatement(block))) =
        (array_local(prelude, array_param), infinite_body(last))
    else {
        return false;
    };
    let [Statement::TryStatement(attempt)] = block.body.as_slice() else {
        return false;
    };
    let Some(handler) = &attempt.handler else {
        return false;
    };
    let handler_rotates = matches!(handler.body.body.as_slice(), [only] if rotates(only, array));
    let pure = || {
        checksum_test(&attempt.block.body, array, expected)
            .is_some_and(|checksum| terms::checksum_is_pure(statements, checksum))
    };
    declarations
        && !prelude_indexes(prelude)
        && attempt.finalizer.is_none()
        && handler_rotates
        && names_stay_bound(statements, array, expected)
        && pure()
}
