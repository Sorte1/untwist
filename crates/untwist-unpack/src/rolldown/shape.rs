use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, LogicalOperator, UnaryOperator};
use untwist_core::ast_util::prop::prop_name;

use crate::esbuild::shape::{arrow_inner, is_name, Inner};
use rustc_hash::FxHashSet;

const REGION: &str = "#region";
const RUNTIME_MARKERS: [&str; 2] = ["rolldown/runtime.js", "rolldown:runtime"];

struct Names<'n> {
    cb: &'n str,
    module: &'n str,
}

pub fn commonjs_min(params: &[&str], inner: &Inner<'_, '_>) -> bool {
    let [cb, module] = params else {
        return false;
    };
    let names = Names { cb, module };
    match inner {
        Inner::Expr(expr) => names.result(expr),
        Inner::Block([Statement::IfStatement(branch), Statement::ReturnStatement(ret)]) => {
            names.guarded(branch)
                && ret
                    .argument
                    .as_ref()
                    .is_some_and(|value| names.is_exports(value))
        }
        Inner::Block(_) => false,
    }
}

impl Names<'_> {
    fn guarded(&self, branch: &IfStatement<'_>) -> bool {
        let unset = matches!(branch.test.without_parentheses(), Expression::UnaryExpression(not)
            if not.operator == UnaryOperator::LogicalNot && is_name(&not.argument, self.module));
        let body = match &branch.consequent {
            Statement::BlockStatement(block) => &block.body[..],
            other => std::slice::from_ref(other),
        };
        let run = match body {
            [Statement::ExpressionStatement(only)] => self.run(&only.expression),
            [Statement::ExpressionStatement(call), Statement::ExpressionStatement(release)] => {
                self.call(&call.expression) && self.releases(&release.expression)
            }
            _ => false,
        };
        unset && branch.alternate.is_none() && run
    }

    fn result(&self, expr: &Expression<'_>) -> bool {
        let Expression::SequenceExpression(sequence) = expr.without_parentheses() else {
            return false;
        };
        let [first, last] = &sequence.expressions[..] else {
            return false;
        };
        self.is_exports(last) && self.or_run(first)
    }

    fn or_run(&self, expr: &Expression<'_>) -> bool {
        let Expression::LogicalExpression(or) = expr.without_parentheses() else {
            return false;
        };
        or.operator == LogicalOperator::Or && is_name(&or.left, self.module) && self.run(&or.right)
    }

    fn run(&self, expr: &Expression<'_>) -> bool {
        match expr.without_parentheses() {
            Expression::SequenceExpression(sequence) => {
                matches!(&sequence.expressions[..], [call, release]
                    if self.call(call) && self.releases(release))
            }
            other => self.call(other),
        }
    }

    fn releases(&self, expr: &Expression<'_>) -> bool {
        let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
            return false;
        };
        assign.operator == AssignmentOperator::Assign
            && matches!(&assign.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == self.cb)
            && matches!(
                assign.right.without_parentheses(),
                Expression::NullLiteral(_)
            )
    }

    fn call(&self, expr: &Expression<'_>) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        if call.optional || !is_name(&call.callee, self.cb) {
            return false;
        }
        let (first, module, rest) = match &call.arguments[..] {
            [first, module] => (first, module, None),
            [first, module, rest] => (first, module, Some(rest)),
            _ => return false,
        };
        let fresh = matches!(first, Argument::StaticMemberExpression(exports)
            if !exports.optional && exports.property.name == "exports"
                && self.fresh_module(&exports.object));
        let module = module
            .as_expression()
            .is_some_and(|m| is_name(m, self.module));
        let rest = rest.is_none_or(|rest| rest.as_expression().is_some_and(|e| self.is_exports(e)));
        fresh && module && rest
    }

    fn is_exports(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
            if !member.optional && member.property.name == "exports"
                && is_name(&member.object, self.module))
    }

    fn fresh_module(&self, expr: &Expression<'_>) -> bool {
        let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
            return false;
        };
        let target = matches!(&assign.left,
            AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == self.module);
        if assign.operator != AssignmentOperator::Assign || !target {
            return false;
        }
        let Expression::ObjectExpression(object) = assign.right.without_parentheses() else {
            return false;
        };
        let [ObjectPropertyKind::ObjectProperty(property)] = &object.properties[..] else {
            return false;
        };
        let keyed = !property.computed
            && property.kind == PropertyKind::Init
            && prop_name(&property.key).as_deref() == Some("exports");
        keyed
            && matches!(property.value.without_parentheses(),
                Expression::ObjectExpression(empty) if empty.properties.is_empty())
    }
}

pub fn function_iife(statement: &Statement<'_>) -> bool {
    let Statement::ExpressionStatement(statement) = statement else {
        return false;
    };
    let Expression::CallExpression(call) = statement.expression.without_parentheses() else {
        return false;
    };
    if call.optional || !call.arguments.is_empty() {
        return false;
    }
    matches!(call.callee.without_parentheses(), Expression::FunctionExpression(function)
        if function.id.is_none() && !function.r#async && !function.generator
            && function.params.items.is_empty() && function.params.rest.is_none()
            && function.body.is_some())
}

fn creates_at_url(creators: &FxHashSet<&str>, expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return false;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    let at_url = matches!(&call.arguments[..], [Argument::StaticMemberExpression(url)]
        if !url.optional && url.property.name == "url"
            && matches!(&url.object, Expression::ImportMeta(_)));
    !call.optional && at_url && creators.contains(callee.name.as_str())
}

pub fn require_shim(creators: &FxHashSet<&str>, init: &Expression<'_>) -> bool {
    let init = init.without_parentheses();
    if creates_at_url(creators, init) {
        return true;
    }
    let Expression::CallExpression(call) = init else {
        return false;
    };
    let Expression::ArrowFunctionExpression(arrow) = call.callee.without_parentheses() else {
        return false;
    };
    let bare = arrow.params.items.is_empty() && arrow.params.rest.is_none() && !arrow.r#async;
    let value = match arrow_inner(arrow) {
        Some(Inner::Expr(value)) => Some(value),
        Some(Inner::Block([Statement::ReturnStatement(only)])) => only.argument.as_ref(),
        _ => None,
    };
    !call.optional
        && call.arguments.is_empty()
        && bare
        && value.is_some_and(|value| creates_at_url(creators, value))
}

pub fn region_path(text: &str) -> &str {
    text.strip_prefix(REGION)
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .map_or(text, str::trim)
}

pub fn runtime_marker(text: &str) -> bool {
    RUNTIME_MARKERS.iter().any(|marker| text.contains(marker))
}
