use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;

use super::identify;

const GLOBAL_OBJECTS: [&str; 7] = [
    "globalThis",
    "window",
    "self",
    "global",
    "top",
    "parent",
    "frames",
];

const GLOBAL_DEFINERS: [(&str, &str); 5] = [
    ("Object", "defineProperty"),
    ("Object", "defineProperties"),
    ("Object", "assign"),
    ("Reflect", "defineProperty"),
    ("Reflect", "set"),
];

#[derive(Default)]
pub struct Trust {
    settled: FxHashSet<SymbolId>,
    global_names: FxHashSet<String>,
    global_dynamic: bool,
}

impl Trust {
    pub fn settled(&self, symbol: SymbolId) -> bool {
        self.settled.contains(&symbol)
    }

    pub fn global_kept(&self, symbol: SymbolId, module: bool, scoping: &Scoping) -> bool {
        if module || scoping.symbol_scope_id(symbol) != scoping.root_scope_id() {
            return true;
        }
        let flags = scoping.symbol_flags(symbol);
        if !flags.intersects(SymbolFlags::FunctionScopedVariable | SymbolFlags::Function) {
            return true;
        }
        !self.global_dynamic && !self.global_names.contains(scoping.symbol_name(symbol))
    }
}

pub fn gather(program: &Program<'_>, scoping: &Scoping, module: bool) -> Trust {
    let strict = module || program.has_use_strict_directive();
    let mut gather = Gather {
        scoping,
        module,
        trust: Trust::default(),
        strict: vec![strict],
        this_global: vec![!module],
    };
    gather.settle(&program.body);
    walk_js::walk_program(&mut gather, program);
    gather.trust
}

struct Gather<'s> {
    scoping: &'s Scoping,
    module: bool,
    trust: Trust,
    strict: Vec<bool>,
    this_global: Vec<bool>,
}

fn is_global_name(id: &IdentifierReference<'_>, scoping: &Scoping) -> bool {
    GLOBAL_OBJECTS.contains(&id.name.as_str())
        && id
            .reference_id
            .get()
            .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

fn is_unresolved(id: &IdentifierReference<'_>, scoping: &Scoping, name: &str) -> bool {
    id.name.as_str() == name
        && id
            .reference_id
            .get()
            .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

fn literal_name<'k>(expression: &'k Expression<'_>) -> Option<&'k str> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked)
            .and_then(|cooked| cooked.as_str()),
        _ => None,
    }
}

fn inert(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::Void | UnaryOperator::Typeof | UnaryOperator::LogicalNot
            ) && inert(&unary.argument)
        }
        Expression::LogicalExpression(logical) => inert(&logical.left) && inert(&logical.right),
        Expression::StaticMemberExpression(member) => is_native_path(&member.object),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if property.kind == PropertyKind::Init
                    && !property.computed
                    && inert(&property.value))
        }),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.is_elision() || element.as_expression().is_some_and(inert)),
        _ => false,
    }
}

fn is_native_path(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(_) | Expression::ThisExpression(_) => true,
        Expression::StaticMemberExpression(member) => is_native_path(&member.object),
        _ => false,
    }
}

fn is_exports(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(id) => id.name.as_str() == "exports",
        Expression::StaticMemberExpression(member) => {
            member.property.name.as_str() == "exports"
                && matches!(member.object.without_parentheses(),
                    Expression::Identifier(id) if id.name.as_str() == "module")
        }
        _ => false,
    }
}

fn exports_boilerplate(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::AssignmentExpression(assignment) => {
            assignment.operator == AssignmentOperator::Assign
                && matches!(&assignment.left, AssignmentTarget::StaticMemberExpression(member)
                    if is_exports(&member.object))
                && (inert(&assignment.right) || exports_boilerplate(&assignment.right))
        }
        Expression::CallExpression(call) => {
            matches!(call.callee.without_parentheses(), Expression::StaticMemberExpression(member)
                if member.property.name.as_str() == "defineProperty"
                    && matches!(member.object.without_parentheses(),
                        Expression::Identifier(id) if id.name.as_str() == "Object"))
                && call.arguments.len() == 3
                && call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_some_and(is_exports)
                && call
                    .arguments
                    .iter()
                    .skip(1)
                    .all(|argument| argument.as_expression().is_some_and(inert))
        }
        _ => false,
    }
}

impl Gather<'_> {
    fn settle(&mut self, statements: &[Statement<'_>]) {
        for statement in statements {
            if !self.settle_statement(statement) {
                return;
            }
        }
    }

    fn settle_statement(&mut self, statement: &Statement<'_>) -> bool {
        match statement {
            Statement::VariableDeclaration(declaration) => self.settle_declaration(declaration),
            Statement::FunctionDeclaration(_)
            | Statement::EmptyStatement(_)
            | Statement::ImportDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportNamedDeclaration(_) => true,
            Statement::ExpressionStatement(statement) => {
                inert(&statement.expression) || exports_boilerplate(&statement.expression)
            }
            _ => false,
        }
    }

    fn settle_declaration(&mut self, declaration: &VariableDeclaration<'_>) -> bool {
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                if let Some(symbol) = id.symbol_id.get() {
                    self.trust.settled.insert(symbol);
                }
            }
            let quiet = declarator.init.as_ref().is_none_or(inert)
                || identify::declarator(declarator, self.scoping, self.module).is_some();
            if !quiet {
                return false;
            }
        }
        true
    }

    fn is_global_object(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(id) => is_global_name(id, self.scoping),
            Expression::ThisExpression(_) => self.this_global.last().copied().unwrap_or(false),
            Expression::StaticMemberExpression(member) => {
                GLOBAL_OBJECTS.contains(&member.property.name.as_str())
                    && self.is_global_object(&member.object)
            }
            _ => false,
        }
    }

    fn written(&mut self, key: Option<&str>) {
        match key {
            Some(name) => {
                self.trust.global_names.insert(name.to_string());
            }
            None => self.trust.global_dynamic = true,
        }
    }

    fn definer_on_global(&mut self, call: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return;
        };
        let Expression::Identifier(owner) = member.object.without_parentheses() else {
            return;
        };
        let name = member.property.name.as_str();
        let definer = GLOBAL_DEFINERS
            .iter()
            .any(|(root, method)| *method == name && is_unresolved(owner, self.scoping, root));
        let target = call.arguments.first().and_then(Argument::as_expression);
        if !definer || !target.is_some_and(|target| self.is_global_object(target)) {
            return;
        }
        let keyed = matches!(name, "defineProperty" | "set");
        let key = call
            .arguments
            .get(1)
            .and_then(Argument::as_expression)
            .and_then(literal_name)
            .filter(|_| keyed);
        self.written(key);
    }

    fn enter_scope(&mut self, strict: bool, this_global: bool) {
        self.strict.push(strict);
        self.this_global.push(this_global);
    }

    fn leave_scope(&mut self) {
        self.strict.pop();
        self.this_global.pop();
    }
}

impl<'a> VisitJs<'a> for Gather<'_> {
    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.settle(&it.statements);
        walk_js::walk_function_body(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let strict = self.strict.last().copied().unwrap_or(false) || it.has_use_strict_directive();
        self.enter_scope(strict, !strict);
        walk_js::walk_function(self, it, flags);
        self.leave_scope();
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.enter_scope(true, false);
        walk_js::walk_class(self, it);
        self.leave_scope();
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.definer_on_global(it);
        walk_js::walk_call_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        match it {
            SimpleAssignmentTarget::StaticMemberExpression(member)
                if self.is_global_object(&member.object) =>
            {
                self.written(Some(member.property.name.as_str()));
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member)
                if self.is_global_object(&member.object) =>
            {
                self.written(literal_name(&member.expression));
            }
            _ => {}
        }
        walk_js::walk_simple_assignment_target(self, it);
    }
}
