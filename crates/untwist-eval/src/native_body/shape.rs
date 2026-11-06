use std::fmt::Write;

use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, AssignmentTarget, BindingPattern,
    Expression, ForStatement, ForStatementInit, Function, IfStatement, SimpleAssignmentTarget,
    Statement, VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
};
use rustc_hash::FxHashMap;

use crate::coerce::number_to_string;
use crate::state::recipe::{fold_constant, Prim};

const MAX_SHAPE_BYTES: usize = 8192;

pub(crate) struct Shape<'a> {
    pub(crate) text: String,
    pub(crate) reads: Vec<Option<&'a Expression<'a>>>,
    pub(crate) names: Vec<&'a str>,
}

#[derive(Default)]
struct Writer<'a> {
    out: String,
    names: FxHashMap<&'a str, usize>,
    order: Vec<&'a str>,
    reads: Vec<Option<&'a Expression<'a>>>,
    constant: Vec<bool>,
    written: Vec<bool>,
}

fn plain_kind(kind: VariableDeclarationKind) -> bool {
    matches!(
        kind,
        VariableDeclarationKind::Var
            | VariableDeclarationKind::Let
            | VariableDeclarationKind::Const
    )
}

pub(crate) fn function_shape<'a>(function: &'a Function<'a>) -> Option<Shape<'a>> {
    let mut writer = Writer::default();
    writer.function(function)?;
    let rewrites_constant = writer
        .constant
        .iter()
        .zip(&writer.written)
        .any(|(constant, written)| *constant && *written);
    if rewrites_constant {
        return None;
    }
    Some(Shape {
        text: writer.out,
        reads: writer.reads,
        names: writer.order,
    })
}

impl<'a> Writer<'a> {
    fn push(&mut self, text: &str) -> Option<()> {
        self.out.push_str(text);
        (self.out.len() <= MAX_SHAPE_BYTES).then_some(())
    }

    fn index(&mut self, name: &'a str) -> usize {
        let next = self.names.len();
        let index = *self.names.entry(name).or_insert(next);
        if index == self.reads.len() {
            self.reads.push(None);
            self.order.push(name);
            self.constant.push(false);
            self.written.push(false);
        }
        index
    }

    fn binding(&mut self, name: &'a str) -> Option<()> {
        let index = self.index(name);
        self.push(&format!("v{index}"))
    }

    fn written(&mut self, name: &'a str) -> Option<()> {
        let index = self.index(name);
        self.written[index] = true;
        self.push(&format!("v{index}"))
    }

    fn declared(&mut self, name: &'a str, kind: VariableDeclarationKind) -> Option<()> {
        let index = self.index(name);
        if kind == VariableDeclarationKind::Const {
            self.constant[index] = true;
        }
        self.push(&format!("v{index}"))
    }

    fn read(&mut self, name: &'a str, node: &'a Expression<'a>) -> Option<()> {
        let index = self.index(name);
        self.reads[index].get_or_insert(node);
        self.push(&format!("v{index}"))
    }

    fn units(&mut self, units: impl Iterator<Item = u16>) -> Option<()> {
        let mut text = String::from("'");
        for unit in units {
            match char::from_u32(u32::from(unit)) {
                Some(c) if c.is_ascii_graphic() && c != '\'' && c != '\\' || c == ' ' => {
                    text.push(c)
                }
                _ => write!(text, "\\u{unit:04x}").ok()?,
            }
        }
        text.push('\'');
        self.push(&text)
    }

    fn number(&mut self, value: f64) -> Option<()> {
        if value == 0.0 && value.is_sign_negative() {
            return self.push("#-0");
        }
        self.push(&format!("#{}", number_to_string(value)))
    }

    fn function(&mut self, function: &'a Function<'a>) -> Option<()> {
        if function.r#async || function.generator || function.params.rest.is_some() {
            return None;
        }
        self.push("F")?;
        if let Some(id) = &function.id {
            self.binding(id.name.as_str())?;
        }
        self.push("(")?;
        for param in &function.params.items {
            if param.initializer.is_some() {
                return None;
            }
            let BindingPattern::BindingIdentifier(id) = &param.pattern else {
                return None;
            };
            self.binding(id.name.as_str())?;
            self.push(",")?;
        }
        self.push("){")?;
        let body = function.body.as_ref()?;
        if !body.directives.is_empty() {
            return None;
        }
        self.statements(&body.statements)?;
        self.push("}")
    }

    fn statements(&mut self, statements: &'a [Statement<'a>]) -> Option<()> {
        for statement in statements {
            match statement {
                Statement::VariableDeclaration(declaration) => self.split(declaration)?,
                other => {
                    self.statement(other)?;
                    self.push(";")?;
                }
            }
        }
        Some(())
    }

    fn split(&mut self, declaration: &'a VariableDeclaration<'a>) -> Option<()> {
        if !plain_kind(declaration.kind) {
            return None;
        }
        for declarator in &declaration.declarations {
            self.push("var(")?;
            self.declarator(declarator, declaration.kind)?;
            self.push(");")?;
        }
        Some(())
    }

    fn statement(&mut self, statement: &'a Statement<'a>) -> Option<()> {
        match statement {
            Statement::VariableDeclaration(declaration) => self.declaration(declaration),
            Statement::ExpressionStatement(node) => {
                self.push("e(")?;
                self.expression(&node.expression)?;
                self.push(")")
            }
            Statement::ReturnStatement(node) => {
                self.push("ret(")?;
                if let Some(argument) = &node.argument {
                    self.expression(argument)?;
                }
                self.push(")")
            }
            Statement::BlockStatement(block) => {
                self.push("{")?;
                self.statements(&block.body)?;
                self.push("}")
            }
            Statement::IfStatement(node) => self.branch(node),
            Statement::ForStatement(node) => self.for_statement(node),
            Statement::EmptyStatement(_) => self.push("empty"),
            _ => None,
        }
    }

    fn branch(&mut self, node: &'a IfStatement<'a>) -> Option<()> {
        self.push("if(")?;
        self.expression(&node.test)?;
        self.push(")")?;
        self.statement(&node.consequent)?;
        if let Some(alternate) = &node.alternate {
            self.push("else")?;
            self.statement(alternate)?;
        }
        Some(())
    }

    fn for_statement(&mut self, node: &'a ForStatement<'a>) -> Option<()> {
        self.push("for(")?;
        match &node.init {
            Some(ForStatementInit::VariableDeclaration(declaration)) => {
                self.declaration(declaration)?
            }
            Some(init) => self.expression(init.as_expression()?)?,
            None => {}
        }
        self.push(";")?;
        if let Some(test) = &node.test {
            self.expression(test)?;
        }
        self.push(";")?;
        if let Some(update) = &node.update {
            self.expression(update)?;
        }
        self.push(")")?;
        self.statement(&node.body)
    }

    fn declaration(&mut self, declaration: &'a VariableDeclaration<'a>) -> Option<()> {
        if !plain_kind(declaration.kind) {
            return None;
        }
        self.push("var(")?;
        for declarator in &declaration.declarations {
            self.declarator(declarator, declaration.kind)?;
        }
        self.push(")")
    }

    fn declarator(
        &mut self,
        declarator: &'a VariableDeclarator<'a>,
        kind: VariableDeclarationKind,
    ) -> Option<()> {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        self.declared(id.name.as_str(), kind)?;
        if let Some(init) = &declarator.init {
            self.push("=")?;
            self.expression(init)?;
        }
        self.push(",")
    }

    fn expression(&mut self, expression: &'a Expression<'a>) -> Option<()> {
        match fold_constant(expression, &[]) {
            Some(Prim::Num(value)) => return self.number(value),
            Some(Prim::Str(text)) => return self.units(text.units().iter().copied()),
            None => {}
        }
        match expression {
            Expression::Identifier(id) => self.read(id.name.as_str(), expression),
            Expression::ThisExpression(_) => self.push("this"),
            Expression::BooleanLiteral(node) => {
                self.push(if node.value { "true" } else { "false" })
            }
            Expression::NullLiteral(_) => self.push("null"),
            Expression::ParenthesizedExpression(node) => self.expression(&node.expression),
            Expression::ArrayExpression(node) => self.array(node),
            Expression::UnaryExpression(node) => {
                self.push(&format!("u{}(", node.operator.as_str()))?;
                self.expression(&node.argument)?;
                self.push(")")
            }
            Expression::UpdateExpression(node) => {
                let fix = if node.prefix { "pre" } else { "post" };
                self.push(&format!("{fix}{}(", node.operator.as_str()))?;
                self.simple_target(&node.argument)?;
                self.push(")")
            }
            _ => self.operator(expression),
        }
    }

    fn array(&mut self, node: &'a ArrayExpression<'a>) -> Option<()> {
        self.push("[")?;
        for element in &node.elements {
            match element {
                ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => {
                    return None
                }
                other => self.expression(other.as_expression()?)?,
            }
            self.push(",")?;
        }
        self.push("]")
    }

    fn pair(
        &mut self,
        tag: &str,
        left: &'a Expression<'a>,
        right: &'a Expression<'a>,
    ) -> Option<()> {
        self.push(tag)?;
        self.expression(left)?;
        self.push(",")?;
        self.expression(right)?;
        self.push(")")
    }

    fn operator(&mut self, expression: &'a Expression<'a>) -> Option<()> {
        match expression {
            Expression::BinaryExpression(node) => self.pair(
                &format!("b{}(", node.operator.as_str()),
                &node.left,
                &node.right,
            ),
            Expression::LogicalExpression(node) => self.pair(
                &format!("l{}(", node.operator.as_str()),
                &node.left,
                &node.right,
            ),
            Expression::AssignmentExpression(node) => {
                self.push(&format!("a{}(", node.operator.as_str()))?;
                self.target(&node.left)?;
                self.push(",")?;
                self.expression(&node.right)?;
                self.push(")")
            }
            Expression::ConditionalExpression(node) => {
                self.push("c(")?;
                self.expression(&node.test)?;
                self.push(",")?;
                self.pair("", &node.consequent, &node.alternate)
            }
            Expression::SequenceExpression(node) => {
                self.push("s(")?;
                for item in &node.expressions {
                    self.expression(item)?;
                    self.push(",")?;
                }
                self.push(")")
            }
            _ => self.access(expression),
        }
    }

    fn access(&mut self, expression: &'a Expression<'a>) -> Option<()> {
        match expression {
            Expression::CallExpression(node) if !node.optional => {
                self.push("call(")?;
                self.expression(&node.callee)?;
                self.arguments(&node.arguments)?;
                self.push(")")
            }
            Expression::NewExpression(node) => {
                self.push("new(")?;
                self.expression(&node.callee)?;
                self.arguments(&node.arguments)?;
                self.push(")")
            }
            Expression::StaticMemberExpression(node) if !node.optional => {
                self.member(&node.object, node.property.name.as_str().encode_utf16())
            }
            Expression::ComputedMemberExpression(node) if !node.optional => {
                self.computed(&node.object, &node.expression)
            }
            Expression::FunctionExpression(function) => self.function(function),
            _ => None,
        }
    }

    fn arguments(&mut self, arguments: &'a [Argument<'a>]) -> Option<()> {
        for argument in arguments {
            self.push(",")?;
            self.expression(argument.as_expression()?)?;
        }
        Some(())
    }

    fn member(&mut self, object: &'a Expression<'a>, key: impl Iterator<Item = u16>) -> Option<()> {
        self.push("m(")?;
        self.expression(object)?;
        self.push(").")?;
        self.units(key)
    }

    fn computed(&mut self, object: &'a Expression<'a>, key: &'a Expression<'a>) -> Option<()> {
        if let Some(Prim::Str(text)) = fold_constant(key, &[]) {
            return self.member(object, text.units().to_vec().into_iter());
        }
        self.push("i(")?;
        self.expression(object)?;
        self.push(",")?;
        self.expression(key)?;
        self.push(")")
    }

    fn target(&mut self, target: &'a AssignmentTarget<'a>) -> Option<()> {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => self.written(id.name.as_str()),
            AssignmentTarget::StaticMemberExpression(node) => {
                self.member(&node.object, node.property.name.as_str().encode_utf16())
            }
            AssignmentTarget::ComputedMemberExpression(node) => {
                self.computed(&node.object, &node.expression)
            }
            _ => None,
        }
    }

    fn simple_target(&mut self, target: &'a SimpleAssignmentTarget<'a>) -> Option<()> {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.written(id.name.as_str())
            }
            SimpleAssignmentTarget::StaticMemberExpression(node) => {
                self.member(&node.object, node.property.name.as_str().encode_utf16())
            }
            SimpleAssignmentTarget::ComputedMemberExpression(node) => {
                self.computed(&node.object, &node.expression)
            }
            _ => None,
        }
    }
}
