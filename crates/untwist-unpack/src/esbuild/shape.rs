use oxc_ast::ast::*;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallForm {
    Direct,
    FirstKey,
}

pub enum Inner<'k, 'a> {
    Expr(&'k Expression<'a>),
    Block(&'k [Statement<'a>]),
}

pub fn simple_params<'a>(params: &FormalParameters<'a>) -> Option<Vec<&'a str>> {
    if params.rest.is_some() {
        return None;
    }
    params
        .items
        .iter()
        .map(|item| match (&item.pattern, &item.initializer) {
            (BindingPattern::BindingIdentifier(id), None) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

pub fn is_name(expr: &Expression<'_>, name: &str) -> bool {
    matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == name)
}

pub fn arrow_inner<'k, 'a>(arrow: &'k ArrowFunctionExpression<'a>) -> Option<Inner<'k, 'a>> {
    match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => Some(Inner::Block(&body.statements)),
        other => other.as_expression().map(Inner::Expr),
    }
}

pub fn lazy_helper<'k, 'a>(init: &'k Expression<'a>) -> Option<(Vec<&'a str>, Inner<'k, 'a>)> {
    let Expression::ArrowFunctionExpression(outer) = init.without_parentheses() else {
        return None;
    };
    if outer.r#async {
        return None;
    }
    let params = simple_params(&outer.params)?;
    if !(2..=3).contains(&params.len()) {
        return None;
    }
    let Some(Inner::Expr(value)) = arrow_inner(outer) else {
        return None;
    };
    let inner = match value.without_parentheses() {
        Expression::ArrowFunctionExpression(arrow)
            if !arrow.r#async && arrow.params.items.is_empty() && arrow.params.rest.is_none() =>
        {
            arrow_inner(arrow)?
        }
        Expression::FunctionExpression(function)
            if !function.r#async
                && !function.generator
                && function.params.items.is_empty()
                && function.params.rest.is_none() =>
        {
            Inner::Block(&function.body.as_deref()?.statements)
        }
        _ => return None,
    };
    Some((params, inner))
}

struct InnerBindings<'a> {
    function: Option<&'a str>,
    caught: Option<&'a str>,
}

fn inner_bindings<'a>(init: &Expression<'a>) -> InnerBindings<'a> {
    let mut names = InnerBindings {
        function: None,
        caught: None,
    };
    let Expression::ArrowFunctionExpression(outer) = init.without_parentheses() else {
        return names;
    };
    let Some(Inner::Expr(value)) = arrow_inner(outer) else {
        return names;
    };
    let statements = match value.without_parentheses() {
        Expression::FunctionExpression(function) => {
            names.function = function.id.as_ref().map(|id| id.name.as_str());
            function.body.as_deref().map(|body| &body.statements[..])
        }
        Expression::ArrowFunctionExpression(arrow) => match arrow_inner(arrow) {
            Some(Inner::Block(statements)) => Some(statements),
            _ => None,
        },
        _ => None,
    };
    if let Some([Statement::TryStatement(attempt)]) = statements {
        let caught = attempt
            .handler
            .as_deref()
            .and_then(|handler| handler.param.as_ref());
        if let Some(BindingPattern::BindingIdentifier(error)) = caught.map(|param| &param.pattern) {
            names.caught = Some(error.name.as_str());
        }
    }
    names
}

pub fn helper_shadows(init: &Expression<'_>, params: &[&str], getter: Option<&str>) -> bool {
    let implicit = params
        .iter()
        .any(|name| matches!(*name, "eval" | "arguments"));
    let getter_param = getter.is_some_and(|getter| params.contains(&getter));
    let read: Vec<&str> = params.iter().copied().chain(getter).collect();
    let inner = inner_bindings(init);
    let named = inner.function.is_some_and(|name| read.contains(&name));
    let module = params.get(1).copied();
    let caught = inner.caught.is_some_and(|name| Some(name) == module);
    implicit || getter_param || named || caught
}

pub fn export_helper(init: &Expression<'_>) -> bool {
    let Expression::ArrowFunctionExpression(arrow) = init.without_parentheses() else {
        return false;
    };
    if simple_params(&arrow.params).is_none_or(|params| params.len() != 2) {
        return false;
    }
    matches!(
        arrow_inner(arrow),
        Some(Inner::Block([Statement::ForInStatement(_)]))
    )
}

struct Names<'n> {
    cb: &'n str,
    module: &'n str,
}

pub fn commonjs_call<'a>(
    params: &[&str],
    inner: &Inner<'_, 'a>,
) -> Option<(CallForm, Option<&'a str>)> {
    let [cb, module] = params else {
        return None;
    };
    let names = Names { cb, module };
    match inner {
        Inner::Expr(expr) => names.result(expr),
        Inner::Block([Statement::TryStatement(attempt)]) => names.guarded(attempt),
        Inner::Block(statements) => names.core(statements),
    }
}

impl Names<'_> {
    fn guarded<'a>(&self, attempt: &TryStatement<'a>) -> Option<(CallForm, Option<&'a str>)> {
        if attempt.finalizer.is_some() {
            return None;
        }
        let handler = attempt.handler.as_deref()?;
        let BindingPattern::BindingIdentifier(error) = &handler.param.as_ref()?.pattern else {
            return None;
        };
        if !self.rethrows(&handler.body.body, error.name.as_str()) {
            return None;
        }
        self.core(&attempt.block.body)
    }

    fn rethrows(&self, body: &[Statement<'_>], error: &str) -> bool {
        match body {
            [Statement::ThrowStatement(throw)] => {
                let Expression::SequenceExpression(sequence) = throw.argument.without_parentheses()
                else {
                    return false;
                };
                matches!(&sequence.expressions[..], [reset, again]
                    if self.resets(reset) && is_name(again, error))
            }
            [Statement::ExpressionStatement(reset), Statement::ThrowStatement(throw)] => {
                self.resets(&reset.expression) && is_name(&throw.argument, error)
            }
            _ => false,
        }
    }

    fn resets(&self, expr: &Expression<'_>) -> bool {
        let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
            return false;
        };
        assign.operator == AssignmentOperator::Assign
            && self.targets_module(&assign.left)
            && matches!(assign.right.without_parentheses(),
                Expression::NumericLiteral(zero) if zero.value == 0.0)
    }

    fn targets_module(&self, target: &AssignmentTarget<'_>) -> bool {
        matches!(target, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == self.module)
    }

    fn core<'a>(&self, statements: &[Statement<'a>]) -> Option<(CallForm, Option<&'a str>)> {
        match statements {
            [Statement::ReturnStatement(ret)] => self.result(ret.argument.as_ref()?),
            [first, Statement::ReturnStatement(ret)] => {
                if !self.is_exports(ret.argument.as_ref()?) {
                    return None;
                }
                self.guarded_call(first)
            }
            _ => None,
        }
    }

    fn guarded_call<'a>(&self, statement: &Statement<'a>) -> Option<(CallForm, Option<&'a str>)> {
        match statement {
            Statement::ExpressionStatement(stmt) => self.or_call(&stmt.expression),
            Statement::IfStatement(branch) if branch.alternate.is_none() => {
                let Expression::UnaryExpression(not) = branch.test.without_parentheses() else {
                    return None;
                };
                if not.operator != UnaryOperator::LogicalNot || !is_name(&not.argument, self.module)
                {
                    return None;
                }
                let call = match &branch.consequent {
                    Statement::BlockStatement(block) => match &block.body[..] {
                        [Statement::ExpressionStatement(stmt)] => &stmt.expression,
                        _ => return None,
                    },
                    Statement::ExpressionStatement(stmt) => &stmt.expression,
                    _ => return None,
                };
                self.call(call)
            }
            _ => None,
        }
    }

    fn result<'a>(&self, expr: &Expression<'a>) -> Option<(CallForm, Option<&'a str>)> {
        let Expression::SequenceExpression(sequence) = expr.without_parentheses() else {
            return None;
        };
        let [first, last] = &sequence.expressions[..] else {
            return None;
        };
        if !self.is_exports(last) {
            return None;
        }
        self.or_call(first)
    }

    fn or_call<'a>(&self, expr: &Expression<'a>) -> Option<(CallForm, Option<&'a str>)> {
        let Expression::LogicalExpression(or) = expr.without_parentheses() else {
            return None;
        };
        if or.operator != LogicalOperator::Or || !is_name(&or.left, self.module) {
            return None;
        }
        self.call(&or.right)
    }

    fn is_exports(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
            if !member.optional && member.property.name == "exports"
                && is_name(&member.object, self.module))
    }

    fn call<'a>(&self, expr: &Expression<'a>) -> Option<(CallForm, Option<&'a str>)> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        if call.optional {
            return None;
        }
        let [Argument::StaticMemberExpression(exports), module] = &call.arguments[..] else {
            return None;
        };
        let fresh = !exports.optional
            && exports.property.name == "exports"
            && self.fresh_module(&exports.object)
            && module
                .as_expression()
                .is_some_and(|m| is_name(m, self.module));
        if !fresh {
            return None;
        }
        self.callee(&call.callee)
    }

    fn fresh_module(&self, expr: &Expression<'_>) -> bool {
        let Expression::AssignmentExpression(assign) = expr.without_parentheses() else {
            return false;
        };
        if assign.operator != AssignmentOperator::Assign || !self.targets_module(&assign.left) {
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
            && untwist_core::ast_util::prop::prop_name(&property.key).as_deref() == Some("exports");
        keyed
            && matches!(property.value.without_parentheses(),
                Expression::ObjectExpression(empty) if empty.properties.is_empty())
    }

    fn callee<'a>(&self, callee: &Expression<'a>) -> Option<(CallForm, Option<&'a str>)> {
        let callee = callee.without_parentheses();
        if is_name(callee, self.cb) {
            return Some((CallForm::Direct, None));
        }
        let Expression::SequenceExpression(sequence) = callee else {
            return None;
        };
        let [Expression::NumericLiteral(zero), Expression::ComputedMemberExpression(member)] =
            &sequence.expressions[..]
        else {
            return None;
        };
        if zero.value != 0.0 || member.optional || !is_name(&member.object, self.cb) {
            return None;
        }
        self.first_key(&member.expression)
            .map(|names| (CallForm::FirstKey, Some(names)))
    }

    fn first_key<'a>(&self, expr: &Expression<'a>) -> Option<&'a str> {
        let Expression::ComputedMemberExpression(index) = expr.without_parentheses() else {
            return None;
        };
        let first =
            matches!(&index.expression, Expression::NumericLiteral(zero) if zero.value == 0.0);
        let Expression::CallExpression(call) = index.object.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(names) = call.callee.without_parentheses() else {
            return None;
        };
        let applied =
            matches!(&call.arguments[..], [Argument::Identifier(cb)] if cb.name == self.cb);
        (first && !index.optional && !call.optional && applied).then_some(names.name.as_str())
    }
}

fn is_typeof_require_defined(test: &Expression<'_>) -> bool {
    let Expression::BinaryExpression(binary) = test.without_parentheses() else {
        return false;
    };
    let Expression::UnaryExpression(unary) = binary.left.without_parentheses() else {
        return false;
    };
    if unary.operator != UnaryOperator::Typeof || !is_name(&unary.argument, "require") {
        return false;
    }
    let Expression::StringLiteral(text) = binary.right.without_parentheses() else {
        return false;
    };
    match binary.operator {
        BinaryOperator::StrictInequality | BinaryOperator::Inequality => text.value == "undefined",
        BinaryOperator::LessThan => text.value == "u",
        _ => false,
    }
}

fn host_or(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::ConditionalExpression(choice)
        if is_typeof_require_defined(&choice.test) && is_name(&choice.consequent, "require"))
}

pub fn is_require_shim(init: &Expression<'_>) -> bool {
    match init.without_parentheses() {
        Expression::CallExpression(call) if !call.optional && call.arguments.len() == 1 => {
            let Expression::ArrowFunctionExpression(arrow) = call.callee.without_parentheses()
            else {
                return false;
            };
            simple_params(&arrow.params).is_some_and(|params| params.len() == 1)
                && matches!(arrow_inner(arrow), Some(Inner::Expr(value)) if host_or(value))
        }
        other => host_or(other),
    }
}

pub fn is_object_member_chain(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StaticMemberExpression(member) if !member.optional => {
            is_name(&member.object, "Object") || is_object_member_chain(&member.object)
        }
        _ => false,
    }
}

pub fn is_object_get_own_property_names(expr: &Expression<'_>) -> bool {
    matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
        if !member.optional && member.property.name == "getOwnPropertyNames"
            && is_name(&member.object, "Object"))
}
