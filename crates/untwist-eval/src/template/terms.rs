use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentTarget, BindingIdentifier, BindingPattern,
    CallExpression, Expression, Function, MemberExpression, SimpleAssignmentTarget, Statement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::stack;

#[derive(Default)]
struct LocalNames<'a> {
    names: FxHashSet<&'a str>,
    bindings: FxHashMap<&'a str, usize>,
}

#[derive(Default)]
struct Written<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Written<'a> {
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

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.names.insert(id.name.as_str());
        }
        walk_js::walk_simple_assignment_target(self, it);
    }
}

impl<'a> VisitJs<'a> for LocalNames<'a> {
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

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.names.insert(id.name.as_str());
        }
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.names.insert(it.name.as_str());
        *self.bindings.entry(it.name.as_str()).or_default() += 1;
    }
}

pub(super) struct Context<'a> {
    pub(super) locals: FxHashSet<&'a str>,
    pub(super) aliases: FxHashSet<&'a str>,
    pub(super) wrappers: FxHashMap<&'a str, &'a Function<'a>>,
    pub(super) primitives: FxHashSet<&'a str>,
}

struct Terms<'s, 'a> {
    locals: &'s FxHashSet<&'a str>,
    aliases: &'s FxHashSet<&'a str>,
    wrappers: &'s FxHashMap<&'a str, &'a Function<'a>>,
    operands: &'s FxHashSet<&'a str>,
}

pub(super) fn wrapper_body<'a>(function: &'a Function<'a>) -> Option<&'a Expression<'a>> {
    let simple = function.params.rest.is_none()
        && function.params.items.iter().all(|param| {
            param.initializer.is_none()
                && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
        });
    if !simple || function.generator || function.r#async {
        return None;
    }
    match function.body.as_ref()?.statements.as_slice() {
        [Statement::ReturnStatement(ret)] => ret.argument.as_ref(),
        _ => None,
    }
}

pub(super) fn wrapper_params<'a>(function: &'a Function<'a>) -> FxHashSet<&'a str> {
    function
        .params
        .items
        .iter()
        .filter_map(|param| match &param.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        })
        .collect()
}

impl<'a> Terms<'_, 'a> {
    fn call(&self, call: &'a CallExpression<'a>, nested: bool) -> bool {
        let Expression::Identifier(callee) = &call.callee else {
            return false;
        };
        let arguments = call.arguments.iter().all(|argument| match argument {
            Argument::SpreadElement(_) => false,
            other => self.pure(other.to_expression(), nested),
        });
        let name = callee.name.as_str();
        if !arguments || call.optional {
            return false;
        }
        match self.wrappers.get(name) {
            Some(function) if !nested => self.wrapper(function),
            _ => self.aliases.contains(name) || !self.locals.contains(name),
        }
    }

    fn wrapper(&self, function: &'a Function<'a>) -> bool {
        let Some(body) = wrapper_body(function) else {
            return false;
        };
        let params = wrapper_params(function);
        let mut locals = self.locals.clone();
        locals.extend(params.iter().copied());
        let mut operands = self.operands.clone();
        operands.extend(params.iter().copied());
        let aliases: FxHashSet<&str> = self
            .aliases
            .iter()
            .copied()
            .filter(|alias| !params.contains(alias))
            .collect();
        let inner = Terms {
            locals: &locals,
            aliases: &aliases,
            wrappers: self.wrappers,
            operands: &operands,
        };
        inner.pure(body, true)
    }

    fn member(&self, member: &'a MemberExpression<'a>, nested: bool) -> bool {
        match member {
            MemberExpression::ComputedMemberExpression(node) => {
                self.pure(&node.object, nested) && self.pure(&node.expression, nested)
            }
            MemberExpression::StaticMemberExpression(node) => self.pure(&node.object, nested),
            MemberExpression::PrivateFieldExpression(_) => false,
        }
    }

    fn pure(&self, expr: &'a Expression<'a>, nested: bool) -> bool {
        match expr {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => true,
            Expression::Identifier(id) => self.operands.contains(id.name.as_str()),
            Expression::ParenthesizedExpression(node) => self.pure(&node.expression, nested),
            Expression::UnaryExpression(node) => {
                node.operator != UnaryOperator::Delete && self.pure(&node.argument, nested)
            }
            Expression::BinaryExpression(node) => {
                !matches!(
                    node.operator,
                    BinaryOperator::In | BinaryOperator::Instanceof
                ) && self.pure(&node.left, nested)
                    && self.pure(&node.right, nested)
            }
            Expression::LogicalExpression(node) => {
                self.pure(&node.left, nested) && self.pure(&node.right, nested)
            }
            Expression::ConditionalExpression(node) => {
                self.pure(&node.test, nested)
                    && self.pure(&node.consequent, nested)
                    && self.pure(&node.alternate, nested)
            }
            Expression::CallExpression(call) => self.call(call, nested),
            other => other
                .as_member_expression()
                .is_some_and(|member| self.member(member, nested)),
        }
    }
}

fn declared_with<'a>(
    statement: &'a Statement<'a>,
    accept: impl Fn(&'a Expression<'a>) -> Option<&'a str>,
) -> Vec<(&'a str, &'a str)> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return Vec::new();
    };
    declaration
        .declarations
        .iter()
        .filter_map(|declarator| {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            Some((id.name.as_str(), accept(declarator.init.as_ref()?)?))
        })
        .collect()
}

fn alias_target<'a>(init: &'a Expression<'a>) -> Option<&'a str> {
    match init {
        Expression::Identifier(target) => Some(target.name.as_str()),
        _ => None,
    }
}

fn primitive_literal<'a>(init: &'a Expression<'a>) -> Option<&'a str> {
    matches!(
        init,
        Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
    .then_some("")
}

pub(super) fn context<'a>(statements: &'a [Statement<'a>]) -> Context<'a> {
    let mut locals = LocalNames::default();
    let mut written = Written::default();
    let mut wrappers = FxHashMap::default();
    let mut aliases = Vec::new();
    let mut primitives = Vec::new();
    for statement in statements {
        if let Statement::FunctionDeclaration(function) = statement {
            if let Some(id) = &function.id {
                wrappers.insert(id.name.as_str(), function.as_ref());
            }
        }
        aliases.extend(declared_with(statement, alias_target));
        primitives.extend(declared_with(statement, primitive_literal));
        walk_js::walk_statement(&mut locals, statement);
        written.visit_statement(statement);
    }
    let unwritten =
        |name: &&str| locals.bindings.get(name) == Some(&1) && !written.names.contains(name);
    let aliases = aliases
        .into_iter()
        .filter(|(alias, target)| unwritten(alias) && !locals.names.contains(target))
        .map(|(alias, _)| alias)
        .collect();
    let primitives = primitives
        .into_iter()
        .map(|(name, _)| name)
        .filter(unwritten)
        .collect();
    Context {
        locals: locals.names,
        aliases,
        wrappers,
        primitives,
    }
}

pub(super) fn checksum_is_pure<'a>(
    statements: &'a [Statement<'a>],
    checksum: &'a Expression<'a>,
) -> bool {
    let context = context(statements);
    let terms = Terms {
        locals: &context.locals,
        aliases: &context.aliases,
        wrappers: &context.wrappers,
        operands: &context.primitives,
    };
    terms.pure(checksum, false)
}
